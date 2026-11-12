# Documentation authority and editing policy

This file defines document authority, selective reading and write ownership.
It does not create product requirements or authorize runtime effects.

## Authority by subject

| Source | Authoritative for | Not for |
| --- | --- | --- |
| [SPEC](../SPEC.md) | Stable top-level product obligations | Directory mapping, version, current status |
| [Functional annexes](spec/INDEX.md) | Scoped normative behavior refining SPEC | Overriding or weakening SPEC |
| [AGENTS](../AGENTS.md) | Agent workflow, preflight, checks, recovery | Product requirements or incident logs |
| [Termux preflight](TERMUX_CONSTRAINTS.md) | Practical device execution checks | A separate product contract |
| [CURRENT](../CURRENT.md) | Last verified source/device facts by SHA and proof boundary | Future acceptance and history |
| [HANDOFF](../HANDOFF.md) | Actual unfinished work, next acceptance and blockers | Past work or duplicate CURRENT |
| [Finite integration plan](development/RELEASE_FRAMEWORK_IMPLEMENTATION.md) | Bounded W-task DAG, file Writer, provenance and change accounting | A second product specification, current-proof ledger or approval gate |
| [README](../README.md), component guides | Supported user setup and operations | New norms or unqualified releases |
| Test dependency declaration | Per-test input documents and absence behavior | Product behavior |
| Fixture-specific instructions | That fixture only | Repository-wide instructions |
| Git history / optional evidence | Completed and superseded provenance | Current task or active authority |

The index has no normative content. An annex is binding in its subject area,
but cannot contradict the top-level product requirements. Actual code and
owner receipts establish implementation facts, not new normative authority.

The single current transfer record is `HANDOFF.md`. `PRO_HANDOFF.md` and
dated/retired handoff variants must not be used as fallback instructions or
silently reconciled into current status. Recover their evidence from Git
history only when necessary for an unfinished acceptance; current deployment
facts belong to `CURRENT.md`.

## Proposals pending product review

- [Session-independent durable orchestration](development/orchestration/SESSION_INDEPENDENT_DURABLE_ORCHESTRATION.md) is a non-normative future design candidate, including the skill/plugin prototype. It does not replace SPEC, current Bridge contracts, or live acceptance evidence.

## Reading route

1. AGENTS; actual HEAD, branch, dirty diff, ownership; this policy and CURRENT.
2. HANDOFF only if continuing that precise unfinished acceptance.
3. SPEC for product behavior; INDEX directly to affected annexes, then actual
   producers/consumers and tests. Do not read every annex or old audit.
4. Termux preflight before local device/process work; component guide only for
   relevant usage. After compaction recheck AGENTS, current state and affected
   contract, never inherited summaries as source of truth.

## Writing, moving and removal

- Change SPEC only when a **top-level obligation** changes; changes to files,
  implementation, tests, releases or document layout do not by themselves
  justify a SPEC edit. Update the owning annex for actual scoped behavior.
- Put dated, bounded observed proof and exact SHA in CURRENT; unfinished
  acceptance and blockers in HANDOFF. Remove completed handoff steps instead
  of accumulating a chronology. Keep user setup in README/component guides.
- Update INDEX for discovery changes, AGENTS for agent behavior, this policy
  for documentation rules and the single test declaration for document inputs.
  Keep each requirement and status fact at one authoritative location.
- Historical retention is **optional**. Keep a minimal record only while it
  is uniquely necessary to unfinished development, acceptance or recovery;
  identify the reason and removal condition. Otherwise delete completed
  audits, plans and old snapshots. Immutable Git history retains provenance;
  cite exact commit SHA and path when needed. Never require a docs/legacy tree
  or an empty goal file as housekeeping.
- Preserve actual backward compatibility and regression fixtures required by
  active consumers even when historical prose is removed. Tests must fail for
  missing required contracts, not for missing obsolete archives, and may not
  silently skip a substantive check after an annex moves.
- Verify links, real affected tests, diff and exact-commit GitHub CI separately.
  Do not alter live services, account state, signed generations or unrelated
  workers as a side effect of documentation maintenance.
