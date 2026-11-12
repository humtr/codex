# Agent working protocol

This file governs developer-agent decisions and actions, not product features,
deployment instructions or a chronology of incidents. Apply it from CLI, TUI
and ChatGPT under the actual user's authorization and current owner controls.

## Start with the real checkout

1. Establish the requested outcome, authorized effects and acceptance boundary.
   Do not replace the task with a smaller demonstration.
2. Inspect `pwd`, `git status --short --branch`, HEAD, remote, relevant
   diffs and `git worktree list`. Identify dirty files and other active
   workers or resource owners before touching their work.
3. Read [documentation policy](docs/DOCUMENTATION.md) for authority;
   [CURRENT](CURRENT.md) for last verified facts; and [HANDOFF](HANDOFF.md)
   only when resuming its unfinished work. Past handoffs are not live orders.
4. For behavior, read [SPEC](SPEC.md), follow the
   [functional index](docs/spec/INDEX.md) to only the affected normative
   annexes, then inspect real producers, consumers and tests. Do not read
   all annexes or historical archives by default.
5. Distinguish a product requirement, hard execution constraint, current
   observation, optional recommendation and past proposal. Implementation
   and past evidence do not silently override a product requirement.

## Decide with the owner

- Map input, decision/mutation owner, consumer and durable result before
  editing a cross-component flow. Identify Native writer/thread, Core
  generation, Provider service, account and source-file owners separately.
- Reuse the actual authority and caller path. Before adding a state machine,
  helper, adapter, cache or compatibility branch, locate existing consumers
  and name a responsibility they cannot perform. Avoid second executors,
  writer registries, transcript stores and release controllers.
- Where code and requirements disagree, diagnose whether the code is
  defective or a product change is deliberately needed. Do not rewrite a
  contract to make a passing test true.
- Preserve every approved external behavior, caller, failure mode and
  required old-format reader when replacing or deleting an implementation.
  Update callers, fixtures and tests in the same cohesive change.

## Check before local effects

- Keep unrelated source, credentials, account homes, sessions, signed active/
  rollback generations, operating services and Android sdcard untouched.
  Prefer owned isolated worktrees and scratch roots; never clean uncertain
  artifacts or override another worker's writer.
- Before running Core, Provider, TUI, Native or a device fixture read the
  [Termux preflight](docs/TERMUX_CONSTRAINTS.md). Use an owner-private short
  canonical socket/TMPDIR root; each complete socket path must fit in
  **107 encoded bytes**, not characters. Keep longer evidence/scratch roots
  separate. Verify installed Core dispatcher and matching actual Provider/
  Native writer identity; a picker or ready screen alone is insufficient.
- Treat a stream disconnect, ambiguous ACK and timeout as unknown delivery.
  Observe the original call ID, receipt, result and process before any retry;
  never silently replay a possible filesystem, external or model effect.
  Stop, transfer or forcibly recover another owner's work only under explicit
  authorization, preserving unresolved effects and independent tasks.
- Preserve first failure cause and original stderr/exit state. Do not convert
  failure to success by fallback account/model, ignored reviewer denial,
  weaker assertion, skipped gate or arbitrary timeout extension.

## Change and verify the complete boundary

- Prefer the smallest cohesive change preserving the requested user flow.
  No live runtime, policy, service, account or release change as an incidental
  effect of documentation or source inspection.
- Check exact call paths and negative scenarios. Change normative contracts
  intentionally for genuine behavior changes; document movement never
  justifies changing an otherwise stable product requirement.
- Use [documentation policy](docs/DOCUMENTATION.md) for one authority per
  fact: requirements, current evidence, unfinished work, user guidance or
  optional historical evidence. Avoid competing ledgers and growing logs.
- Select tests by changed producer/consumer, meaningful failure and recovery.
  Distinguish source, unit, synthetic integration, Native, actual model/app,
  visible client, signed publication and device activation. Never claim a
  stronger result from a weaker proof or an old CI run.
- Inspect final references, `git diff --check`, actual exits and changed
  files. Make each commit a coherent verified unit, then separately verify
  the remote push and exact-current-commit CI status.

## Recover context and report

- After compaction, handoff or uncertain delivery reread this file,
  actual HEAD/diff/ownership, CURRENT and only the affected contract.
  Reconcile unfinished original calls before starting new effects. A
  summary is a hint, not execution authority.
- Report the actual files, deltas, commit, tests, evidence boundary, failure
  and remaining acceptance. Explain a real blocker and continue unaffected
  work. Never claim another model, reviewer or app was consulted without
  evidence.
