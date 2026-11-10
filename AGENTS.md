# Agent working principles

This file governs **how an agent works** in this repository. It is not a
product specification, a deployment manual, a model-specific prompt, an active
task list or a record of completed work. The same principles apply to every
selected model and entrypoint.

## Serve the user's actual objective

- Respect the user's intent, priorities, scope and informed decisions within
  applicable permissions and higher-priority constraints. Do not substitute a
  smaller, easier task or an unrequested architecture for the desired outcome.
- Distinguish a mandatory boundary from a convention, a recommendation and an
  unverified concern. Investigate uncertain constraints before treating them as
  prohibitions. Explain any genuine block and continue unaffected work.
- Make reasonable implementation choices directly. Ask only for information or
  authority that is genuinely missing; do not seek repeated approval for the
  same already-authorized operation.
- Revise conclusions when evidence contradicts them. Do not defend a previous
  suggestion merely because it was written earlier.

## Understand before editing

- Check the actual checkout, branch, HEAD, user changes, current work ownership
  and the relevant existing implementation before changing files.
- Read [SPEC.md](SPEC.md) and its linked normative contracts for product behavior;
  read [CURRENT.md](CURRENT.md) for what is actually running, proven and unfinished.
- Read [docs/DOCUMENTATION.md](docs/DOCUMENTATION.md) for authority, mandatory
  reading order, bounded writing and legacy archival; read
  [docs/TERMUX_CONSTRAINTS.md](docs/TERMUX_CONSTRAINTS.md) before using any
  Termux socket, temporary root, Core/Provider binding or live TUI.
  Use [PRO_HANDOFF.md](PRO_HANDOFF.md) only for the current handoff/work order.
  A release record, prior implementation or passing test does not override the
  product contract. [README.md](README.md) is the user-facing guide.
- After compaction, recovery or handoff, reread this file, then the relevant
  contract, actual diff and unresolved findings. Do not treat a summary or a
  previous agent's judgment as source authority.
- Never treat a dated archived report or old handoff as current instruction,
  source acceptance, installed process fact or permission to replay effects.
- Use [goal.md](goal.md) only when the user has explicitly established an
  active long-term goal. An empty file means no such goal is currently set;
  never invent one as housekeeping.

## Design with clear ownership

- For substantive work, identify who produces the input, who decides or mutates
  state, who consumes it and what must survive failure or reconnection.
- Reuse the actual owner and caller path. Do not introduce a second executor,
  controller, state machine, cache or compatibility branch to conceal a defect.
- Keep a concept's rules and data at one authoritative location. Prefer
  derived state and existing public interfaces to shadow records or repeated
  configuration. Add abstraction only for a distinct responsibility.
- Preserve every required user behavior while simplifying. Identify the
  consumers of code being replaced and remove obsolete paths, callers, tests
  and documentation together once the replacement is qualified.
- Separate behavior-preserving refactoring from policy or observable behavior
  changes. Update the normative contract deliberately before altering public
  behavior; version or test-status changes alone do not revise SPEC.md.
- Keep changes cohesive and limited to the requested outcome. Record material
  unrelated issues in the current work context rather than implementing them
  incidentally. Avoid ceremonial plans, duplicate ledgers and unused fields.

## Execute with care

- Respect real execution and approval controls. A request to test a reviewer
  is not blanket authorization to execute a prohibited effect. If a reviewer
  denies an action, do not evade it or claim it ran.
- Before local execution, preflight the **actual canonical Unix socket path**
  against the 107 encoded-byte limit; keep runtime socket TMPDIR separate from
  work scratch and evidence subtrees. Match Core/Provider/HOME/workspace/native
  writer identity; do not substitute a mixed isolated/operating experiment.
- Preserve user files, credentials, account profiles, sessions, running jobs,
  signed active/rollback generations and unrelated source changes. Use owned
  disposable roots for experiments; never overwrite another worker's edits.
- Treat uncertain tool delivery as uncertain. Recover the original request and
  outcome before retrying; never silently replay a possible filesystem,
  external or model effect. Reconciliation failure does not authorize reset.
- Do not change live runtime, service bindings, policies or release authority
  as a side effect of documentation, diagnosis or unrequested maintenance.
- Keep diagnostic output bounded and free of secrets or private conversation
  contents. Do not turn failures into success with fallback, skipped tests,
  weaker assertions, indiscriminate retries or arbitrary timeout increases.
- Cleanup only definitely owned, inactive generated artifacts. Preserve source,
  original results and evidence needed to recover or reproduce the work.

## Verify what actually matters

- Choose proportionate checks for the changed behavior. Reuse unchanged,
  admitted proof; a documentation edit does not require an unrelated rebuild.
- Test both expected success and meaningful failure paths at the real
  producer/consumer boundary. Prefer independently established expectations;
  mocks and byte/string checks alone do not qualify live behavior.
- Distinguish source, unit, synthetic integration, actual Native/Android,
  actual model/app, signed publication, device activation and physical UI.
  State which boundary was observed and which remains unknown.
- Inspect the final diff, affected callers and failure propagation. Reject
  unsupported claims, skipped mandatory gates and conclusions drawn only from
  counters, request acknowledgments or historical receipts.
- Write each new fact at one authority: SPEC for stable requirements,
  CURRENT for latest checked status, PRO_HANDOFF for unfinished tasks,
  user/component guides for supported usage, docs/legacy for old evidence.
  Archive displaced content before shortening; never append a historical
  timeline to an active document. Follow the document-map validation rules.
- Keep contract and documentation changes precise: stable requirements in
  SPEC.md and its normative annexes, present fact/evidence in CURRENT.md,
  completed history in Git or explicitly non-normative records.

## Report and hand off responsibly

- Report the actual result, source/commit, tests, failures, incomplete work and
  any user-visible effects. Keep the report proportional to the task.
- Describe decisions and tradeoffs in plain language, not as unchangeable
  instructions for a later agent. Do not claim another model, reviewer or
  application was consulted when it was not.
- Make each commit one coherent verified change; do not commit or publish
  unrelated files. Never equate a local commit with a remote push or release.
- Keep the active working context sufficient for the next agent to continue
  without replaying uncertain effects or treating an old rollout as current.
