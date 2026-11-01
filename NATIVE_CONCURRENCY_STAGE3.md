# Native concurrency: Stage 3 execution-boundary integration

Source baseline: `rewrite/rust-core` at
`ac50bbd337d0c36a3cd3bd05b498261d1b1e296a`. The normative
requirements remain `SPEC.md` NC-01 through NC-29. This report is a
source/test handoff, **not** a claim of deployed or fully verified safety.

## Execution path and actual protection

| Route | Stage 2 | Stage 3 source changes |
| --- | --- | --- |
| Provider Direct `Broker::offer/settle_prepared` | Provider-only in-memory and durable Journal | Also reserves and settles a shared, process-flocked effect ledger before forwarding / after the original receipt |
| Provider AppHost `admit_external_tool/settle_external_tools` | Same Provider-only Journal | Joins that shared ledger using owner lane, original tool ID and correlated native result; no re-execution |
| Upstream Codex Core | No boundary | Pinned 0.166.0 `ToolRegistry::dispatch_any_with_state` patches the actual pre-handler / post-handler boundary through `crates/release-builder/src/source.rs`; the same ledger and file-effect classifier are compiled into the prepared upstream source |
| CLI/TUI/AppServer/Code Mode Native tools | No boundary if bypassing Provider | The upstream Core registry covers tools that actually dispatch through that registry; custom tool handlers outside this registry or unmanaged OS writers remain outside it |
| Provider Native work creation | Path/record overlap could reject with `native_workspace_writer_busy` | Removed that repository-wide gate; per-work controller, home, ownership and execution-phase checks remain |

The ledger lives beneath `$PREFIX/var/tmp/codex-native-effects/shared-native-v1`
with private 0700 directories, a short OS `flock`, atomic/fsynced state
transitions and no Drop-based release. Its global *admission lock* is not a
repository-wide *execution lock*. Named file effects are compared using
actual canonical parents, names, inode identity, base digest and patch anchors.
Known independent file mutations in one or more worktrees coexist; opaque
mutations conservatively conflict with other unproven writers, including
shared Git refs across linked worktrees. A bounded Core conflict retry lasts
at most 20 seconds per tool and respects cancellation. An unresolved effect
does not expire merely because a work was stopped or its host disconnected.

Only a small, syntactically vetted subset of command strings is classified
as reads. Every other shell command, unrecognized tool and Git mutation is
opaque. Tool stdout after the native `Output:` header cannot manufacture
an exit observation. A Provider result is committed to the original Journal
before the shared reservation is released. An error between these two durable
records retains the claim conservatively; it never authorizes re-execution.

The immutable original upstream Cargo.lock is preserved. The exact pinned
Core direct dependency on already-present `sha2 0.10.9` is inserted in
source normalization without modifying external packages. The audited
normalized lock SHA-256 becomes
`80cfec077caedee103d1bc841fb7b842061eae2f3ce94c53cac272662bb55be7`;
external packages unchanged: 1,308; workspace packages normalized: 170.

## Test and acceptance evidence

- **Provider:** all 164 Rust library tests passed, zero failed, two
  pre-existing ignored. Tests include actual separate-process file claims,
  independent files and roots, conflicting/same-file path claims, safe
  reads, unknown shell session preservation, Core/Provider cross-boundary
  reservation simulation, AppHost/Direct reconciliation and existing
  controller/receipt/owner/approval regressions.
- **Release Builder:** all 46 library tests passed (zero failed), including
  six `source::tests`. On a fresh
  disposable pinned 0.166.0 checkout, `prepare-source`,
  `normalize-lock`, `git diff --check` and Rust `rustfmt` parse of
  generated `core/src/tools/registry.rs` succeeded.
- **Not passed / not attempted:** a full `codex-core` Cargo check and
  actual installed Core CLI/TUI/AppServer tool executions. Offline check
  lacked pinned `rusty_v8`; an online check became stuck retrieving V8's
  Chromium Git submodules and was cancelled without touching the installed
  runtime. The generated source has **not** been type-checked as a complete
  Core crate. No operating Core or Provider was replaced or installed.

### NC-01 through NC-29

`Unit` = relevant local source/Provider test; `partial` = only some
required effects proven; `open` = full contract not demonstrated. **No
NC case is declared fully accepted end-to-end without an executable,
independently controlled multi-work upstream Core test.**

