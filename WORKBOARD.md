# Rust Core Workboard

Authority: SPEC.md -> GOAL.md -> WORKBOARD.md. User-approved equivalent primary
Lead; workers OFF. Current bundle SHARED-CONFIG-BOUNDED-RETENTION-V1.

Bound source: rewrite/rust-core d20b245a88323fc476cf46cf7e3b8e802422413c,
tracked clean plus pre-existing .github/scripts/__pycache__/. Public main
3224a5294d22818350a105d2adef6bc32b7e565f; public/live sequence 28 / 0.160.0.
Baseline: corrected wrong codex-termux-core package selector; real codex target
164 passed, one device test explicitly ignored. Native cache defect reproduced
using installed Core/runtime in a temporary HOME, with no live state mutation.

## Ordered proof slices

1. **Warm configuration refresh** — shared_server.rs and its focused regressions.
   Publish owned config atomically within the server's existing FD-34 directory
   on reuse under its namespace lock, preserving PID, directory inode and active
   threads. Prove enable/disable/readback, collision rejection, same PID/inode,
   full access and argv/TTY/signals. Native installed-runtime temporary gate.
   State: closed. Four shared_server regressions pass, including
   shared_server_reuse_refreshes_config_without_restarting_active_server; exact
   diff inspected, Core compiled. Official 0.160.0 native runtime confirms hook
   enable/disable readback with the same server PID and a loaded thread retained.
   Probe red restored: thread/start returns a lazy rollout path before a file
   exists; prove the loaded thread independently and inspect an inode only when
   present. Wait for test-owned server shutdown before temporary-root teardown.
   KEEP signed upstream server; COLLAPSE config publication onto
   the existing directory; DELETE startup-only snapshot assumption.
2. **Bounded artifacts** — Core launch/update boundaries and one maintenance
   module, relevant tests only. Directory leases fence in-flight update/exec;
   exclusive maintenance + state writer lock protects authoritative roles;
   inspect real process references; safely retire only inactive qualified obsolete
   servers; prune owned generations/publications and abandoned staging.
   Prove active/rollback/guard/baseline/open-FD protection, contention/concurrency,
   symlink/invalid-state/crash/partial cleanup, idempotence, real public path and
   bounded repeated updates. State: closed after focused and grouped acceptance.
   Red restored: removal-count fixture omitted its publication cache (five
   directories, not four); corrected the exact assertion without product change.
   Stop-on-red in the complete Core gate: cleanup on a failed update removed
   the complete higher-sequence candidate required by existing root-sync fault
   retry. KEEP that contract; retain at most one highest-sequence complete
   pending candidate using existing release/descriptor metadata, with no registry.
   Catalog-parent flock preserves the existing write/execute-only root-sync
   fault boundary; the exact retry gate now passes. Four focused retention
   regressions map protected roles, one pending candidate/open staging,
   update/launch leases, crash/partial cleanup and the real entrypoint. Five
   shared-server regressions cover active-client/writer preservation and idle
   graceful retirement. Native syscall tracing identified a terminated fixture retained as a zombie:
   exe returned ENOENT and Android denied its fd directory. Explicit stat state
   plus a single remaining thread proves the process group exited; the new named
   zombie regression and combined ten-test class gate pass. Live visibility
   failures still disable pruning. Run grouped checks serially to isolate /proc;
   lease contention is explicitly tested across processes. No session SQL subsystem.
   Grouped Python discovery revealed five pre-existing stale workflow assertions:
   historical source SHA/action multiplicity, manual-only authorization before
   accepted scheduled publication, and superseded generation-centric no-op text.
   COLLAPSE onto immutable SHA pins plus existing checkout/readback invariants
   and current schedule/human-output contract; retain all security gates.
   Last full gate: 170 Core cases passed; one old probe test expected a failed
   candidate to remain forever even after a newer accepted generation. Update
   that regression to verify the pending failed candidate before replacement,
   then its disposal after acceptance; accepted current/rollback remain signed.
   This follows the selected bounded-retention contract, not a product bypass.
   Production proof mapping: `owned_directory`/`invalid`, `launch_lease` and
   `UpdateLease::{acquire,drop}` -> retention_leases_recovery_and_partial_failure
   and retention_real_launch; `dependencies`/`referenced_generation` and live
   `process_references` -> retention_preserves_pointer_guard_baseline_and_open_files;
   exited-process branch -> retention_ignores_exited_zombies; `pending_candidate`,
   `staging_name`/`prune_directory`/`prune` -> retention_bounds_pending_candidates
   plus the roles/partial-failure/public-launch cases. Names above are prefixed
   `generation_` and suffixed as defined in Core tests. Main launch/bootstrap/
   update lease boundaries additionally use the existing signed public update,
   root-sync retry and activation/rollback fault gates. Shared refresh and
   `retire_unused` map to their two named regressions above.
3. **Accept/deploy/clean** — grouped full gates, protected surfaces, diff review,
   implementation commit/push, exact-source signed release/public readback,
   ordinary live activation. Native warm config/active-thread/bare/CWD/All
   proof, recent session prefix/inode preservation, then authorized removal of
   unreferenced artifacts, old private SQLite and expired conversation remnants
   without backup. Record actual reclaimed bytes and repeat-maintenance behavior.
   State: source acceptance closed; signed publication/live activation in progress.
   Exact scripts/check.sh: Core 171 pass + one explicit device ignore, Manager
   16 + 9 pass, builder 21 pass; Python 15 operator + 75 publication cases pass.
   Total 217 Rust / 90 Python. Release build, fmt and diff check pass; all 15
   protected auth/config/resolver files keep exact bytes/modes/inodes during
   source/disposable work. Native official-runtime temporary launch prunes old
   generation/staging/publication roots and refreshes hooks both directions on
   the same PID while a thread stays loaded. No lazy-rollout inode claim.
   Historical backup payload inspection: recent payload exists canonically with
   exact record prefixes; one stale backup index points to an expired transcript
   already excluded by the accepted seven-day transition. No new backup.
   No resolver/auth mutation; no user writer interruption.
