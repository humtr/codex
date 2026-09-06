# Rust Core Workboard

This file owns only the active implementation bundle. Accepted evidence and
historical disposition belong in `GOAL.md`; normative behavior belongs in
`SPEC.md`.

## Current routing

- Repository: `humtr/codex`.
- Active branch: `rewrite/rust-core`.
- Bound R3 implementation base:
  `rewrite/rust-core@c9873ae6e678ad0af1d2ff4299a396536b6d51dd`.
- M2-R1 review follow-up and its three-finding remediation are accepted at
  `rewrite/rust-core@33b4bf3f6a4fcff7d2f7bbf67bb1d24b76b73d48`; detailed
  evidence and disposition are recorded in `GOAL.md`.
- M2-R2 independent-review remediation is accepted at the implementation
  commit above; detailed evidence and disposition are recorded in `GOAL.md`.
- The independent product review is complete at
  `rewrite/rust-core@5a7a5292f38876087a5c9b5a41b1dd7e8dbf082b`; the review-only
  clippy cleanup is included there and did not expand product behavior.
- Remote `origin/rewrite/rust-core` remains at
  `253156c37a2bd22af8faae0bce03587999ffd136`; the local branch is ahead and
  no push is authorized.
- Current milestone: MGR-7 remote publication readback and operational
  qualification is definition-bound; no production slice is active yet.
  MGR-0 through MGR-6 are complete in `SPEC.md`;
  Core R9.2 remains accepted in source, live runtime, remote Release/index,
  and disposable-consumer qualification. The Manager definition keeps
  `codex termux` optional and separate from Core ownership, and defines the
  profile/session/notification/repair boundaries. No Manager artifact or live
  state has been changed.
- The inherited pre-commit hook references absent
  `tools/update-wrapper-version.sh`; do not modify it. If it alone rejects an
  exact, revalidated staged tree, use the established `--no-verify` closure.
- `legacy/monolith` remains sealed at
  `bf30a7dc94d4dad7f58836c69028160856e63c58`.
- The pre-promotion local `main` at
  `37f0a775ddc64d1641655a0cc83c0c2e681df704` was not contained by the sealed
  `legacy/monolith` history. It is preserved by the exact local backup branch
  `legacy/main-pre-m2-20260904`.
- The prior local `main` at `57034e4cd2d4f259c9046ac11073dc0b7f7dbb47` was
  preserved before the latest replacement as
  `legacy/main-pre-r9.2-20260905`. `main` is publication authority and now
  contains the accepted `rewrite/rust-core` lineage through the current
  definition checkpoint; this is not a merge or rebase. Remote refs remain
  unchanged and no push was performed; the authorized local live cutovers are
  recorded in `GOAL.md`.
- Worker mode is OFF. The primary Lead owns implementation, validation,
  authority updates, commit, and acceptance.

- R8, R9, R9.1, R9.2, MGR-0, MGR-1, MGR-2, MGR-3, MGR-4, MGR-5, and MGR-6
  are closed above or in `GOAL.md`. MGR-7 is the definition-bound current
  bundle.

## Product-speed policy

- The accepted B11 path is qualification-first. Its concrete legacy gap was the
  absence of an explicit safe handoff from a non-v3 entrypoint to the accepted
  Core; the implementation is limited to the exact bootstrap boundary in
  `SPEC.md`.
- Use a release-built, locked Core and signed local artifacts as the only
  product input. Build outputs stay in private temporary roots; no generated
  artifact is committed.
- The bwrap resolution is an explicit product boundary: Core does not invoke
  or repair bwrap on Termux. Ordinary launch uses the accepted no-sandbox
  policy; explicit unsupported Linux sandbox requests fail closed with a clear
  error; the official input archive may contain bwrap, but final generation
  selection consumes and discards that resource.
- Fresh-Termux and legacy-upgrade checks must target separate disposable
  environments. The current installed launcher, resolver, Manager state,
  auth/profile/session data, and package state are protected.
- Do not copy legacy implementation or internal data models into the rewrite.
  Inspect legacy behavior only through observable qualification outcomes.
- Do not introduce another trust source, promotion wrapper, or compatibility
  layer without first updating `SPEC.md` and mapping a focused regression. R7's
  local path is one transport fallback into the existing signed admission and
  activation authority, not a second updater or an unsigned escape hatch.

