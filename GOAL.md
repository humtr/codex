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

## Checkpoint Planning Policy

- Planning assistance enabled: true
- Planner role: one reusable read-only Sol Technical Lead per milestone,
  combining bounded checkpoint planning with material implementation-problem
  consultation
- Planner model: `gpt-5.6-sol`
- Reasoning effort: `max`
- Planner context: `fork_context: false`
- Reuse rule: create one Technical Lead at milestone start, reuse that identity
  with delta-only `send_input` throughout the milestone, and do not create
  parallel planners or advisors
- Checkpoints: initial milestone bundle; completion of the current bundle;
  material deviation, blocker, or authority change; and milestone transition
- Routine cadence: no Technical Lead call for an individual edit, test, commit,
  evidence append, or other action inside an accepted bundle
- Planner wait window: 300 seconds for one bounded result, with at most one
  retry for a confirmed transient invocation failure
- Implementation authority: the primary implementing agent performs routine
  microplanning, validates every proposal against `SPEC.md` and this file,
  owns all mutation and evidence, and records the executable bundle in
  `WORKBOARD.md`
- Policy source and timestamp: user decision on 2026-08-28
- Review policy: no additional planning agents or checkpoint reviewers;
  independent product review only after the Milestone 2 acceptance candidate
  is complete

## Sol Technical Lead Context Policy

- Assistance enabled: true
- Scope: bounded checkpoint planning plus a material implementation problem
  that remains unresolved after focused local diagnosis or prevents confident
  selection of the next action
- Access mode: packet-only; the Technical Lead may not call repository,
  filesystem, shell, web, MCP, delegation, or other tools, enumerate files, or
  scan the repository
- Initial packet budget: at most 12,000 estimated input tokens or 48,000
  characters without a tokenizer, containing the bound branch/commit, one exact
  checkpoint or problem, governing contract excerpts, no more than eight
  relevant source/test snippets or diffs, concise evidence and attempts,
  protected surfaces, and one required decision
- Follow-up budget: delta-only `send_input` of at most 5,000 estimated input
  tokens or 20,000 characters, containing prior/current commits, changed paths,
  relevant new diff/evidence, and changed authority only
- Missing-evidence rule: the Technical Lead may request one exact additional
  snippet or evidence item; the implementing agent retrieves it, and no
  independent Technical Lead read is allowed
- Output budget: request no more than 900 words
- Rotation rule: replace the Technical Lead only at a milestone boundary,
  confirmed identity loss, or failure to restate the bound commit and governing
  authority; do not persist its live identity in tracked files
- Authority: read-only plans and advice only; the implementing agent validates
  the result and retains all mutation, contract, implementation, review, and
  acceptance decisions
- Record rule: persist only a concise checkpoint or problem, evidence, proposal,
  disposition, bound commit, packet-size class, evidence-request count, and next
  gate when the interaction materially changes this goal or its acceptance
  ledger
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
- Keep execution and routine microplanning with the primary implementing agent.
- Use one reusable packet-only `gpt-5.6-sol` Technical Lead at `max` reasoning
  per milestone for bounded work-bundle planning and material unresolved
  problems, with delta-only follow-ups and hard context budgets.
- Do not create additional planning agents or checkpoint reviewers. The
  Technical Lead is not a repository reader, implementation owner, mutation
  authority, or acceptance reviewer; review the complete Milestone 2 candidate
  separately.
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
- No Technical Lead plan or implementation-problem consultation has yet been
  completed; the configured policy is workflow readiness, not product proof.

### Checkpoint Plans

- Plan status: enabled; the initial Milestone 1 Technical Lead plan is pending.
- Before the first implementation mutation, the implementing agent sends the
  bounded initial packet, validates the returned proposal, and records the first
  executable bundle in `WORKBOARD.md`.
- Reuse the same Milestone 1 Technical Lead at the configured checkpoints. Do
  not invoke it during routine work inside the accepted bundle and do not add a
  checkpoint reviewer.

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
- Stop if no accepted work bundle remains and the required Technical Lead plan
  is unavailable after the bounded retry/resume policy. Record the unavailable
  plan and exact dependent decision rather than improvising a new bundle.
- Exhausting the current `WORKBOARD.md` bundle is a planning checkpoint, not a
  blocker. If the milestone gate is incomplete, obtain and record the next
  bounded bundle from the same Technical Lead.

Resume by reading `SPEC.md`, then this file, then `WORKBOARD.md`. Continue only
the selected current milestone. When its gate is proven, update this ledger,
rotate the Technical Lead at the milestone boundary, replace `WORKBOARD.md`
with the next milestone plan, and continue without a routine user pause.

## Handoff

Resume through the installed `$goal-md` workflow with
`/goal resume codex-goal.md`; it must resolve to this file on
`rewrite/rust-core`. If no current Technical Lead plan exists, the next
implementer first obtains the bounded Milestone 1 plan, validates it, records
the executable bundle in `WORKBOARD.md`, and then implements it. The legacy
branch may be inspected for behavior discovery but no source file may be copied
into the rewrite.
