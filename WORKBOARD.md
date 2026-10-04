# Rust Core Workboard

SPEC -> GOAL -> WORKBOARD; goal-md bound, approved equivalent primary, workers OFF.

CORE-MANAGER-INDEPENDENCE source accepted 2026-10-04; evidence and disposition
in GOAL. Core independently executes/diagnoses/updates/rolls back when an installed
optional Manager is unavailable. Candidate signatures/inventories and required
assets stay strict. Focused3, grouped acceptance, optimized build, actual release
entrypoint private-fixture proof and protected22 comparison passed. Source only;
no live cutover. No public command retired and no profile launch policy changed.

## Current milestone: Core / Manager boundary alignment

Planning baseline: clean `rewrite/rust-core` at
`edcc59d7a0d81cd5e35be6916a3709bd2b96e024` (2026-10-04).
SPEC4.5 owns the decision criteria; GOAL owns the success threshold. Criteria and
plan are the current user-requested deliverable. Production work has not started.

The next implementation bundle is **PROFILE-BOUNDARY** (slices P0-P2 below).
Server, notification and repair work are subsequent separate bundles within this
milestone. Close each bundle before selecting the next; do not accumulate their
independent mutations behind a single final test phase. Workers remain OFF.

### P0 — establish exact native constraints and runnable profile proof

- Observable outcome: direct Core and Manager-selected account launches have a
  documented, reproducible execution contract for the supported upstream runtime.
- Read paths: SPEC profile/runtime sections; Core `shared_layout.rs`, profile
  requirements and public launch in `main.rs`; Manager profile code in `lib.rs`;
  `tests/profile_commands.rs`; affected operator topology declarations in
  `scripts/shared_state_migrate.py` and `scripts/shared_visibility_transition.py`.
  Inspect exact official upstream CODEX_HOME, SQLite/rollout/lock paths, config
  precedence, shared-server discovery and installer behavior. Legacy branches are
  behavior evidence only. Rebind every version-sensitive conclusion to the
  qualified runtime (currently 0.160.0), not historical 0.155.1 assumptions.
- Writable paths: WORKBOARD findings and SPEC amendments only until proof runs.
  Build/check relevant Core and Manager targets in an owned temporary target root;
  run nonzero `declared_identity_shares_only_conversations_and_rejects_legacy_or_substitution`,
  `test_shared_profile_requirements_are_exact_conflict_safe_and_stale_free`,
  `profile_lifecycle_and_isolated_exec_are_publicly_wired` and
  `upstream_resume_is_forwarded_through_the_selected_execution_profile` baselines.
- Required disposition: enumerate every surviving topology producer, reader and
  affected probe, including operator scripts; classify KEEP/COLLAPSE/DELETE.
  Verify the existing dotted-ID discrepancy through the public launch path,
  rather than treating isolated parser output as a complete failure proof.
- Protected: live homes/auth/conversations/writers, resolver, installed artifacts;
  no model traffic, transcript inspection or automatic migration. New red gates
  stop behavior work. State: **next; not run on this planning revision**.

### P1 — one shared-path preparation owner for supported account launches

- Observable outcome: every accepted Manager profile ID launches through Core
  with the same intended shared conversation store; direct declared-home launches
  work without Manager. Arbitrary external homes retain isolation.
- Target: Manager registers private metadata and a private new account home;
  Core prepares/validates the exact shared execution topology before upstream
  exec. Remove Manager's duplicate canonical-directory/link production and the
  parallel SQLite launch signal if Core derives the same contract safely.
  Manager list/use validates registration/home identity; Core validates execution
  readiness. Existing complete homes remain valid; nonempty conflicting homes
  fail without repair or migration. Accepted dotted IDs must not be stranded.
- Amend SPEC's ID, create/list/use, readiness and shared-requirements contracts
  before code. Prefer the existing launch path; do not invent a preparation
  command, persisted registry or generic cross-layer framework for this change.
  Retain independent bounded validation at each trust boundary.
- Production/test scope: Core `shared_layout.rs` and relevant `main.rs` paths;
  Manager profile portions of `lib.rs`; their existing unit/integration tests.
  Operator scripts change only where the same contract requires it, preserving
  historical bounded migration behavior. No server/notification/repair mutation.
- Focused proof: extend the P0 regressions and add named real-entrypoint tests
  for dotted IDs, create/list/use before first launch, direct default/Manager/
  declared-external homes, arbitrary isolation, unavailable Manager and unsafe or
  conflicting topology. Assert unchanged account-local markers and rejection
  without partial writes. Native disposable discovery/resume proof is required
  for claims beyond synthetic environment/route behavior.
- Protected: per-account auth/config, native writer coordination, original argv,
  generation admission and rollback. Map each changed definition/branch to proof,
  inspect actual diff and close compile/focused gates. State: **pending P0**.