## Mandatory execution gates

1. Rebind branch, HEAD, dirty state, authority revisions, and protected live
   identities before every resume.
2. Use the canonical project registry and revision-3 private project root for
   all disposable qualification environments.
3. Keep each slice vertical: production change if any, focused regression,
   nonzero focused invocation, relevant compile/test success, and diff review.
4. Stop on compilation failure, zero tests, stale expectations, unexpected
   warnings/dead paths, or missing proof mapping.
5. Each selected bundle must close with its mapped nonzero focused regression,
   relevant compile/test proof, and actual diff inspection before the next
   independent contract starts.
6. The final bundle batch must repeat grouped acceptance, protected-surface
   verification, authority updates, and the implementation commit.
7. Reserve a fresh independent product review for the completed Milestone 2
   acceptance candidate; this bundle directly closes the recorded findings.

## R7 closure

1. Remote-hit path — completed in the focused
   `test_r7_signed_channel_hit_skips_local_build_and_publication` regression;
   the existing signed index/release acquisition and activation remain the
   unchanged path.
2. Transport-absence fallback — completed in the focused local fallback
   regression; only automatic-channel transport unavailability enters local
   production, while signed-channel verification failures remain terminal.
3. Local production — completed by running the prebuilt release-builder routines with the
   official latest metadata (or one bounded explicit version), bind the exact
   stable version and AArch64 package digest, fetch the exact official archive,
   sign with the current update key, persist publication locally, and feed its
   signed release into the existing local activation path. Completed with a
   nonzero end-to-end fixture using the official metadata shape and a real
   Termux probe binary.
4. Optional publication — completed after local activation only, using authenticated
   `$PREFIX/bin/gh` to publish release files before `update-index-v1` to
   `humtr/codex`/`main`; no key upload and no rollback on upload failure. Add a
   fake-GitHub-CLI regression proving ordering and activation isolation;
   completed for successful ordering and authenticated upload failure
   isolation.
5. Stabilization — completed with the focused and grouped workspace suites,
   clippy, release build, format/diff checks, protected identity checks, actual
   staged diff audit, and commit `55d8a49`. Do not cut over the live runtime or
   push remote refs in this source bundle.

R7 is closed. No new implementation slice is active. A bounded live cutover or
authenticated external publication, if later desired, requires its own explicit
operational authorization; neither is part of this source commit.

## R8 doctor slice map (closed)

1. Human upstream presentation — accepted in the current working tree. The
   normative wording now removes synthetic upstream headings/status
   duplication, and the renderer preserves safe upstream SGR through
   composition. Focused regression: `test_doctor_report_and_usage_keep_bounded_public_contract`.
2. Legacy-shaped Termux presentation — accepted in the current working tree.
   The public route keeps the observable legacy header and Runtime/Support/
   Wrapper/State/Store groups without importing legacy source or models.
   Focused regression: `test_r3_public_main_human_doctor_composes_upstream_and_termux_reports`.
3. Bundle acceptance — accepted pending the authority commit. Focused doctor
   suite: 10 passed; workspace: Core 124 passed/1 ignored and release-builder
   9 passed; locked clippy, release build, format, diff, and protected-surface
   checks passed. Remote publication transport remains a separate follow-up
   bundle because it changes the external release transport.

## R9 remote publication slice map (closed)

1. Immutable asset transport — accepted. The large-generation Contents API
   upload is replaced by an authenticated GitHub Release asset upload using a
   tag equal to the validated generation identity and a matching
   `releases/download/<generation_id>/` base. Asset regular-file/aggregate
   bounds and child-process wait time are enforced. Focused proof:
   `test_r9_authenticated_github_release_publication_is_ordered_and_activation_independent`,
   `test_r9_github_release_asset_inventory_rejects_symlink`, and
   `test_r9_github_publication_wait_is_bounded`.
2. Signed index handoff — accepted. The small `update-index-v1.sig` then
   `update-index-v1` Contents updates occur only after Release asset success;
   the local fallback index and HTTPS-only redirect transport are covered by
   `test_r7_bare_update_transport_fallback_builds_and_activates_local_release`
   and the pinned curl-argv regression.
3. Bundle acceptance — accepted pending the authority commit. Workspace Core
   126 passed/1 ignored and release-builder 9 passed; locked clippy, release
   build, format, diff, and protected-surface checks passed. No external
   publication or live runtime replacement was performed by this source bundle.

