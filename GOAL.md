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

### Historical Proof

- The predecessor's behavior and history remain available only as sealed
  legacy evidence. They are not promoted into proof for the rewrite.

### Not Proven

- No Rust Core source, build, test, artifact, installation, update, activation,
  rollback, fresh-device behavior, or production readiness is proven yet.
- No Manager implementation or Core/Manager integration is proven.
- No primary-Lead implementation bundle, worker run, or integration verification
  has yet been completed; the configured policy is workflow readiness, not
  product proof.

### Checkpoint Plans

- Plan status: lead-owned and pending; the primary Lead must author the initial
  Milestone 1 implementation bundle directly.
- Before the first implementation mutation, the Lead records the exact bundle,
  worker scope, validation, protected surfaces, and completion gate in
  `WORKBOARD.md`, then invokes one bounded worker.
- At bundle completion the Lead records its own diff/test disposition here. No
  worker report, planning subagent, or checkpoint reviewer substitutes for that
  integration decision.

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