### P2 — current identity reports the identity an ordinary launch would use

- Observable outcome: after selecting an account in one child, a fresh ordinary
  launch and `profile current` do not disagree about the effective account.
- Target: current identity derives from the caller's inherited CODEX_HOME or the
  default home. Saved last selection is history, never implicit account switching;
  retain it only with an explicit useful presentation, separate from effective
  identity. Recognize the already-supported homes without importing them. Write
  exact public output and history disposition into SPEC before implementation.
- Scope: Manager current/selection formatting, bounded metadata reads and profile
  tests; Core only if necessary for the same public identity contract. Preserve
  child-only environment changes and no credential/path disclosures.
- Focused proof: update `inherited_codex_home_is_reported_without_revealing_paths`
  and profile lifecycle tests; add a real-entrypoint select-then-fresh-launch
  regression covering default, known homes and arbitrary external CODEX_HOME.
  Obsolete last-selection assertions must be replaced, not compatibility-shimmed.
- State: **pending P1**. Close PROFILE-BOUNDARY through the final batch below.

### Subsequent bundles, selected only after profile closure

**SERVER-BOUNDARY — minimum qualified upstream server launch.** Rebind native
discovery/daemon/installer behavior to the exact runtime. Inspect every production
definition in Core `shared_server.rs`, its `main.rs` callers and
`maintenance.rs` retirement caller. KEEP short account-distinct sockets, qualified
runtime and FD/environment binding. COLLAPSE/DELETE custom readiness, reuse,
config refresh or retirement only where native behavior demonstrably supplies
the same invariant. No broad lifecycle handoff based on documentation alone.
Focused existing tests: `shared_server_selection_preserves_explicit_user_modes`,
`shared_server_reuse_refreshes_config_without_restarting_active_server`,
`shared_server_signed_process_fds_reuse_and_namespace_are_bound`,
`shared_server_retirement_preserves_clients_writers_and_bound_roles`,
`shared_server_rejects_unsafe_records_directories_and_sockets`.
Add exact-runtime disposable bare/resume, long-profile, two-account, config-refresh
and generation-change proof without unmanaged installation, lost FD bindings or
active-writer restart. Define the actual writable scope/proof map before mutation.

**NOTIFICATION-BOUNDARY — validated policy with minimal Core projection.** Inspect
all producer/record/parser/projector paths in Manager and Core. Manager owns
presentation and delivery; Core needs only the validated integration inputs.
Collapse repeated policy interpretation while retaining bounded version/shape/
path validation. Reuse an existing contract if possible; a new projection file
or schema is not the default solution. Focused proof includes
`test_mgr3_notification_record_projects_hooks_on_real_upstream_launch`,
`notification_input_request_projects_actual_upstream_matcher_without_duplicates`
and Manager delivery regressions. Protect malformed-state handling, unavailable
Manager isolation, input-versus-permission preference, completion, one-line
notification and existing terminal focus. No live notification test in source work.

**REPAIR-BOUNDARY — disposition of the redundant repair facade.** Preferred
disposition is DELETE if exact public-path review confirms no result beyond Core
doctor/update/rollback. Before removal, amend public grammar and all specific
SPEC ownership/request contracts. Exhaustively remove affected Manager commands,
Core request-env/parser/planner/dispatch, obsolete probes/tests and entrypoint
documentation; preserve ordinary Core recovery. Replace the current repair tests
with non-mutating retired-route rejection and public doctor/update/rollback
regressions. Do not leave a hidden request backdoor or add intelligent repair.
This remains a planned retirement decision, not an already-amended command.

### Final acceptance and stop rules for each implementation bundle

At resume bind branch, HEAD and exact dirty source diff. Before mutation record
the bundle's concrete named regressions and every baseline red gate here. Close
one slice vertically: nonzero focused run, relevant compile/test success, complete
definition/proof map and actual diff review. Unexpected warnings, zero tests,
superseded assertions and missing proof stop new behavior until corrected.

At the stabilized bundle boundary run `scripts/check.sh` once, a locked release
Core build (Manager build when changed), real public-entrypoint disposable proof,
and protected-surface verification. Preserve raw argv/TTY/streams/signals/status,
strict signed candidate admission, update failure retention and explicit rollback;
reuse same-revision successful gates. Required native qualification is separate
from synthetic routing proof. Record accepted evidence/disposition in GOAL, commit
and replace the live slice map; do not retain another evidence hierarchy.

All filesystem proof uses owned private HOME/PREFIX/TMPDIR roots. No installed
launcher/runtime, account state, auth, conversations, resolver or Manager
preferences may change. Stop if preserving accepted sharing/auth/writer semantics
requires a migration or an unproven native behavior. Source acceptance does not
mean deployed acceptance. Publication and live activation are separate bounded
operations; this plan does not initiate either.