No source implementation slice is active after R9. A live update/publication
qualification is an operational step against the already-authorized device,
not source-test evidence.

## R9.1 signed-index publication repair (closed)

1. Base64 serialization — repaired the production `write_github_base64` path so
   a final one- or two-byte chunk cannot reuse bytes from the preceding chunk.
   Focused proof encoded the exact 64-byte `release.sig` shape and asserted
   the canonical bytes; compile and diff inspection passed as slice gates.
2. Publication retry — rebuilt the accepted Core, ran one authorized bare
   `codex update` with the live official fallback path, and verified that the
   resulting Release assets and both signed Contents files were present in the
   required order. The already activated generation was preserved on any
   remote failure; no private key was uploaded and no protected non-Core state
   was mutated. Because the current stable launcher was an older Core artifact,
   the newly built launcher was bound to the signed generation's
   `core_artifact_digest`, and one same-directory atomic launcher replacement
   was performed so the doctor fix was actually made live.
3. Product verification — ran the installed/live doctor and remote-index
   read-only checks, confirmed the index release base resolves to the published
   Release asset set, verified the live activation state, and inspected the exact
   source diff before authority closure.

R9.1 source and live closure passed the focused regression, relevant workspace
compile/test gates, warnings-denied release build, protected-surface checks,
authority update, and one implementation commit. Live publication is recorded
in `GOAL.md` as user-authorized qualification evidence.

R9.1 was closed with no independent source implementation slice active at that
point. The current R9.2 scope is:

## R9.2 Termux doctor color override (closed)

1. Public contract — add the exact `codex doctor --color` mode to the normative
   command and doctor contracts. It is human-only, TTY-only, mutually exclusive
   with `--json`, and does not mutate the caller environment. Focused proof:
   parser and usage regression.
2. Upstream capture — carry the explicit override through the real public
   doctor route, remove `NO_COLOR` only in the bounded child when a TTY is
   available, and retain the existing Termux `script` PTY, SGR sanitization,
   redaction, and upstream-first layout. Focused proof: a fake upstream that
   emits SGR only when `NO_COLOR` is absent, reached through production capture.
3. Bundle acceptance — complete. The nonzero focused suite, locked workspace
   tests, repeated parallel runs, clippy, release build, format/diff checks,
   protected live identity checks, and bounded Termux PTY public-path proof all
   passed. A subsequent bounded local signed publication activated generation
   `local-1788578457-0-1` and atomically reflected the accepted Core launcher;
   the five Release assets and ordered signed index publication are complete;
   the disposable automatic-channel consumer qualification is also complete;
   no source-history push was performed.

R9.2 source closure is accepted with the evidence recorded in `GOAL.md`. No
source implementation slice is active after R9.2; its authorized live
reflection is complete.

## MGR-0 Manager v1 definition (closed)

1. Ownership boundary — `SPEC.md` now makes Manager optional, independently
   qualified, and unable to write Core generations, trust, activation, resolver,
   or journal state. Core remains the only owner of update, rollback, doctor,
   installation, and runtime execution.
2. Public command boundary — the four Manager families are defined as
   profile, session, notification, and repair. Unknown `termux` commands do
   not fall through to upstream, and legacy runtime install/update/remove
   paths are not reintroduced as Manager authority.
3. State boundary — the Manager root, profile-home containment, versioned
   metadata, atomic publication, credential/session-content exclusions, and
   child-only `CODEX_HOME` behavior are defined. No implementation or live
   state change is part of MGR-0.
4. Next-slice mapping — MGR-1 owns profile grammar, profile metadata and
   create-new publication, list/current behavior, isolated Core launch, and
   the associated protected-surface proof.

## MGR-1 profile selection and isolated launch (closed)

1. Exact grammar — closed in source: `profile list`, `profile current`,
   `profile create <PROFILE_ID>`, and `profile use <PROFILE_ID> [--]
   [UPSTREAM_ARGS...]` parsing rejects traversal, reserved names, malformed
   IDs, and unsupported Core selectors before mutation; focused parser tests
   are nonzero and passing.
2. Core handoff — closed in source: the qualified Manager execution boundary
   provides and validates the versioned Core API/entrypoint handoff, while
   keeping Core-owned selectors out of the ordinary upstream argv path; the
   Core probe and Manager invalid-handoff regression are nonzero and passing.
