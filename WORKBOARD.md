# Rust Core Workboard

Authority: SPEC.md -> GOAL.md -> WORKBOARD.md. Workers OFF; approved current
primary Lead implements and validates directly.

## SHARED-SERVER-CROSS-ACCOUNT-V1

Bound source: rewrite/rust-core f264892869577bbd776075513df77f0a330726c2.
Tracked source clean; pre-existing untracked .github/scripts/__pycache__/ retained.
Public/live baseline: sequence 27, upstream 0.160.0; main 2dc79bd11842c9e5970b8c73c470886b780f2ff8.
Baseline runnable: Core 159 passed, 0 failed, 1 explicit device smoke ignored.
Native shared-server red gate restored by the exactly qualified UDS adaptation. Exact source diff identity at baseline: empty tracked diff.

## Ordered slices

1. **Unchanged argv/full access** — Core planner and system-config projection,
   affected Core fixtures/tests only. Focused proof: public dispatch, raw non-UTF8
   argv, final exec, version, policy ordering, config full-access defaults.
   Protect resolver, upstream user argv, auth/session state, updater/rollback.
   State: source focused gates green (dispatch 3, sandbox 3, final boundary 4,
   version 1, notification projection 1; compiled successfully); actual diff
   inspected. Additional same-class sweep removed synthetic doctor/version
   probe overrides and fixture prelude stripping. Complete affected Core target:
   159 passed, 0 failed, 1 explicit device smoke ignored. DELETE synthetic CLI prelude; KEEP policy validation;
   COLLAPSE default into existing FD-34 system config.
2. **Qualified shared server** — Core launch/runtime boundary and focused native
   upstream probe. Bind server to signed active runtime, inherit FD 33/34,
   profile-distinct private bounded socket, exclude installer/update authority.
   State: source slice closed. Regression map: eligible/explicit mode selection ->
   shared_server_selection_preserves_explicit_user_modes; private directory,
   namespace, records/socket rejection -> shared_server_rejects_unsafe_records_directories_and_sockets;
   process/peer binding, FD 33/34/35, reuse, profile/generation isolation and failed-child
   cleanup -> shared_server_signed_process_fds_reuse_and_namespace_are_bound;
   final descriptor/argv/TTY/signal boundary -> final_ (4 passed); TTY startup ->
   bare_tty_startup (2 passed); mandatory owned full-access config ->
   core_full_access_default_requires_owned_config_and_preserves_collisions (1 passed).
   Builder exact artifact/offset-only rewrite and drift -> uds_artifact_patch_is_exact_bounded_and_rejects_drift;
   report policy/digest/count binding -> uds_publish_report_binds_exact_policy_artifact_and_byte_count (2 passed).
   Official 0.160.0 archive build succeeded: adapted runtime ba94d1d0d9d416a7fb2f13d6ffcc6e0d9ec981ee1793158bbc9b1d96d5f51ab3,
   v2 policy, total 97 changed bytes (54 prior FD substitutions + 43 UDS-only).
   Final artifact drift proof also binds the version ceiling: future upstream
   versions require fresh UDS qualification and cannot silently publish v1.
   Builder target rerun after this guard: 21 passed, no warnings.
   Temporary native bare TTY stays alive, no fallback/installer/packages,
   native WebSocket config/read succeeds, full-access true, physical socket 106 bytes.
   Actual diff inspected. KEEP upstream protocol, UID/0700 checks and path hash;
   COLLAPSE into one FD boundary; DELETE unmanaged installer dependence.
   Resolved red gates: E0603 utility visibility, E0425 fixture digest helper,
   generated fixture warnings/edition, wrong package selectors (no tests),
   native /tmp EACCES, default-config function-boundary parse error, and a
   rejected mismatched legacy-doctor builder invocation (rerun with accepted R10 metadata).
3. **Cross-account discovery** — declared shared topology and bounded operator
   consolidation; source review of official 0.160.0 thread/list shows no creator
   account predicate. Current external .codex-profiles identities still have
   independent sessions and SQLite; inspect every store before choosing change.
   Proof: existing/new cross-profile sessions, IDs/history, no duplicates,
   unchanged CWD/all filtering and auth isolation. State: source and native disposable gates green; final acceptance next.
   User narrows retention to every conversation active in latest 7 days and
   declines backup. Latest read-only inventory: 28 retained UUIDs / 52 excluded,
   3 CWDs, 0 divergent histories. Active FD audit: 3 retained threads / 2 runtimes,
   0 multiple active copies / 0 active nonmaximal copies. Online inode-preserving
   bounded handoff is therefore the selected route; recheck at apply.
   Production definitions: declared-home topology validation + existing SQLite
   requirements; one-shot operator selection/prefix dedup/atomic inode move/directory exchange.
   Stop-on-red: amended foreign-config regression initially asserted empty
   harness stdout; corrected to assert the runtime version marker is absent.
   Stop-on-red: temporary-root migration gate found Termux Python omits os.link;
   libc linkat also returned EACCES under Android. COLLAPSE onto atomic
   same-filesystem rename for both retained payload and writer-lock inodes;
   DELETE hardlink helper and any copy fallback.
   Focused proof: declared_identity_shares_only_conversations_and_rejects_legacy_or_substitution,
   sharedprofile_requirements (5 combined shared_ tests passed); Python transition
   regression 5 passed, including retained-prefix union, auth/config separation,
   active inode and lock preservation, divergence/dependency/alias refusal, exchange
   failure and staging collision. Native production operator with official runtime:
   3 retained / 1 expired; two identities have exact CWD union 2, all union 3,
   deduplicated, existing cross-identity resume; upstream-created new session is
   immediately discovered and resumed from the other identity. No backup created.
   Resolved probe-fixture reds: v3 activation requires seven records including empty
   previous fields, installed config directory was missing, and native rollout
   listing requires upstream year/month/day topology rather than flat fake files.
   KEEP upstream thread/list/resume and provenance; COLLAPSE declared conversations
   onto one upstream store; DELETE private discovery partitions.
4. **Acceptance/release/live** — grouped workspace/Python/format gates, protected
   state comparison, actual diff review, implementation commit; exact-source
   signed production + public readback/disposable update + ordinary live update;
   installed bare TTY and CWD/all old/new cross-account resume gates.
   State: grouped gate GREEN after resolving its first red run (161 passed, 3 failed, 1 ignored):
   manually launched final exec fixtures inherited the live declared profile;
   shared-server immediate reuse met a transient coordinator lock held across a
   concurrent fork/exec. Isolated the child probe at its own boundary; the final
   TTY boundary fixture explicitly selects upstream embedded mode, while the
   separate bare TTY fixture proves shared-server startup. Bounded coordinator
   waiting is covered by an actually held lock in the signed-process regression.
   Focused final_ 4 and shared_server 3 passed; complete grouped check passed:
   Core 164 + Manager library 16 + Manager integration 9 + builder 21 = 210,
   Python 14; one explicit device smoke ignored. Format and diff checks green.
   Protected file digest/inode comparison: 17 unchanged (launcher, activation,
   resolver, profile auth/config). Actual product/test diff inspected.
   User authorizes bounded live deployment and session proof.
