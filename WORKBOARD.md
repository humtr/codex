# Rust Core Workboard

SPEC -> GOAL -> WORKBOARD; goal-md bound; workers OFF, primary direct implementation.

Current bundle: SWITCH-RENAME-INTEGRATION, selected by the user 2026-10-08.
Bound clean experiment/profile-tui HEAD2ed58ff1148a3e1f5adbf1c03b06a159fb3cd2a3;
rewrite/rust-core b949afafb6a9a8224f7fee0fb4b45fe9aafac48d is its ancestor.
User authorizes ordinary source merge into rewrite and a separate web experiment.
No installed cutover, official signing/publication or account calls selected.

Ordered slices:
S1 CURRENT: baseline restored9/9 Manager and5/5 Python, pristine patch applies.
  SPEC-first rename public native entry
  /profile to /switch with no alias (upstream --profile/config profiles untouched).
  Native SlashCommand dispatch and all current native/probe tests move together.
  Build gate inspects pristine pinned upstream names/aliases before applying the
  patch, refuses collision or unknown name representation, and is required by the
  actual hosted native build. Focus: pristine/current dispatch preservation,
  name/alias collisions and zero-test refusal; actual native focused nonzero gate.
  Protected: upstream other commands/CLI args, source-return logic, all installed
  assets/profiles/auth/sessions/jobs/resolver, main/sealed legacy.
S2: grouped Rust/Python/fmt/Clippy/normal compile, actual native /switch arrows/Enter,
  source-return refusal, fallback and terminal/state protection; inspect actual diff.
  Commit/push accepted experiment then fast-forward rewrite (same orphan lineage,
  no main/legacy import); verify remote exact source. Source integration alone does
  not assert signed generation/frontend admission or replace installed /profile.
S3: create separate experiment/web-provider branch/worktree under prj/web without
  overwriting the existing independent report. First account-free slice repairs
  full HOME/Core isolation, exact artifact attribution and tool result/call-ID/nonce
  proof. Provider/model and execution profile remain separate. No live state calls.

Baseline red: first Cargo command used nonexistent codex-termux-manager package,
exit101, zero tests; rejected as invocation error. Correct package is codex-manager.
Second invocation reached9 tests but all failed because the moved target cache
embedded the former CARGO_BIN_EXE path (missing executable); rejected. Rebuild
only the owned Manager cache to restore artifact attribution. Product mutations
freeze lifted after rebuilt9/9 Manager tests and5/5 Python pass; no product
mutation preceded this runnable baseline.

Deferred coherent signed-generation/frontend pairing, ordinary update/rollback,
actual-account attribution, full history/agents/active-owner chooser remain gates
in GOAL.md; this rename/source merge does not close them.

S1 diff inspection caught an over-broad string edit changing internal module paths.
Corrected before build; internal profile_manager/profile_transport paths retained.
Only slash variant/dispatch/probe text and its command-test name change.

S1 local gate:10/10 Python; exact pristine62 variants and all names/aliases
preserved with Switch as sole addition; patch applies, workflow YAML/all shell
steps parse and diff check passes. Existing25-test log exercised only the log
validator, not new native acceptance. Native hosted candidate remains PENDING.

S1 complete diff found the same broad-edit defect in probe-only profile paths.
Restored every private filesystem path from2ed58ff; only the actual slash input
and docstring change. No native owned invocation used the invalid probe. Hosted
source4c791dd remains exact native candidate; this correction changes no build input.