3. Profile state — closed in source: publish `state-v1` and `profile.meta`
   through the declared create-new/atomic path, reject symlinked roots and
   collisions, and prove
   that no auth, session, log, Core, or resolver file is read or written by
   Manager.
4. Isolated launch — closed in source: invoke the validated Core entrypoint
   with default `CODEX_HOME` removed or a custom profile home set only in the child; the
   integration proof covers raw non-UTF-8 argv, standard streams, PTY, signal,
   exit status, handoff removal, and caller-environment preservation.
5. Bundle gate — closed: Manager unit tests passed 9/9, public integration
   tests passed 4/4, the Core handoff regression passed 1/1, and the optional
   Manager build/publication regressions passed 2/2. The locked workspace suite
   passed Core 128/0/1 ignored, Manager 9/9, Manager integration 4/4, and
   release-builder 11/11. Locked workspace clippy with `-D warnings`, the
   warnings-denied release build, formatting, diff check, and the release-mode
   disposable profile-root smoke 4/4 passed. Protected live identities were
   unchanged and no live Manager artifact or state was changed.

Worker mode remains OFF; the primary Lead owns every slice, validation step,
authority update, and acceptance decision.

MGR-2 source closure is accepted with the evidence recorded in `GOAL.md`.
No MGR-2 implementation slice remains active, and no Manager artifact or live
state was changed.

## MGR-3 notification configuration and delivery (accepted)

MGR-3 is closed with implementation, focused regressions, grouped debug and
release acceptance, protected-surface verification, and authority evidence in
`GOAL.md`. No notification slice remains active. `target/` remains untracked
Cargo output and is outside the publication surface.

## MGR-4 repair planning through Core (accepted)

MGR-4 is closed with implementation, focused regressions, grouped debug and
release acceptance, protected-surface verification, and authority evidence in
`GOAL.md`. No repair slice remains active. `target/` remains untracked Cargo
output and is outside the publication surface.

## MGR-5 Manager artifact build and qualification (accepted)

MGR-5 is closed with implementation, focused regressions, grouped debug and
release acceptance, protected-surface verification, authority evidence in
`GOAL.md`, and the implementation commit. No Manager artifact or live state
was changed; `target/` remains untracked Cargo output outside the publication
surface.

## MGR-6 Manager artifact distribution and disposable qualification (accepted)

MGR-6 is closed with the signed-manifest-to-asset completeness fix, focused
regressions, release-mode fresh/legacy Manager consumer proof, grouped debug
and release acceptance, protected-surface verification, authority evidence in
`GOAL.md`, and the implementation commit. No live runtime, remote ref, or
Manager state was changed; `target/` remains untracked Cargo output outside
the publication surface.

## MGR-7 Remote Publication Readback and Operational Qualification

MGR-7 is definition-bound in `SPEC.md` before product or external-state
mutation. It adds no public command, persistent state, trust source, or
release format; the current workboard is definition-only and no production
slice is active.

1. Candidate binding — record one accepted generation ID, local publication
   directory, fixed repository/branch target, and whether device qualification
   is requested. Do not infer any of these from live active state.
2. Authorized publication — when separately authorized, exercise the existing
   Release-assets → `update-index-v1.sig` → `update-index-v1` ordering and
   prove Manager asset completeness, timeout/failure isolation, no key upload,
   and no OpenAI-repository target.
3. Signed readback — in a private disposable consumer, fetch the signed index,
   control files, and every inventory asset over bounded HTTPS; verify the
   Manager digest/mode and run the actual release-built Manager probe/handoff.
4. Device qualification — use separate private fresh-install and legacy-
   upgrade roots; run doctor and isolated Manager checks and verify argv,
   streams, TTY, signal, exit, child-only `CODEX_HOME`, ANSI output, bwrap
   non-use, and protected identity preservation.
5. Bundle gate — focused nonzero regressions, grouped debug/release tests,
   clippy, warnings-denied release build, formatting/diff checks, protected
   surfaces, authority update, and commit. MGR-7 source work does not itself
   authorize remote push or live runtime replacement; those require a separate
   exact-target operational authorization.

Worker mode remains OFF; the primary Lead owns every future definition,
implementation, validation, authority update, commit, and acceptance decision.
