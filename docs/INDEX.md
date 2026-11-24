# Documentation Map and Source of Truth

Read this page before interpreting an older report or conditional annex as
active code. The shipped product is the **standalone Termux Wrapper around
official Codex** (`rewrite/wrapper`), not the archived `rewrite/rust-core`
distribution.

## 1. Document Directory and Authority

| Question | Authority | Lifecycle / Rule |
| --- | --- | --- |
| What are the settled top-level product invariants? | [SPEC.md](../SPEC.md) | Stable release-independent contract (bundled in releases); updated only on deliberate invariant change |
| What are the detailed delivery, CLI, or conditional Stage 2 contracts? | [Normative annexes](#normative-annexes) (`docs/spec/*`) | Detailed Stage 1 mechanics + conditional Stage 2 requirements; **not** proof that code exists |
| What does the current codebase **actually implement**, and who owns each file/state? | [ARCHITECTURE.md](ARCHITECTURE.md) | Source-traceable active/deferred ownership map; updated when routes, modules, or owners change |
| What is currently running, verified, and unfinished? | [CURRENT.md](../CURRENT.md) | Crisp live snapshot + recent dated evidence ledger (newest-first); rotate superseded checkpoints to `docs/history/` |
| How does a Termux user install, update, diagnose, and recover? | [README.md](../README.md) | User guide (bundled in releases); no architecture or release authority |
| What is the active execution work order on this branch? | `HANDOFF.md` *(ephemeral, repo root when present)* | Sole location for active milestone execution/checklists; delete or archive to `docs/history/` when complete |
| What is the deferred Stage 2 integration plan and open verification backlog? | [ROADMAP.md](ROADMAP.md) | Unimplemented Stage 2 sequencing, lineage rules, and deferred verification criteria; grants no activation authority |
| What principles govern coding agents in this repo? | [AGENTS.md](../AGENTS.md) | Engineering, verification, and handoff discipline; not product functionality |
| What happened in completed Stage 1 and Stage 2 milestones or earlier lineages? | [Historical archives](#historical-archives) (`docs/history/*`) | Non-normative provenance only |

### Normative Annexes

- [docs/spec/DELIVERY.md](spec/DELIVERY.md): Stage 1 installation (`install.sh`), local upstream updates/rollback, artifact retention, `codex doctor`, Termux compatibility, release execution, and **conditional** Stage 2 signed-distribution/source-build contracts.
- [docs/spec/USER_INTERFACES.md](spec/USER_INTERFACES.md): Stage 1 CLI failure scoping, `CODEX_SHARED_HOME`/`CODEX_SQLITE_HOME` binding, and Stage 2 Slice 2 standalone `codex-ext` (`cdx`) profile, switch, provider, and notification contracts.
- [docs/spec/MCP_SHELL.md](spec/MCP_SHELL.md): Stage 2 Slice 3 standalone `@TermuxShell` (`termux-shell-mcp`) independent host shell worker contract.
- [docs/spec/SUPERVISION.md](spec/SUPERVISION.md): Stage 2 Slice 3 opt-in `runit` 3-state generation supervision contract (`selected`, `running`, `pending_activation`, atomic `current`/`previous` rollback).
- [docs/spec/MCP_CODEX.md](spec/MCP_CODEX.md): Stage 2 Slice 4 standalone `@TermuxCodex` (`termux-codex-mcp`) stateless 5-tool MCP bridge contract over official `codex app-server daemon` JSON-RPC.
- [docs/spec/CHATGPT_PROVIDER.md](spec/CHATGPT_PROVIDER.md): Stage 2 Slice 5 immutable `host/chatgpt-desktop` generation lifecycle and slim `chatgpt-provider` (single CDP pipe, clean-room `page_driver.js`, `/v1/responses`, authenticated `/control/*`) contract.
- [docs/spec/MCP_CHATGPT.md](spec/MCP_CHATGPT.md): Stage 2 Slice 6 standalone `@TermuxChatGPT` (`termux-chatgpt-mcp`) stateless 6-tool MCP bridge contract over `chatgpt-provider` `/control/*`.
- [docs/spec/BRIDGE.md](spec/BRIDGE.md): Stage 2 decoupled Provider and 3-MCP (`@TermuxShell`, `@TermuxCodex`, `@TermuxChatGPT`) ownership, annotation, receipt, and XDG namespace overview.
- [docs/spec/ORCHESTRATION.md](spec/ORCHESTRATION.md): **Stage 2 Slice 7** optional `@TermuxOrch` (`mcp/termux-orch`) durable multi-role orchestration contract (`F · L · W · V · R`, Tier 0–3 progressive durability, `Work Ledger` + `Typed Inbox` + `Wait-Set`, project/session naming, `inject_and_notify`).

### Historical Archives

- [docs/history/PRODUCT_POST_DEPLOY_AUDIT_20261124.md](history/PRODUCT_POST_DEPLOY_AUDIT_20261124.md) records the component-by-component audit, signed artifacts, and deployment checks.
- [docs/history/APPHOST_SHUTDOWN_AUDIT_20261123.md](history/APPHOST_SHUTDOWN_AUDIT_20261123.md): Independent 2026-11-23 audit of Android Termux ChatGPT AppHost orphan/zombie shutdown behavior, PRoot `v5.1.107.102` `--sysvipc` subreaper deadlock counterexample, and conditional `host-supervisor` design & acceptance matrix (non-normative).
- [docs/history/STAGE2_MILESTONES.md](history/STAGE2_MILESTONES.md) records Slices 1–7, all seven signed release readbacks, the live two-account and four-path session checks, and the daemon cache reclamation.
- [docs/history/STAGE1_MILESTONES.md](history/STAGE1_MILESTONES.md): Completed 2026-11-16 – 2026-11-19 Stage 1 P0/review/maintenance milestones and retired work orders.
- [docs/history/GENERATION_ADMISSION.md](history/GENERATION_ADMISSION.md): Earlier pre-rewrite patches, release admissions, and legacy byte policies.
- [docs/history/LEGACY_RUST_CORE.md](history/LEGACY_RUST_CORE.md): Archived `rewrite/rust-core` (`0.167.x`) development evidence. Referring to it does **not** restore its implementation.

---

## 2. Status Vocabulary (Do Not Collapse These)

| Term | Meaning |
| --- | --- |
| **Contracted** | Required by [SPEC.md](../SPEC.md) or an annex when selected; may not yet be implemented on `HEAD` |
| **Implemented** | Reachable, inspected source exists on the branch; alone proves neither integration nor device acceptance |
| **Qualified** | Named test boundary passed with reproducible input/results (unit, synthetic, Linux+Bionic, Android, authenticated, connected client are distinct) |
| **Published** | Exact peeled Git tag commit, release metadata, and downloaded asset bytes passed independent public readback |
| **Activated** | Explicitly selected binary/generation is active on a device; unrelated services retain their own selection |
| **Deferred** | Absent from the active distribution; a saved config flag, old service, or planned CLI spelling is not delivery |
| **Unknown / warning** | Evidence is insufficient or another owner reported degradation; never silently normalize to healthy |

---

## 3. Documentation Lifecycle and Change Propagation

1. **Keep one authority per question:** Avoid duplicate feature ledgers in `README.md`, `CURRENT.md`, `AGENTS.md`, or `docs/history/*`.
2. **Use `HANDOFF.md` for active work orders.** At closeout, record verified receipts in [CURRENT.md](../CURRENT.md), preserve significant milestones in the matching [historical archive](#historical-archives), and retire the work order.
3. **Keep `CURRENT.md` concise and newest-first.** Start with the live snapshot, then record open work and recent verified receipts. Move superseded checkpoints to the matching [historical archive](#historical-archives).
4. **Propagate changes in order:**
   - Deliberate invariant/behavior change → update [SPEC.md](../SPEC.md) / relevant annex first, then source & tests.
   - Code/module/route change → update [ARCHITECTURE.md](ARCHITECTURE.md) and user-facing [README.md](../README.md) when reachable.
   - New verified boundary → update [CURRENT.md](../CURRENT.md) only after that boundary actually passes.
   - Deferred roadmap change → update [ROADMAP.md](ROADMAP.md).
5. **All linked local Markdown targets must be Git-tracked** and verified by `.github/scripts/check_doc_links.py`. Never claim publication or live activation from a source/doc CI pass alone.
