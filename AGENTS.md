# Development in humtr/codex

Maintain a small Termux compatibility layer for upstream Codex. These rules are
optimized for GPT-6.1 Sol: concise task context, direct implementation, judgment
about scope and effort, and verification that reaches the real product path.
The repository owns this development plane; it needs no external workflow skill.

## Collaboration principles

- Complete the user's requested outcome. Investigation, explanation, review and
  safety judgment serve that purpose.
- Within applicable higher-priority instructions and permission boundaries,
  respect the user's explicit goals, priorities and decisions. Do not turn a
  technical preference or recommendation into an obligation for the user.
- Distinguish facts, assumptions, recommendations and mandatory constraints.
  Departures from convention, discouraged settings and abstract risks alone do
  not establish a prohibition. Verify whether an uncertain constraint applies.
- Preserve the request's intent and required capability. Reduce uncertainty
  through investigation and bounded verification; do not remove functionality
  or substitute another task merely to make qualification easier.
- Explain concerns through concrete effects and evidence. Once the user
  understands and authorizes those effects, proceed unless an independently
  applicable constraint prevents the work.
- Update judgments when new evidence arrives. Prioritize factual accuracy and
  the user's outcome over consistency with an earlier assertion.
- When blocking or stopping work, explain the applicable constraint, the
  specific conflicting action and the work that can still proceed.

## Context and ownership

- SPEC.md owns product behavior, runtime/security boundaries and update/rollback.
- CURRENT.md records the operating baseline, remaining work and useful proof.
  Consult it when relevant; update stale state instead of appending a diary.
- README.md is the user entrypoint. Keep history in Git; ordinary changes need
  no extra plan, template, ledger or design hierarchy.
- AGENTS.md is the development-rule authority. After context compaction, recovery
  or session handoff, reread it before editing or reviewing; a summary or a prior
  read does not substitute. Inspect the current contract, diff, added/replaced
  paths, temporary-code removal conditions and unresolved findings. Existing code
  and earlier claims of safety are evidence to examine, not design authority.
- Check branch, HEAD and user changes. Read only the contracts/code needed to
  resolve the task; widen the investigation when evidence shows a shared defect.
- Change the product contract before implementing different public behavior.

## Design and implementation

- Before a substantive change, state the intended responsibility and data flow
  in one or two sentences: who produces the data, who owns each decision and
  which consumer uses the result. Find existing implementations and actual
  callers; decide what to retain, improve, merge, replace or delete. Existing
  file/class placement is not a requirement. Keep cohesive responsibilities
  together; do not split or combine code to meet file/line counts.
- Fix defects at the responsibility that causes them. Do not add a second route
  beside or above it to hide a symptom. Prefer one implementation per responsibility;
  remove replaced functions, branches, settings, tests and documentation in the
  same change, and update all affected callers. Before deletion, establish why
  the old path exists and preserve its required behavior. Retained compatibility
  needs an actual consumer; temporary code needs an explicit removal condition.
- Judge additions by effect, not names. Helpers, wrappers, adapters, bridges,
  shims, hooks, flags and runners are justified by a distinct responsibility or
  real system boundary, not by renaming duplicate work. Remove pass-through
  layers, immediately wrapped/unwrapped values and avoidable round-trip
  conversions. Separate pure calculation from I/O and lifecycle coordination
  where it clarifies ownership, without inventing forwarding-only layers.
- Keep each rule, constant and definition at one authoritative source; name
  units and explain non-obvious origins. Derive values instead of copying them.
  Account/workspace identifiers, installation paths and tunnel/connector choices
  belong in configuration or verified runtime discovery, not account-specific
  source branches. Stable protocol constants may remain code. Follow suitable
  existing interfaces, ordering, errors and persistence conventions; an improved
  convention updates its producers, consumers, tests and documentation together.
- Add only fields, files, options and abstractions required by current behavior.
  Identify each new datum's meaning, actual consumer and decision or operation.
  Future extensibility or traceability alone does not justify unused data.
- Report failures with cause-specific reasons. Do not turn failure into success
  through silent fallback, swallowed errors, weaker assertions or unexplained
  clamps, sleeps, longer timeouts or retries. Defensive code needs a reachable
  input/state/failure path and a concrete protected contract. Remove redundant
  checks within the same trust boundary; keep necessary revalidation across
  external input, concurrency, time, process/device or authority boundaries.
  Low probability is not structural impossibility.
- Distinguish behavior-preserving refactoring from semantic changes, including
  defaults, numerical methods, units, tolerances and failure behavior. Preserve
  original evidence when meaning changes. Compare reuse, improvement and
  replacement by correctness, cost, simplicity and verification burden. Measure
  performance claims on the same inputs; structural complexity and runtime cost
  are different questions. Use actual GPT Pro advice when requested and available,
  then verify locally; do not substitute a model label or claim an unperformed review.
- Keep scope focused. Record material unrelated debt in the existing work context
  rather than mixing its cleanup into the change. Necessary removal of the
  replaced path is part of the change, including safety code and tests.

## Work

- Turn the request into an observable outcome and finish its authorized scope.
  Resolve routine choices directly; clarify only genuinely missing information.
- First establish the full requested capability with the permissions needed to
  exercise it; then reduce steps, execution paths and structure using real proof.
  Do not preemptively narrow the tool surface to make qualification convenient.
- Authorization persists across turns. Reuse verified destination ownership and
  session approval; resolve review objections with narrow evidence instead of
  restarting permission discussions. Required platform controls still apply.
- Adjust investigation and reasoning to uncertainty and consequences. Simple
  edits should stay simple; difficult architecture or diagnosis merits depth.
  No mandatory maximum effort, fixed leadership role or checkpoint cadence.
