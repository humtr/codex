# Documentation map and source of truth

Read this page before interpreting an older acceptance report as current
implementation. The product is the **standalone Termux Wrapper around official
Codex**, not the entire former `rewrite/rust-core` distribution.

## Which document answers which question?

| Question | Authority | Rule |
| --- | --- | --- |
| What must the product do, and who is allowed to do it? | [SPEC](../SPEC.md) and its [normative annexes](#normative-detail) | Requirements and future opt-in contracts; **not** evidence that code exists |
| What does the current Wrapper **actually implement**, and where? | [Stage 1 ownership and implementation map](architecture/STAGE1_BOUNDARIES.md) | Source-traceable active/deferred matrix; revisit when routes or owners change |
| What was verified on this device and in CI, and what is still unverified? | [CURRENT](../CURRENT.md) | Dated evidence, live/isolated/public distinctions, open findings; do not substitute old results |
| How does a Termux user install, update and recover? | [README](../README.md) | Commands and operational cautions; no architecture or release authority |
| What may a coding agent change, and how must it verify? | [AGENTS](../AGENTS.md) | Process/ownership principles; not product functionality |
| Which standalone Wrapper review findings need fixes, and how will each fix be verified? | [Wrapper review work order](plan/WRAPPER_REVIEW_WORK_ORDER.md) | Reproductions, implementation scope and acceptance criteria from the 2026-11-18 review |
| How do legacy dedicated daemon-store notes relate to Stage 1 default linking, `/switch`, and `session_index.jsonl` checks? | [Shared daemon packages & handoff verification plan](plan/SHARED_DAEMON_PACKAGES_PLAN.md) | Stage 1 default `current -> standalone/current` supersedes neutral store / 3-gen daemon pruning; optional legacy copy cleanup and UUID `/switch`/`session_index.jsonl` checks remain separate |
| What is the order of **unimplemented** optional integration? | [Stage 2 plan](plan/STAGE2_PLAN.md) | Sequencing, prerequisites, proposed implementation; no permission to activate services |
| What happened in an earlier generation? | [History](#historical-evidence) | Non-normative evidence; not a current component dependency |

### Normative detail

- [USER_INTERFACES](spec/USER_INTERFACES.md): current standalone public routes and **conditional** Stage 2 Manager/TUI/notification/profile requirements.
- [DELIVERY](spec/DELIVERY.md): installation, local upstream update, crash recovery, artifact provenance, and **conditional** signed-distribution contracts.
- [BRIDGE](spec/BRIDGE.md): **conditional** Native/ChatGPT Provider/Bridge integration contracts. A running external connector does not make it a Wrapper-built Stage 1 module.

### Historical evidence

- [GENERATION_ADMISSION](history/GENERATION_ADMISSION.md): prior patches, release admissions, legacy decisions.
- [LEGACY_RUST_CORE](history/LEGACY_RUST_CORE.md): earlier Rust Core/Manager/Provider/TUI implementation and findings. Its implementation is **not** restored merely by referring to it.
- [goal.md](../goal.md): presently empty; **not** an active plan.

## Status vocabulary (do not collapse these)

| Term | Meaning |
| --- | --- |
| **Contracted** | Required by SPEC or an annex if its phase is selected; may be absent from HEAD |
| **Implemented** | Reachable, inspected source exists on the branch; this alone proves neither integration nor device acceptance |
| **Qualified** | Named test boundary actually passed with reproducible input/results (unit, synthetic, Android, authenticated, connected client are different boundaries) |
| **Published** | The exact source/tag and bytes passed independent public readback |
| **Activated** | An explicitly selected binary/generation is running/selected on a device; other services keep their separate selection |
| **Deferred** | Not in the active distribution; a test, old service or planned CLI spelling is not delivery |
| **Unknown / warning** | Evidence is insufficient or a separate owner reported degradation; do not silently normalize to healthy |

## Maintenance and change propagation

1. Check the active branch, source tree and command dispatcher **before** editing status.
   [STAGE1_BOUNDARIES](architecture/STAGE1_BOUNDARIES.md) links each owner to code.
2. Behavior/policy changes go through SPEC and relevant annex first; update source,
   tests and user-facing README together. An evidence-only change belongs in
   CURRENT, not SPEC.
3. If a feature becomes implemented, update its row in the implementation map,
   the Stage 2 plan and the interface's active/deferred label; add exact tests
   to CURRENT **only after** their boundary passes.
4. Public artifact/tag refresh and operating activation are distinct decisions
   from source acceptance or a documentation push. Never claim the former from
   a passed doc CI run.
5. Avoid a second feature ledger in README, AGENTS, history or a plan. This map
   is a **source/owner index**, CURRENT is the **evidence ledger**, and the
   annexes are **contracts**, even where the same command is referenced.