| NC | Source/local evidence | Full-contract status |
| --- | --- | --- |
| 01 | Work record path gate removed; owner/receipt tests | Partial: two live Native works not exercised |
| 02 | Proven read does not acquire writer claim | Partial: Core parallel reading unexecuted |
| 03 | Read admitted during independent writer reservation | Partial: Core execution unexecuted |
| 04 | No pinned repeatable-read snapshot | Open |
| 05 | Independent file claims in same physical worktree | Partial: Core simultaneous edits unexecuted |
| 06 | Same-file claim order, old-side anchors and bounded Core retry | Partial: real two-writer merge unexecuted |
| 07 | Version/anchor conflict tests and serialized publication | Partial: external race not closed |
| 08 | Provider receipt and shared scoped call fingerprint tests | Partial: Core runtime roundtrip unexecuted |
| 09 | Exact parent/name entry claims | Partial: all rename/create/delete combinations not tested |
| 10 | Independent entries in a common directory | Partial: actual concurrent publication unexecuted |
| 11 | Unbounded directory/permission mutations treated as opaque | Partial: no precise descendant-set capture |
| 12 | Real-path/inode/alias and version tests | Partial: unmanaged path swap still possible |
| 13 | Common ledger covers named aliases and opaque shared Git state | Partial: nested Git and precise metadata ownership untested |
| 14 | Git mutation opaque; narrow reads free | Partial: safe live `git status/diff` not established |
| 15 | Different named file edits can coexist | Open: independent Git index/HEAD/ref effects not separately identified |
| 16 | Git writes conservatively opaque | Partial: no specific ref/index lock, staging isolation or HEAD CAS |
| 17 | Independent known files may coexist | Open: independent builds often opaque; per-work outputs deferred |
| 18 | Shared unknown output treated as opaque | Open: named cache/package/artifact arbitration deferred |
| 19 | No per-work TMPDIR implementation in Stage 3 | Open (Stage 4) |
| 20 | Unknown recursive cleanup opaque | Open: shared temporary descendants not independently owned |
| 21 | Distinct physical paths in different repos can coexist | Partial: unknown global resource commands remain conservative |
| 22 | Existing owner/home/approval protections retained and tested | Partial: explicit shared HOME/socket resource claims not complete |
| 23 | Existing publication authorization preserved | Open: external side effects not target-scoped |
| 24 | Unproven commands reserve the common mutation ledger | Partial: no scoped OS sandbox/staged filesystem publication |
| 25 | Content/version verified at offer and Core dispatch | Partial: unmanaged external writer cannot be atomically fenced |
| 26 | One controller per work and effect-based reservations | Partial: actual two-work same-tree execution untested |
| 27 | No work-phase/record overlap lock; unresolved *effects* persist | Partial: runtime work lifecycle tests not exercised |
| 28 | Crash/cancellation preserves claims and child-process sessions | Partial: automatic termination reconciliation missing |
| 29 | Atomic admission, scoped fingerprint, Provider receipts, bounded retry | Partial: compound partial-application rollback and provenance UI missing |

## Required next verification and Stage 4 handoff

1. Obtain complete pinned Core dependencies (including the V8 Git/submodule
   checkout); run `cargo check --locked`, original Core/TUI/CLI tests and
   a disposable built upstream binary under a separate build directory.
   Repair any actual type/behavior incompatibility discovered before
   considering this implementation ready for installation.
2. Run two independently controlled Native works in the **same** physical
   worktree and test concurrent reads, separate-file edits, overlapping and
   independent same-file hunks, rename/delete, uncertain child processes,
   cancellation/restart, Git index and refs, and Provider Direct/AppHost
   mixed routes. Check actual file bytes and original receipt ownership.
   In particular, verify whether an AppHost tool event can also pass through
   the Core registry; if so, its two sightings must be correlated without
   double admission or allowing any other work to masquerade as that call.
3. Implement isolated per-work `TMPDIR`/build output/cache scopes and
   targeted Git/metadata output claims; this remains the intentional Stage 4
   feature. Preserve global publication and credential ownership.
4. Implement verified write-set enforcement or staged publication for opaque
   commands, snapshot semantics for repeatable reads, and safe handling of
   external unmanaged file changes at actual commit time.
5. Add bounded, owner-verified reconciliation for orphaned durable claims,
   partial multi-file effects and lost terminal results. Never expire a
   potentially running writer on a timer alone.

No operating Core/Provider installation, remote push or worktree migration is
authorized by this Stage 3 change.