- Own actual diff review and validation. Use collaborators only when the user
  authorizes them; current preference is direct execution. No concurrent edits
  in a shared worktree and no recurring checkpoint or planner layer.
- Use TMPDIR for temporary work/checkouts; use XDG cache for compiler output.
  Never put temporary projects or installed binaries under prj.
- At each completed development/qualification batch, reclaim generated outputs.
  Preserve source, diffs, original results/native records and immutable current/
  rollback artifacts before pruning. Use scripts/clean_development.py to archive
  manifest-matched payloads of inactive fixtures; keep its removal/restore record.
  Never prune a live/unknown work tree or infer ownership from age/name alone.
- Keep the active reusable compiler cache around15GiB or less. After a completed
  batch, if it exceeds that budget, retain any executables needed by the next
  armed proof outside the mutable target first, then prune idle debug outputs.
  Prefer stripped debug information and disabled incremental output for repeated
  disposable qualification builds; record those build settings. Full symbols
  remain available when an actual debugging task needs them. Do not rerun accepted
  development merely to replace archived executable copies.

## Explicit development experiments

- Distinguish technically discouraged configurations from work that must be
  refused. In a user-owned and user-controlled environment, do not categorically
  reject an explicitly authorized development experiment solely because tool
  annotations differ from actual behavior or may affect approval UI. Explain
  the concern and expected effects, then verify within the authorized scope
  while respecting applicable higher-priority instructions and platform controls.
- Preserve requested read/write development capability. Do not substitute a
  read-only or synthetic trial; use an isolated disposable environment.
- Reuse existing authorization for follow-up work. Do not repeat the same
  concern or seek approval again for an already authorized action.
- A refusal requires an applicable higher-priority constraint or verified
  permission control and a specific conflicting action. Do not infer an
  unverified prohibition and declare the work unsupported. Continue unaffected
  work.

## Verification and publication

- Passing checks is evidence, not completion. Verify the requested producer-to-
  consumer behavior on its actual product path. Distinguish source/build success,
  simulated integration, actual Android/native/app execution, signed public
  delivery, device activation and physical UI observation. Do not infer process
  exit, no transmission or device readiness from a request, mock reply or journal
  field alone; use the observation at the boundary that owns that fact.
- Select scripts/check.sh docs, python, rust, provider or full from the changed behavior.
  Use meaningful affected-path proof; low-impact reversible edits need no made-up
  tests. Runtime/state/security/installer changes and release candidates need
  full acceptance, including applicable Android execution proof.
- Keep tests focused on public contracts with independently established expected
  results. Identify the fault, injection boundary and observable violation; ensure
  doubles do not fix the defect or bypass the actual producer/consumer path.
  Delete or merge tests/guards that repeat an already-covered fact or freeze
  incidental code shape/call counts. Preserve required failure detection, including
  safety behavior; never obtain a pass by skip/xfail or weaker acceptance criteria.
  For a substantive new behavioral regression, briefly introduce the targeted
  fault in disposable source, confirm the test fails and restore it. A mutation
  check must not modify live state; documentation-only changes need none.
- Reuse green evidence only when its actual inputs/environment/scope still apply.
  Failed or zero-test required gates remain unfinished. Do not repeat passing
  checks without a change or unresolved concern.
- Use the existing applicable static checks, then review the final diff and
  affected production flow, including relevant unchanged code. Resolve findings
  as fixes or reasoned rejections; record material findings and their dispositions
  in the PR or CURRENT.md. For substantive work, seek independent read-only review when
  authorized and available; its scope includes contracts, failure propagation,
  duplication and test necessity. Iron Laws and GPT Pro are not installed gates
  here: use them when explicitly requested/provisioned, distinguish their results
  from local checks, and state an unavailable or unperformed review honestly.
- Hashes and separate receipts need an actual consumer that must distinguish
  changed inputs or untrusted provenance. Prefer Git, existing version/input
  records and already-validated data when they satisfy that need. Do not add
  per-step receipts, duplicate guards or repeated hashing merely for a claim of
  integrity. Preserve SPEC-required signing, exact-source admission and validation
  across real authority boundaries. Reading historical records does not itself
  require generating that format for new work.
- rewrite/rust-core owns implementation and reusable release logic; main is a
  caller pinned to an accepted exact source. Keep independent histories separate.
  Preserve published histories and use the non-forced stable promotion path.
- Public assets are immutable. Deployment requires build/smoke, signing, complete
  public readback, disposable update proof and non-forced stable promotion.
  Device activation is separate; report physical observations separately.

## Reporting and handoff

- Scale the report to the change, using the PR, CURRENT.md or final response
  rather than a new mandatory checklist file. State the responsibility/data flow,
  what was added and removed, affected callers, reasons for retained indirection
  or compatibility, temporary-code removal conditions, semantic changes, actual
  verification and remaining boundaries. Explain deleted/merged test coverage and
  mutation results when applicable; link material review findings and dispositions.
- Handoffs name this file and its reread requirement, plus the current diff/work
  context and unresolved findings. Newly added paths do not become legitimate
  architecture simply by surviving a session. Rereading is not an instruction
  to rerun accepted checks with unchanged inputs and contracts.
- Keep version/session diaries out of code comments; explain current intent and
  constraints instead. Commit one logical change at a time, explaining why and
  what obsolete behavior/path was removed when relevant.

## Protected state

Preserve user changes, credentials, profiles, conversations and running jobs.
Do not remove local user files as an incidental test or cleanup step. Inspect
and preserve unique content before an authorized removal; routine cleanup covers
only verified owned generated artifacts.
Use disposable roots for tests. Do not edit live launcher/runtime/state except
for a user-authorized bounded device action; never expose credential values or
private session contents, and never modify a system resolver file. Ordinary Core
launch must work when optional Manager or update discovery is unavailable.
