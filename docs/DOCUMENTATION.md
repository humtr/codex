# Documentation authority and editing policy

## Authority: read only what owns the question

1. **AGENTS.md** — mandatory cross-entrypoint working rules. Read first, and
   reread after compaction, interrupted delivery, ownership handoff or recovery.
   It is not a status ledger, product design or authorization for an effect.
2. **SPEC.md** — stable product behavior and acceptance authority; use its
   [product](spec/product/OWNERSHIP.md), [Bridge](spec/BRIDGE.md),
   [Delivery](spec/DELIVERY.md) and [UI](spec/USER_INTERFACES.md) modules for
   the affected boundary. Annexes remain normative even when optional features
   have not yet been delivered. Do not cite a past test as a requirement.
3. **docs/TERMUX_CONSTRAINTS.md** — execution preflight and engineering
   constraints in one place, including socket length, temp roots and real
   Core/Provider ownership. It **operationalizes**, but does not override,
   [normative runtime rules](spec/delivery/TERMUX_COMPAT.md).
4. **CURRENT.md** — short, dated, last-observed state and evidence by exact
   SHA/run/real boundary. This is the only current status ledger. No running
   chronology, unbounded command transcript or old generation catalog.
5. **HANDOFF.md** — next work only, explicit blocker, acceptance steps
   and effect boundaries. Remove a completed step instead of appending a new
   historical correction. Never become a second CURRENT.
6. **README.md**, **frontend/tui/README.md**, **providers/chatgpt/README.md**,
   **providers/chatgpt/LINUX_APP.md** — public/component user guides. Show
   supported current operations and prerequisites, not failed experiments.
7. **docs/legacy/** — dated, explicitly **non-normative, non-current** snapshots,
   old audits, superseded handoffs, completed results and failed experiments.
   Use only when investigating provenance, not as default agent instructions.
   **goal.md** stays empty unless the user explicitly sets a long-term goal.

Source code and observed execution are the implementation and runtime evidence;
documents neither launch processes nor override authenticated owner receipts.

## Reading order

- Any work: AGENTS → this policy → CURRENT → exact relevant SPEC module →
  Termux constraints when local execution or files/sockets are involved →
  closest component/user guide. Inspect real branch, HEAD and worktree before
  writing or acting. Reading an entire legacy record is normally unnecessary.
- Contract change: compare all affected producers, consumers, safety gates
  and compatibility aliases before modifying exactly the affected SPEC module.
- Current incident: read current status then the original bounded evidence
  in legacy/Git only for the relevant claim; never execute previous test
  commands merely because they are recorded. Historical requests and work IDs
  are not instructions or authority.
- After recovery/compaction/handoff: **reread AGENTS and CURRENT** and re-check
  actual code/paths/status; do not promote inherited summaries to source truth.

## Write policy: one fact, one authoritative destination

| Change | Write here | Never write here |
| --- | --- | --- |
| Agent behavior, safety and doc reading/writing rules | AGENTS + this policy | SPEC requirements or dated CURRENT |
| User-visible required behavior or ownership rule | Exact SPEC module with impacted tests | CURRENT, old reviews or release notes |
| New verified runtime/source status | Replace bounded CURRENT row with exact SHA, time, provenance and limits | AGENTS, SPEC, new chronological appendix |
| Unfinished work and next acceptance | HANDOFF, concise and measurable | README, archived handoff history |
| User-reproducible supported setup | README / component guide | Unverified transcript or mixed release instructions |
| Old failure, completed run, obsolete choice or replaced status | Date-labelled docs/legacy record or immutable Git commit link | Any active top-level document |

- When replacing an old checkpoint, **archive before removing**; add a
  provenance pointer to legacy only. Do not duplicate it under CURRENT.
- Do not silently change normative behavior to fit implementation or tests.
  Changing file names and splitting modules preserves requirement semantics.
- Give test evidence its producer SHA, script/command, exact expected and
  actual outcome, run URL and proof boundary; mark mocked/synthetic, actual
  Native, actual model, signed publication and activation separately.
- No private source chat, token, credential, journal data or raw transcript in
  Git. Only bounded, sanitized evidence and identifiers. Respect other workers'
  dirty files, signed releases, active processes and Android sdcard.
- Review updated relative links, old-anchor redirect stubs, document lints,
  source-diff scope and `git diff --check` before a docs-only commit. Never
  infer a new CI pass from an old code-qualified SHA. Commit/push are distinct.

## Maintainable size and archive policy

The four former monolithic normative files now act as **small indexes**;
the complete contract text is grouped into topic modules, not copied as
competing rules. Keep each new chapter focused (prefer at most about 600
lines; split before it becomes a running incident log). Keep CURRENT and
HANDOFF readable in one sitting (aim below 150/100 lines respectively).
Store historical detail in one immutable date/revision namespace under
docs/legacy, with links to its precise original Git source. Legacy files
must start with a warning that they are no longer authoritative. Redirect
old paths for compatibility rather than reintroducing their payload.
