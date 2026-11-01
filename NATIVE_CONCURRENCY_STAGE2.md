# Native concurrency: Stage 2 execution-boundary handoff

Baseline: `rewrite/rust-core` at `151a95b73da9195a78c6861493cdf7e6fc1125d7`.
The normative contract remains `SPEC.md` NC-01 through NC-29.
This record describes implemented boundaries, not full acceptance of all 29 cases.

## Protected in the source (not yet installed)

- **Direct MCP**: the existing Provider `Broker` classifies the original
  `native_tool` call at enqueue, persists a resource reservation **before**
  publishing it to upstream via `offer`, and releases it **after** a
  correlated result has been durably committed by `settle_prepared`.
- **Provider AppHost Native Tool events**: the existing `Operator::exchange`
  verifies work ownership and uses the **same** Broker claim journal before
  forwarding a Native `Event::Tool`. The subsequent, correlated upstream
  `Request::results` settles it. Duplicate/uncertain Event IDs are not
  resubmitted. Neither path starts a second executor.
- **Known patch writes**: the verified worktree root, physical parent entry,
  file inode/aliases and initial SHA-256 are recorded. Disjoint file effects
  can be offered concurrently. Same-file effects are offered in sequence.
  Later disjoint text hunks can rebase only when their old-side anchors still
  match uniquely. Deleted/overwritten anchors return
  `native_resource_version_conflict` with `executed:false`. The upstream
  patch tool still decides whether each actual patch succeeds.
- **Unrecognized effects**: commands not proven harmless or bounded are
  treated as opaque, never presumed read-only from annotations. While offered,
  they conservatively exclude conflicting Broker-managed mutations. A
  recognized ongoing `exec_command` session retains its claim through
  `write_stdin` polling until a Native exit observation. Tool stdout cannot
  forge an exit marker.
- **Recovery**: unresolved claims and offered-call identity remain in the
  existing private Provider journal across interrupted requests or process
  replacement; a completed work record alone does not release them.
  `codex_status.native_effects` shows bounded owner/kind/session information,
  not the underlying file contents or shell commands.

## Coverage and limits

| NC contract | Current evidence | Not yet established |
| --- | --- | --- |
| 01-04 | Broker admits calls without work-lifetime mutex; a narrow proven-read class runs alongside writes | Provider create/fork overlap rejection, arbitrary read-only shell, pinned snapshots, upstream-only tools |
| 05-08 | Independent paths, conflict serializing, same-file hunk rebase, version rejection and original-call receipts have tests | Real multi-process upstream same-file merge and non-patch writers |
| 09-13 | Parent entry, path containment, symlink swap, inode aliases and atomic file replacement unit tests | Arbitrary directory rename, nested Git repositories, inaccessible Android hardlink fixture (the inode comparison is separately tested) |
| 14-16 | Unclassified Git commands remain opaque instead of being incorrectly marked read-only | Separate index/HEAD/ref effect claims and conflict-free preservation of user staging against unmanaged Git writers |
| 17-23 | Known named files are independently claimed; other command effects conservatively conflict | Per-work TMPDIR, shared artifact/output identities, external resources, reliable independent-root disjointness |
| 24-25 | Opaque fallback excludes Broker-managed mutation; pre-offer physical file version validation | OS-staged opaque execution, unknown write-set detection, post-offer writes by external CLI/editor |
| 26-29 | Owner-bound durable claims, unresolved effect status, idempotent Native receipts, async command session tests | Automatic reconciliation after unobservable Native/process loss; status is not authority to discard effects |

The Core launch path ends in an upstream `exec`. An upstream Native tool,
standalone Core/CLI/TUI path, independently running process, or external editor
which does **not** pass one of the two Provider boundaries above is **not**
covered. A claim in Provider does not lock the operating-system files against
such writers. Opaque effects can cause conservative, potentially long-lived
blocking; we cannot safely invent a timeout to unlock a still-running process.
Stage 2 is therefore source implementation with **partial NC coverage**, not
permission to remove the current Provider admission gate yet.

## Stage 3: required Provider integration contract

Retain the existing Native work ID, controller, generation, turn, execution-home
owner, approval decisions, tool-call ID, result provenance and Native history.
Attach each actually executable upstream Native tool dispatch to the same
resource record (`owner_work, native_call_id, workspace, effect, version,
claim_state, result_receipt`); use the existing execution/transport boundary,
not another orchestrator. Before broadening workspace create/resume/fork
admission, verify coverage of direct upstream Core/TUI tools as well as both
Provider paths. If a route lacks a hook, report it as unprotected and keep
appropriate isolation rather than silently grant shared-writer safety.

Replace `native_workspace_writer_busy` admission based on overlapping work
records with operation-level effect coordination **only after** that coverage
is established. Keep read and disjoint-work admission independent; preserve
unsettled claims until original Native results or explicit verified recovery
prove final effects. Add isolated, per-work TMPDIR, build/Git/output metadata
claims, safe opaque command staging or observed write sets, and tests against
real concurrent Native threads plus external tools. Runtime deployment and
installed Android/Web Provider behavior were not changed in this stage.

## Local validation

Run from repository root:

```sh
cargo fmt --manifest-path providers/chatgpt/Cargo.toml --all -- --check
CARGO_TARGET_DIR="$HOME/.cache/codex/provider-target" cargo test --manifest-path providers/chatgpt/Cargo.toml --lib -- --test-threads=3
git diff --check
```
