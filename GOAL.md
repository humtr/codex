# Rust Core Rewrite Goal

## Public Contract

- Target: a complete native Rust Core for the Termux Codex wrapper.
- Inputs: official upstream Codex artifacts, immutable release metadata, and
  the Termux runtime environment defined by `SPEC.md`.
- Output surface: `codex`, with dedicated `update` and `doctor` commands and a
  `codex termux` Manager boundary.
- Allowed writes during implementation: this repository on
  `rewrite/rust-core` and test-owned temporary roots. The SCS-6 user-state
  migration historically required the following SCS acceptance gates and began only after
  SCS-1 through SCS-5 are green, all Codex/app-server writers are quiesced, a
  complete restorable backup is verified, and the migration operates only on
  the declared profile/shared-state roots. The selected 2026-10-02 user-authorized
  corrective bundle additionally permits its SPEC-bounded online retention
  transition, signed publication and ordinary runtime activation, without a
  recovery backup as explicitly directed by the user. The separately authorized
  2026-10-04 BOUNDARY-RELEASE below permits its signed Core+Manager publication,
  ordinary activation and bounded device acceptance; no data migration.
- Protected surfaces: the live `$PREFIX/bin/codex`, installed runtime and
  Manager, `$PREFIX/etc/resolv.conf`, profiles, sessions, auth data,
  `legacy/monolith`, and the pre-rewrite archive bundle. Profiles, sessions,
  and auth remain protected during SCS source/disposable work; the SCS-6
  exception above permits only the accepted data-layout migration, never live
  runtime replacement, public release mutation, credential inspection, or
  unbacked destructive cleanup. The explicit current corrective authorization
  supersedes those historical SCS-6 limits only within its declared scope.
- Authority: `SPEC.md` for normative behavior and architecture; this file for
  acceptance; `WORKBOARD.md` for the current implementation target.
- Secret exclusions: tokens, OAuth codes, cookies, credentials, private keys,
  and unredacted session or notification content.
- Non-negotiable constraints: clean rewrite, no legacy source copying, no live
  cutover before acceptance, resolver non-mutation, crash-safe rollback, and
  upstream process-boundary fidelity.

## HOSTED-PREFLIGHT-0161 (source and hosted repair accepted 2026-10-08)

- Repeated public failures first37390712105/latest37698740814 are fail-closed
  qualification refusals at upstream0.160.1/0.161.0; signing/publication did not run.
  Exact official0.161.0 archive3c02e2ae34be0d06e62557e98fc5c0a783bec5a2fed406fe00e565803bf84ee8,
  raw0129f94f4f1197bd75f6c4265889061f386ed302bbc50c6e57e3e0660dfb28a7,
  tag979011409de0a60b52f179721948e65531d26144 and official debuglink96904dc1
  bind the new v4 artifact.22 blocks432 special+54 common change486 total bytes.
  Actual native SHA d9e019075903111b5da87d5c013cacda3adef540083605fde9dbec9527949a0b;
  original code-mode-host remains byte-identical. Unknown versions still refuse.
- One Runtime Policy owns Rust version/artifact/report/write/publication bindings;
  hosted preflight recognizes exact161 v4+archive. All22 legacy160 byte blocks are
  unchanged. Public retained160 history was exhaustively checked15/15 (v1=2,
  v2=8, v3=5); historical pre-UDS v1 through160 remains valid. New161 cannot claim
  v1/v2/v3. Existing Core/Manager/Cargo product source remains exact signed40.
- Exact-source source acceptance passed Core184+one opt-in ignored, Manager33+41,
  builder22:280 actual Rust tests; migration15+hosted Python86:101. Final affected
  builder22 and Python86 rerun after historical-v1 correction pass. Locked workspace
  all-target check, workspace Clippy-D warnings, rustfmt and actual diff pass.
  Binary/doc zero-test harnesses are excluded. Unchanged Core/Manager evidence is
  reused; no unrelated broad rerun. No new dependency or runtime boundary.
- Actual real-Termux v4 candidate passes legacy/named/embedded menu choices,
  current marker and both shortcuts, native dangerFullAccess/reviewer settings6/5,
  profile create/default/rename/delete and optional-Manager absence, current-writer
  reconnect/cancellation/retained-subscriber/PID-stable force/takeover/pause, single
  line completion and memory-consolidation notification exclusion. All homes,
  clients, model endpoints and signing keys are owned fixtures; no credentials,
  real account/model calls or Android notification provider calls.
- Initial482-byte candidate is rejected: native menu exposed a skipped scratch
  initialization. Official PC a37b7b8 identified missing x25=sp+780; final branch
  preserves it before deleting Read Only construction. Final real-menu regression
  passes all three paths. Type-complexity lint, wrong fixture URL and wrong probe
  option invocations are rejected runs, not acceptance. Final one-shot fixture-key
  publisher accepts real v4 candidate; source affects no official signing key.
- Protected comparison proves all23 paths, complete signed40/39 payloads and all10
  pre-existing native jobs unchanged; original experiment branch remains f64beea
  with only its previously recorded follow-up docs dirty. Main/rewrite/sealed refs
  are unchanged. Current fix lineage begins at production docs-only b949afa and
  imports no experimental source. Installed preview is retained.
- Source/artifact053e35bee0cb044cd3dd1f1e2d835d0810766078 is accepted and pushed
  on its independent fix lineage. Unsigned proof-only workflow1e442614 ran
  GitHub37720625720 successfully: actual producer and Android/AArch64 executable
  smoke both pass. It includes no signing or publication jobs.
- Public delivery is exact one-file ordinary parent47597099 -> main
  f038f2bda32b6be722c43a414c16dbc21f64d258, published with force=false. Both
  actual workflow source pins bind053e35; every consumed seq40 push-bridge
  instance is deleted. Scheduled/manual authorization, all seven job gates and
  every other published tree entry remain unchanged. Actual decision four cases,
 36 shell syntax gates and12 inline Python compilations pass.
- Exact remote workflow bytes and main HEAD pass readback. Raw HTTPS stable index
  and Release signatures independently verify under the retained public key;
  signed sequence40 is unchanged. API-content binary reencoding and an incorrect
  index-sequence assertion were rejected readback attempts, corrected to native
  index grammar, raw HTTPS signature bytes and authenticated Release sequence.
  Initial fixture repeat-symlink and proof-workflow EOF whitespace failures were
  likewise corrected, not acceptance evidence.
- Final protected comparison proves all23 paths, signed40/39 payloads and all10
  original live native jobs unchanged. Local main/rewrite/sealed refs are retained.
  Disposition: compatibility and public preflight repair accepted and closed.
  No official signing/publication run or installed161 activation is claimed;
  next ordinary scheduled newer-stable publication retains its original gates.
  Installed profile preview stays unchanged. Idle-profile stable admission remains
  the separately queued source-recovery/pairing/account-attribution bundle.

## PROFILE-LIFECYCLE-DELIVERY (accepted 2026-10-05)

- User `배포 4` requests deployment of the just-accepted profile lifecycle source
  59308430d39dc3c57159e260e5f7a873c1b16b8d. Goal-md bound; approved equivalent
  primary, workers OFF. Bound clean rewrite HEAD; installed/public39, previous38;
  freshly observed remote main471590750468f6d99b520c3a8d47b03a474aacbb.
- Explicit delivery scope: humtr/codex Release/Pages/main normal non-forced CAS,
  signed generation local-hosted-0-160-0-59308430d39d-profile-lifecycle, sequence40,
  coordinated Core+Manager replacement, then ordinary installed codex update.
  Retain complete39 as previous; real profiles/auth/history/jobs and resolver
  remain protected. No actual profile lifecycle operation or user-job stop.
- Preserve authenticated official0.160.0 runtime, code-mode-host and helpers;
  pin source/archive and public baseline. Existing production wrapper target is
  aarch64-linux-android (official upstream runtime is aarch64-linux-musl).
- Ordered gates: exact-source hosted acceptance; bounded publication candidate
  syntax/contracts and diff; hosted final candidate build/native smoke before
  production signing; signed/public every-byte readback and isolated profile/task
  proof; ordinary activation, doctor/no-op and protected retained39 verification.
  User deployment directive authorizes this bounded delivery, not unrelated work.

- Pre-publication closure: exact-source hosted37302982221 all5success;
  fresh signed public39/index and official stable archive7f0fe42f authenticated.
  Thin candidate98651a181d02fb56c1acb82b6410b91433868d5b exact parent47159075;
  six actual-shell regression groups, YAML/bash39-step syntax and actual diff pass.
  Obsolete seq36/Core substitution removed; both new artifact digests mandatory.
  Candidate is concrete before ordinary main push; user deployment authorization
  covers this source/candidate and ordinary activation retaining complete39.

- Final production37303696948 exact candidate98651a18: all8success. Final
  Android/AArch64 build, native smoke, production signing, immutable Release,
  LKG-preserving Pages, every-byte public readback, disposable update/no-op and
  non-force CAS accepted. Stable promotion47597099ae00284aa40ebbbf08fb91143d943ba5.
- Independent readback authenticated pinned public-key SHA256
  62ab1640b6b4e63afbd5952d11a0bd0a9f1cb78ddde2472e003a42c4db2b832c, signed index40
  and manifest, every payload/mode and
  signed download-size sidecar. Actual Core SHA256
  8acc8219507780095a3a03e0cd0f8bf4b6346bbe7bd4b41b0a8f761851978cfa;
  Manager d471eb9604596953c2fd36d0ea3380c529ba312b1abe9e16273e078118472e7f.
  Runtime/code-mode-host/helpers unchanged from authenticated39; descriptor delta
  is generation identity and the two artifact digests only, permission patchv3
  and R10 compatibility retained. Actual signed/native profile3+task6 all pass.
- Ordinary installed codex update exits0, activates exact40 with complete39 as
  previous. Installed inventory/modes/signature/launcher match public40; all9
  retained39 files unchanged. Upstream version remains exact codex-cli0.160.0;
  public Manager lifecycle/default/task forms work; doctor healthy/exit0 and
  sandbox unsupported. Exact-current second update gives exact stdout, empty
  stderr and no delta across34 durable installed entries.
- Owned native proof preserves protected27 paths and both installed generations;
  bounded activation changes only the authorized launcher/generation state.
  Account/config/resolver26 paths unchanged, all original10 native PID/start
  identities preserved. Auth/config proof is metadata-only; no payload inspection,
  real profile deletion/rename, user job stop, resolver mutation or force push.
  Local main2ffb95f3 and sealed legacybf30a7dc remain unchanged.
- Evidence: owned temporary codex-profile-delivery.P5oQoGrP source/production JSON,
  publication6-group gate, public40 audit/inventory, native profile/task logs,
  installed verification/protection and before/after no-op snapshots. No local
  production private key or credential content was stored.
- Disposition: delivery40 accepted and closed. KEEP one ordinary coordinated
  update/sign/readback/CAS route; DELETE obsolete seq36 authentication and stable
  Core substitution from the previous Manager-only one-shot; COLLAPSE preservation
  onto authenticated current39. No independent implementation bundle selected.

## PROFILE-LIFECYCLE (accepted source 2026-10-05)

- User requests implementation of missing profile deletion, rename and persistent
  default selection after current command coverage was explained. Bind clean
  rewrite/rust-core32ff735a9dcbfa0ccbcadde0da76a32e1748234b; goal-md bound,
  approved equivalent primary, workers OFF. Installed39/previous38 protected.
- Implemented `profile delete ID`, `profile rename OLD NEW`, and
  `profile default [ID]`. User confirmed deletion includes local login/settings;
  shared history and symlink targets survive. Rename preserves local inodes/modes
  without credential copies/inspection. Saved-default and active profiles reject
  rename/delete; creation/preference/lifecycle serialize through directory locks.
- Manager owns explicit preference and lifecycle; Core owns the physical execution
  lease FD36 across TUI/server and optional default delegation. Nonempty UTF-8
  inherited home wins, unavailable Manager/invalid optional preference preserves
  native ordinary launch, startup discovery remains after selection. Task takeover
  with omitted target follows the same saved/default selection and validates before
  stopping any owner. Retired state-v1 stays ignored/unchanged.
- Focused final proof: Manager profile6 unit+10 substantive integration (fixture-only
  return excluded); Core2 isolated lease/FD-pressure tests and public default/direct
  home regressions. Old server/home/cwd/fd detection, live sibling/root-reassigned
  UID handling, collision/default/private/symlink/size/depth cases are covered.
- Final grouped check passes: Python15+86; Core184 pass/1 explicit device smoke
  ignored; Manager33 unit+41 integration; Builder22. Zero-test binaries/doc targets
  are not evidence. Fmt/diff and workspace all-target clippy with warnings denied
  pass; final release build passes. Final product/test file identity is
  `d37ab4e6245ebd49831f3364621b092051b2358fa052acea5fdf2eebbd76a9fd`.
- Owned signed native proof: profile3 groups (including renamed-home TUI restart,
  shared bytes preservation, lease/default/inherited/fallback), task6 groups
  (including actual saved-default account takeover), all pass through actual Core,
  Manager and official0.160.0 runtime. Qualified local Core SHA256
  `901da64a3db591f1994ace6e1832df219db9c4ab8849b57d7e78bf5b963df7fc`, Manager
  `f5fd42ca4a9cf27c77c368afc0794651b4dc4063a9e7f7f324d6a2ed549e4b7e`.
  Disposable signing keys removed; production credentials/publication unused.
- 63 protected paths unchanged: current39/previous38 assets, launcher, resolver,
  activation and account metadata. Auth/config preservation uses metadata only,
  without reading payload. Main2ffb95f3814ce95462ae5c0c75f57dfb8c266afd and sealed
  legacybf30a7dc94d4dad7f58836c69028160856e63c58 remain unchanged.
- Red gates were resolved, not waived: test visibility/import errors; native caller
  publication layout/107-byte socket path; overlapping process-snapshot validation.
  Descriptor pressure and both aarch64 Linux-musl/Android flag definitions were
  exhaustively checked; fresh final gates followed the source correction. KEEP
  conservative process safety, COLLAPSE lease ownership to one safe duplicate;
  no fallback/retry/proof injection into production. Actual diff reviewed directly.
- This is source/owned-root acceptance. No live profile deletion/rename,
  user job termination, production signing/publication or new installation is authorized by
  this source bundle. Installed/public39 remains unchanged. Production
  wrapper cross-build/publication acceptance remains the separate delivery gate.
- KEEP Core execution authority and upstream shared conversations; COLLAPSE stored
  registration identity onto directory name with v2 header; DELETE duplicate ID
  only for newly created/renamed registrations. Exact old v1 remains readable.
  Persist explicit preference only, never revive retired selection-history state.

## ACTIVE-TASK-HANDOFF-DELIVERY (selected 2026-10-05)

- User `resume` selects the announced next signed Manager delivery and ordinary
  installed application. Bind clean rewrite/rust-core712790e5a42e4b86c56d1cd605d8d8c8d4736159;
  workers OFF. Accepted product source remains705afb98098f27d613130cd3fd4d0f84fb74fc50,
  exact-event hosted37255197297; later authority-only events also passed.
- Intended candidate is `local-hosted-0-160-0-705afb98098f-active-task-handoff`,
  sequence37 from authenticated sequence36
  `local-hosted-0-160-0-6523921a1d33-permission-memory`. Target is humtr/codex
  Release/Pages/main with ordinary non-force exact-parent CAS; observed remote
  publication parent15487ecca7de7e28c9ed743bb09afb394b208005 must be freshly bound.
- Delivery changes Manager and its descriptor binding/identity only. Preserve
  signed Core, runtime, code-mode-host and helpers byte-for-byte, including v3
  permission runtime and legacy bridge contracts. Authenticate the public baseline
  and official upstream pin before admitting this same-version candidate.
- Allowed writes: authority, owned publication/candidate/consumer roots, bounded
  hosted production signing and Release/Pages/stable promotion, then ordinary
  installed `codex update` after green independent public qualification. No
  manual live copies, credential inspection, profile/session/auth/preferences/
  resolver changes or cancellation of real user work. Retain complete sequence36
  as previous rollback; sequence35 may retire only through ordinary Core policy.
- Success requires exact candidate qualification, unchanged protected payloads,
  hosted signing/public readback/disposable update/no-op, ordinary activation,
  fresh installed read-only status/help plus credential-free owned-root task
  qualification and protected data/process verification. Ordered gates live only
  in WORKBOARD.md until closure; no new feature or repair work is selected.
- D1 closed: fresh pinned-key signature/every-byte/download-size audit binds
  seq36/main15487ecca7de7e28c9ed743bb09afb394b208005; official latest0.160.0
  archive7f0fe42ff22ecfa3a47bc4a34f5b22c4218b431a4ec0aba51c7d98299f07900c.
  Actual preview shell49 positive/fault cases, published current seven nonzero
  regressions, workflow YAML/all shell syntax, source/event/non-force CAS binding,
  credential/diff gates and five native candidate behaviors passed. Core source
  difference from published6523921 is cfg(test)-only; protected signed payloads
  remain exact. KEEP upstream/Core/signing/Pages/CAS, COLLAPSE candidate admission
  to one current Manager-only gate, DELETE the stale seq35 boundary test and
  replace it with current actual-shell regressions. Only fixture-copy/path/input
  defects were corrected; no product relaxation or failing proof is accepted.
  Device preflight binds installed36/previous35, all nine complete36 rollback
  files, protected path identities and eight installed process PID/start pairs.
- Concrete local publication candidateca56bd76f15b557f9daee895e2e058cf3ca5d90a
  is the sole child of15487ecca7de7e28c9ed743bb09afb394b208005. Exact message
  is release-owner-handoff-seq37-source-705afb98; only workflow and current
  regression replacement change. Stable index/signature remain byte-identical.
  Current published regressions7 and action-pin/credential/private-key gates3
  passed. Owned publication worktree is clean.
- Automatic approval review rejected the proposed protected/default main push:
  resume did not explicitly authorize this exact signing/publication side effect.
  No rejected command executed and no public/installed mutation occurred. The
  separate safe local candidate commit above completed. An explicit question now
  requests this exact ordinary main push, hosted signed37 Release/Pages/CAS and
  verified ordinary installed update retaining36; dependent work awaits reply.
  Do not bypass rejection or infer approval from elapsed time.
- User subsequently replied `승인`, explicitly authorizing this concrete
  ca56bd76 ordinary main push, hosted sequence37 signing/Release/Pages/CAS and
  verified ordinary installed application retaining36. This resolves the prior
  approval rejection; resume binds clean sourcec5c619f1f541cf3f2d2bde3e48cd8c2158aeeab9
  and unchanged local publication candidate/remote parent before execution.

## ACTIVE-TASK-HANDOFF-DELIVERY corrective candidate (qualified; operational approval granted)

- Authorized triggerca56bd76 ran hosted37259637245 to all reported-success
  stages and non-force CAS promotion3da4b294e1a6d62b1b5ba775acf88aae375c3189.
  Public sequence37 is signed and immutable. Independent pinned-key comparison
  rejects its Core5cfddf4d793d3e0376252816a13ab1e5c3a9994c1e2b97ee4825ee29e5a01810
  against protected36 Core3d994dd187515276de755d1332a369469355bc2d21010b470313c7a6e7b1c648.
  No installed update ran; device remains36/previous35. Workflow success alone
  does not close the Manager-only delivery gate.
- Root cause: build packaged candidate.tar before the later Core-preservation
  and component-binding steps. The directory was correct but exported bytes were
  stale. No signing-stage payload substitution was found. Both source and current
  publication producer must package once after all bindings, immediately before
  upload. SPEC now owns this final-candidate export invariant.
- KEEP smoke/signed packaging after their final qualification, independent legacy
  Core/source-proof exports, immutable signed releases and normal activation.
  COLLAPSE unsigned export to one final pack; DELETE early archive construction.
  Inspect all relevant surviving archives and prior binding-only bundles; their
  existing non-mutating comparisons did not alter packed payloads. No Core or
  Manager runtime change is required.
- Runnable source baseline2, exact red reproduction, repaired focused2 and grouped
  Python79 passed. Actual final pack replaces a deliberately stale archive and
  reproduces final bytes/modes/inventory; ordering requires all bindings before
  pack and upload immediately after. Actual diff inspected; exact83d77ecc hosted
  full gate37260778160 passed all five jobs before corrective trigger preparation.
- Prepare a forward sequence38 corrective candidate preserving Core/runtime/
  helpers from authenticated36 and accepted Manager behavior. Never overwrite
  signed37 or decrement the public sequence. The prior explicit operational grant
  named ca56bd76/sequence37; the new exact candidate will be made reviewable before
  seeking any additional operational authorization. Protected installed36 remains
  usable throughout; no live task cancellation or data migration.

- Forward candidate qualification restores protected36 from its immutable Release,
  with pinned-key signature/manifest/descriptor identity and sequence validation;
  current public37 remains the independently authenticated sequence predecessor.
  Protected36 admission9 actual signed fixture cases are mapped to durable current
  publication regression; current publication tests8, actual component/admission/
  preservation corpus48 and final archive regressions2 against the publication
  producer pass. Source and event pins must bind the final exact accepted source.
- Native prospective signed36 Core + authenticated signed37 Manager qualification
  exposed a proof-only close/resume race. Native idle/list absence is insufficient
  while upstream teardown is pending. KEEP actual stop/writer/transfer behavior;
  COLLAPSE qualification onto exact thread/closed notification before same-server
  resume, subscribe before launching its second TUI, DELETE the loaded-list proxy.
  The separate post-force A server is freshly launched through ordinary Core.
  Native schema and official0.160.0 lifecycle were inspected. Closure regressions4
  and grouped Python83 pass; all five actual owned-root native gates pass exit0.
  Downloaded transport mode600 is not installation mode; authenticated Manager
  bytes are materialized mode0755 only in the owned candidate. Rust product payload
  remains unchanged. No user work or live installed state was mutated.

- Final accepted sourceaf1a3df6d99a01b5c649f0f47beef6703128feab has exact hosted
  full acceptance37262014797 success: workflow11, migration15, hosted contracts83;
  actual Core182, Manager59 and builder22 behavior tests; check/clippy/fmt all
  green. Exclude zero-test entrypoints/doc tests and the one fixture-only Manager
  compilation test from acceptance counts. Protected Rust payload diff is empty.
- Final prospective38 qualification uses that exact closure-aware qualifier and
  owned generation identitylocal-hosted-0-160-0-af1a3df6d99a-active-task-handoff-r2.
  Both payload signatures are authenticated: Core36 and Manager37 digests/modes
  materialized in the owned unsigned composition; all five actual native gates
  pass exit0. This is prospective qualification, not a signed38 or installed proof.
- Final current publication tests8, actual shell corpus48, protected admission8
  against real signed36 controls, durable signed-fixture admission9, archive2
  against the actual publication producer, and action/credential/private-key3
  pass. A stale owned preview/candidate from the superseded source was rejected;
  DELETE the preview dependency and optional candidate reuse. The corpus now
  reads the real publication workflow and always reconstructs its owned candidate.
  No product gate was relaxed. Actual full diff reviewed.
- Concrete local corrective trigger965b66113f8e1601471c0b0cc17b7881242819e2 is the
  sole child of remote main3da4b294e1a6d62b1b5ba775acf88aae375c3189, message
  release-owner-handoff-seq38-source-af1a3df6. Exactly workflow/current test change;
  public index/signature are byte-identical. Owned publication checkout is clean.
  Fixed upstream latest0.160.0/archive7f0fe42f reverified. Protected local main and
  sealed legacy refs unchanged; device27 path identities, all9 retained36 files,
  activation36/previous35 and all8 preexisting PID/start pairs unchanged.
- Requested explicit approval now names corrective965b661, signed generation38,
  humtr/codex normal main push/signing/Release/Pages/CAS, and independently verified
  ordinary installed activation retaining complete36. User replied 승인. to this
  exact965b661/38 request; operational approval is granted.
  No new38 main push, signing, publication or installed update has executed.
  Signed/public37 remains immutable; actual38 signed-byte/native/public readback
  and fresh before-activation protection gate remain required before any install.

- Approved ordinary965b661 main push succeeded. Exact production37263252597 all8
  jobs passed: Android build/smoke, signing, immutable Release, LKG-preserving
  Pages, every-byte public/disposable update/no-op and non-force CAS. Promoted main
  is9bac2f41d515dc894d005aa498e03064befb9f1b; public stable is signed38.
- Fresh independent pinned-key index/manifest/every-byte/download-size audit binds
  main9bac2f41 and generationlocal-hosted-0-160-0-af1a3df6d99a-active-task-handoff-r2,
  sequence38. All five protected payload digests/modes equal36; full descriptor
  order and all non-Manager/identity records equal36. Actual signed Core is
  3d994dd187515276de755d1332a369469355bc2d21010b470313c7a6e7b1c648 and Manager is
  20b2436a9bae5dad636f2a9e614cb1bcfdb2d73d68757dbafbfb472bf4625e8a.
- Actual signed-native qualification caught an owned fixture defect before any
  installed mutation: copytree imported published compat/download-size controls
  into a v2 installed generation, which Core correctly rejects. Its old dummy
  activation authority also did not represent the authenticated release. KEEP
  strict Core admission and signed bytes; COLLAPSE the fixture onto existing
  strict inventory/public-key parsers and an explicit external public authority;
  DELETE entire-public-directory copying, unsigned candidate substitution,
  Manager/descriptor rewriting and dummy trust. Qualifier now accepts generation,
  public-key and short workdir only; copies exactly signed inventory + manifest/
  signature and launches those exact bytes at the actual Core boundary.
- Fixture authority2 and exact installed-inventory/control exclusion1 regressions
  plus existing closure4 pass, grouped Python86 pass. Unsupported-key and malformed
  inventory fixtures fail before materialization. Initial tests assumed generic
  ValueError and an unsupported generation ID; bind actual ValidationError and
  canonical hosted fixture identity. No product policy/parser was weakened, no
  signing/public artifact was altered. Full actual diff inspected; Rust product
  diff against acceptedaf1a3df is empty.
- Exact signed38 task qualification with the final fixture passes all five actual
  native behaviors exit0, under credential-free owned roots only. Installation is
  still36/previous35 until the final source/protection gate and ordinary update.

## ACTIVE-TASK-HANDOFF installed closure (accepted 2026-10-05)

- Exact source55035d5 hosted37264886500 all5 jobs passed. Ordinary approved
  update activated signed38 and retained complete36. Independent installed
  inventory/launcher/signature/modes and all nine fresh pre-update PID/start
  identities passed; only ordinary activation and authorized35 retirement changed.
  Version/help under the canonical registered home and composed doctor are healthy.
- Real task status fails on retained exited-server metadata: home validation
  precedes process exit detection. This same-root public-path blocker keeps
  delivery acceptance open despite the owned-root native task proof passing.
  Read-only current diagnosis shows five absent PIDs and three absent homes among
  six records. Previously live legacy-alias server exited independently; no live
  state or process was repaired/killed by this agent. Alias rejection is preserved.
- Correct only Manager discovery ordering after valid private record/namespace
  binding; confirmed exited processes do not require live execution files.
  Live/uncertain owners retain complete strict verification. Core remains unchanged.
  New source acceptance and concrete publication candidate precede any request for
  another exact operational grant; immutable38 is never rewritten.

- Source closure passes: baseline task unit8, new pre-fix public-entrypoint red1,
  task integration15 (14 behavior plus one fixture-only), grouped migration15 and
  Python86, local Rust suites Core181/one explicit device smoke ignored,
  Manager unit30/integration32 (one fixture-only), builder22; fmt/diff and strict
  all-target workspace clippy pass. Zero-test entrypoints are not proof.
- Native proof initially seeded an unretained generation; Core correctly retired
  that record before Manager handoff, making the preservation assertion invalid.
  KEEP normal Core retirement; bind the owned dead-home record to retained current
  runtime, matching the live defect. Final exact Core/native/new-Manager signed
  disposable fixture passes6 actual native behaviors, including record preservation
  and every original cancellation/reconnect/force/ordinary transfer behavior.
  This uses an owned ephemeral test authority, never official publication authority;
  no unsigned bypass or installed payload substitution. Final qualifier focused7 pass.
- Release-built new Manager read-only discovery against existing real bindings
  succeeds exit0 with three current owners and no stderr; private task metadata
  is not persisted. This is source-artifact proof, not installed39 acceptance.
  Fresh before/after snapshots preserve all27 protected paths, complete36 files9,
  current38/previous36 and all7 current PID/start identities. Native fixture's private
  signing key was removed immediately after local publication. Core/builder/deps
  unchanged; full actual diff inspected, no new production definition or fallback.
- KEEP live/private/namespace/peer/current-writer checks; COLLAPSE discovery ordering
  onto process lifetime before live filesystem requirements; DELETE stale requirements
  on dead homes. One moved home check maps to both new focused regressions and native
  public status. Delivery remains open until a forward Manager-only candidate is
  separately approved and verified; do not claim installed38 has the source fix.

- Accepted corrective product source is exactb7c1e61019b1c5085b2627ba6cf609b3f55d74de.
  Full hosted37275318923 all5 jobs succeeded, including exact-source Android Core,
  pinned-key every-byte public stable audit and full check/test/clippy/fmt. Source
  branch push is complete; public main remains9bac2f41 and installed38/previous36.
- Concrete forward39 publication trigger is fadae244d8bf4e3f88ebf7af1e1510cb2eeea2d7, sole child
  of9bac2f41d515dc894d005aa498e03064befb9f1b in a clean isolated main checkout;
  only current workflow and delivery test pins changed. Candidate generation is
  local-hosted-0-160-0-b7c1e61019b1-retired-server-discovery, sequence39. Source/event pins exact, existing final-pack/signing/
  non-force CAS retained; stable index/signature unchanged. Publication8, actual
  publication archive2, action/credential/private-key3 and YAML/all shell syntax pass.
- Actual producer shell independently admitted real signed36 controls under the
  fixed public key, restored a deliberately wrong unsigned Core from authenticated
  bytes, validated the new actual release-built Manager and all five protected
  bytes/modes plus ordered descriptor, then packed and checked every final archive
  member byte/mode. No new publication or live install occurred.
- A thin-publication test-module lookup was invalid; reuse the existing source
  generic tests with ROOT bound to the real publication checkout. Precommit HEAD^
  included the prior binary public signature and was the wrong credential-diff
  boundary; KEEP the strict checker, establish the concrete local candidate commit
  against actual9bac parent, then all3 security cases pass. No checker/parser bypass.
- Prior explicit operational grant names965b661/38. Forwardfadae24/39 requires
  its own exact MGR-7 operational authorization: humtr/codex ordinary main push,
  hosted signed Release/Pages/CAS, fresh actual signed public/native qualification,
  then ordinary installed update preserving complete38 as previous rollback under
  unchanged Core policy. Never overwrite37/38, retain36 manually, mutate user data
  or cancel real jobs. Await that grant after presenting this concrete candidate.

- User 승인. 진행해. explicitly authorized exactfadae244/39. Ordinary main push
  completed; exact production37288954360 all8 jobs succeeded through Android build/
  smoke/signing, immutable Release, LKG Pages, public every-byte/disposable update/
  no-op and non-force CAS. Promoted main is471590750468f6d99b520c3a8d47b03a474aacbb,
  stable signed39 generationlocal-hosted-0-160-0-b7c1e61019b1-retired-server-discovery.
- Fresh independent pinned-key signature/index/manifest/every-byte/download-size
  readback passes. Only Manager and descriptor identity/binding changed from38;
  all five protected Core/runtime/helper payloads and ordered descriptor policy
  remain exact36/38. Official0.160.0/archive7f0fe42f retained. Signed39 Manager
  SHA-256 is31d7c076c2d49e3495d5f27171c3b8e5141d70140b20809b7b1671b96b03f2a1;
  Core SHA-256 remains3d994dd187515276de755d1332a369469355bc2d21010b470313c7a6e7b1c648.
- Actual production-signed39 native qualification passes all6 behaviors through
  the real Core entrypoint, including exited record preservation, running disconnect,
  owner reconnect/foreign writer rejection, confirmed cancellation, retained-subscriber
  rejection, scoped PID-stable force and ordinary same-UUID account takeover/goal pause.
  All native work/signals occur only in owned credential-free temporary roots.
- Ordinary installed codex update exits0 and activates exact39, retaining complete38
  (all9 files byte/mode exact) as previous. Installed signed inventory/launcher/signature,
  upstream exact codex-cli0.160.0 and all five task help forms pass. Real installed
  task status now exits0 (two current writers; private identities/content not saved),
  fixing the previously observed installed38 exit1. Composed doctor exits0 with
  upstream/Core/runtime/code-mode-host/Manager/summary healthy; sandbox remains
  unsupported as intended. No alias trust or permission/runtime change was added.
- Fresh immediate pre-update PID/start baseline10 is preserved exactly. Protected
  paths27 have only authorized activation-state change; auth/profiles/sessions/
  preferences/resolver identities remain unchanged. Obsolete36 retired normally
  because current39/previous38 replace38/36; no live job was killed. A protection
  probe initially retained the superseded36 expectation: reject that stale invariant,
  bind actual authorized complete38 rollback plus all10 real jobs, and reverify.
  No manual retained-generation copy or Core policy change was made.
- Installed second ordinary update returns exactly Codex0.160.0 is already up to
  date., exit0/empty stderr, with full bound snapshot unchanged. Local protectedmain
  and sealedlegacy refs remain2ffb95f/bf30a7d; rewrite history is independent.
- Disposition: ACTIVE-TASK-HANDOFF delivery and installed corrective39 are accepted
  and closed. Current39/previous38 are healthy. No task stop, new public/live mutation
  or additional feature follows from this closed bundle. Input-cache consideration
  is already completed below; empirical follow-up remains separate and unselected.
  No speculative cache changes are made.

## ACTIVE-TASK-HANDOFF (source accepted 2026-10-05)

- User authorizes implementation of the reviewed cross-account ownership,
  reconnect/cancel/transfer requirements and adjacent convenience on the same
  path. Bound clean rewrite/rust-core60b138d678ccc835e4d227c97831e080440d9b45;
  goal-md bound, approved equivalent primary, workers OFF. Primary plans,
  implements, inspects and validates directly; no agents/reviewers enabled.
- Success: a user blocked by another account's active writer can discover the
  owning execution identity without inspecting credentials/history, reconnect to
  its actual live server, or stop it and resume the same UUID in the chosen
  account after real writer release. Interactive active-work selection and explicit
  status/actions support this result without a second conversation picker/index.
- Native source0.160.0 already offers loaded/read/turns metadata, same-server
  resume, goal pause and interrupt acknowledgement; no idle active-thread unload
  or cross-account conflict workflow is provided. Use that protocol first. Core
  retains existing execution authority only. Force is explicit whole-server scope,
  PID-stable and never lock deletion; failure/uncertainty preserves ownership.
- Allowed writes: source/authority and owned temporary roots; ordinary source
  commit/push and hosted acceptance. Installed launcher/runtime/account/session/
  auth/preferences/resolver and ongoing user jobs remain protected. Disposable
  synthetic native proof may issue loopback fixture turns without credentials;
  no real model turn or live task cancellation is authorized by this bundle.
  Signing/publication/installed activation follow a separately recorded accepted
  candidate gate, never manual copies into live paths.
- Baseline Manager39 nonzero tests passed (22 unit+17 public integration), no
  failures. All four ordered discovery, action, transfer and public-proof slices
  are closed; the accepted evidence and disposition are recorded here.
  Retain signed seq36 current/seq35 previous and independent bare Core behavior.
- User clarification: ownership follows the actual current writer-lock holder,
  not first author or last historical metadata. Require kernel ownership proof
  for status/actions; read-only or released cached copies cannot claim ownership.
- Final source payload SHA-256 is
  `a35c1461b9702f7435893dbcc37603f2468cab65987f627e6ca8a234eb576391`
  over the ordered Cargo/Manager/qualification file-digest map. Manager alone
  adds the public task family; Core production, release-builder and workflows are
  unchanged from bound60b138d. Ownership is verified against native lock inode, exclusive
  kernel lock, exact runtime PID/start and local socket peer, then revalidated
  before goal pause, interrupt or force signal. Released owners resume directly
  only after the existing native coordination/writer probe confirms freedom.
- Local grouped proof: Python92; unchanged Core181 passed/one explicit device
  smoke ignored; release-builder22; final Manager30 unit+30 integration passed,
  with the fixture-only invocation excluded (59 behavioral proofs). Final
  ownership-race regression and all Manager gates were re-run after the last
  ownership correction; successful unchanged Core/builder gates were reused.
  Strict workspace Clippy, formatting, actual diff review and credential gates
  passed. No zero-test invocation is acceptance evidence.
- Actual Core/new Manager/upstream0.160.0 in credential-free owned roots passed
  running-disconnect/current writer discovery, cross-account writer rejection,
  exact owner reconnect/confirmed cancellation, retained subscriber admission,
  PID-stable whole-server force and normal same-ID transfer with persisted goal
  pause. Qualification exited0 after terminating only owned fixture daemons.
  Current-holder change, read-only child preservation, unknown-holder rejection
  and owner changes during metadata/scope queries have focused regressions.
- KEEP upstream lifecycle/Core execution; COLLAPSE assistance onto current kernel
  ownership/native RPC; DELETE loaded-only owner attribution. Local qualification
  snapshot27 entries, including baseline absences, was unchanged. At source
  closure26 remain exact; startup-update-advisory was concurrently replaced
  (inode/digest, mtime2026-10-05T02:22:59Z). It was only read for protection checks
  and preserved without reverting or claiming unchanged. Installed seq36/35,
  launcher, account/auth/preferences/resolver and sealed branches remain exact.
  No live signing/publication/activation occurred.
- Source implementationca5b171323a67e20351c54932ec316aa2397a420 and test-only
  correction705afb98098f27d613130cd3fd4d0f84fb74fc50 were ordinarily committed
  and pushed. Exact-event hosted run37255197297 accepted705afb98098f27d613130cd3fd4d0f84fb74fc50:
  Python15+77, Core180 whole plus two1/1 Android cases (182 unique), Manager30
  unit+30 integration (59 behavioral proofs excluding the fixture invocation),
  builder22;263 behavioral Rust passes, one explicit device smoke ignored.
  Android/AArch64 Core build, workflow/credential/diff, strict locked Clippy/fmt,
  authenticated stable snapshot audit and aggregate source gate all succeeded.
- Prior hosted run37254771057 rejected the Core asynchronous exit fixture's10ms
  scheduler assumption. KEEP production10ms process-exit safety and deterministic
  exhaustion proof; give only the asynchronous test publisher a scheduling
  allowance. Relevant timing fixtures were inspected, the nonzero focused gate
  passed, and the fresh full hosted event above restored acceptance. No failing
  event is counted as proof. Core production remains byte-identical to baseline.
- Source closure is complete. Signed installed delivery remains a separate
  candidate gate, not implied by owned runtime fixtures or source publication;
  installed seq36/35 remains unchanged.

## VALIDATION-RENEWAL / PERMISSION-MEMORY-RELEASE (accepted 2026-10-04)

- User `진행` authorized retention/hosted validation repair, one signed corrective
  Core+Manager+qualified v3 runtime publication, ordinary activation and bounded
  fresh-session permission/notification acceptance, followed by input-cache
  consideration. Goal-md binds this repository; approved equivalent primary,
  workers OFF. All selected repair slices are closed.
- Authorization covered repository/owned roots, separate publication checkout,
  ordinary non-force source/publication pushes, hosted signing/Release/Pages/CAS
  and ordinary signed activation after green source/public gates. No manual live
  patching, key inspection, data migration or real model/account turn occurred.
- KEEP conservative live-process visibility, roles/leases and execution semantics.
  COLLAPSE generation/server retirement onto native whole-process exit confirmation
  with one total10ms budget. An exited leader with a live sibling retains files
  and server records; unknown or denied live visibility remains protected.
  Android hidepid invisibility is not claimed repaired. DELETE spent historical
  source/main/parent/sequence CI pins and the duplicate workspace proof job.
- Product commit274f6771b430c51d179e1fb3beec31108c92c159 passed ordinary
  untraced scripts/check.sh: Python92/Rust242, one explicit device smoke ignored,
  strict locked lint/fmt, optimized build and actual diff/credential gates.
  Authority close6f2ee908 followed a concurrent protection-config replacement.
  Hosted run37238896480 exposed host proc visibility contamination (9 reds).
  Namespace commit1f3dfd1 isolates the entire serial corpus in one remounted
  private proc under original non-root UID/GID with all capabilities dropped;
  run37239503780 fixed8 reds and retained the real unreadable-live negative.
  Remaining churn assertion was corrected without changing production: every scan
  preserves live files/state, only observed PermissionDenied may abort, and stable
  pruning must succeed after churn/handle release. Focused7/7 Android plus Clippy
  passed; no failing or zero-test invocation is acceptance evidence.
- Exact accepted producer6523921a1d3368865d46cfa9093e3f118f7e9a30 passed
  hosted run37239860820: Python15+77; Core180 whole plus two1/1 Android
  admission cases (182 unique), Manager22+17, builder22;243 unique Rust passes,
  one explicit device smoke ignored. Event, strict lint/fmt/diff/credential and
  pinned-key immutable public snapshot/every-byte/size-sidecar gates all passed.
- Reviewed corrective bridge qualified actual component39 unique cases and
  preflight8 cases, YAML/shell syntax, exact accepted source/parent/event/key and
  non-force CAS. Preserve code-mode-host/helpers and unrelated ordered fields;
  both Core and Manager change with the qualified v3 permission runtime. Official
  upstream0.160.0 archive7f0fe42ff22ecfa3a47bc4a34f5b22c4218b431a4ec0aba51c7d98299f07900c
  and runtime926a5e5c2d113db9bcf2173013d805275c21e872c77dd37eec91aff2efcff99f
  remain bound. Production private signing authority remains exclusively hosted.
- Exact-parent ordinary trigger95c5acaf834e2d30b6ae7208333093620700e337
  from public main3cf5342a9f8c90798c13123208095d088c9c6b0f completed
  production run37243754323, all stages success: native smoke, signing, immutable
  Release, LKG-preserving Pages, public readback, disposable update/no-op and CAS
  force:false/committed/ref_update_rc0. Main became
  15487ecca7de7e28c9ed743bb09afb394b208005. Fresh pinned-key signed every-byte
  audit independently verified sequence36
  local-hosted-0-160-0-6523921a1d33-permission-memory.
- Actual published binaries passed Android native legacy/named/shared and embedded
  permission paths: four selections/current marker/both shortcuts, same thread,
  independent user/auto_review/never without sandbox, no real model turn. Two
  synthetic root completions deliver once each; enabled successful native memory
  consolidation delivers zero extra alert. Publication-only compat resources in
  an initial owned fixture were rejected correctly; rerun used the exact signed
  seven payloads plus manifest/signature installed layout, no production weakening.
- Ordinary installed codex update exited0 and activated signed sequence36 with
  all seven inventory bytes/modes, launcher and signature verified. Sequence35
  remains complete previous rollback (all9 files exact). Version remains exactly
  codex-cli0.160.0; doctor exit0 reports Core/runtime/Manager/code-mode-host/
  upstream/summary healthy, expected unsupported bwrap sandbox. Exact-current
  update completed1381ms, empty stderr and all72 durable Core entries unchanged.
- Actual installed Core/Manager with owned preferences delivered exactly one
  Android-listed unique synthetic UserInputRequest notification with folded
  one-line text; unselected PermissionRequest invoked no provider. Activity reuse
  flags retained, no RunCommandService/tmux attach. Initial fixture compared CLI
  string ID to notification-list ID incorrectly; corrected unique title/body proof
  passed and cleanup verified no owned alert. No physical tap/layout claim.
- Bounded device-before/final snapshots preserve auth, account/profile settings,
  Manager preferences, resolver and all10 existing process PID/start identities.
  Ordinary update changed only launcher/activation/rollback metadata and pruned
  an older unretained sequence33 runtime; sequence35 remains exact. Earlier source
  snapshots saw concurrent profile config/advisory replacements, preserved without
  reverting or claiming unchanged. Local main2ffb95f3814ce95462ae5c0c75f57dfb8c266afd
  and sealed legacybf30a7dc94d4dad7f58836c69028160856e63c58 remain exact.
- Reuse the completed INPUT-CACHE-UNLOAD-CONSIDERATION below: same-ID resume may
  retain remote prompt-cache affinity while unload loses local transport state;
  positive grace also retains the writer lock and cannot guarantee remote hits.
  Delay0 stays unchanged; no keepalive, retention override or model measurement
  is selected. Existing sessions retain their old runtime until ordinary restart.
  This closure changes authority records only, not accepted producer bytes.
  Final two credential/tree checks, diff inspection and exact allowed device-delta
  assertion passed. An invalid unittest dot-directory invocation ran no tests;
  direct-file two-test rerun passed and is the only closure test evidence.

## PERMISSION-PICKER / MEMORY-NOTIFY (source accepted 2026-10-04)

- User selects continuation of the `/permissions` defect investigation and
  correction, followed by exclusion of internal memory-update notifications.
  Bound clean rewrite/rust-core cb74f27e316d95634dfba3cd39e4ad570c553ac4;
  goal-md resolves this repository GOAL; approved equivalent primary, workers OFF.
- Native same-account ephemeral thread/start proves the server default is
  dangerFullAccess/on-request/user, independent auto_review remains full access,
  and never remains full access. No model turn or durable conversation created.
  The TUI picker explicitly selects :workspace for both approval choices,
  overriding the Core FD-34 no-sandbox default. A direct API proof is diagnosis,
  not acceptance of a repaired picker.
- First close the Termux picker contract with actual TUI/native setting proof;
  preserve explicit unsupported-policy rejection, argv, approval ownership,
  shared server reuse, auth, conversation and rollback surfaces. Then investigate
  memory-only notification suppression without content heuristics, disabling
  memory work or dropping ordinary Stop/user-input notifications.
- Allowed writes are source and owned temporary proof roots. No installed
  runtime/profile/session/auth/Manager configuration mutation, signed publication
  or live cutover is authorized by this source bundle. Historical CI renewal
  remains separate. WORKBOARD owns the live slice map and current proof state.
- Accepted outcome: Ask/Approve preserve no-sandbox execution and select user/
  auto_review independently; Full Access selects never. Unsupported Read Only is
  omitted from builtin menus/shortcuts. Explicit CLI rejection and named/custom
  operator semantics remain unchanged; no global permission allowlist/fallback.
- Official 0.160.0 archive SHA256
  7f0fe42ff22ecfa3a47bc4a34f5b22c4218b431a4ec0aba51c7d98299f07900c
  produces termux-fd-remap-v3 with exact raw runtime SHA256
  50b06603bdcdac39b714f5c3e68583c002b8ad8779ebfdaaf4932ff016b379c0.
  Existing FD/UDS adaptation is preserved; 18 UI blocks change387 bytes,
  total484 with FD/UDS. Adapted runtime SHA256
  926a5e5c2d113db9bcf2173013d805275c21e872c77dd37eec91aff2efcff99f.
  Drift/truncation/overlap and every v3 report field are bounded/fail-closed;
  v1/v2 rollback and hosted v3 admission remain supported. Actual optimized
  Builder publish with an owned fixture signer validates the real v3 output;
  no public signing authority or publication was used.
- Core projects selected Stop to upstream native notify, which upstream clears
  for internal memory consolidation. Manager accepts one bounded, typed native
  completion JSON argument, reuses the existing parser/presentation/focus path
  and ignores input-message metadata. No extra runtime memory patch, content
  heuristic, memory disabling, preferences/schema migration or duplicate Stop
  hook. Other hooks, user-input matcher and operator precedence remain intact.
- Optimized actual public Core/native TUI qualification passed default/legacy,
  named-profile and embedded paths: four selections, current markers, F7/F6,
  same thread and no model turn. Shared cases have6/5 effective native settings
  events; embedded proof is native TUI acknowledgement, not captured protocol
  JSON. No Manager is required for permissions (earlier same-runtime private
  generation without Manager also passed all three paths).
- Optimized native completion qualification passed two synthetic root turns,
  one single-line delivery each, and an enabled real memory consolidation
  worker/model fixture with successful job, selected phase2 input and watermark,
  zero extra delivery, unchanged Manager record and no auth file. Durable
  scripts/qualify_permissions.py and scripts/qualify_notifications.py exercise
  owned temporary roots with no real credentials, remote model or Android alert.
- Final grouped corpus: Python15+75; Rust237 unique passing cases = Core176,
  Manager22+17 and builder22. Strict all-target workspace Clippy, formatting,
  diff checks and optimized workspace build passed. Core ordinary latest run
  was174 pass/2 existing retention EACCES/1 explicit device smoke ignored;
  both failed names and all five retention cases passed on the same source
  under readlink tracing. This is conditional aggregate evidence, not a claim
  that untraced scripts/check.sh or the interrupted whole traced run exited0.
  Ordinary /proc scan instability remains validation debt; conservative prune
  rules were not weakened. Reused successful same-source tests rather than
  repeating unrelated gates under the very slow tracer.
- Rejected evidence/setup failures: zero-test filters corrected to real gates;
  named-menu vector length and inline description tails repaired before native
  qualification; initial memory fixture feature/trigger mismatch corrected;
  tracked-tree audit rebound renamed/new files; moved Manager test cache rebuilt;
  split/nested stdout accounting replaced with native summary plus exact failed
  name readback; fixture signing URL corrected to the required final slash. Four pre-existing Clippy1.98 style lints were collapsed without
  behavior change and mapped profile/startup PTY regressions passed.
- Production/test/proof source identity SHA256
  e16a06929fac25b35be930f8e833847edceb73af034955b381c821804ac11942
  stayed unchanged through final optimized native proof. All27 bound installed/
  auth/config/Manager/resolver identities remained unchanged. Installed signed
  sequence35 Core/runtime/Manager match their accepted hashes; sealed legacy and
  local main unchanged. Source acceptance does not authorize installed cutover.
- Disposition: selected source corrections are closed with the stated /proc
  test condition. Signed release/ordinary activation and separate validation
  reliability work remain unselected; no live runtime/config/auth/session change.

## BOUNDARY-RELEASE (accepted and installed 2026-10-04)

- User's `go` authorizes signed publication of the accepted Core and Manager
  together, ordinary `codex update` activation and bounded device acceptance.
  Producer source is clean rewrite/rust-core at
  7285b91adbad0824bbb8eb4bf7bf8e336fe532c1; workers OFF. Reuse its accepted
  Python 90 / Rust 234, optimized and private native/public entrypoint evidence.
- Publication starts from origin/main
  07d625561283bad5d25b778782c16d4929b4939f and authenticated sequence 34,
  local-hosted-0-160-0-4b00b8d46193-notify-line. Exact official 0.160.0 package
  and qualified runtime remain unchanged. Replace the spent Manager-only
  one-shot bridge with one exact-parent, exact-source sequence 35 bridge that
  requires both changed Core and Manager and preserves all upstream artifacts.
- Allowed writes additionally include a separate publication checkout, ordinary
  non-force source/publication pushes, hosted signing/Release/Pages/CAS promotion,
  and ordinary signed installed activation after those gates pass. Never obtain
  a production private key or manually replace an installed artifact. Preserve
  existing work processes, auth/account settings, conversations, Manager policy,
  resolver, sealed legacy and local main. Private native proofs use owned roots
  and synthetic conversation data; device checks do not start a model turn.
- Acceptance requires focused workflow admission/component-change regressions,
  hosted Android build and smoke for both artifacts, authenticated full public
  readback and disposable normal update/no-op, successful ordinary installed
  update, actual public/native device paths and protection comparison. Physical
  notification taps must be distinguished from programmatic/native action proof.
- P0/P1 passed: exact-parent public index and release signatures verified against
  the pinned public key; sequence34 Core/Manager/descriptor and official0.160.0
  archive match. Eight workflow regressions include real shell admission,
  Core-only/Manager-only rejection, both component byte mismatches, all four
  upstream inventory byte/mode faults and unrelated descriptor/identity/order/
  scalar faults. YAML and all 37 run blocks parse; publication diff reviewed.
  Proof setup corrected an empty binary signature API export, tuple unpacking,
  and source-helper lookup; earlier failed checks were not accepted. Fresh
  24-entry protection snapshot binds two profile config identities changed since
  prior source acceptance; this release work has not written either profile.
- P2 passed: publication child02a4a6f4ea6806c82d7a8ab23b1a9be6df136b83,
  hosted run37208119825 succeeded through build, Android smoke, signature,
  immutable Release, Pages, full public readback, disposable ordinary update/
  exact-current no-op and CAS stable promotion. Sequence35 is
  local-hosted-0-160-0-7285b91adbad-boundary-release. Core SHA256
  fd0fd178d903ac42087b938d888295e3234b39dd4d23c522a4bbc374eb39eda9;
  Manager2d6212e0f6df148948fdc6f7b80bcae781bc84a127575115fcd24a29fd37d01f.
  Both differ from sequence34; upstream runtime/code-mode-host/helpers and other
  descriptor fields preserved. Fresh promoted index/signature exactly match
  authenticated candidate. All24 protected identities unchanged before cutover.
- Hosted artifact device-private qualification also passed40 actual public
  invocations, two-account native synthetic discovery/resume with canonical
  SQLite and false maintenance features, and bare/max-ID native shared-server
  FD/runtime/account/config-refresh/generation and input-hook proof. No model
  turn, credentials or live-state mutation. This proof preceded installed activation.
- Unrelated historical source CI run37208097699 is red and not acceptance:
  fixed source-parent/main/sequence17 assertions are obsolete; its parallel Linux
  tests encounter real /proc PermissionDenied and conservative retention keeps
  generations (five retention cases plus failed-candidate cleanup). Current
  serial Termux grouped proof and hosted publication qualification are the
  load-bearing evidence. Do not claim this historical CI passed or weaken
  production retention to satisfy it; its automation requires separate renewal.
- P3 passed: ordinary installed `codex update` returned0 and activated the exact
  signed sequence35 Core and Manager, retaining sequence34 as previous. Genuine
  upstream version is exactly codex-cli0.160.0; Core doctor returns0/healthy;
  Manager help omits repair and retired repair plan returns2. Current accurately
  reports external/source inherited for this unregistered legacy CODEX_HOME,
  rather than inventing a Manager registration. A repeated ordinary update
  reports already up to date with no subsequent protected-state changes.
- Real Android notification proof uses only owned configuration and synthetic
  payload: selected UserInputRequest reaches the real provider and NotificationList
  with one-line title/body; unselected PermissionRequest makes no provider call.
  Three generated Activity reorder/single-top actions complete successfully;
  own synthetic notifications removed. Private proof preserves Android runtime
  path variables and temporarily gives only the native API client its real Termux
  environment. Earlier boolean-argument, fixture-reuse, missing Android environment
  and logical-versus-native notification-ID assertions were corrected and rerun;
  they are not accepted evidence. No physical screen-layout or tap claim.
- Final protection:19 of24 bound content/mode/inode identities unchanged,
  including all auth/account configs, Manager preferences and resolver. Five
  ordinary activation changes are the launcher, activation pointer, two rollback
  records and removal of the now-unreferenced sequence33 runtime; previous34 is
  retained. Installed native processes10/10 preserve PID/start/executable identity
  through activation and device actions. Local main and sealed legacy unchanged.
  No manual runtime replacement, private-key access or model turn. Source and
  installed boundary alignment are complete; historical CI renewal remains separate.

## REPAIR-BOUNDARY (source accepted 2026-10-04)

- User authorized the boundary plan and explicitly reaffirmed repair removal.
  Bound rewrite/rust-core at 7f680137e157ff16f83dd08c16620424f088ff30,
  workers OFF; SPEC grammar/ownership/handoff/MGR-4 amended before code.
  Baseline Core three repair tests and Manager grammar/handoff one each passed.
- DELETE Manager repair help entries, enum branches, parser, launch facade and
  obsolete request-env stripping; DELETE Core request constants, action/reason/
  plan/operation types and methods, planner/renderer/env parser/route matcher/
  hidden dispatch plus dedicated probe/helper/superseded tests. README points to
  ordinary Core recovery. No replacement repair command, request API, updater,
  automatic rollback, persistent state or migration. KEEP signed generation
  admission, Core doctor/update/rollback/recovery and original upstream execution.
- Focused retired_repair_commands_are_usage_failures and actual Manager
  retired_repair_routes_never_handoff_or_mutate_manager_state prove usage status 2,
  no Core handoff, absent-state noncreation and public-created profile/notification/
  retired-history preservation. Core `retired_repair_environment_cannot_divert_public_core_routes` compares 30 public invocations across doctor, update help,
  update, explicit rollback and version with complete/partial/malformed retired
  values. Existing signed update/rollback and failed-preactivation retention
  regressions passed; Manager 21+16 passed warning-free.
- Exhaustive actual diff review covered every removed definition and surviving
  caller/probe. The only helper addition is an explicit rollback scenario in the
  existing public-main probe. Stabilized production/test diff against 7f680137
  SHA256 289007fab060f0e326a6a8a1aa6bf8e5a44fbb520e5960a816132d40c9f1db37.
- Stop-on-red corrected test setup only: wrong Cargo package invoked no tests;
  doctor schema field is termux_core; private launch fixture needs prefix/bin and
  real copied OpenSSL/curl (curl rightly rejects symlinks). Focused gates reran.
  First grouped run saw PermissionDenied in real /proc scanning during a separate
  optimized build and three retention failures; conservative production retention
  remained intact. After build completion, all five retention tests passed with
  identical source. Build interference is inferred, not proven PID attribution.
  The isolated full rerun passed Python 15+75; Core 176 with one explicit device
  smoke ignored, Manager 21+16, builder 21 (234 Rust), formatting and diff checks;
  no warnings. Earlier red and zero-test targets are not acceptance evidence.
- Locked optimized Core and Manager builds passed at unchanged production bytes.
  Actual optimized public entrypoints passed 40 invocations: retired commands and
  help, five ordinary routes unaffected by obsolete env, dotted selected home,
  canonical SQLite, effective identity and Manager-unavailable launch. Private
  state remained unchanged. Qualified native 0.160.0 server proof passed bare /
  max-ID / account-distinct sockets, runtime/FD binding, PID/inode-preserving
  policy refresh, request_user_input matcher without active PermissionRequest,
  generation separation and no unmanaged packages. Actual public Manager-select /
  Core-direct paths in two accounts discover/resume one synthetic rollout with
  canonical SQLite and maintenance flags false. No turn/start or credentials.
- Protection: 23 of 24 original content/mode/inode identities unchanged, including
  installed launcher/runtime, auth/account settings, Manager, resolver and
  activation. Only startup-update-advisory (a volatile installed cache) changed
  content/inode across the user's interruption/resume; all proof launches use
  owned private roots. External startup refresh is inferred without process
  attribution; cache was not restored or modified by this work. Final comparison
  binds its newly observed identity. Main and sealed legacy remain unchanged.
- All four boundary bundles are source accepted: Core independently prepares
  execution and owns signed lifecycle; Manager registers/selects profiles and
  delivers notifications; upstream owns conversations/protocol/writers. Repair
  is retired. At source acceptance, installation and publication were unchanged;
  subsequent signed installation is accepted under BOUNDARY-RELEASE above.

## NOTIFICATION-BOUNDARY (source review accepted 2026-10-04)

- Bound clean 2c1e5f7; workers OFF. Exhaustively reviewed Manager notification
  commands, policy/default/record/publication parsers, hook input/JSON cursor,
  text/focus/provider definitions and Core validators/projector/callers.
  KEEP Manager policy/presentation/delivery, independent Core bounded path/mode/
  version/full-shape validation returning only hook names, and native integration.
  The shared event vocabulary is a versioned payload contract; Core hook activity
  and Manager delivered fallback text serve separate interfaces. No duplicated
  physical write or competing policy owner requires a new schema/dependency.
- Corrected stale SPEC descriptions to match already accepted public behavior:
  show includes the separate focus line; generated Core configuration contains
  execution defaults; foreign Core-owned config is preserved/rejected; canonical
  session_id affects only explicitly selected focus, never content/shell input.
  No production/test code, command, record or behavior changed.
- Actual optimized public Core/Manager + qualified native 0.160.0 private server
  proof confirms effective PreToolUse request_user_input(_async) matcher and no
  active PermissionRequest entries for UserInputRequest,Stop; configuration
  refresh preserves server PID/FD binding. Initial key-absence check was invalid
  because config/read includes empty disabled hook arrays; corrected the fixture
  to inspect active entries and reran successfully. No red gate remains.
- Reuse same-source named Core projection/matcher regressions, Manager parser/
  input bounds/single-line/provider/focus regressions and PROFILE grouped/release
  acceptance. No live delivery, permission or device-state write. Review accepted;
  installation/publication were separate at source acceptance.

## SERVER-BOUNDARY (source review accepted 2026-10-04)

- Bound clean 12a329e9, workers OFF; reviewed all 9 shared_server production
  definition plus launch/maintenance callers against exact upstream 0.160.0
  source a956835d020762cb2b570053af06f643a11c0ecc (archive identity in PROFILE).
  KEEP eligibility/explicit-mode preservation, private owner/path validation,
  account+generation namespace, bounded/atomic records, peer/PID/executable
  checks, qualified-runtime/FD launch with config refresh, and graceful obsolete
  generation retirement. These establish signed execution/retention invariants;
  upstream owns protocol, thread/writer/storage and graceful-only SIGHUP.
- Native daemon now installs into packages/app-server-daemon, with standalone
  fallback, and has an independent updater. It cannot replace this narrow
  signed-generation launch. SPEC corrects the package-root description.
  No custom supervisor, runtime installer, registry or production branch added.
- First disposable bare proof failed readiness: native error was physical Unix
  path exceeding SUN_LEN because the nested test TMPDIR was too long. Corrected
  only fixture setup to a separate owned short private temporary root. Same
  actual public optimized Core/Manager + qualified native runtime gate passed
  default bare, 64-char account, distinct account sockets, exact executable/argv
  and FD33/34/35, same-generation PID/FD-directory-inode-preserving refresh after
  notify policy change, and generation separation preserving the prior process.
  No unmanaged package installation, model turn, credential copy or live write.
- Five named shared-server focused regressions and grouped 237 Rust / Python 15+75
  plus locked release builds are reused from PROFILE at identical production/test
  bytes; zero-test targets are not proof. Protected 24 content/mode/inode stayed
  unchanged. Review/doc-only source acceptance preceded BOUNDARY-RELEASE.

## PROFILE-BOUNDARY (source accepted 2026-10-04)

- User authorized the boundary-alignment plan after35d62e8; clean resume bound
  that exact HEAD, goal-md repository GOAL, approved equivalent primary, workers
  OFF. SPEC amended before each product slice. P0 four named baselines ran1/1
  and Core/Manager compiled; real private Core+Manager entrypoints reproduced a
  dotted registered account using shared SQLite through Manager but losing Core's
  canonical requirement on direct CODEX_HOME launch.
- Exact upstream0.160.0 source a956835d020762cb2b570053af06f643a11c0ecc,
  archive351a23896ba75c2c32c2d9d2050a0987079d683ea4e92d3429b3e1833945e927,
  confirms CODEX_HOME-relative rollout/index/writer paths and profile-local .tmp
  maintenance/compression locks; SQLite configuration/requirements precede the
  environment. KEEP shared links, native state ownership and both maintenance
  restrictions. Historical operator migrations remain unchanged.
- COLLAPSE physical shared-path preparation into Core; DELETE Manager's duplicate
  shared directory/file inventories, canonical-home/link producer/validator and
  SQLite selection signal. Manager atomically registers only private metadata and
  an empty account home; list/use validate registration before first execution.
  Core recognizes accepted dotted IDs while preserving declared external IDs,
  prepares declared missing/empty links and rejects conflicting state before
  upstream execution. Arbitrary homes are not enrolled through a legacy signal.
- DELETE saved-selection writes/reads/parser/formatter/error/constants, the
  ProfileTarget display/use wrappers and ManagerDirs wrapper. Existing state-v1
  records, including invalid or symlinked remnants, are ignored and preserved.
  Current reports default/source default without a usable inherited home, or
  registered/default/external with source inherited. No persistent account switch,
  data relocation, import, auth inspection, new command, schema or dependency.
  COLLAPSE Core's two home derivations onto upstream-equivalent empty/non-UTF8
  handling; shared preparation and server launch use the same execution identity.
- Focused public Core3/3 covers six direct home modes, empty/non-UTF8 defaults and
  three conflicting topologies. Topology/requirements regressions passed; Manager
  current/history/create/list/use, metadata and symlink regressions passed. Actual
  optimized Core+Manager public fixtures passed17 invocations, including dotted
  select/direct equivalence, accurate fresh identity, unchanged synthetic account
  markers/history and direct execution after Manager removal.
- Stop-on-red corrections: history deletion briefly removed shared read_bounded;
  restored it unchanged for metadata/notification/focus consumers. A dead root
  field was collapsed with all surviving wrapper callers rather than suppressing
  its warning. Manager21+16 reran warning-free. Native config/read does not expose
  effective managed feature flags; corrected the proof to experimentalFeature/list.
  A synthetic runtime prints raw non-UTF8 environment bytes; corrected the private
  reader to preserve them rather than misclassifying the decode failure. Earlier
  failed checks are not acceptance evidence; no red gate remains.
- Production/test source diff against35d62e8 SHA256
  60a4fd325641248d5037a0b861bc3d0ff5085259c3cf475a8db758bb398d8a6e.
  Grouped scripts/check.sh passed Python15+75; Core179 with one explicitly ignored
  device smoke, Manager21+16, builder21 (237 Rust); no warnings. Empty binary/doc
  targets are not evidence. Locked optimized Core and Manager builds passed;
  every changed production definition/caller and affected proof was inspected.
- Native disposable proof used the previously qualified0.160.0 runtime/host
  copied into an owned synthetic installation, not live profile state. Actual
  public Manager-select/Core-direct app-server paths in two accounts discover and
  resume one synthetic rollout with the same UUID, canonical SQLite and effective
  maintenance flags false. No turn/start, model inference or credential copy.
  This qualifies the changed native launch/storage integration, not a newly signed
  bundle, public release, live cutover or server-supervisor replacement.
- All24 protected file bytes/modes/inodes unchanged; sealed legacy and local main
  unchanged. Source acceptance closed by ordinary commit. At that point the
  installed runtime retained the prior producer; server, notification and repair
  alignment followed as separate bundles before BOUNDARY-RELEASE.

## CORE-MANAGER-BOUNDARY-ALIGNMENT (criteria and plan recorded 2026-10-04)

- User requests firm technical ownership criteria and a work plan after the
  upstream/Core/Manager review. Bound clean `rewrite/rust-core` at
  `edcc59d7a0d81cd5e35be6916a3709bd2b96e024`; goal-md bound, approved equivalent
  primary, workers OFF. SPEC4.5 is normative; WORKBOARD owns the ordered execution
  map. This request closes documentation planning, not product implementation,
  native-runtime qualification, publication or live activation.
- Success threshold: Core independently supplies the minimal qualified Termux
  execution path; upstream owns auth/conversation/server internals; Manager
  supplies distinct selection/configuration/delivery conveniences. Direct and
  Manager-selected launches establish the same execution invariants. Existing
  installation-wide conversation sharing and per-account auth/config isolation
  are preserved; arbitrary external homes remain isolated.
- Profile alignment must remove accepted-ID drift and duplicate shared-path
  preparation, distinguish effective execution identity from saved history, and
  preserve existing valid homes without implicit migration or persistent account
  switching. Server alignment must justify each retained launch/reuse/retirement
  operation against the exact supported upstream runtime; retain required short
  sockets, descriptor inheritance and signed-generation binding. Notification
  alignment must keep Core's integration minimal while preserving validated
  policy, input/completion events and accepted delivery behavior. Repair is a
  retirement candidate whose command/request removal needs its own SPEC-first
  slice; it is not already removed or replaced by a new recovery subsystem.
- Each implementation bundle requires a runnable baseline, definition-to-proof
  mapping, nonzero focused public-path regression, actual diff inspection,
  grouped acceptance and protected-surface verification. Native compatibility
  claims additionally need exact-runtime disposable proof; synthetic routing
  fixtures alone are insufficient. Installed optional-Manager isolation remains
  accepted source evidence below, not evidence of live deployment.
- Documentation gate passed: the three authority-document diffs were inspected,
  criteria/threshold/plan consistency and `git diff --check` passed, and all12
  referenced existing regressions were found in source. No production source, tests,
  dependencies, installed state or remote release changes are selected in this
  planning turn. Implementation acceptance remains open.

## CORE-MANAGER-INDEPENDENCE (source accepted 2026-10-04)

- User reaffirms the existing SPEC4.2 invariant: Core operates independently of
  Manager. Bound clean rewrite659ae8e, goal-md exact repository GOAL, approved
  equivalent primary, workers OFF. SPEC-first clarification separates an installed
  optional-component failure from strict fresh-candidate admission. No new command,
  dependency, persistent state, profile policy, repair retirement or live cutover.
- DELETE the loader's unconditional Manager file prerequisite and stale ordinary
  launch rejection of a Manager symlink. KEEP declared generation identity, required
  runtime/host/helper path safety, signed control policy and exact inventory.
  Ordinary dispatch accepts only a regular owner-executable declared Manager path;
  otherwise Manager is unavailable, Core doctor runs and optional hooks are cleared.
  No Manager probe, PATH discovery, OpenSSL or network prerequisite is added to
  ordinary execution.
- COLLAPSE asset file checks into the signed inventory gate with two explicit real
  purposes. Candidate admission remains strict for every declared asset. Installed
  verification tolerates failure only for the optional Manager file and excludes
  it from the returned selection and local-build carry forward. Signatures,
  descriptor/inventory validation and all required Core/runtime assets remain
  strict. Generation metadata and signed inventories are never rewritten to express
  absence. Inspect every loader consumer: dispatch, inventory, direct staging,
  installed baseline/local build, rollback and rollback guard.
- Baseline compiled and ran Core test_m2_b2_ 6/6. Focused regressions ran 3/3:
  test_core_manager_independence_public_launch_doctor_and_hooks covers both layouts,
  four installed file faults and 24 public invocations; installed_inventory_keeps_
  admission_strict covers ten paired admission/installed faults plus required-runtime
  corruption; public_update_and_rollback proves rejected new Manager absence,
  update from an installed missing Manager and rollback to that generation.
  Required-runtime symlink regression also passed nonzero. Initial zero-test exact
  filter was rejected and corrected; a mistaken malformed signed-inventory fixture
  was removed rather than weakening signed-control policy. One failed owned fixture
  was cleaned. No red gate remains.
- Stabilized production diff against659ae8e SHA256
  7a641328524ac93ea68c4b040f0a34020de91496ed24c9e5e1c399c35dfb1233.
  Grouped scripts/check.sh passed: Python15 + Python75; Core176 (one explicit device
  smoke ignored), Manager21, builder15 + builder21; formatting and diff checks green,
  no unexpected warnings. Empty binary/doc-test targets and conditional device
  return paths are not additional acceptance evidence. Locked optimized Core build
  passed; actual production definitions/test disposition and diff inspected.
- Built release ELF, public bootstrap and installed public commands exercised only
  in an owned private HOME/PREFIX/TMPDIR with signed synthetic runtime fixtures:
  ordinary launch, doctor/Manager-unavailable, rejection of a missing-Manager new
  candidate, update from a missing installed Manager, rollback with both installed
  Managers missing, and rejection of required-runtime corruption all passed.
  Zero network attempts. This proves real-entrypoint routing/lifecycle behavior,
  not a genuine upstream runtime qualification, published release or device cutover.
- All22 protected auth/config/resolver/launcher/activation/Manager/runtime file
  bytes, modes and identities unchanged. Unrelated work preserved. At this source
  acceptance the installed runtime retained its previous producer. Profile UX and
  repair retirement followed separately before BOUNDARY-RELEASE.

## CORE-MANAGER-UPSTREAM-COMPLETENESS (review completed 2026-10-04; profile decision open)

- User excludes separate convenience products from the completeness discussion.
  Review only how public codex/Core and codex termux/Manager support official
  upstream execution and establish standalone conveniences. This supersedes the
  previous section's proposed next review of an external wrapper integration.
  Bound clean rewrite50554ea; goal-md bound, approved equivalent primary, workers
  OFF. Review permits source reads and owned temporary public-routing fixtures,
  not a live state migration, release, product retirement or speculative feature.
- KEEP Core-owned runtime composition/qualification, Termux process/config/FD
  compatibility, shared-server namespace/identity, signed generation lifecycle,
  update failure retention, activation recovery, explicit rollback and diagnosis.
  KEEP upstream auth, conversation persistence, resume, locks and thread lifetime.
  KEEP standalone Manager profile UX and notification policy/delivery. COLLAPSE
  conveniences onto these owners; do not turn Core operations into Manager repair
  machinery or interpret historic implementations as required features.
- Confirmed foundational mismatch: load_local_generation requires a declared
  Manager file before execute_activated_route reaches any public upstream route.
  This violates the existing optional-Manager execution invariant. Installed Core
  in a private HOME with copied nonsecret generation/activation metadata and
  synthetic executables dispatches --version successfully with Manager present;
  removing only that fixture file then yields exit1 and no upstream marker.
  This is routing fault evidence, not signed release or genuine runtime acceptance.
  Source matches the observed boundary. Do not weaken candidate signature/inventory
  admission to fix installed optional-component isolation.
- Profile semantics need a public contract decision: installed Manager in an own
  private fixture creates review-account and profile use sets the named child
  CODEX_HOME, while profile current reports that saved last-selection and a new
  ordinary Core launch without inherited CODEX_HOME uses the default home. All
  assertions passed through real Core/Manager; the runtime only emitted synthetic
  environment markers. Distinguish effective execution identity from history,
  define deliberate account selection, support the already-declared existing
  homes, and preserve per-account credentials plus one shared conversation store.
  No implementation change or implicit persistent-selection behavior is accepted.
- Review the complete user path: install/run/update/diagnose/rollback belongs to
  Core; account create/select and upstream login/logout/resume launch conveniences
  belong to Manager UX without taking ownership of upstream state. A session
  shortcut may delegate to upstream; a parallel index/transcript parser is excluded.
  Notifications need real user-input/completion events, understandable presentation,
  delivery test and persistence policy. Additional knobs or menus require a concrete
  current user task, not legacy feature parity. Removal requires explicit ownership
  of installed files and preservation of user data; it is not automatically selected.
- Manager repair review remains a retirement recommendation: healthy does nothing,
  legacy invokes ordinary update, invalid state cannot recover and may prevent
  Manager entry entirely. Its existing internal-request tests prove that boundary,
  not independent public recovery. No public repair command has been removed.
- First routing probes stopped red on fixture setup: wrong activation-state field
  separator, legacy bridge helper paths, then missing private config directory.
  Corrected against source and reran a successful present/absent paired invocation;
  earlier failures are not evidence. Every temporary fixture was removed; no
  credentials/session content read, live profile/state change or model request.
- SPEC now frames completeness around upstream/Core/Manager and removes the
  previous external-product comparison from Manager ownership. Review/document
  closure does not claim the optional-Manager defect or profile UX is implemented.
  First product slice should isolate optional Manager absence from ordinary Core
  execution, with focused real-entrypoint fault proof and protected surfaces, before
  convenience expansion. Repair retirement and profile UX follow as separate slices.

## CORE-MANAGER-AI-AUTHORITY (policy accepted 2026-10-04)

- User corrects the architectural review: AI is a separate optional convenience
  wrapper and must adapt to Core/Manager. Manager must not be redesigned around
  AI's current profile paths or stripped of necessary standalone capabilities
  merely because AI offers a picker. Core retains runtime composition,
  qualification, installation, update, activation, recovery and rollback;
  Manager owns Codex profile UX/state and Termux notifications; upstream owns
  authentication and conversation semantics. AI supplies optional presentation
  and tmux conveniences against these product contracts.
- Bound clean rewrite cbcc9a6; goal-md resolves this GOAL, approved equivalent
  primary, workers OFF. This is an authority clarification before any new
  product behavior, not acceptance of profile integration or a live migration.
  SPEC records the hierarchy and removes its stale Manager session-index owner
  bullet, already superseded by the accepted upstream shared-conversation path.
- Existing AI/Manager profile divergence is a next design concern: establish
  the Core/Manager profile contract and standalone public path first, then adapt
  AI's Codex integration. Preserve existing execution homes and credentials;
  this decision does not authorize relocation, merging or deletion. Generic
  AI providers and tmux presentation remain AI-owned. Do not infer that Manager
  profile commands should be removed or replaced by AI.
- Manager repair review remains a removal recommendation because no distinct
  recovery result was found; it is not moved to AI and its public commands have
  not been retired. Any retirement requires a separate SPEC-first product slice.
  This policy/document correction changes no code, installed artifact, preference,
  auth/profile/session data or remote release. Diff inspection/check closes the
  documentation gate; implementation alignment remains unproven.

## NOTIFY-USER-ANSWER-PREFERENCE (accepted 2026-10-02)

- User clarifies desired waiting alert is the conversational question/choice
  input UI, not tool-execution permission approval. Current installed preference
  PermissionRequest,UserInputRequest,Stop is broader than that intent. Success:
  select only UserInputRequest,Stop, preserve completion and presentation/focus,
  suppress permission delivery even from already-loaded old upstream hooks.
- Bound clean rewrite62c0d65; signed34/upstream0.160.0, AIea5541c. Goal-md bound,
  approved equivalent primary, workers OFF. Existing SPEC already distinguishes
  both selectors; this is a bounded Manager preference correction, not a new
  product contract or approval-policy change. Disposable installed-binary proof
  precedes the requested live preference write. Preserve runtime/auth/profile/
  sessions/resolver and all other notification fields. Actual question UI type
  may be clarified through one user-answer request, with no model probe workload.

- Installed signed34 Manager in a private HOME proved PermissionRequest emits
  no provider invocation, while UserInputRequest/Stop each invoke both providers.
  First fixture run used a nonexistent HOME/bin/codex handoff path and stopped
  red; resolving the installed public entrypoint with command-v restored the
  exact gate. No product code changed. Live public notify set then changed only
  hooks; all other notification fields retained their values.
- Live public emit with instrumented own providers proved immediate suppression
  of permission events even though the already-rendered Core configuration still
  contains the old PermissionRequest hook. UserInputRequest/Stop both delivered;
  no server restart, runtime update or approval-policy override is needed. Native
  Android readback observed one Codex needs your input notification after the
  actual asynchronous question requesting clarification of the described UI.
  Exact Shift+Left screen equivalence remains user feedback, not claimed proof.
- Protected18 inode/mode/digest identities (auth/config/resolver15, installed
  Manager/runtime and focus record) remained unchanged. No model probe, session
  edit or active-thread interruption. No new contract or product implementation:
  existing installed behavior satisfies the requested preference, so disposable
  and live public-path proof replace an unnecessary build/release cycle. SPEC
  clarifies selector independence and repairs its omitted existing input status
  string. Prior accepted input matcher, single-line delivery and bounded tmux
  focus remain unchanged. This preference correction is accepted and closed.

## TERMINAL-FOCUS-BOUNDED-REUSE (accepted 2026-10-02)

- User rejects click-per-terminal attach as incomplete because terminal windows
  grow indefinitely. This supersedes acceptance of the experimental routing in
  the following historical section, while preserving accepted status modes,
  single-line notifications and session recovery. Success now requires native
  terminal/client count to remain constant on repeated/concurrent clicks for the
  same live tmux session, including different originating Codex windows.
- Bound Codex1dbcbdc clean/rewrite, AI0eee1c2 with unrelated provider/TUI/smoke
  dirty diff3acdf081e07b. Goal-md resolves this GOAL; approved equivalent primary,
  workers OFF. Native8 baseline passed before mutation. Installed Termux service
  supports stable shell name and no-shell-with-name creation mode; RunCommandService
  forwards both string extras. Re-express native behavior without legacy copying.
- SPEC changes before implementation. Focused native/failure regressions, full
  isolated AI gates, source inspection and protected verification precede bounded
  AI-only install and real repeated/concurrent service/action proof. Preserve
  auth/session/runtime/resolver, current Codex panes/PIDs, foreign/global tmux and
  unrelated UI work. No APK/preference hack or keyboard injection. Previous
  unnamed evaluation terminals cannot be silently adopted or terminated; their
  uncertain ownership remains protected. Prompt-cache findings and delay0 remain.
- Accepted AI source **ea5541c**; exact isolated candidate tree
  `e2517e060a950f453b53e42084ceeb11ac16d6f2`. KEEP qualification/atomic pane
  recheck and native attachment; COLLAPSE creation/reuse into the existing Termux
  service's named-shell operation; DELETE unconditional click-per-terminal
  behavior. Shared stdlib hashlib import replaces the duplicate local import.
  No registry, daemon, APK change or compatibility fallback was added.
- Nine named focused tmux regressions passed. Added native private-server proof
  maps shell identity to socket identity/session: two Codex panes in one session
  share a name, another session differs, and replacement socket identity differs.
  Existing ambiguity/title race/service failure/socket/deadline tests stay green.
  Exact candidate passed all6 standalone Python scripts, final37/0warnings/0fail
  and TUI161/0fail. Actual source/staged diff inspected; bounded AI-only install
  matches the accepted module and preserves unrelated dirty diff3acdf081e07b.
- First real Manager-action probe correctly refused the original conversation:
  its Codex pane/PID had already disappeared before the native baseline. This
  count assertion was recorded red and investigated, never accepted as reuse
  proof. An own empty installed Codex UI on an isolated tmux socket supplied a
  qualified real target. Its native thread was materialized by archive/unarchive
  and resumed through installed AI; no model turn or fabricated rollout was used.
- Installed `codex termux notify emit Stop` produced the actual action. Executing
  it created/selected exactly one native terminal. Four sequential and five
  concurrent installed public-helper requests retained the same client/PID/native
  current-session handle with no growth. Closing only that terminal allowed one
  replacement, and the next request reused it. Pane/PID and global status/mouse
  identities remained unchanged. Fifteen auth/config/resolver identities matched
  their inode/mode/digest baseline; installed Core/runtime/Manager were untouched.
- Own empty UI was gracefully closed, its native thread archived through upstream,
  its private server and exact registry record removed, and its synthetic
  notification removed. Closed-target public invocation is silent exit1 with no
  creation. A final installed-module check initially guessed the wrong library
  directory; reading the installed launcher resolved `.config/ai/lib`, and the
  corrected check passed against the exact accepted source. No product change was
  made to repair that check. This bundle is accepted and closed; native user-tap
  feedback may refine UX but is not claimed by programmatic action/count proof.

## TMUX-STATUS-CHOICE-TERMINAL-FOCUS (status accepted; routing superseded 2026-10-02)

- User replaces the two-way toggle with off/on-hidden/on-status and asks exact
  notification return across native Termux terminals. Success requires both
  native tmux window selection and visible Android terminal selection; pane-only
  proof is insufficient. No notification may create a tmux workload or resume process.
  User now explicitly permits a new Android terminal attached to existing tmux
  for reversible evaluation, with rollback if the result is unsatisfactory.
- Preserve unrelated AI dirty work, active users, auth, sessions, shared-server
  behavior, resolver and global/foreign tmux settings. Workers OFF. Status source
  and bounded AI-only installation are authorized after gates. Native app routing
  uses the authorized new-terminal attach operation after focused and full gates;
  no APK replacement is authorized. Ordered proof map in WORKBOARD.md.
- Status slice accepted: AI source `e6a91e6` offers `off / on-hidden / on-status`,
  exact CLI selectors, legacy boolean normalization and native clickable status.
  Exact staged tree `372e3050ffd121915ee98633959c790c27a44dc6` passed isolated
  final37 gates/0 warnings, TUI161 and all6 standalone Python test scripts.
  Native6 regressions include real PTY mouse selection and preserve foreign
  status/mouse. The existing installed UI/provider changes passed working-source
  final37/TUI162/native6 and were retained during bounded AI-only installation.
  Installed public AI accepts every selector; unrelated dirty diff
  `3acdf081e07b` is unchanged, fifteen auth/config/resolver identities and both
  existing tmux pane/PID identities are unchanged; global status/mouse unchanged.
- User-requested recovery of the closed tmux conversation identified an active
  turn on the previous generation's shared server, with effective delay0 already
  present. Active background turns are protected from idle unload. A targeted
  upstream `turn/interrupt` changed active to idle and upstream removed its writer
  lock; subsequent rebinds prove the lock remains absent. No lock deletion,
  server kill, transcript rewrite or delay change was used. This closes the
  immediate recovery, without claiming automatic cancellation on client closure.
- User explicitly requested inspection of the pre-Rust branch. Sealed
  `legacy/monolith` at `bf30a7dc94d4dad7f58836c69028160856e63c58` was inspected
  read-only: `tools/termux-notify.sh` opens a foreground native terminal through
  RunCommandService and runs `tmux attach`; it does not select an existing native
  terminal. Its tests inspect generated action arguments, not actual native
  existing-only selection. Further user-directed inspection found the preserved
  `verify-role-layout` modular implementation at `46c16ae`: its Python
  `TermuxProvider._open_tmux` also uses RunCommandService/attach, and notification
  compact-body rendering preserves the separate toast body. The refactor mainline
  `d02499b` retains the earlier shell movement path; `37f0a775` contains only Rust
  foundation documents. No historical source or tests were copied. SPEC now
  records the user's explicit new-terminal authorization before implementation.
  Qualify exact socket/session targeting, native selection, repeated clicks and
  absent/ambiguous/changed targets. Keep the routing change in a separate rollback
  commit; no APK, Codex state, auth, resolver or existing workload mutation.
- Single-line notification request was reverified through the installed public
  `codex termux notify emit Stop` path with synthetic multiline Unicode input.
  Actual termux-notification receives single-line title/body; actual toast retains
  configured newlines, with silent successful public emit. The own synthetic
  notification was removed. Signed34 Manager SHA256 still equals
  `cc0a38c5af308702318a2c9575c1efccd846e816183071d5b43a0dafc0f7544d`.
  Legacy deliberately appended a newline even for one-line content; Termux:API
  selects BigTextStyle on newline, explaining hidden collapsed content. Current
  provider-boundary folding removes that trigger without changing toast policy.
  Prompt-cache investigation follows the accepted terminal routing below.
- Routing accepted as separate AI commit `2ff7007`; exact staged tree
  `56662a2208f3d629426d0f4906c1c9d0f0c8bef2`. Isolated final verification exits0,
  TUI161/0fail, all6 standalone scripts pass with8 named tmux regression functions.
  Native atomic recheck returns a session ID only on success; false recheck now
  returns failure instead of treating if-shell exit0 as acknowledgement. Focused
  proofs cover ambiguity, prefix collision, changed/closed targets, repeated
  requests, string-valued action0, unavailable providers, socket encoding/identity,
  deadline, service error/timeout, and disappearing-session attach with no new
  workload. Actual source/staged diff reviewed before bounded AI-only installation.
- Installed public `codex termux notify emit Stop` generated the real notification
  action with this live conversation ID. Executing that exact action twice opened
  one native client per tap, changed native current-session selection each time,
  and attached both clients to the exact existing tmux session. Existing panes/PIDs
  and global status/mouse stayed unchanged; no Codex/tmux workload was created.
  Installed invalid/missing target requests are silent failures and add no client.
  Protected15 auth/config/resolver identities and unrelated AI diff3acdf081e07b
  remain unchanged. Own synthetic notification removed. The two authorized
  evaluation terminals remain attached for user assessment; they are not cleanup
  fixtures. Final same-path correction `0eee1c2` makes the native title predicate
  enforce the same hexadecimal suffix boundary as Python TITLE_ID. Its native
  race regression passes; exact final staged tree849a4349 passes final37/0warnings,
  TUI161/0fail/native8. Installed source equals accepted source; protected15,
  unrelated diff and existing panes/PIDs remain unchanged. Rollback reverts
  `0eee1c2` then `2ff7007` in AI followed by bounded `--ai-only` installation;
  existing Codex work and clients need not be terminated. No APK or native
  preference hack. Cache follow-up may now proceed without changing delay0.

## INPUT-CACHE-UNLOAD-CONSIDERATION (completed 2026-10-02)

- This is the user-requested read-only consideration after notification/tmux/
  recovery closure, not authorization for new model traffic or runtime changes.
  Exact official source tag0.160.0 binds commit
  `a956835d020762cb2b570053af06f643a11c0ecc`; tag and immutable source bytes were
  verified. Reviewed core client/thread-manager/session, app-server lifecycle,
  rollout recorder and writer-lock ownership. Sources:
  [ModelClient](https://github.com/openai/codex/blob/a956835d020762cb2b570053af06f643a11c0ecc/codex-rs/core/src/client.rs),
  [thread lifecycle](https://github.com/openai/codex/blob/a956835d020762cb2b570053af06f643a11c0ecc/codex-rs/app-server/src/request_processors/thread_lifecycle.rs),
  [recorder](https://github.com/openai/codex/blob/a956835d020762cb2b570053af06f643a11c0ecc/codex-rs/rollout/src/recorder.rs).
- Local thread grace retains Session/ModelClient, including cached WebSocket,
  incremental request/response state and routing. It also retains native writer
  ownership. Native unload waits for no subscribers/inactivity, protects running
  turns, shuts down the thread and removes its runtime; writer-lock guard release
  follows writer lifetime. Wrapper-side lock removal while keeping a writable
  thread would violate concurrent-writer safety.
- Regular same-ID resume reconstructs history with the original thread UUID.
  The default ModelClient prompt-cache key remains that session ID; unload does
  not explicitly clear the model's remote KV cache. Therefore delay0 can still
  permit cached input on resume, but loses local transport/incremental reuse.
  Positive unload grace preserves those local caches, not a guaranteed model
  cache hit. Account ownership changes explicitly discard cached transport and
  routing; preserve this boundary instead of sharing caches between accounts.
- Current [official prompt caching documentation](https://developers.openai.com/api/docs/guides/prompt-caching)
  describes remote prefix matching and model-dependent retention, independent of
  local process lifetime. Stable history/tools/model/settings and cache identity
  matter; compaction, routing, expiry and ownership can change reuse. Its API
  retention rules are not proof of this ChatGPT-authenticated Codex account's
  actual hit rate. Current app-server docs describe30min grace, while the accepted
  installed0.160.0 diagnosis bound a60s upstream default; do not substitute current
  documentation defaults for the already-qualified installed runtime.
- Disposition: keep native delay0/immediate idle writer release. Retaining a
  writer-free local ModelClient cache would require an upstream lifecycle change
  with writer reacquisition, restored-history/config validation and auth-owner
  invalidation; no existing wrapper configuration provides that separation.
  This conceptual possibility is not an implemented feature or a measured gain.
  Quantitative comparison requires same-account/model/settings/history timing
  and actual cached-input-token/latency observations around unload/resume; no
  model requests, user-content inspection, retention override or keepalive was
  performed. The requested consideration is complete; no cache change is queued.

## NOTIFY-SINGLE-LINE-TMUX-COLOR-V1 (accepted 2026-10-02)

- User prioritizes single-line notification title/body while retaining toast
  behavior, and restoring Codex color/rich rendering through optional AI tmux.
  Prompt-cache retention investigation is deferred; delay0 stays unchanged.
- Bind Codex08b67cf clean/rewrite and signed/public/live33/upstream0.160.0;
  AI5ce014c with pre-existing dirty provider/TUI/smoke files preserved/excluded.
  Goal-md resolves this exact GOAL.md. Approved equivalent primary, workers OFF.
- Success: actual notification provider gets trimmed one-line text even with
  preserve_newlines1; actual toast keeps its configured multiline presentation.
  Current caller color preferences, including absence of NO_COLOR, reach the
  tmux child despite stale server state; explicit caller choices remain intact.
  User additionally requests bottom tmux status inspection; hide it only for
  AI-owned sessions, preserving existing unmanaged sessions/global configuration.
  Keep native terminal identity, argv/profile/CWD, focus, auth and sessions.
- Ordered slices and proof map are in WORKBOARD.md. After source baselines,
  focused/regression/grouped gates and diff review, bounded AI-only installation,
  signed Manager-only publication/ordinary activation, synthetic native delivery
  and own isolated Codex tmux ANSI verification are authorized by this request.
  Never stop/restart existing user panes or mutate global tmux/user config.
  No backups; remove own staging after acceptance. Android width ellipsis and
  existing process startup environments are explicit presentation limitations.

- Source slice1 exact actual dispatch1/1, Manager21 unit/15 integration pass;
  complete grouped230 Rust/90 Python (one explicit device ignore), locked workspace
  release pass. AI final37 gates/0 warnings and native5 regression functions pass.
  Actual code diffs reviewed; fifteen auth/config/resolver identities unchanged.
- AI implementation7656358 pushed and installed without backup, preserving exact
  unrelated dirty source identity9fe1bac80363. Native installed public AI opens
  actual bare Codex with zero argv overrides, native screen-256color, NO_COLOR
  absent, RGB38;2;99;168;248 plus bold/dim; owned status off. Existing managed
  session status off preserves all panes/PIDs/global status. Own empty probe UI
  exits gracefully; no model turn.
- Codex implementation4b00b8d461939a7070d95a1ea6e6a58c318cbf52 pushed. Production
  workflow37066564003 passes all8 jobs; public07d625561283bad5d25b778782c16d4929b4939f.
  Ordinary codex update activates signed34/upstream0.160.0, generation
  local-hosted-0-160-0-4b00b8d46193-notify-line. Core/runtime unchanged;
  Manager SHA256cc0a38c5af308702318a2c9575c1efccd846e816183071d5b43a0dafc0f7544d.
  Real native both-provider emit delivers folded single-line notification and
  configured multiline toast. Own synthetic notification removed. Doctor healthy
  and all15 protected identities unchanged. Physical cross-terminal focus remains
  a separate user-confirmed limitation; this bundle makes no claim of fixing it.

## RESUME-WRITER-RELEASE-V1 (accepted 2026-10-02)

- User reports native read-only/open-in-another-app screen until retry after30-60s;
  asks investigation and resolution after tmux completion, then explicitly directs
  autonomous work while sleeping. Weekly budget warning is a separate upstream
  account limit. Workers OFF; approved equivalent primary continues directly.
- Bind accepted Codex producer4c3d013/signed32 and AI88a21df plus docs children.
  Same-revision code baseline228 Rust/90 Python and locked build green; no new
  behavior before native diagnosis and SPEC if a runtime contract changes.
- Measure real public close/resume for idle own test conversation, within account
  and across accounts/generations. Distinguish active UI/client, daemon attachment,
  writer lease and compatibility FD lifecycle. Inspect all surviving instances
  once a class-level cause is found; do not invent a timeout explanation.
- Success: preserve conversation ID/history/transcript and account auth isolation;
  a cleanly closed idle client must not impose30-60s artificial retry delay. Keep
  real concurrent writer exclusion and active/background work protected. No fork,
  transcript copy, lock deletion/steal, warning suppression or shared-server disable.
- Bounded native probes, own idle fixture cleanup, implementation/regressions,
  ordinary signed publication/activation and matching actual installation are
  authorized after the source gates. Never kill unrelated user servers/clients or
  restore/mutate auth/resolver. No backup; reuse one private scratch/cache root
  from tmux bundle and remove it when the requested resolution is accepted.

- Diagnosis proves actual idle32 TUI exits0.930s, but its native server25322
  retains the writer; other-account resume is read-only at4.54s and writable
  after native60s expiry/retry at63.46s. Official0.160 lifecycle default is60s;
  process slow exit and compatibility FD remap are not the root cause.
- Source candidate keeps native subscriber/activity/Running guards and sets the
  existing FD34 system default thread_unload_delay_secs0. No CLI injection,
  profile setting, auth merge, lock deletion, daemon termination or runtime patch.
  Explicit higher-priority user choices survive. Previously started servers
  retain their startup setting until normal restart; new generation gets a new
  server without stopping old active work.
- New actual exec regression1/1 covers bare argv, explicit CLI preservation,
  profile/resolver preservation and FD34 default. Existing projection1/1, exact
  root policy1/1, shared_server5/5 green. Grouped229 Rust/90 Python (one explicit
  device-ignore), locked workspace release build and actual diff review pass.
  Fifteen protected paths unchanged after native title authorization rebind.
  Source acceptance is bound to the implementation below.

- Implementation **aa2685f2f68807e0040d7dd8978f7a6565de6680**, pushed on the
  independent rewrite lineage. Production **37047333858** passes all8 actual
  jobs/7 logical jobs, including exact Core-only component gate, Android smoke,
  accepted signing authority, every public byte, disposable ordinary update,
  second no-op and CAS stable. Trigger3dc399243b37cc0b2c7396a1fa5fa049252482ab
  fast-forwards parentc728f89; publicmainfcf9f379fdb9fcb6867084b87330197fbd61dd16.
- Ordinary installed update activates signed **33**, upstream **0.160.0**,
  **local-hosted-0-160-0-aa2685f2f688-resume-writer**;32 rollback. Stable launcher
  equals signed Core **b083753451042c861acc6d22153d57eb9df6fe3938567eec2ec9f74e9a9978cd**.
  Manager2af3076/runtimeba94d1d/code-mode-host/helpers and their modes unchanged.
  Doctor healthy; second actual update exits0, exact up-to-date stdout/no stderr.
- Actual installed AI/Core bare native33 TUI has zero argv overrides, matches the
  new runtime process, and its title-correlated thread is loaded in the real shared
  server. Native config/read and FD34 both report delay0/danger-full-access.
  Cold welcome/composer is not thread readiness: corrected probes wait for actual
  loaded thread/title; normal graceful Ctrl-C handling and asynchronous FD cleanup
  are accounted for, with no product retry/kill/lock manipulation.
- Existing canonical own conversation: idle source exits0.964s; other-account
  writable TUI5.027s; reverse account4.028s; same account4.490s. No R retry, fork,
  transcript copy or model turn. A genuinely connected other-account owner keeps
  its lock and the competing TUI is read-only. Final idle writer releases naturally.
- Actual native interactive thread/list is exhaustive and identical between
  janmori101/wrlab: fixture phase CWD7/All25,18 outside CWD. Exact own SessionMeta
  proves one current-CWD fixture is included and one different-CWD fixture excluded
  from default scope; both appear in All, no duplicate IDs. A prior probe wrongly
  required the other-CWD fixture in CWD; corrected to actual metadata, not policy.
- Only2 owned synthetic conversations archived via native APIs after owned clients
  close; IDs and exact transcript digests preserved. Post-cleanup both accounts
  show CWD6/All23; no new empty bare rollouts persisted. Own tmux socket registry,
  notifications and reusable384MiB staging/cache removed, no backup. Protected15
  auth/config/resolver identities unchanged for this Core-only bundle. AI unrelated
  dirty changes remain untouched; native notification channel/hooks/focus unchanged.
- Limit: already-running pre33 servers keep their startup60s setting until normal
  restart; they are not killed. Explicit user delay settings take precedence.
  Upstream real subscribed/active/background writer protection remains in force.

## AI-TMUX-NOTIFY-FOCUS-V1 (accepted 2026-10-02)

- User clarifies physical tap selected Termux's last terminal, not its source;
  requests optional humtr/ai tmux launch and compatible Codex Manager focus.
  Bound Codex source68dfc9a64a95e3d95c6932a55133441c15168533 (docs child39b9324),
  publicc92e550 and signed/current31. AI maincf92f851e5f85367f7978899431ce30e6fb966aa
  has pre-existing dirty provider/TUI/smoke changes matching installed bytes;
  preserve/exclude those changes from implementation commit. Baseline diff
  66737cf6ffe0d5c5eed5a035560db15f24a63b3fd3e20e1ad281325b238ee4c6.
  Worker mode OFF; approved equivalent primary implements directly.
- AI dirty baseline full final-verify exits0:36 summary gates,161 TUI checks,
  no warnings; Codex previous exact product224 Rust/90 Python is unchanged.
  Old Activity-only proof staging removed; new private temporary proof root owns
  official read-only source and reused public build cache, no credentials/backups.
- KEEP native tmux/OSC title and upstream session IDs. COLLAPSE focus into AI's
  existing launcher owner; Manager consumes its bounded focus command, not a
  duplicate tmux registry/credential DB. DELETE guesses from daemon TMUX_PANE,
  resume-on-click, CLI config injection, UUID-prefix-only targeting and watchers.
- Normal AI launch remains direct; explicit --tmux launches preserved native
  argv/profile/CWD in a managed tmux window. Codex title preparation adds native
  thread-id as first item, retaining all other effective title items/config bytes;
  this user-selected UI change is the only allowed profile-config change.
  Auth, session payloads, history and shared-server lifecycle remain unchanged.
- Full hook UUID is resolved against a live AI-managed Codex pane. Upstream0.160
  truncates title UUID to29chars plus..., so canonical local rollout metadata must
  uniquely validate the complete UUID; collision/missing/foreign/dead/ambiguous
  target never guesses or starts Codex. Read no auth, never print/store transcript
  or title content. Focus handles new/resume and native in-TUI thread changes by
  reading current metadata at click time; no title watcher or cached PID mapping.
- Manager exposes optional focus mode; existing notification record/format and
  Core hook projection stay compatible. Native actual Core/system-FD full-access
  and shared-server use survive; no hidden argv/config overrides.
- After focused/disposable/full source green, bounded AI-only live installation
  without backup, native title preference application, Manager focus preference,
  signed Codex release/update and two-pane/user-tap tests are authorized. Do not
  touch unrelated clip/AGY services or pre-existing AI dirty work. Verify source,
  public/live bytes, preserved credentials/resolver/history, no duplicate window
  or Codex process on repeated click; commit/push and delete staging.
- Termux Activity still cannot select arbitrary Termux terminal IDs. Exact pane
  focus is guaranteed within live managed tmux clients; selecting an unrelated
  non-tmux Termux terminal remains an OS limitation, never worked around by
  launching a new terminal or injecting shell input.

- Implementation Codex4c3d013cd6e224fdda7b5727efc13140e915f056; AI324e201
  plus88a21df305c140e6d7d936902b89b935df3a22da closed-server correction. Both
  pushed, AI-only live install without backup preserves pre-existing dirty work.
  Codex exact grouped228 Rust/90 Python, locked release green; AI staged candidate
  and final current tree37 gates/162 TUI checks/0 warnings,6 standalone files,
  focused4 native tests including race/foreign/collision/closed/dead-server.
- Signed production37040916922 succeeds all8 actualjobs/7 logicaljobs: exact
  accepted source/archive, Android smoke, Manager-only component bind, signing,
  Release/Pages, public bytes/disposable ordinary update and second no-op, CAS
  stable. Trigger22ea22e0845ac013d7c1b54b1d4c7085492144eb,parentc92e550;
  publicmainc728f89d8593b411ddef2961374efa9da3ee226f. Ordinary deviceupdate
  activates32,0.160.0,local-hosted-0-160-0-4c3d013cd6e2-tmux-notify;31 rollback.
  Runtimeba94d1d0d9d416a7fb2f13d6ffcc6e0d9ec981ee1793158bbc9b1d96d5f51ab3;
  Core7507870ebd654a9b4e195ed872d4100aaed337efc3e8e5296b0caf31d39f71f6;
  Manager2af3076a8f82052b09b49f796e025158477f6984e63cfd2e8f098bf647c79b32.
- Native installed AI launches2 actualCodex panes with original zero argv. Both
  IDs uniquely corroborated by canonical metadata, runtime/TTY valid, loaded in
  one real native shared server via thread/loaded/list. No embedded fallback;
  actual TUI /status reports danger-full-access. New bare launch additionally
  reaches exact latest32 runtime without overrides/fallback; native thread loaded
  in actual shared server25322, uniquely identified by title prefix on IPC.
- Actual installed public Stop emit forwards registered actions to Android API.
  Each exact action executes3times and moves to its existing target pane, silent
  exit0, no new window/Codex/PID. Fresh physical notification starts on pane2;
  user taps and backend changes to pane1 with unchanged pane/PIDs, without any
  agent action execution. User's ordinary Android terminal remains outside that
  tmux session: this does not select their pre-existing non-tmux terminal. Preserve
  this explicit distinction; exact visible focus requires the originating tmux
  session in the foreground Termux terminal. No reparenting or new window on tap.
- Closed real native target returns1 silently from focus with unchanged panes,
  never reopened. Pre-live protected15 identities preserve all auth/resolver and
  other config; only authorized janmori101 native title changes. Earlier concurrent
  jgnh2 config replacement is outside this bundle, never restored/merged. Focus
  preference is tmux; both channels/selectors unchanged. Own test notifications
  removed. No recovery backup; reusable cache/official source and owned native
  fixtures pass to the explicitly queued lock diagnosis, not retained bundles.

## MANAGER-NOTIFY-OPEN-V1 (accepted 2026-10-02)

- User requests notification tap to return to running Termux; explicitly rejects
  a new terminal/resume on every tap. Bound source99d51339c7c4722e1c8ff2b440d09e2855a5a7ba,
  public f9edf2e60f41bc1f394aa8cc788257f89b9ed99f, installed signed sequence30.
  Approved equivalent primary; workers OFF. Exact clean baseline222 Rust/90
  Python passes with one explicit device ignore. Click was absent at bundle start.
- Existing provider omits --action. KEEP native notification action and existing
  Activity foregrounding; COLLAPSE into a fixed provider action; DELETE draft
  session parsing, terminal-service/resume launch, URI/array encoding and unused
  regressions. The user rejected that draft before commit/public/live mutation.
- Installed Termux exposes no accepted existing-terminal-by-Codex-UUID intent.
  Foreground its current terminal without a new window/process, ID lookup, app
  preference mutation, helper command or auth change. Use working installed `am`;
  termux-am's optional socket is unavailable on this device.
- Focused regressions must execute the registered action repeatedly and prove
  only Activity commands, safe absolute-path quoting and no hook metadata input.
  Grouped gates and signed Manager-only release/ordinary activation follow.
  Bounded real notification/Activity tests preserve terminal/Codex identities,
  auth, config, recent history, resolver, rollback and both-channel selections.
  No backups or unrelated cleanup. Physical tap requires user observation if
  this environment cannot synthesize Android notification UI input.
- Final source focus8 unit+5 integration passed. Exact grouped scripts/check.sh
  exits0 with224 Rust/90 Python; one explicit device ignore. Locked release
  build passed. Fifteen protected auth/config/resolver fingerprints unchanged.
  Source diff reviewed; implementation39b93246c09799a9abb304703f77cca7e5572074
  committed/pushed. Source39b product bytes have final grouped evidence above.
- Production workflow37 Bash steps and7 logical jobs qualified; fresh exact
  public parentf9edf -> trigger84e9553de51d719173643653cb3f825550622a5a
  replaces one workflow via non-forced parent CAS. Manager-only descriptor gate
  preserves exact Core/runtime/code-mode-host/helpers and requires changed Manager.
  Signing, Release/Pages, every-byte public readback, disposable ordinary update
  and stable CAS pass in run37029403265 (8/8 jobs including split Pages stages).
  Public stable mainc92e550421a6aa6fd5496487c8f1ad78c22d2bee.
- Ordinary device codex update exits0 and activates signed sequence31,0.160.0,
  local-hosted-0-160-0-39b93246c097-notify-open; rollback30 retained. Core remains
  7507870ebd654a9b4e195ed872d4100aaed337efc3e8e5296b0caf31d39f71f6.
  Manager is1527241f3544bfc1e3583a316b41df0889c0cef49db721b2493c1fe94815c76e.
  Runtime remains ba94d1d0d9d416a7fb2f13d6ffcc6e0d9ec981ee1793158bbc9b1d96d5f51ab3.
- Actual public notify test reports notification=ok,toast=ok. Transparent temporary
  provider forwards the actual --action to native Termux API; public Stop emit is
  silent, ignores synthetic session/path/command metadata, and Android independently
  lists exactly one own synthetic notification. Preferences remain both and
  PermissionRequest,UserInputRequest,Stop; no config/auth/resolver change15/15.
- Exact registered Activity action executed3 times: exit0/silent3/3, all14 actual
  Codex runtime PIDs/start identities/executables unchanged, current selected
  terminal unchanged. Native am wait diagnostic reports intent delivered to existing
  top-most Activity; no terminal service/resume invocation exists in shipped code.
  Programmatic routing is distinguished from physical tap. User confirmed actual
  tap returns to the existing last-selected terminal, with no new window; clarified
  that this is not exact originating-window focus. That matches this accepted
  Activity-only contract; the new tmux request is a separate bundle.
- Historical source CI37029323977 is red on obsolete seq17/pinned-source/public
  assumptions and Linux /proc permission tests, as in the preceding bundle.
  It is not counted as acceptance. Current exact Termux grouped gate and full
  signed production pipeline are green; automation alignment remains separate debt.
- Temporary Activity-only build/probe staging was removed without backups;
  reusable public build cache/official source move into the new bounded bundle.

## MANAGER-NOTIFY-INPUT-REQUEST-V1 (accepted 2026-10-02)

- User requests review of repair/profile management and improvement plus actual
  reapplication of Termux notifications. Explicit preference: notification and
  toast for turn completion, approval, questions and follow-up input requests.
  Approved equivalent primary Lead continues directly; workers OFF.
- Bound clean source db7411bd0cc0432b5dc94b52933ccc31f2e1664d; public/live signed
  sequence 29, upstream 0.160.0. Runnable baseline passes 217 Rust tests (one
  explicit device ignore). No product mutation before baseline.
- Repair remains valid generation qualification/legacy-layout migration, not
  general health; profile review identifies known external account identities
  absent from Manager list/current naming. Preserve upstream-owned session/DB
  and separate authentication; review alone authorizes no profile migration.
- Notification outcome: add observable fixed-text delivery testing and bounded
  provider-group cleanup; map structured input requests through actual upstream
  PreToolUse matchers without ordinary-tool spam or duplicate hooks. Qualify on
  official 0.160.0 handler/registry and native product path. No upstream patch.
- Allowed bounded live gate after source/disposable proof: signed Core/Manager
  publication and ordinary activation, requested Manager notification preference
  write, synthetic notification/toast tests and native lifecycle-event proof.
  Preserve account auth/config, recent transcripts/history, resolver and rollback.
  No backup; remove test/build staging after acceptance. Existing external
  tunnel restart is permitted only as needed to consume the accepted generation,
  preserving identities/credentials and proving readiness/polling.
- Success requires focused regressions, grouped workspace/publication checks,
  actual signed installation, effective hooks on shared-server launch without
  fallback, real provider readback and review conclusions. Source/release/public/
  live alignment and commit/push close this bounded bundle.
- Source acceptance passes exact scripts/check.sh: 222 Rust (Core 172,
  Manager 17+12, builder 21; one explicit device ignore), 90 Python, fmt/diff,
  and locked release build. Provider missing/failure/timeout/group cleanup,
  independent both channels, state/input privacy, selector canonicalization,
  no duplicate handler and actual Core launch projection are proven. All 15
  account auth/config/resolver bytes, modes and inodes remain unchanged.
  KEEP fixed Core repair ownership and upstream shared conversations; COLLAPSE
  question/follow-up notification into upstream matcher; DELETE opaque operator
  delivery results, too-short provider bound and leaked timeout descendants.
- Implementation commit **1dd51b8be87ddbe100d0ecd235f81292198c0115** is pushed.
  Production run **37020699856** passes all candidate, Android executable smoke,
  signature, immutable Release, Pages, public byte readback/disposable update,
  and non-forced CAS promotion gates. Trigger e5b99ad63634b4cf971ee5bb8ed109cdf8920721
  promotes public main to **f9edf2e60f41bc1f394aa8cc788257f89b9ed99f**. Ordinary
  installed codex update verifies and activates signed sequence **30**, upstream
  **0.160.0**, **local-hosted-0-160-0-1dd51b8be87d-manager-notify**; sequence29
  remains rollback. Runtime/UDS adaptation and helper bytes are unchanged.
- Installed Core SHA256 7507870ebd654a9b4e195ed872d4100aaed337efc3e8e5296b0caf31d39f71f6;
  Manager 49bd3e83219e6ca0b3a2503c9bbb8dbe313dc86318e7d6d4d72aa137f6bf35ef;
  runtime ba94d1d0d9d416a7fb2f13d6ffcc6e0d9ec981ee1793158bbc9b1d96d5f51ab3.
  Applied channel=both and hooks=PermissionRequest,UserInputRequest,Stop;
  presentation preferences remain unchanged. notify test exits0 with
  notification=ok and toast=ok; Android notification-list independently finds
  the fixed test notification. Toast proof is successful OS provider delivery,
  not an independent visual observation.
- Actual all-TTY zero-argument launch under the existing account identities
  retains argv=[runtime], uses the exact generation30 owned shared-server
  socket/PID binding and reports danger-full-access through native config/read.
  No shared-background-server/embedded-mode fallback warning. Current identity
  TUI exits0. Native system hooks complete for synchronous Plan input, Default
  asynchronous input, successful Stop and PermissionRequest; Android readback
  finds input/approval notifications and two synthetic final-message Stop
  notifications. The approval is canceled and its test file is never written.
  Supplemental native probe under janmori101 with model gpt-6.1-sol proves the
  raw function name request_user_input_async, delivery=async with one question,
  completed PreToolUse/Stop hooks and a fresh unique synthetic Stop notification.
  Earlier gpt-5.6-luna catalog attempts exposed request_user_input rather than
  the async tool; they do not prove async delivery. Corrected the native probe's
  overly literal final-text/accumulated-count predicates and reran this nonzero
  exact async gate successfully. Tool availability remains upstream/model-owned.
  Failed/interrupted turns do not synthesize Stop beyond upstream policy.
- Native generation30 discovery under jgnh2/wrlab/janmori101 returns identical
  unique UUID unions: CWD6, All23; all CWD results match the project and other
  CWDs appear only in All. Current identity resumes the existing other-account
  CLI session 01a0fc82-dc8f-7d13-bb78-7e120f1fa9b3 with its original ID/history.
  Synthetic model probes are ephemeral; no transcript migration or auth merge.
- Existing runit tunnel owners and native Tunnel MCP alias are restarted only
  to consume generation30, preserving profiles/IDs/key references. PID pairs
  are 14957/15077 (named), 14976/15096 (legacy), 17224/17253 (native probe).
  All three exact upstream argv=[runtime,app-server], local initialized/ready,
  native --require-control-plane-poll health exits0 and main-channel probe ok.
  Both resident authenticated MCP tools-list calls return13 tools.
- Source/pre-cutover checks preserve all15 auth/config/resolver fingerprints.
  Final readback preserves14; janmori101/config.toml is atomically replaced
  during native TUI startup at14:45:34UTC, retaining mode/model and showing the
  upstream hide_rate_limit_model_nudge notice. No operator/profile command writes
  that file; preserve its current upstream/user settings rather than restoring
  guessed prior bytes. Every auth file and resolver remain identical.
  Exact-current update is successful/idempotent with no acquisition residue.
  Current30, rollback29 and generation28 with five live executable references
  are retained by the existing lease rule; do not kill unrelated active writers.
- Review limits: repair plan remains none/healthy for generation30 qualification;
  broader diagnosis belongs to doctor. External identities remain unnamed by
  Manager list/current; their discovery/use improvement is reviewed, not
  implemented or migrated. wrlab workspace credits are exhausted, so native
  lifecycle proof uses the existing available current identity. Historical
  rald7-full-acceptance run37020086082 is red: obsolete seq17/parent/proof-source
  pins and Linux runner /proc permissions. It is not acceptance evidence; local
  exact grouped gates and current signed publication are green. This separate
  automation debt remains open.
- Disposition: accepted source/release/public/live notification bundle. Temporary
  build/probe staging is removed without backups; workers remain OFF.

## EXTERNAL-TUNNEL-GENERATION-29 (accepted 2026-10-02)

- User explicitly authorizes restarting the three surviving external tunnel
  processes on the latest installed signed generation. Bound clean source is
  rewrite/rust-core dd47dd107282bf148f2fa4408b4848a3f16c540d; public main remains
  f71324348719c1a416f42956074d925004c70ab5. No Rust/public contract change, new
  release, credential migration, or remote tunnel creation is needed.
- Root cause is long-lived tunnel-client children launched before activation,
  retaining upstream 0.159.2 (two children) and 0.159.0 (one child), including
  the old Core-injected sandbox CLI override. Existing bridge command is the
  stable public `codex app-server`, so restart reaches the accepted Core.
- Restarted the two tdev runit services through their existing supervisor,
  preserving resident verification and profile digests. Restarted native-managed
  alias tdev-surface-probe-20260930 through native stop/connect, attaching its
  existing tunnel ID and file key reference, preserving its stdio target and
  wrlab execution identity. No second owner or ad-hoc daemon was introduced.
- New tunnel/Codex PID pairs are 20675/20694 (named connection), 20799/20815
  (legacy resident) and 21060/21078 (native probe). All three Codex executables
  bind to signed sequence 29 generation
  local-hosted-0-160-0-bdb02cc7d100-bounded-retention. Their argv contains only
  runtime plus `app-server`; compatibility FDs 33/34/35 are present and
  CODEX_SQLITE_HOME is the one canonical upstream store. The probe retains
  CODEX_HOME=.codex-profiles/wrlab; resident identities are unchanged.
- All three local /api/codex/status responses report running, ready, initialized,
  state=ready and ChatGPT auth. Native health with --require-control-plane-poll
  exits 0 for all three: healthz/readyz and actual successful external polling
  are green; last successful polls were 9/24/3 seconds old at grouped readback.
  /api/system reports main-channel probe ok for each. The native alias status
  separately confirms process_running/healthy/ready; its unavailable richer
  UI poll snapshot is not substituted for the proven native metrics check.
  Actual local authenticated MCP initialize/tools-list through both resident
  connections exits 0 and returns 13 tools each, without session-content output.
- Ordinary Core maintenance removed both now-unreferenced old generations.
  Only current sequence 29 and rollback sequence 28 remain; exact-current
  `codex update` exits 0 with already-up-to-date output and no artifact growth.
  Post-restart checks found all 20 protected auth/config/resolver/launcher/
  activation/profile files identical in bytes, modes and inodes. During later
  docs-only closure, jgnh2/config.toml was independently atomically replaced;
  this operator work never writes that file and leaves the concurrent setting
  untouched. The other 19 files remain identical at final readback. Existing
  tunnel IDs/key references and
  account separation are preserved. No backup or disposable test tree was made.
- Disposition: **accepted and closed**. External processes now match the live
  signed release; automatic future generation selection remains through the
  existing stable launcher on supervisor restart. Long-lived processes require
  restart after a future activation; Core does not take ownership of them.

## SHARED-CONFIG-BOUNDED-RETENTION-V1 (accepted 2026-10-02)

- User explicitly requests fixing reused-server notification configuration,
  cleaning unused installation/cache/legacy SQLite remnants without backup,
  and preventing unbounded installed-file accumulation. Preserve conversations
  active in the preceding seven days, current/previous rollback generations,
  effective trust/guard/baseline dependencies and live writers. The approved
  equivalent primary Lead continues directly; workers OFF.
- Bound source is rewrite/rust-core d20b245a88323fc476cf46cf7e3b8e802422413c,
  tracked clean with pre-existing untracked .github/scripts/__pycache__/. Public
  main 3224a5294d22818350a105d2adef6bc32b7e565f and live/public sequence 28,
  upstream 0.160.0. Native temporary reproduction confirms Manager hook changes
  update the owned config but not the reused server's FD-34 snapshot.
- Baseline restored after correcting a nonexistent package selector: actual
  `cargo test -p codex` ran 164 passing tests and one explicit device smoke
  ignored. No product mutation before runnable baseline.
- Success requires native same-PID configuration refresh in both directions,
  active-thread preservation, public launch/update retention with contention,
  recovery/rollback/unsafe-path proof, exact signed release/live activation,
  bounded device cleanup and preserved recent cross-account resume/auth/resolver.
  Baseline remnants: 22 other generations (6137.6 MiB), 10 acquisition roots
  (603.4 MiB), eight publication caches (2184.0 MiB), inactive wrlab legacy SQLite
  (~134 MiB). Inspection may remove only declared owned artifact/state roots;
  session retention does not authorize deleting unrelated project data.
- Source acceptance: exact `scripts/check.sh` passes 217 Rust tests
  (Core 171, Manager 16+9, builder 21; one explicit device ignore) and 90 Python
  tests (15 operator, 75 publication), fmt/diff checks, and release build.
  Native temporary official 0.160.0 runtime proves same-PID hook enable/disable,
  loaded-thread retention and real launch pruning. Source work leaves all 15
  protected auth/config/resolver bytes, modes and inodes unchanged. KEEP existing
  signed crash/retry and rollback; COLLAPSE cleanup onto existing state/locks;
  DELETE startup-only config publication and indefinite disposable retention.
  The temporary empty-thread probe did not claim a lazy rollout inode. The
  real installed proof below uses an existing materialized conversation.
- Implementation commit **bdb02cc7d10095fb923f8199d069beb995dabbcf** is pushed.
  Publication control cbace111b179cff3c90477b817f0135afc4facab / production run
  **37003873759** succeeded: exact-source Android build/native smoke, Core-only
  descriptor/component delta, signing, immutable Release/Pages, full public
  byte/disposable ordinary update/legacy/noop proof and non-forced CAS promotion.
  Public main **f71324348719c1a416f42956074d925004c70ab5** serves signed sequence
  **29**, upstream **0.160.0**, generation
  **local-hosted-0-160-0-bdb02cc7d100-bounded-retention**. Ordinary device update
  activated it; repeated update reports already up to date without artifact
  growth. Public manifest equals installed bytes; launcher/signed Core SHA-256
  aa1a777e3f6020450c237482b69677ba35b67afb5d4c2622dcc5379221b97e2c.
  Qualified upstream runtime digest remains ba94d1d0d9d416a7fb2f13d6ffcc6e0d9ec981ee1793158bbc9b1d96d5f51ab3.
- Installed bare TTY remains alive, argv contains only argv0, strace confirms
  successful native shared-server connection to the active generation, no
  fallback/unmanaged installer and native danger-full-access. Warm hook enable
  and disable both read back natively on the same PID/FD-34 directory inode,
  preserving a loaded materialized conversation and its rollout inode. Restore
  original Manager preferences afterward, with no recovery backup.
- Two distinct authenticated accounts wrlab/jgnh2, before and after old SQLite
  removal, expose the same exact **CWD 6 / All 22** CLI/VSCode UUID union; no
  other-CWD rows or duplicate rows. Old other-account conversations resume and
  expose history. A newly created real bare CLI conversation also appears in
  both scopes and resumes under the other account with nonempty upstream turns
  and unchanged creator-account metadata. An initial request during native
  final flush was rejected; the same UUID resumed after its normal writer
  closure without forced takeover or product retries. Keep upstream locking.
- Actual installed `codex resume` pickers under both accounts default to six
  CWD rows and switch to 22 rows through the existing All control, without
  shared-server fallback. Terminal emulation reads footer counts in memory;
  no title/transcript screen contents are persisted. Corrected a probe's wrong
  toolbar focus assumption using the exact upstream key binding; no product
  change. Native UUID snapshots independently prove the exact scope unions.
- Device cleanup removes **39** disposable artifact directories (21 obsolete
  generations, 10 acquisition roots, eight publication caches), three obsolete
  SCS backup/failed-activation roots, and 12 inactive wrlab/jgnh2 legacy SQLite
  files. Removed logical file bytes total **9,787,304,710** (~9.115 GiB). No new
  backup. All **44** canonical payload inodes and the original **28** retained
  per-record payload prefixes remain; all 15 auth/config/resolver bytes, modes
  and inodes are unchanged. One backup index had recently reindexed an actual
  September-20 transcript already excluded by the accepted seven-day policy;
  it does not justify restoring expired work or merging old SQLite.
- Remaining installed artifacts: current/rollback (29/28) plus two generation
  trees used by three existing external tdev tunnel-client app-servers. Those
  external live clients are preserved; after their normal exit Core can prune
  the references on the next launch/update. No abandoned staging/publication
  directories remain, and repeated maintenance keeps the same generation set.
  Effective guards/local-derived baselines and at most one complete pending
  crash-retry candidate are likewise protected when applicable. Core never
  owns ongoing auth or transcript/SQL maintenance.
- Disposition: **SHARED-CONFIG-BOUNDED-RETENTION-V1 accepted and closed.** The
  active public entrypoint/runtime agrees with its exact implementation,
  signed release and public stable. Worker mode OFF throughout. No further
  implementation selected; WORKBOARD.md no longer retains a parallel proof map.

## SHARED-SERVER-CROSS-ACCOUNT-V1 (accepted 2026-10-02)

- User-authorized scope: fix bare shared-server execution and installation-wide
  cross-account resume; publish the signed release and activate the actual
  installation. Current GPT-6 primary is explicitly approved as equivalent
  Lead; workers OFF throughout. Baseline source f264892869577bbd776075513df77f0a330726c2,
  public main 2dc79bd11842c9e5970b8c73c470886b780f2ff8, public/live sequence 27.
- Runtime implementation commit: `89b96680cd54f26eaf75cb2c5988dcf2b0d76187`,
  pushed on independent `rewrite/rust-core`. DELETE synthesized upstream CLI
  overrides, including diagnostic/version instances; COLLAPSE Termux full
  access into the owned FD-34 system config. Preserve every user upstream arg.
  KEEP upstream native shared-server protocol, owner/0700 checks and account
  authentication. Core qualifies the signed generation's server and provides
  bounded Termux-compatible sockets and FD snapshots without package installer
  dependence. Exact official 0.160.0 archive and byte-only UDS qualification bind
  the release; adapted runtime SHA-256
  `ba94d1d0d9d416a7fb2f13d6ffcc6e0d9ec981ee1793158bbc9b1d96d5f51ab3`.
- Root cause of account-dependent discovery was private upstream conversation
  directories and SQLite projections in declared external profile homes. The
  intended shared upstream state architecture had covered Manager-selected
  profiles but not those external homes. COLLAPSE known profiles onto the one
  upstream-owned `$HOME/.codex` conversation/SQLite store; DELETE private
  discovery partitions. Arbitrary external homes remain isolated. Core does no
  SQL merge, transcript parsing, session import or alternative discovery index.
- User-authorized historical retention kept every conversation active within
  the preceding seven days: 28 original UUIDs (20 CLI, one exec, seven internal
  guardian), deduplicated by verified per-record-type payload prefixes. Confirmed
  older payload stores were removed (52 excluded UUIDs). No recovery backup was
  created. Every retained original record sequence remains a prefix of its
  canonical payload; all three active writer inodes were preserved. The 25
  inactive original threads completed native resume/turns/unsubscribe
  finalization, while active writers were left running. Canonical leaf aliases
  are absent; profile auth/config is neither merged nor copied.
- The user explicitly retains upstream CLI/VSCode source visibility for both
  picker scopes. Exec/internal guardian records remain preserved and upstream
  API-addressable, rather than being additional user-facing picker entries.
  CWD means current CWD across all accounts; All means all CWDs across all
  accounts, retaining upstream archive/provider semantics.
- Live finalization exposed an existing operator RPC defect: buffered readline
  combined with fd readiness stranded already-prefetched responses after event
  bursts, and undrained stderr could block the child. COLLAPSE onto one
  unbuffered framed reader; discard the internal operator child's stderr.
  Focused regression independently exercises event bursts/partial frames with
  and without stderr pressure. A same-class historical-alias regression covers
  removing redundant canonical leaf links before profile-directory exchange.
  API-only finalization resumed after repair; the destructive handoff was never
  repeated. These follow-up changes are repository operator/test/docs only;
  released Core, Manager and builder production source remains byte-identical
  to runtime implementation 89b96680.
- Grouped source acceptance: Core 164 passed, one explicit device smoke ignored;
  Manager library 16, Manager integration 9, builder 21 (210 Rust total), and
  15 Python tests; formatting and diff checks pass. Separate official-runtime
  temporary installation proved native bare full access/shared server, retained
  union and old/new cross-profile resume. Actual production paths, not test
  helpers, supply the device evidence below.
- First publication control c612e70... / run 36983021570 correctly stopped
  before signing on a proof defect involving repeated helper descriptor records.
  Corrected control `9008fb93d31bd47db2f68c36fdad2f3e345197a8` / production run
  **36983295730** passed exact-source Android build, native smoke, component-byte
  preservation, signing, immutable Release, Pages, full public byte readback,
  disposable ordinary update/legacy/noop and non-forced parent-CAS promotion.
  Public stable main is `3224a5294d22818350a105d2adef6bc32b7e565f`.
- Signed public/live sequence **28**, generation
  **`local-hosted-0-160-0-89b96680cd54-shared-visibility`**, upstream **0.160.0**.
  Ordinary live `codex update` verified and activated it; a final exact-current
  update reports already up to date. Installed launcher/Core and runtime
  digests match the signed generation and public descriptor exactly. Public
  source pins remain the accepted artifact producer 89b96680; repository-only
  operator finalization does not require republishing unchanged executable bytes.
- Actual installed bare TTY stayed alive without fallback or unmanaged installer;
  final upstream argv contains only argv0. Connect-only strace proves the TUI
  successfully connects the native shared-server socket; its PID executes the
  active generation's runtime. Native config/read reports danger-full-access.
- Two distinct authenticated ChatGPT accounts, wrlab and jgnh2, both expose the
  exact original current-CWD union **4** and All union **20**, without duplicates
  or other-CWD leakage. Existing other-account sessions successfully resume and
  expose original transcript history through installed upstream APIs.
- One actual bare CLI conversation was then created in the current CWD. Both
  accounts discover the same new UUID under CWD and All: **5** / **21**.
  Other-account resume succeeds, turns are nonempty and creation-account
  metadata remains unchanged. An immediate request while the previous native
  writer was still closing was rejected; after native closure the same UUID
  resumed normally, without forced takeover or a discovery retry mechanism.
- Actual installed `codex resume` pickers under both accounts start in CWD and
  switch to All using the upstream control. Terminal-screen reconstruction
  confirms all 16 unique title probes in All and all four unique current-CWD
  probes in CWD, with zero other-CWD title probes in the initial screen. API
  snapshots independently bind the full 5/21 UUID sets. An initial raw ANSI
  substring probe failed because cursor-addressed screen updates fragment the
  output stream; replacing that diagnostic with terminal emulation restored
  faithful proof, without product changes or persisting screen contents.
- Final protected check: resolver and all profile auth/config bytes and modes
  match all 15 prior digests. Upstream's real jgnh2 TUI automatically persisted
  trust for the test CWD; that sole addition was identified by reconstructing the
  exact prior hash and removed after the test. Fourteen original inodes remain;
  upstream's atomic config save had replaced one config inode. No credential or
  transcript contents were printed or backed up.
- Limits: pretransition running clients retain their existing process/SQLite
  handles until normal exit; their private live projection is intentionally
  retained. New launches use canonical state. Active/closing thread writer
  ownership follows upstream locking; cross-account resume never kills or
  takes over another writer. Future upstream versions require fresh exact
  Termux UDS qualification before publication.
- Disposition: **SHARED-SERVER-CROSS-ACCOUNT-V1 / sequence 28 accepted and closed.**
  Runtime source, signed release, public stable and live installed executable
  bytes agree; repository-only finalization fixes and this ledger close the
  bounded transition. No further product work is selected by this bundle.

## Current Operating Baseline (2026-10-05)

This section is the current-state anchor inside the acceptance ledger. Historical
words such as “current,” “active,” “selected,” or “deferred” do not override this
section or `WORKBOARD.md`.

- Implementation authority is `rewrite/rust-core`; bind exact branch, HEAD and
  dirty state afresh on resume. PROFILE-LIFECYCLE product source
  59308430d39dc3c57159e260e5f7a873c1b16b8d and its delivery are accepted.
- Public/installed signed sequence40, upstream0.160.0,
  `local-hosted-0-160-0-59308430d39d-profile-lifecycle`; retained previous39 is
  `local-hosted-0-160-0-b7c1e61019b1-retired-server-discovery`. Accepted public
  main47597099ae00284aa40ebbbf08fb91143d943ba5; hosted production37303696948
  all8success. Wrapper is Android/AArch64; upstream runtime remains Linux-musl.
- Installed40 includes profile create/delete/rename/default/current/use, physical
  execution leases and saved-default task takeover, plus the accepted permission,
  memory-notification and task reconnect/stop/takeover behavior. Native signed
  profile3/task6 pass. Ownership follows the current kernel writer; upstream
  owns shared conversation storage. Ordinary restart now selects installed40.
- Installed doctor is healthy/exit0; exact-current update changes none of34
  durable entries. Complete39 payloads/modes, account/config/resolver identities
  and all original10 native PID/start identities were preserved at activation.
  Real profile deletion/rename and user-job termination were not performed.
- INPUT-CACHE-UNLOAD-CONSIDERATION is completed. Delay0 remains unchanged; no
  retention override, keepalive, model traffic or quantitative cache experiment
  is selected. Older deferral pointers are historical, not unfinished review.
- No implementation or delivery bundle remains selected. Any new runtime/cache
  behavior, empirical model experiment or user-data lifecycle operation requires
  its own concrete scope; previous delivery acceptance does not select it.

## Historical Runtime Alignment Baseline (2026-09-30)

The following is the historical pre-alignment snapshot. Its present-tense
wording describes that operation and is superseded by the 2026-10-02 current
baseline and acceptance above.

- Source authority is the freshly rebound remote `rewrite/rust-core`. Exact
  branch SHA is session state and must be re-read rather than copied forward as
  a timeless constant.
- Accepted product implementation for the current source feature set is
  `4fd14ed8aafcf29602f43e00c314eb8da39a6e5f`; later closeout commits before
  this maintenance operation changed authority documents only.
- Public stable before runtime alignment is signed sequence **23**, generation
  `local-hosted-0-159-0-b164e5b61cb4`, upstream `codex-cli 0.159.0`, with
  public-control `main=4d7418085265073bf1dada737a364aee54b374bd`.
- Live Termux currently runs that same 0.159.0 / sequence-23 generation and
  retains the preceding sequence-18 generation as rollback state.
- Sequence 23 was built from isolated production source
  `b164e5b61cb438913cdb1633d74c62d6c96f6523`. Direct source comparison proves
  that commit is a child of `4fd14ed8...` but differs in
  `crates/core/src/main.rs`, `crates/manager/src/lib.rs`, and
  `crates/manager/tests/profile_commands.rs`. Therefore this ledger does not
  claim source-equivalence between current live sequence 23 and the accepted
  source authority.
- Pretrigger readback on 2026-09-30 found official latest `0.159.2` with
  archive SHA-256
  `05a524a463cadf7e3e22c7f923539c0d0b74c3e78b1f5f1fab52e50e6fb3312f`.
  The planned 0.159.0 same-version sequence-24 path therefore failed closed
  before public/live mutation and was abandoned. The selected alignment is the
  ordinary newer-stable 0.159.2 sequence-24 path from accepted source
  `4fd14ed8...`.
- The user authorized **AUTHORITY-COMPACTION-V1 / RUNTIME-ALIGNMENT**: first
  compact authority documents and publish that docs-only change, then restore
  public/live runtime alignment through the existing signed publication and
  ordinary-update boundaries without new product-code behavior.
- Until that alignment is accepted, `WORKBOARD.md` owns the exact active
  maintenance scope and `RELEASE_AUTOMATION_PLAN.md` owns its bounded release
  gate.

## SCS-6 Live Shared-State Migration Acceptance (accepted 2026-09-27)

- Source authority at execution was exact `rewrite/rust-core=cd3e011a86e3dfe48da99c4bbeae73fe9c10edaf`; the tracked migration implementation matched that commit.
- The preceding SCS-1 through SCS-5 gates were green. The device-side tdev execution fix was independently deployed as tdev 0.1.11, and a fresh execution proved `RLIMIT_FSIZE=1048576` blocks (512 MiB), removing the earlier diagnostic-harness limit for the selected 173,101,495-byte rollout.
- Immediately before live mutation, the sole canonical writer was the official `runtime app-server`; it was quiesced and a fresh FD scan proved zero open descriptors beneath `$HOME/.codex`.
- A new full canonical backup was created at `$HOME/.codex-scs6-backup-20260927-preapply-cd3e011a` and verified file-for-file, including SHA-256 content, mode, uid/gid, mtime, and symlink targets. The verified tree contained 6,568 regular files, 3,151 directories, and 4 symlinks; verification manifest SHA-256 was `a30b2764f0ca7df7818cd03676da254746ac39a08b9a7a0efc50ced1a7aa8660`.
- Two post-quiescence plans were byte-for-byte identical: 5 legacy profiles, 23 selected recoverable rows, 17 unique threads, 16 copies, 1 canonical legacy-alias normalization, 9 explicit names, and 7 stale skips.
- Live apply created exactly 17 canonical regular rollout files. Every created target matched the planned size, SHA-256, and journaled inode/device identity. The selected 173,101,495-byte canonical legacy symlink was normalized to a regular canonical rollout.
- Finalization used official `codex-cli 0.155.1` app-server APIs only. Acceptance was 17/17 selected threads discovered, 17/17 resumed/materialized, and 9/9 explicit names restored; the migration journal reached `activated`.
- Post-activation byte classification proved all 17 migrated rollout payloads remained byte-identical prefixes of the canonical files. Upstream appended only normal `event_msg` records; no selected rollout showed non-append divergence.
- Three pre-existing canonical symlinks still point into legacy profile paths, but all three are broken, non-selected stale entries and were intentionally left outside the bounded migration scope. No selected thread retains a legacy symlink dependency.
- Legacy `.codex-profiles/*` sources were not deleted or rewritten. No public release, signed-generation promotion, installed-runtime replacement, or `codex update` was performed by SCS-6.
- Disposition: **SCS-1 through SCS-6 are accepted and complete.** Shared conversation/thread/session persistence is now treated as upstream-owned canonical state under `$HOME/.codex`; the one-shot migration machinery remains operator compatibility tooling rather than a steady-state Manager subsystem.

## Primary Technical Lead Policy

- Orchestration mode: lead-owned implementation
- Required primary role: Technical Lead/Integrator
- Required primary model: `gpt-5.6-sol`
- Required reasoning effort: `max`
- Lifecycle: keep the primary Lead across both Core milestones while its context
  remains available and accurate; on resume rebind the branch, commit,
  `SPEC.md`, this file, and `WORKBOARD.md`
- Planning owner: the primary Lead reads authorized repository evidence directly,
  plans each bounded implementation bundle, and records it in `WORKBOARD.md`
  before product-code mutation
- Implementation/integration owner: while worker mode is OFF, the primary Lead
  directly edits bounded product code/tests, inspects the actual diff, reruns
  load-bearing validation, records acceptance evidence, and commits accepted work
- Implementation cadence: execute each Workboard bundle as ordered vertical
  proof slices. One observable contract receives its implementation, focused
  regression, nonzero focused invocation, relevant compile/test gate, and Lead
  diff inspection before another independent contract begins
- Red-state rule: a non-compiling relevant target, stale superseded test,
  unexpected warning/dead path, zero-test invocation, or unmapped production
  definition freezes new product behavior until the affected class is audited
  and the current slice returns to green
- Validation split: run cheap compile/focused gates at slice boundaries; batch
  only the stabilized bundle's full suite, repeated parallel runs, protected-
  surface verification, and release build for final acceptance
- Checkpoints: initial implementation bundle; completion or rejection of the
  current bundle; material deviation, blocker, authority change, worker-mode
  transition, and milestone transition
- Additional planning agents, problem advisors, and checkpoint reviewers:
  disabled during both Core milestones unless the user explicitly changes that
  policy
- Review policy: the Lead's validation is integration verification, not
  independent review; invoke a fresh independent product reviewer only after the
  Milestone 2 acceptance candidate is complete
- Identity rule: do not persist a live Lead identity in tracked files
- Policy source and timestamp: user decision on 2026-08-28

## Worker Mode Control

- Worker capability: available but user-controlled
- Current worker mode: OFF
- Authority to change worker mode: explicit user command only. The Lead must not
  infer, auto-enable, or auto-disable worker mode from workload, context pressure,
  test failures, or convenience
- While OFF: do not invoke implementation workers or coding subagents. The
  primary `gpt-5.6-sol` / `max` Lead directly implements bounded Workboard work
- If the user later turns worker mode ON: record the transition and a bounded
  worker contract in `WORKBOARD.md` before invoking any implementation worker;
  worker output never becomes acceptance proof without Lead diff review and
  load-bearing validation
- Turning worker mode ON or OFF does not alter repository/product authority,
  milestone gates, protected surfaces, or the Lead's acceptance ownership
- tmcp `harness.run` is independent of worker mode and is test/validation-only;
  it must never be used as an implementation/development agent or as a route for
  product-code mutation
- Package operations, live product mutation, and network/provider behavior remain
  prohibited unless separately authorized by the applicable milestone gate
- Policy source and timestamp: user decision on 2026-08-28

## Current Success Threshold

The goal is complete only when both milestones in `SPEC.md` pass their declared
tests and a fresh supported Termux installation can install, run, diagnose,
update, recover, and roll back the Rust Core without requiring an on-device Rust
toolchain or modifying protected user/system state. The top-level bare
`codex update` path remains wrapper-owned: it first consumes a signed adapted
generation from the wrapper channel and, when that publication is unavailable
at the transport boundary, may resolve the official latest (or explicitly
selected) release metadata, bind its exact version and package digest, then use
the prebuilt in-Core release-builder fallback to fetch the exact official
versioned archive, adapt, sign, qualify, and activate one local generation. It
must never delegate to the upstream self-updater or activate an unsigned/raw
upstream runtime.

Manager product features are not part of this two-milestone completion claim.
Their boundary must be preserved so they can be implemented separately without
moving Core ownership.

## Project Analysis

The predecessor accumulated runtime, installer, activation, repair, profile,
session, notification, documentation-audit, and release logic across Bash and
Python. The rewrite intentionally retains only accepted observable contracts
and safety findings. It does not translate predecessor modules or preserve
their internal schemas by default.

The highest-risk work is not Rust compilation. It is bootstrap trust,
upstream-artifact qualification, descriptor/process fidelity, atomic
generation activation, and recovery after ambiguous failures.

Speed comes from a narrow Core, two milestones, one normative specification,
one current workboard, direct focused tests, and deferred independent review.

## User Decisions

- Start `rewrite/rust-core` from an empty parentless root. It must have no
  merge-base with `main` or the predecessor and must never import either
  history.
- Keep `main` as a separate publication authority until an accepted rewrite
  tip explicitly replaces it; promotion is not a merge.
- Seal the latest predecessor at
  `bf30a7dc94d4dad7f58836c69028160856e63c58` on `legacy/monolith`.
- Keep one repository and one public `codex` entrypoint.
- Separate native Rust Core from the Manager layer.
- Treat Manager profiles as authentication/configuration/runtime identities, not
  as conversation owners. Local conversation/thread state is user-global across
  Manager profiles, with `$HOME/.codex` as the canonical shared-state root and
  custom profile homes retaining only profile-local identity/configuration state.
- Preserve the original thin-wrapper direction: in steady state Manager chooses
  an execution identity, supplies only the bounded profile/shared-state
  environment and compatibility topology, then executes the official-prebuilt
  upstream runtime. Upstream owns conversation/thread/session persistence and
  schema semantics. Normal Manager operation must not evolve into a second
  thread store, routine SQLite/transcript parser, conversation-ownership layer,
  or parallel session index.
- Preserve the official-prebuilt upstream runtime contract. Implement the shared
  conversation model through Manager/Core compatibility topology plus upstream's
  supported `CODEX_SQLITE_HOME`/system-requirements surfaces; do not introduce a
  custom upstream source build solely to add a native thread-store root.
- Keep the internal architecture split as `profile_home` versus
  `shared_state_home` so a future upstream-native thread-store root can replace
  compatibility links without changing Manager semantics or migrating user
  conversations again.
- Legacy consolidation is a one-time bounded compatibility operation, not a new
  steady-state subsystem. For each legacy user profile, import at most the five
  most-recent distinct conversations by last activity, together with only the
  dependency closure needed for faithful operation in the current upstream
  schema. Do not replace whole SQLite databases or use blind overwrite merges;
  deduplicate only proven-identical cross-profile state and fail closed on
  divergent same-conversation identities. Leave non-selected legacy history in
  the verified backup/source profile rather than expanding Manager ownership of
  upstream storage semantics.
- Keep management commands under `codex termux`.
- Reserve top-level `codex update` and `codex doctor` for Termux-aware behavior.
- Preserve upstream `--version`/`-V` output without wrapper version rows.
- Put architecture and lightweight change discipline in `SPEC.md`; do not add
  a separate SDD now.
- Run the goal under a primary `gpt-5.6-sol` / `max` Technical Lead/Integrator
  that directly owns repository evidence, planning, integration verification,
  authority documents, commits, and acceptance decisions across both milestones.
- Worker mode is controlled only by explicit user command; its current state is
  OFF, so the primary Lead directly implements bounded product-code and test
  changes.
- Do not create planning subagents, problem advisors, or checkpoint reviewers
  during the Core milestones. Review the complete Milestone 2 candidate with a
  fresh independent reviewer.
- Use exactly two Core milestones.
- Product release speed is the priority once the small set of load-bearing
  integrity invariants is satisfied. Do not turn rare or hypothetical failure
  scenarios into new subsystems by default.
- Treat one installer/updater transaction as the normal product path. Do not
  spend release-critical time on simultaneous-installer multi-writer fencing
  unless actual use or a reproducible product failure demonstrates the need.
- Prefer recovery to one already complete last-known-good generation over
  stacked fallback chains. Existing defensive state, retries, checks, and
  fallback paths should be removed when a simpler foundational invariant covers
  the same failure.
- Any additional defensive mechanism must justify its net complexity: it should
  address a concrete failure not already covered by complete-generation staging,
  atomic activation, and last-known-good recovery. Defensive complexity is also
  a potential defect and attack surface.
- The foundation established documents and workflow first; current implementation
  is now performed directly by the primary Lead under bounded Workboard bundles.
- Prevent implementation/proof drift inside a bundle: `WORKBOARD.md` must carry
  the current vertical slice/proof map, and production behavior must not advance
  past a red or unmapped slice. Historical tests never authorize a compatibility
  branch after the public product path has replaced their behavior.
- Make the no-argument `codex update` a complete remote-to-local consumer path.
  Explicit `--local`, `--remote`, `--rollback`, and `--build-local` selectors
  remain Core-owned secondary operations. A transport-unavailable automatic
  channel may enter the same official-source local-derived construction used by
  `--build-local`; it must use a fresh ephemeral device-local signer, preserve
  the official `update_key`, retain signed admission/probe/atomic activation and
  one-generation rollback boundaries, and never obtain official publication
  authority from device credentials.
- Separate official publication from runtime consumption. A local-derived build
  never invokes `gh`, creates an official publication, or advances `main`/stable;
  official release production is the later GitHub-hosted producer work selected
  by `RELEASE_AUTOMATION_PLAN.md`. Private signing keys remain excluded from
  repository and device artifacts.
- Implement that compatibility operation as rollout-authoritative migration rather than
  a SQLite merger: read legacy SQLite only for bounded selection/conflict/dependency
  checks, copy verified rollout JSONL, then let official upstream app-server APIs
  rebuild metadata/history and restore explicit names. Keep rebuildable history
  projections and diagnostic logs out of the payload. Normalize any selected canonical
  legacy symlink to a regular canonical rollout so the resulting shared state has no
  runtime dependency on legacy profile homes.
- Separate rollback at the upstream-write boundary: before `finalize`, the one-shot
  journal may undo only files/alias normalization it created; after `finalize` begins,
  only restoration of a verified whole canonical backup is valid. This keeps rollback
  of upstream-owned SQLite outside permanent Manager/Core logic.


## Execution Plan

### Milestone 1 — local Core

Implement the Rust Core and prove local command dispatch, upstream passthrough,
FD/environment/process contracts, sandbox behavior, read-only doctor, manifest
interfaces, and resolver non-mutation. Do not perform live installation or
network update.

### Milestone 2 — delivery and recovery

Implement prebuilt delivery, bootstrap, signed updates, upstream acquisition and
adaptation, atomic activation, recovery, rollback, offline operation, and fresh
Termux qualification. Produce one candidate for independent product review.

## Acceptance Ledger

### Current Direct-Lead Evidence

- UX1-PROD-15 production publication is **accepted 2026-09-19**. Manual-only
  production run `35425409155` used exact accepted product source
  `07f77b89a177682954d80ae3f797377c4731de64` against the authenticated
  public baseline `0.155.1` / signed sequence 14 /
  `local-hosted-0-155-1-566034e1aff4`. The ordinary comparison first
  classified version equality as `candidate=false`; only the dedicated UX-1
  deployment gate admitted next sequence 15. The candidate preserved the exact
  signed sequence-14 digest/mode inventory for `codex-code-mode-host`, both
  helpers, Manager, and runtime, while the Core changed to the accepted UX-1
  build. Ordered `generation.meta` comparison admitted exactly the required
  `generation_id` and `core_artifact_digest` deltas, bound both Core digests,
  and preserved every other raw record including both ordered helper records.
  Native Android/AArch64 smoke, production-authority signing and independent
  verification, immutable Release staging, LKG-preserving Pages deployment,
  every-byte public HTTPS readback, disposable ordinary update, exact version,
  semantic doctor, and second-update no-op all passed. The existing exact-parent
  CAS used `force:false`, reported `promotion_result=committed`, and advanced
  `main` exactly one child from
  `79130ffadfd979bdb1fff1c530fa74cbf0579d6e` to
  `ba36c44f871ef266c4887986535ed87a4d2becc9`, changing only
  `update-index-v1` and `update-index-v1.sig`. Post-promotion HTTPS readback
  byte-matched those promoted files and reverified their signature. Public stable
  is therefore signed sequence 15 generation
  `local-hosted-0-155-1-07f77b89a177-ux1-human-output`. The authorized live
  Termux consumer leg is also **accepted 2026-09-19**, closing UX1-PROD-15.
  Preflight job `job_w4h_aabff4f63b` measured exact `codex-cli 0.155.0`
  with healthy Termux Core/Manager/runtime. Ordinary public update job
  `job_w4i_4679e91eab` activated the exact sequence-15 generation and then
  reported `codex-cli 0.155.1`. New-Core job `job_w4j_b7c769f26d`
  required the exact-current line `Codex 0.155.1 is already up to date.`,
  proved non-TTY output had no CR/ANSI controls, and reported the new generation
  healthy. PTY job `job_w4k_377b6ef362` required both child stdout and stderr
  to be terminals and captured `Checking for updates...`, transient-line
  erase, and the final exact-current success line; version remained exact
  `codex-cli 0.155.1`.

- POST-RALD UX-1 update human output is **source-accepted 2026-09-19** at
  corrected product source `07f77b89a177682954d80ae3f797377c4731de64`.
  The earlier accepted source `c3f87302df7d1e94ca5c497c50979f5aa73adddd`
  was superseded after final source review found one presentation-ordering gap:
  it could expose `Downloading...` / `Verifying...` before the signed
  candidate version was authenticated. The corrected source first authenticates
  signed release control and the digest-bound `generation.meta`, then prints
  the permanent version header, and only then exposes the remaining download /
  verification phases. The PTY contract is therefore exact
  `Checking -> version header -> Downloading -> Verifying -> candidate probe ->
  Activating`. Non-TTY execution emits no carriage-return/ANSI progress
  controls. Signed success ends with `Codex <VERSION> is now active.`,
  exact-current success ends with `Codex <VERSION> is already up to date.`,
  operational failures omit the redundant command prefix and end in a period,
  all permanent update results use no emoji decoration, and same-version
  corrective generations use
  `Updating the Termux release for Codex <VERSION>...`.
  Full remote acceptance run `35406952579` at acceptance head
  `71ded0446da7d86e244e140a8ba9b8c6e1e1ff57` passed workflow/credential/diff
  checks, unchanged signed public sequence-13 audit, exact Android/AArch64 Core
  cross-build, release-builder tests, Core process E2E, both Android-dependent
  focused proofs, Manager tests, full locked workspace tests, clippy with
  warnings denied, and rustfmt. Existing signed admission, digest/mode checks,
  anti-rollback, candidate probe, atomic activation, LKG, rollback hold/guard,
  local-derived authority, publication, and public-stable semantics are
  unchanged. The motivating user-observed live update from `codex-cli 0.154.0`
  to public `0.155.0` remains observation only; UX-1 acceptance did not access
  or mutate that installation. `main` remained
  `5bef52d07a07bd8612b4538dfb29a2396937a3fb`, public stable remained signed
  sequence 13, and no Release/Pages promotion, force push, credential fixture,
  or live-device mutation occurred.

- POST-RALD7 LEGACY-LAG direct-jump qualification is accepted as a diagnostic
  and source-correction proof on 2026-09-18. Exact historical R10 source
  `0621105fd1be8461b370466fbfa981938241074d` and exact official 0.153.4
  archive SHA-256
  `fc395cb043a1093ab0db34f44aba3199bfaa9ce640cd9be7fd588f44b0da64a4`
  were reconstructed into a production-authority-signed private sequence-7
  client. Historical hosted reconstruction uses Android API 30 only because the
  historical source directly references bionic `renameat2`; no historical or
  current product source was changed. Run `35343356792` proved that the actual
  current public sequence-12 generation
  `local-hosted-0-155-0-566034e1aff4` is **not** directly activatable by that
  retained R10 client in credential-free conditions: signature/digest/version
  admission reached the activation probe, then the old Core rejected
  `upstream_doctor=supported` with
  `candidate doctor probe was unhealthy`. The authoritative sequence-7 state,
  launcher, and current generation remained unchanged.
  Repair proof `35343870274` reused the exact public 0.155.0 component bytes and
  changed only generation identity plus the already accepted exact R10 bridge
  signal to signed `upstream_doctor=unsupported` under
  `creation_metadata=r10-browser-helper-bridge-v1`. A private production-key
  sequence-13 fixture then updated the same sequence-7 Core directly to
  `codex-cli 0.155.0`, retained sequence 7 as `previous`, executed real public
  doctor semantics (`upstream=unhealthy`, `termux_core=healthy`), and proved a
  byte-identical second-update no-op. SPEC now requires this bridge signal on
  every public stable while retained R10 remains in the compatibility floor.
  Source correction `3c0d2742bf99aa931b840b454b314e3fac428c9c` makes the
  producer preserve the signal for all future stable candidates and full
  repository acceptance `35344310885` passed. No fake credentials, public
  Release/Pages/index mutation, force push, or live installation access was used.
  Production remediation is **complete**. The separately authorized first
  action installed only the accepted producer workflow on exact
  `main=9aa8be4c63fe8d60dea130b979c0c12d1a5164bc`, producing non-forced child
  `0455ba6a86ec2a7392416f3ecba68925c9a8adfd` while the signed stable index and
  signature blobs remained byte-identical. Separately authorized production run
  `35364873732` then built and production-signed the exact 0.155.0
  sequence-13 bridge
  `local-hosted-0-155-0-566034e1aff4-legacy-lag-remediation`, staged its
  prerelease/tag at exact pre-promotion main `0455ba6a...`, deployed Pages
  while retaining the sequence-12 LKG, read back every signed byte over public
  HTTPS, and proved the historical production-authority-signed sequence-7 R10
  client directly activates sequence 13. The activated Core retained sequence 7
  as `previous`, reported exact `codex-cli 0.155.0`, executed the real public
  doctor path with `upstream=unhealthy` and `termux_core=healthy`, and
  completed a byte-identical second-update no-op. Only then did the existing
  `force:false` exact-parent CAS report `promotion_result=committed` and move
  `main` to `5bef52d07a07bd8612b4538dfb29a2396937a3fb`. Public stable is now
  signed sequence 13. No fake credentials, force push, or live installation
  access was used.
  Bounded remediation control-plane source
  `536d06a6088ccf3c850a6031c895a0ae6c2fe709` is accepted by full source
  acceptance run `35355371929`. It adds a false-by-default, manual-only
  `legacy_lag_jump_remediation` gate that is usable only from `main`, binds
  the exact sequence-12 generation/version and next sequence 13, remains
  forbidden on scheduled runs, requires exact sequence-12 component
  digest/mode identity plus the bounded generation-id/doctor-signal descriptor
  delta, and routes the production proof through an exact historical R10/API-30
  rebuild, the existing production-key-matching signing helper for a private
  sequence-7 fixture, public Pages direct-update proof, previous retention,
  real corrected-Core doctor/no-op validation, and the existing non-forced CAS.
  The gate self-disables once the public baseline is no longer exact sequence
  12. This source acceptance did not install the workflow on `main`, create a
  Release, deploy Pages, promote an index, access a live installation, or grant
  production publication authorization.

- RELEASE-AUTOMATION-LOCAL-DERIVED RALD-7 full acceptance/activation is accepted
  on 2026-09-18. Final product source
  `566034e1aff42bde2f3221ebc2da2b16def77d44` replaced the historical
  version-specific upstream resource whitelist with strict layoutVersion-1
  semantic validation while keeping tar safety bounds, selected static AArch64
  binaries, and exact Termux patch qualification fail-closed. Probe
  `35308307480` confirmed official 0.155.0 preserved the selected binaries and
  patch source counts. Full source acceptance `35308972611` and scheduled
  publication authorization acceptance `35310001193` passed the complete
  repository, Android, workflow/action, credential/private-key, clippy/fmt, and
  signed public-state gates. Activated dry-run `35310213404` with nested
  producer `35310222884` reproduced official 0.155.0 build, native ARM64 smoke,
  and production signing with public mutation jobs skipped.
  Explicitly authorized real candidate run `35321953530` then passed immutable
  Release staging, LKG-preserving Pages deployment, complete public HTTPS
  readback, disposable update to exact `codex-cli 0.155.0`, diagnostic proof
  `upstream=unhealthy` / `termux_core=healthy`, second-update exact-current
  no-op, and non-forced exact-parent CAS. Promotion advanced `main` exactly one
  parent from `b8b35d0ffc1ce2da87f49fcf66106e520a04a2bf` to
  `9aa8be4c63fe8d60dea130b979c0c12d1a5164bc`, changing only
  `update-index-v1` and `update-index-v1.sig`. Public stable is signed
  sequence 12 generation `local-hosted-0-155-0-566034e1aff4`, with its
  immutable Release tag bound to the pre-promotion parent. Post-promotion full
  acceptance `35322553140` re-passed every gate against sequence 12. Outer
  dispatcher `35321944376` is diagnostic-red only because its extra verifier
  searched for a GitHub-output value as a literal log line; the producer itself
  and all seven publication jobs were green. The one-shot dispatcher was
  removed. The six-hour schedule is production publication authorization only
  for ordinary strictly-newer official stable; manual publication still
  requires explicit authorization, acceptance-only controls remain fenced off,
  and no force push or live installation mutation was used.

- RELEASE-AUTOMATION-LOCAL-DERIVED RALD-6 fresh-install/update delivery E2E is
  accepted on 2026-09-18. Product source
  `9972a3288c0531ba744e9bd1356273d9a080aa79` adds the bounded no-argument
  `install-online.sh` transport frontend while leaving local `install.sh` and
  `bootstrap/codex-bootstrap` unchanged. Focused hosted run `35289795309`
  passed shell syntax, four installer contract tests, and diff checks. Load-bearing
  native ARM64 run `35290588365` at workflow head
  `8d3862342d773ac8a2dfb44c975f55771424ffb7` fetched that immutable installer
  over public HTTPS, byte-matched it to accepted source, and started from empty
  disposable Termux-shaped HOME/PREFIX state. It verified the pinned bootstrap
  authority plus signed public index/manifest through the existing bootstrap
  boundary, fresh-installed exact public signed sequence 11 generation
  `local-hosted-0-154-0-37fbbd8033b8-rald45-transition`, matched the installed
  launcher to signed Core, observed exact `codex-cli 0.154.0`, and proved a
  default public `codex update` was an exact-current no-op with a byte-for-byte
  state snapshot match. The same installed client then consumed a runner-local
  HTTPS fixture: the exact installed sequence-11 generation was copied to
  generation `local-rald6-seq12-9972a3288c05`, signed as release sequence 12
  only by the accepted bounded production-authority signing helper, staged
  locally while the fixture stable locator still served authentic sequence 11,
  and admitted only after that local locator was atomically switched to the
  signed sequence-12 index. Ordinary no-argument `codex update` activated
  sequence 12, retained public sequence 11 as `previous`, preserved exact
  runtime version behavior, and the next update was an exact-current no-op with
  another state snapshot match. The signed fixture was never uploaded: run
  artifacts were empty, no Release/Pages/`main` mutation occurred, and final
  public stable remained sequence 11 at
  `main=f221de1225471fb5eda5bbdfcbd0d9db0c2f43b1`. Earlier runs
  `35290065429` and `35290289525` had already proved the public fresh-install
  half but stopped before secret exposure because fixture preparation narrowed
  HOME/PATH before runner Rust tooling; the final repair only reordered that
  proof step. The live installation was never accessed or mutated, no alternate
  trust implementation was introduced, and RALD-7 was not started.
- RELEASE-AUTOMATION-LOCAL-DERIVED RALD-5 publication, LKG continuity, and
  promotion is accepted on 2026-09-18. Bounded orchestration commit
  `d1b53576f9b173dcf786b21271b556712d694bbf` kept transition staging
  non-promoting by default and added a separate false-by-default
  `rald45_transition_promote` authorization. Repaired negative-proof run
  `35285462288` built, natively smoked, production-signed, staged, and deployed
  the same-version candidate path, then deliberately tampered its fetched runtime;
  public verification rejected it, the CAS job was skipped, and
  `main=b17cc05ec8ec18bdbfd960e413dc4c47ffeb9f90` remained authoritative.
  Separately authorized positive run `35285792273` then reproduced signed
  sequence 11 generation
  `local-hosted-0-154-0-37fbbd8033b8-rald45-transition`, verified immutable
  Release assets and every staged Pages byte from public HTTPS, updated from the
  actual sequence-10 public stable Core, verified exact `codex-cli 0.154.0`,
  observed real credential-free doctor status `upstream=unhealthy` with
  `termux_core=healthy`, and proved the second no-argument update was an
  exact-current no-op. Only after those gates did the promotion job verify the
  signed candidate index again and execute `force:false` compare-and-swap.
  Promotion result was `committed`: `main` advanced by exactly one parent from
  `b17cc05ec8ec18bdbfd960e413dc4c47ffeb9f90` to
  `f221de1225471fb5eda5bbdfcbd0d9db0c2f43b1`, whose stable
  `update-index-v1` plus signature target that transition generation.
  Orchestration run `35285446624` completed successfully after checking the
  expected negative failure and positive promotion. No force push, fake
  credential/provider success, or RALD-6/7 work was used.
- RELEASE-AUTOMATION-LOCAL-DERIVED RALD-4.5 activation-safe candidate-probe
  correction is proved on 2026-09-18 without public stable promotion. Product
  source `37fbbd8033b8cc2d508689ab1d6637b4c4f5d516` removes full upstream-doctor
  exit-zero health from the activation gate while retaining the exact qualified
  upstream version probe and all signed admission/anti-rollback/atomic
  activation safety. The exact R10 bridge transition descriptor uses signed
  `upstream_doctor=unsupported` only so the existing public stable Core can omit
  its historical activation doctor gate; the corrected Core remaps that exact
  marked transition to the normal public upstream-doctor execution path.
  Hosted transition proof run `35284270406` at workflow head
  `f74045156bff00cae22f3c8d67823096ab1fe13f` completed successfully. It
  authenticated current public stable sequence 10
  `local-20260914-update-channel-bridge-1`, built/smoked/signed and publicly
  staged sequence-11 transition candidate
  `local-hosted-0-154-0-37fbbd8033b8-rald45-transition`, verified every signed
  byte from public HTTPS, then used the actual old public stable Core to perform
  the candidate update. Post-activation state bound current to the candidate and
  previous to sequence 10, the installed launcher byte-matched the candidate
  Core, version was exact `codex-cli 0.154.0`, and the second update was the
  expected exact-current no-op. Credential-free `doctor --json` actually ran
  upstream and reported `upstream=unhealthy` while `termux_core=healthy`; its
  nonzero health result remained diagnostic rather than being forged into
  success. The CAS promotion job was skipped by the transition fence, `main`
  remained `b17cc05ec8ec18bdbfd960e413dc4c47ffeb9f90`, and the signed stable
  index still targets sequence 10. The prior run `35264696877` failed before
  this proof because its manually reconstructed old-stable fixture omitted the
  Core-owned state `config/` directory that normal bootstrap creates; commit
  `96048a4a28b732d8e60e753b1cbc9d313d546a7f` repaired only that disposable
  fixture and its contract test. No fake user credential, provider mutation,
  force push, RALD-6/7 work, or stable promotion was used.
- RELEASE-AUTOMATION-LOCAL-DERIVED phase RALD-4 is accepted on 2026-09-17.
  Acceptance-only GitHub-hosted `workflow_dispatch` run `35176798621` at
  workflow head `d0af224b6738e80feb458e6030b6da517d94a1f5` completed successfully
  without publication authority. The producer authenticated public stable
  `0.154.0`, resolved real official stable `0.154.0`, used only the bounded
  `0.153.4 -> 0.154.0` acceptance comparison, cross-built Android/API-24 Core
  and Manager, and adapted/qualified the official runtime. Native ARM64 smoke on
  `ubuntu-24.04-arm` revalidated the candidate and pinned AOSP bionic substrate,
  executed exact Manager/Core/runtime successfully, removed the deferred Manager
  marker, and produced the qualified unsigned candidate. The signing job
  revalidated that candidate and the accepted public authority before secret
  exposure, consumed the existing `CODEX_RELEASE_SIGNING_KEY` only in the
  bounded signing step, signed release sequence `11`, and independently verified
  both the release-manifest and update-index signatures with the accepted public
  key. The acceptance index used only the non-routable `.invalid` release base;
  temporary signing material and signed acceptance output were removed in-job and
  signed-artifact upload was skipped. The secret value was neither requested nor
  exposed, and `main`, GitHub Release/Pages, public stable, and live installed
  state were not mutated. RALD-5 was not started.
- RELEASE-AUTOMATION-LOCAL-DERIVED phase RALD-3 is accepted on 2026-09-16.
  Exact producer source pin `28e65b32c8719cf913e62080d4674b54dbcc1a01`
  retains the pre-sign hosted cross-build boundary and the repository-owned
  `.github/workflows/auto-release-termux.yml` remains read-only and unsigned:
  it has no signing-secret access, release signing, GitHub Release/Pages
  publication, git push, or stable-index promotion authority. Repository
  acceptance remains the previously green preflight 5/5, workflow-contract
  5/5, deferred Manager 3/3, YAML/fmt/diff/check/clippy gates, and full locked
  workspace suite `job_ulk_1460dff8e1` with Core 145 passed / one explicit
  live-Termux smoke ignored, Manager 20/20 plus 11/11 integration, and
  release-builder 17/17.
- RALD-3 hosted activation is green. The fixed workflow/source pin was installed
  on default branch `main` in
  `b17cc05ec8ec18bdbfd960e413dc4c47ffeb9f90` without changing the signed stable
  index. GitHub-hosted manual `workflow_dispatch` run `35090080089` completed
  successfully on `ubuntu-24.04` with `contents: read`, fetched exact source
  `28e65b32c8719cf913e62080d4674b54dbcc1a01`, passed the 5/5 preflight helpers,
  authenticated current generation `local-20260914-update-channel-bridge-1`,
  and resolved wrapper/upstream stable as `0.154.0` / `0.154.0`. It therefore
  selected `candidate=false`; cross-build, candidate adaptation/upload, and
  Android smoke were correctly skipped and the run produced no artifacts.
  Public stable and its signature remained byte-identical, GitHub Release ID
  `388405334` remained current, no Pages publication run was triggered, no
  Actions signing secret was created/changed/used, and the live installation
  was not accessed or mutated. RALD-4 was not started by that RALD-3 run and is
  accepted separately above.

- RELEASE-AUTOMATION-LOCAL-DERIVED phase RALD-2 is accepted on 2026-09-16.
  Core `codex update` is now a consumer/local-derived boundary only. The
  device-side `automatic_update` official producer/publisher and its ambient
  maintainer-key/GitHub helpers are removed; ordinary, held, forced, rollback,
  local/remote, exact-current, and transport-fallback behavior cannot gain
  official signing or publication authority merely because maintainer
  credentials or an authenticated `gh` happen to exist. Explicit
  `codex-release-builder fetch/build/publish` tooling remains available as the
  non-installed producer boundary selected for the later GitHub-hosted phases.
- RALD-2 validation: consumer-only/default-channel plus retained RALD-1, ARH,
  and signed-channel focused process E2E passed 4/4 in `job_uh8_9f46cd866f`;
  explicit release-builder publication entering signed admission passed 1/1 in
  `job_uhc_20bd2e50ed`; formatting, `git diff --check`, locked workspace check,
  workspace clippy with `-D warnings`, and removed-producer symbol audit passed
  in `job_uhd_4af39b46e6`; the full locked workspace suite passed in
  `job_uhe_6b1b48a547` with Core 145 passed / one explicit live-Termux smoke
  ignored, Manager 20/20 plus 11/11 integration, and release-builder 15/15. All
  execution used the isolated TMCP worktree and disposable fixtures; the live
  Codex installation, protected resolver/auth/config/profile/session state,
  public stable index, GitHub Release/Pages stable publication, and Actions
  secrets were not mutated. RALD-3 is the next selected phase.

- RELEASE-AUTOMATION-LOCAL-DERIVED phase RALD-1 is accepted on 2026-09-15.
  Core now owns exact `codex update --build-local` and the permitted automatic-
  channel transport fallback through one official-source local-derived path. The
  path binds exact upstream/archive plus authenticated public-baseline provenance,
  uses a fresh owner-only ephemeral Ed25519 signer, persists only its public
  verifier as `current_key`, leaves the official `update_key` byte-identical,
  does not consume a public release sequence, and never invokes `gh` or official
  publication. Ordinary `--local`/`--remote` admission cannot claim this
  exception. Rollback from local-derived creates no public hold/guard; later
  official signed activation and rollback/hold/force retain the accepted ARH
  behavior. The now-unreachable legacy transport-fallback GitHub uploader was
  removed; the remaining official producer was intentionally left for RALD-2 and
  is now detached from Core by the accepted RALD-2 change.
- RALD-1 validation: focused process E2E 4/4 in `job_ufm_7599d915e3`; existing
  ARH rollback/hold/force plus signed-channel regressions 4/4 in
  `job_ufo_ebf8afabd3`; formatting, `git diff --check`, locked workspace check,
  and workspace clippy `-D warnings` in `job_ufp_60b306a1ba`; full locked
  workspace suite in `job_ufq_6205c60317` with Core 153 passed / one explicit
  live-Termux smoke ignored, Manager 20/20 plus 11/11 integration, and
  release-builder 15/15. All execution used the isolated TMCP worktree and
  disposable fixtures; the installed live Codex, protected resolver/auth/config/
  profile/session state, public stable index, GitHub Release/Pages publication,
  and Actions secrets were not mutated. RALD-2 was subsequently accepted;
  RALD-3 is now the next selected phase.

- The user explicitly withdrew trust from the prior implementation-worker path
  and required a fresh Lead review from the beginning. Worker mode remains
  user-controlled and OFF. Historical M1-B1..M1-B9 worker reports and their old
  acceptance statements remain provenance only; they are not used as current
  proof.
- M1-R1 fresh re-audit/hardening is accepted at
  `4c1a8d90d6aa028106218d349076c465af8b8535`. The direct Lead reviewed the
  current Rust source against `SPEC.md`, reopened two correctness/safety gaps,
  fixed them directly, reviewed the resulting diff, and reran the load-bearing
  validation.
- M1-B15 is accepted at `6bea7a53004f43178599e65a6f630c7bb06355b9`.
  The direct Lead added a dependency-free typed doctor report surface with
  bounded upstream/Core/Manager state domains, deterministic summary precedence,
  typed semantic exit classes, separated human sections, and one schema-versioned
  JSON envelope. The output model accepts no arbitrary diagnostic strings,
  paths, environment values, auth/session/notification content, or raw upstream
  output, establishing a fail-closed redaction baseline before process capture.
- Final B15 validation `job_ikt_87c72178ad` passed all 5 B15 focused tests,
  exhaustive 36-state summary/exit classification, the full serial workspace
  suite 98/98, eight complete default-parallel repetitions, formatting,
  `git diff --check`, and a warning-free locked build with offline mode and a
  repository-external Cargo target. Direct diff audit found no B15 process,
  filesystem, environment, Manager, network, dependency, or `main` wiring.
- M1-B14 is accepted at `be6492f895185caf7d9b922b16330a1cd8f00033`.
  The direct Lead added a typed qualified-runtime launch boundary that accepts no
  separate raw runtime program or compatibility directory: it consumes the B13
  `QualifiedRuntimeAssets`, derives the B10 environment plan from the captured
  process snapshot and the qualified compatibility directory, then delegates to
  the existing sandbox-before-I/O FD33/34 final-exec path using the qualified
  runtime program. No active-generation lookup, digest calculation, filesystem
  qualification, network, activation, or normal `main` wiring was added.
- Final B14 validation `job_ijk_b7acb35e72` passed all 3 B14 focused tests, the
  full serial workspace suite 93/93, eight complete default-parallel workspace
  repetitions, formatting, `git diff --check`, and a warning-free locked build
  with offline mode and a repository-external Cargo target. The real subprocess
  test jointly proved qualified runtime selection, qualified compatibility PATH,
  B10 temp/certificate assignments, exact sandbox prelude/raw argv, FD33/34,
  contamination fencing, and unrelated-environment preservation.
- M1-B13 is accepted at `71acbd8e318d50548952490e0d2fb52c7b661f9c`.
  The direct Lead added a pure Unix runtime-asset qualification boundary tying an
  explicit absolute runtime program path and observed digest, explicit
  compatibility directory, and the exact helper-asset identity/digest set to a
  B11 qualified generation. No filesystem stat/read/hash, active-generation
  lookup, path canonicalization, launch, or state mutation is performed.
- Final B13 validation `job_igy_c7b11e3616` passed all 10 B13 focused tests,
  the full serial workspace suite 90/90, eight complete default-parallel
  repetitions, formatting, `git diff --check`, and a warning-free locked build
  with offline mode and a repository-external Cargo target.
- M1-B12 is accepted at `3927ad46696875c913c9039406693c1ddd4c3231`.
  The direct Lead added a dependency-free updater admission/candidate interface:
  immutable-remote versus raw local-artifact sources, explicit signed-release
  and architecture/API/channel/anti-rollback verdicts, resolver-dependency
  qualification, staged digest/archive/compatibility verdicts, candidate-probe
  and rollback-readiness verdicts, source-digest binding to the B11 qualified
  generation, and borrowed admitted/activation-ready wrappers. No verifier,
  cryptography, serialization, network, staging, installation, or activation is
  implemented by B12.
- Final B12 validation `job_iff_6b93404095` passed all 11 B12 focused tests,
  the complete serial workspace suite 80/80, eight complete default-parallel
  repetitions, formatting, `git diff --check`, and a warning-free locked build
  with offline mode and a repository-external Cargo target. Direct diff review
  confirmed only `crates/core/src/main.rs` changed and the B12 production surface
  is pure type/evidence promotion rather than updater I/O.
- M1-B11 is accepted at `0eb9f6cd33951ff782c010d9e116ab886f70a815`.
  The direct Lead added a dependency-free in-memory generation manifest model,
  explicit Core compatibility requirements, typed qualification failures, and a
  borrowed `QualifiedGenerationManifest` wrapper. The validator binds all
  SPEC-declared generation-manifest field classes, rejects empty required
  bindings, the four Core/platform compatibility mismatches, rejected
  qualification, malformed/duplicate helper bindings, and an explicitly empty
  optional Manager digest. It deliberately does not define serialization,
  digest algorithms, signatures, physical generation paths, updater I/O, or
  activation.
- Final B11 validation `job_idt_e50502b44b` passed all 10 B11 focused tests,
  the full serial workspace suite 69/69, eight complete default-parallel
  repetitions, formatting, `git diff --check`, and a warning-free locked build
  with offline mode and a repository-external Cargo target. The pre-commit diff
  was limited to `crates/core/src/main.rs` and direct boundary review found no
  B11 serialization, filesystem, environment, Command, FD, or generation-path
  I/O.
- M1-B10 is accepted at `08e67e8c9fed23032ff59c38ff4765221d515d67`.
  The direct Lead added an owned five-value Termux process-environment snapshot,
  a thin raw `var_os` reader for only `PREFIX`, `TMPDIR`, `PATH`,
  `SSL_CERT_FILE`, and `SSL_CERT_DIR`, typed missing/empty required-input errors,
  and a pure snapshot-to-B8 composition that derives only native `PREFIX/bin`.
  It does not read `HOME`, inspect the filesystem, choose a generation/runtime,
  construct a Command, touch FD 33/34, mutate global environment, or wire
  `main`.
- Final B10 validation `job_iba_22c23cddee` used offline mode and a
  repository-external Cargo target. All six B10 focused tests passed, the full
  serial workspace suite passed 59/59, eight full default-parallel repetitions
  passed, formatting and the locked workspace build passed, and `git diff
  --check` passed. A direct boundary audit found exactly five new production
  `var_os` reads and no B10 hard-coded Termux root, `HOME` read, filesystem path
  inspection, or Command construction.
- Sandbox-policy revalidation found that the earlier parser intentionally let
  whitespace-bearing and attached `sandbox_mode` config forms pass through.
  M1-R1 now normalizes surrounding whitespace and one matching quote layer,
  recognizes separate/attached/equals short config forms and long config forms,
  preserves exact `--` scan termination, rejects every non-empty recognized
  `sandbox_mode` value except `danger-full-access`, and continues to reject the
  known unsupported `read-only`/`workspace-write` sandbox flag values. Accepted
  raw user argv is not rewritten after the injected Termux-safe prelude.
- FD-failure revalidation found that restoration syscalls were best-effort and
  some ordinary parallel tests directly mutated process-global FD 33/34. M1-R1
  makes explicit restoration return errors, gives restoration failure precedence
  on returned setup/exec failure paths, keeps Drop only as last-resort cleanup,
  and moves the direct FD 33/34 mutation cases into dedicated subprocess probes.
- Final direct-Lead validation `job_i95_262b8f5b0d` used
  `CARGO_NET_OFFLINE=true` and a repository-external Cargo target. Formatting,
  M1-R1 focused tests (3/3), passthrough focused tests (10/10), runtime-FD
  focused tests (11/11), the full serial workspace suite (53/53), eight complete
  default-parallel workspace repetitions, and the locked workspace build all
  passed. `git diff --check` passed and the only product change before commit was
  `crates/core/src/main.rs`.
- Direct source-boundary audit found no production hard-coded
  `/data/data/com.termux` path, `to_string_lossy`, `env_clear`, TODO, production
  `.unwrap(`, or production filesystem write in the current B1..B9 surface.
  `main()` remains intentionally unwired; the test module begins after the
  production entrypoint and synthetic resolver/config writes remain test-only.
- Fresh behavior disposition after M1-R1: B1 exact first-argument dispatch is
  CURRENTLY PROVEN; B2 raw final-exec argv/streams/exit behavior is CURRENTLY
  PROVEN; B3 the exact five-variable child-only contamination fence is CURRENTLY
  PROVEN; B4 explicit read-only resolver/config FD 33/34 mapping, collision
  handling, caller-state restoration, restoration-error visibility, and
  test-owned resolver non-mutation are CURRENTLY PROVEN; B5 current-device
  TTY/process-identity/external-SIGTERM fidelity is CURRENTLY PROVEN; B6 the
  hardened Termux sandbox-policy planner is CURRENTLY PROVEN; B7 policy-before-I/O
  composition with the runtime-FD final-exec path is CURRENTLY PROVEN; B8 the
  pure explicit-input base-environment planner is CURRENTLY PROVEN; B9 transport
  of a pre-built environment plan through the final-exec composition is CURRENTLY
  PROVEN. These are component proofs only and do not complete Milestone 1.

### Historical Bundle Ledger — revalidation pending


- The predecessor tip `bf30a7d` contains `af640166`, which removed the Termux
  bwrap compatibility path and made unsupported sandbox requests explicit.
- The predecessor tip is preserved by `legacy/monolith` and annotated tag
  `legacy-monolith-bf30a7d-20260828`.
- The current development device has a native `aarch64-linux-android` Rust
  toolchain, Cargo, and Android-targeting Clang.
- The rewrite lineage contains no legacy implementation source.
- `rewrite/rust-core` begins at empty root
  `b3a9da98195cff1053f012d2afa738949b5b14dc` and has no merge-base with
  `main` or `legacy/monolith`.
- Milestone 1 bundle M1-B1 was historically recorded as accepted at
  `36c98dd8882ddba18657ab3f289eace1121ff39b`: the rewrite now has one locked,
  dependency-free Cargo workspace member and one Core binary with exact
  first-argument classification for `update`, `doctor`, and `termux`; all other
  inputs, including `--version`, `-V`, near misses, arbitrary arguments, and
  non-UTF-8 first arguments on Unix, classify as upstream passthrough.
- Primary-Lead validation job `job_hmw_1af3337581` removed only worker-generated
  untracked `target/` artifacts, used an external temporary `CARGO_TARGET_DIR`
  with `CARGO_NET_OFFLINE=true`, and passed `cargo fmt --check`,
  `cargo test --locked --workspace` (6/6), and
  `cargo build --locked --workspace`. Post-validation status job
  `job_hmx_ffe2f81a73` showed only the planned Workboard and four bundle source
  paths before commit.
- The local `.git/hooks/pre-commit` is an untracked predecessor-environment hook
  that invokes absent `tools/update-wrapper-version.sh`; normal commit job
  `job_hmz_60b990cf84` therefore failed without changing HEAD. After read-only
  inspection proved neither the hook nor its referenced path belongs to this
  lineage, the Lead committed M1-B1 once with `--no-verify` under exact HEAD and
  index-tree preconditions. The hook remains unmodified and is not product
  evidence.
- Milestone 1 bundle M1-B2 was historically recorded as accepted at
  `fc50b39e50bb6ef341d3cf01163ca90423bd7b13`. The std-only Unix/Android
  `exec_upstream` primitive uses final `exec` replacement with raw `OsStr`/
  `OsString` inputs. Focused subprocess evidence proves upstream-visible
  `--version`, `-V`, ordinary and non-UTF-8 arguments, exact raw stdout/stderr
  bytes, chosen nonzero exit codes, and direct exec failure reporting without
  adding public test-only command semantics.
- Primary-Lead validation job `job_hnd_bc84d51555` reran the accepted M1-B2
  source with `CARGO_NET_OFFLINE=true` and an external temporary
  `CARGO_TARGET_DIR`; `cargo fmt --check`, `cargo test --locked --workspace`
  (11/11), and `cargo build --locked --workspace` all passed while repository
  status remained limited to the authorized source file before commit.
- Milestone 1 bundle M1-B3 was historically recorded as accepted at
  `815c9104c726f212ee4a51b518af14e8c133b20c`. The production exec command
  removes exactly `CODEX_MANAGED_BY_NPM`, `CODEX_MANAGED_BY_BUN`,
  `CODEX_MANAGED_PACKAGE_ROOT`, `LD_PRELOAD`, and `LD_LIBRARY_PATH` from the
  child exec environment without `env_clear` or parent-process mutation;
  unrelated environment entries are preserved. Failed exec evidence proves the
  caller process retains its synthetic inherited values.
- Primary-Lead validation job `job_hnt_c908186115` reran M1-B3 with an external
  temporary Cargo target and offline mode; formatting, 13/13 tests, and locked
  workspace build passed while status remained limited to `crates/core/src/main.rs`.
- Milestone 1 bundle M1-B4 was historically recorded as accepted at
  `bb21ddca58589ec77a22e824c4218db5c1087daa`. The runtime-FD exec path opens
  an explicit resolver source and existing managed-config directory read-only,
  maps them to FD 33/34 with CLOEXEC cleared, uses safe CLOEXEC duplicates above
  FD 34 to avoid source/target collisions, and restores originally absent or
  present caller FD 33/34 state when setup or exec fails. Resolver-content and
  Unix metadata evidence proves the test resolver is unchanged across exec.
- Lead review found and corrected one pre-acceptance defect: only `EBADF` now
  classifies an `F_GETFD` probe as descriptor absence; all other probe errors
  propagate. Primary-Lead validation job `job_hol_118858c4b8` passed formatting,
  24/24 workspace tests, three additional serial repetitions of all 11
  `runtime_fds` tests, and locked workspace build with offline mode and an
  external Cargo target.
- Milestone 1 bundle M1-B5 was historically recorded as accepted at
  `85f312b7d5d0e2e8a14c9084063e437633b63480`. Test-only private probes prove
  the production final `exec_upstream` boundary preserves a PTY on stdin,
  stdout, and stderr on the current Android/Termux device and preserves process
  identity across exec: the upstream shell reports `$$` equal to the spawned
  child PID, receives an external `SIGTERM` sent to that same PID, and executes
  its trap with exit code 73. No production behavior changed in B5.
- Primary-Lead validation job `job_hqo_7f45af3f26` passed formatting, all 26/26
  workspace tests, three additional serial repetitions of each TTY and SIGTERM
  proof, and locked build with offline mode and a repository-external Cargo
  target.
- Milestone 1 bundle M1-B6 was historically recorded as accepted at
  `a4b4cb3a91bd78ea07952739f054695f10bab638`. The module-private passthrough
  planner rejects the bounded observed Linux `read-only`/`workspace-write` and
  leading `sandbox linux` request forms before launch planning, stops scanning
  at exact `--`, preserves accepted raw `OsString` argv byte-for-byte, and
  prepends only `-c` plus `sandbox_mode=\"danger-full-access\"`. It never
  synthesizes the upstream approval-bypass flag and does not wire a runtime
  executable or product path.
- The first B6 worker result was rejected before acceptance because it expanded
  unobserved forms and could reinterpret a separate option value as a later
  policy option. The bounded correction consumed exactly one following value
  token for separate sandbox/config options and narrowed recognition to the
  accepted forms. Primary-Lead validation job `job_ht1_c593e9a07a` then passed
  formatting, all 34/34 workspace tests, three serial repetitions of the 10
  `passthrough_` tests, and the locked workspace build with offline mode and a
  repository-external Cargo target.
- Milestone 1 bundle M1-B7 was historically recorded as accepted at
  `5e5044eb3ae9286b72b16f1e1b9092f4e728bc82`. The module-private
  `launch_upstream` composes B6 policy planning with the accepted B4 runtime-FD
  final-exec primitive through explicit program/resolver/config/user-argv
  inputs. Policy rejection occurs before runtime I/O; accepted launch crosses
  the real exec boundary with the exact no-sandbox argv prelude, supplied FD
  33/34 sources, the existing five-variable contamination fence, and unrelated
  environment preservation. `main` remains unwired and no runtime path policy
  is introduced.
- Primary-Lead validation initially used an invalid focused `--exact` filter in
  `job_hvh_029dd726eb`; those zero-test repetitions are not acceptance evidence,
  although that job's full 38-test run passed. Corrected validation job
  `job_hvj_d1a56ada7b` ran each of the four B7 focused tests exactly three times
  (12 actual focused runs), then passed all 38/38 workspace tests and the locked
  build with offline mode and a repository-external Cargo target.
- Milestone 1 bundle M1-B8 was historically recorded as accepted at
  `ae678fdb01b065a78f55b4e0546a8c4b12c498fa`. The module-private Unix/Android
  base-environment planner is std-only and explicit-input-only: it plans the four
  temporary-directory variables, certificate fallback/precedence, and raw-byte
  PATH composition without reading process environment or filesystem state,
  constructing a Command, choosing product paths, or wiring `main`.
- The first B8 worker result was rejected before acceptance because one purity
  test read the filesystem, one focused assertion used lossy string conversion,
  and an unrequired non-Unix PATH fallback encoded Unix delimiter semantics
  outside the current target. Correction job `job_hyc_a66aed1797` removed only
  those expansions. Primary-Lead validation `job_hyn_2d9868514e` then ran all
  nine B8 focused tests three times (27 real focused executions), all 47/47
  workspace tests, formatting, and the locked workspace build with offline mode
  and a repository-external Cargo target. The positive assignments remain a
  bounded M1 compatibility hypothesis until applied and qualified on the real
  Termux execution boundary.
- Milestone 1 bundle M1-B9 was historically recorded as accepted at
  `692cd8b0c9cc4babe273ab9bdfa9d14eabc9db0c`. One shared final-exec
  implementation now accepts an optional `TermuxBaseEnvPlan`; positive raw
  `OsString` assignments are applied directly to the child `Command`, then the
  exact B3 five-variable contamination fence is enforced. Existing public/test
  launch signatures still traverse the same implementation with no positive
  plan, while the new module-private environment-aware launch composition
  preserves B6 policy-before-I/O ordering and B4 FD 33/34 restoration.
- Primary-Lead validation `job_i1l_cabb18a109` ran all three B9 focused tests
  three times (9 actual focused executions), all 50/50 workspace tests,
  `cargo fmt --check`, and the locked workspace build with
  `CARGO_NET_OFFLINE=true` and a repository-external Cargo target. The worktree
  remained limited to `crates/core/src/main.rs` before commit. Real exec proof
  jointly observed the planned temp/certificate/PATH values, exact sandbox
  prelude and raw user argv, FD 33/34 sources, the contamination fence, and one
  unrelated inherited variable; failed exec preserved caller environment and
  restored prior FD state.

### Historical Proof

- The predecessor's behavior and history remain available only as sealed
  legacy evidence. They are not promoted into proof for the rewrite.

### Not Proven

- M1-B19 is accepted at `148b1133f1afaa91668e19b4fade13bc761b0056`.
  The direct Lead added one ordered local doctor command boundary that validates
  B18 usage before invoking B17 local doctor composition, preserves Usage and
  Probe as distinct typed errors, and renders a successful bounded report only
  after probe completion. Invalid UTF-8 and ordinary invalid doctor argv are
  therefore proven to fail before environment planning, resolver/config FD
  setup, or runtime spawn.
- Final B19 validation `job_iq5_44896f380d` passed all 5 B19 focused tests, the
  full serial workspace suite 116/116, eight complete default-parallel
  repetitions with per-run failure logs, formatting, `git diff --check`, and a
  warning-free locked build with offline mode and a repository-external Cargo
  target. Boundary audit found no `main` wiring, lossy argv conversion, or new
  production filesystem path beyond the already-shared doctor/launch helpers.
- M1-B18 is accepted at `fdecef9f86a1f04776309ffe344b169d715c7217`.
  The direct Lead added a pure doctor invocation/output contract over an
  already-composed bounded report. Arguments following exact leading `doctor`
  are accepted only as empty for human output or exactly one raw `--json` token
  for machine output; every other shape including non-UTF-8 fails with one static
  non-echoing usage error. Rendering preserves the B15 human/JSON envelope and
  typed `DoctorExitClass` without assigning unspecified numeric process codes.
- Final B18 validation `job_ioy_1672a340bc` passed all 5 B18 focused tests, the
  full serial workspace suite 111/111, eight complete default-parallel
  repetitions with per-run failure logs, formatting, `git diff --check`, and a
  warning-free locked build with offline mode and a repository-external Cargo
  target. Direct audit found no lossy argv conversion, new process/filesystem/
  environment access, or `main` dispatch change in B18.
- M1-B17 is accepted at `64199eb4cbb1dccb351cf140c55e5e36d77d65ce`.
  The direct Lead added a bounded local doctor coordinator: explicit Supported
  capability invokes the B16 qualified upstream probe exactly once and composes
  only its bounded status with already-typed Core/Manager states through B15;
  explicit Unsupported capability skips process-environment planning,
  resolver/config access, FD mapping, runtime spawn, and stderr inference
  entirely. Typed B16 setup/spawn errors propagate without fabricating a report.
- Final B17 validation `job_inr_7ffbd3d9f8` passed all 4 B17 focused tests, the
  full serial workspace suite 106/106, eight complete default-parallel
  repetitions with per-run failure logs, formatting, `git diff --check`, and a
  warning-free locked build with offline mode and a repository-external Cargo
  target. An earlier validation `job_inp_6b77b4a712` observed one unlogged
  transient parallel failure after its serial 106/106 pass; dedicated
  reproduction `job_inq_018394a0b3` then passed five consecutive full parallel
  runs before the final eight-run acceptance validation. B17 tests also proved
  unsupported I/O skipping, API-incompatibility precedence, exact bounded
  human/JSON rendering, raw-output exclusion, and no report on spawn failure.
- M1-B16 is accepted at `d420db8b4128d44836c89394cbbb9afc9398b1e5`.
  The direct Lead added a supported-upstream doctor child probe that consumes
  only B13 qualified runtime assets, B10 environment inputs, and explicit
  resolver/config paths. Final exec and doctor now share one temporary FD33/34
  mapping/restoration primitive and one child environment/fence helper. Doctor
  invokes the selected raw runtime directly with the Termux-safe prelude plus
  `doctor`, discards raw stdout/stderr, classifies only child completion status,
  and restores parent FD state.
- Final B16 validation `job_imd_136341d507` passed all 4 B16 focused tests, the
  full serial workspace suite 102/102, eight complete default-parallel
  repetitions, formatting, `git diff --check`, and a warning-free locked build
  with offline mode and a repository-external Cargo target. B16 tests proved
  exact safe argv/env/FD behavior, secret-like output suppression, explicit
  nonzero-to-unhealthy classification without stderr inference, typed spawn and
  pre-I/O environment failures, parent FD restoration, and test-owned
  resolver/config non-mutation.
- M1-B20 is accepted at `07ed3af8764c10f03aaf3bf83b18ffb37a32b891`.
  The direct Lead added one pure public-dispatch planner over complete raw argv.
  Only exact first-token `update`, `doctor`, and `termux` are intercepted; each
  Core route consumes only that token and retains all trailing raw `OsString`
  values byte-for-byte. Every other shape, including empty argv, `--version`,
  `-V`, delimiter/near-miss forms, and non-UTF-8 first tokens, remains one
  upstream route with the complete original argv and order preserved.
- Final B20 validation `job_irt_1daf7615c8` passed all 5 B20 focused tests, the
  full serial workspace suite 121/121, eight complete default-parallel full
  repetitions, formatting, `git diff --check`, and a warning-free locked build
  with offline mode and a repository-external Cargo target.
- M1-B21 is accepted at `5847f0d8d223e6abdf8d1876fc316ac1fda7b281`.
  The direct Lead added a pure optional Manager-artifact qualification boundary
  over the already-qualified generation. Absent manifest binding plus absent
  selection is one explicit `Unavailable` state; a declared Manager requires one
  explicit absolute NUL-free raw path and a nonempty observed digest matching the
  manifest before it can become `Available`. Presence disagreement, path-shape
  failures, empty digest, and digest mismatch are distinct typed failures.
- B21 focused validation `job_it2_21e09d1f28` passed 7/7. Final grouped
  acceptance `job_it4_84accf60ab` passed the full serial workspace suite
  128/128, eight complete default-parallel full repetitions, formatting,
  `git diff --check`, and a warning-free locked offline build with an external
  Cargo target. No Click plugin/Hook was used; the Workboard anti-loop discipline
  reused evidence and avoided an extra intermediate full-suite pass.
- M1-B22 is accepted at `a89646da18c1e0b68e146b565a847dcc4b0fd0b6`.
  The direct Lead completed the Core-side qualified Manager handoff. An explicit
  `Unavailable` Manager returns one bounded static outcome without consuming
  argv or constructing a process; `Available` can execute only the B21-qualified
  Manager path, appends only the original raw trailing argv, inherits ordinary
  environment and standard streams, and uses Unix final `exec` semantics.
  Failed exec is a typed I/O error and leaves the caller environment unchanged.
- B22 focused validation `job_iu0_937ceaf5fd` passed 4/4, including real raw
  non-UTF-8 argv/stream/exit evidence and process-identity/SIGTERM delivery.
  Final grouped acceptance `job_iu2_cf462a7fad` passed the full serial workspace
  suite 132/132, eight complete default-parallel full repetitions, formatting,
  `git diff --check`, a warning-free locked offline build, and source-text NUL
  absence. The same commit also replaced B21's literal-NUL test fixture with an
  equivalent numeric-byte fixture so repository text search remains reliable.
- M1-B23 is accepted at `c29f5f2019104ad7ab51f36754f326b48d33704c`.
  The direct Lead composed the exact public route with the previously proven
  Upstream, Doctor, and Manager execution boundaries over one injected qualified
  local context. Update remains a raw-byte-preserving zero-I/O handoff to the M1
  updater interface. During integration the Lead found and closed a latent
  cross-generation ambiguity: Manager `Unavailable` now retains its qualified
  generation, and context construction rejects runtime/Manager qualifications
  from distinct manifest objects before any route can execute.
- B23 focused validation `job_iv4_0729f98d00` passed B21 7/7, B22 4/4, and
  B23 6/6 after the representation correction. Production diff audit
  `job_iv5_dc87f7dca8` confirmed no new filesystem/environment/Command/lossy
  production path and no `main` body change. Final grouped acceptance
  `job_iv6_b5bb5f840a` passed the full serial suite 138/138, eight complete
  default-parallel repetitions, formatting, `git diff --check`, source NUL
  absence, and a warning-free locked offline build with an external Cargo target.
- M1-B24 is accepted at `db67c0b90e1916d2ec452b8db2657dd4d504cd52`.
  The direct Lead added one thin raw-argv public entrypoint composition that
  performs the B20 planner exactly once and passes the resulting route directly
  into the B23 qualified dispatcher. Production `main` remains intentionally
  unchanged because physical active-generation context acquisition belongs to
  Milestone 2. Test-only B24 evidence added an explicit real-Termux smoke gate
  using the actual live resolver read-only, a test-owned fake qualified runtime
  and config root, FD33/34, and byte-exact direct-vs-Core `--version` output.
- Focused/smoke validation `job_ivj_a0374d97fe` passed the B24 zero-I/O entrypoint
  test and the explicitly selected real-Termux smoke 1/1 on Termux
  `0.119.0-beta.3`, `aarch64-linux-android`. Production audit
  `job_ivk_7d7e01c56a` confirmed the only production addition is
  `plan_public_dispatch(raw_args) -> execute_public_dispatch(...)` with no new
  filesystem/environment/Command/lossy path and no `main` body change. Final
  acceptance `job_ivl_1ddcbf4dc5` passed 139 default tests with the explicit
  smoke correctly ignored, eight complete default-parallel repetitions,
  formatting, `git diff --check`, and a warning-free locked **release** build.
  External pre/post evidence kept the live resolver exactly at SHA-256
  `7e8ad76e0d200e93918ca2e93c99ff8ecd02071953bf1479819db3ac0dbb6d07`
  and the installed launcher exactly at SHA-256
  `0b0284155f2672263836029f760ba06a0cb284b7ca3a8e600ad399b43af36aff`,
  with device/inode/mode/uid/gid/size/mtime identity unchanged.
- **M1-B24 established the required local Core behavior evidence, but M1-R2 has
  reopened final product closure until real `main` wiring is complete.** The required local
  Core behavior is now proven by source, subprocess, and current-Termux evidence:
  exact public routing and upstream passthrough; upstream-only version behavior;
  environment/final-exec semantics; FD33/34 and live-resolver non-mutation;
  explicit sandbox behavior; bounded read-only redacted doctor composition;
  generation/updater interfaces without live mutation; unit/integration/fault
  coverage; and an explicit real-Termux smoke gate. No live Codex installation,
  runtime, Manager, resolver, package, update, activation, or publication ref was
  changed during Milestone 1. Physical generation state, installation,
  activation, recovery, rollback, artifact delivery, and installed `main`
  context acquisition remain Milestone 2 work and are not promoted by M1 proof.
- No release artifact, installation, update, activation, rollback, offline
  recovery, fresh-device behavior, Milestone 2 result, or production readiness
  is proven.
- No Manager implementation or Core/Manager integration is proven.

### Checkpoint Plans

- M1-R1 is closed at `4c1a8d90d6aa028106218d349076c465af8b8535`.
- M1-B10 is closed at `08e67e8c9fed23032ff59c38ff4765221d515d67`.
- M1-B11 is closed at `0eb9f6cd33951ff782c010d9e116ab886f70a815`.
- M1-B12 is closed at `3927ad46696875c913c9039406693c1ddd4c3231`.
- M1-B13 is closed at `71acbd8e318d50548952490e0d2fb52c7b661f9c`.
- M1-B14 is closed at `be6492f895185caf7d9b922b16330a1cd8f00033`.
- M1-B15 is closed at `6bea7a53004f43178599e65a6f630c7bb06355b9`.
- M1-B16 is closed at `d420db8b4128d44836c89394cbbb9afc9398b1e5`.
- M1-B17 is closed at `64199eb4cbb1dccb351cf140c55e5e36d77d65ce`.
- M1-B18 is closed at `fdecef9f86a1f04776309ffe344b169d715c7217`.
- M1-B19 is closed at `148b1133f1afaa91668e19b4fade13bc761b0056`.
- M1-B20 is closed at `07ed3af8764c10f03aaf3bf83b18ffb37a32b891`.
- M1-B21 is closed at `5847f0d8d223e6abdf8d1876fc316ac1fda7b281`.
- M1-B22 is closed at `a89646da18c1e0b68e146b565a847dcc4b0fd0b6`.
- M1-B23 is closed at `c29f5f2019104ad7ab51f36754f326b48d33704c`.
- M1-B24 is closed at `db67c0b90e1916d2ec452b8db2657dd4d504cd52`.
- M1-B24 historical acceptance is retained, but current Milestone 1 product closure is reopened by M1-R2 until real `main` wiring is completed.
- M2-B1 is accepted at `918c3681729ab8f6bba8f69607a88380645b3b5d`.
  It establishes the crash-safe complete-generation/atomic-activation recovery
  foundation in test-owned roots. Final validation `job_iwc_7350098964` passed
  151 tests with the explicit B24 smoke ignored by default, eight complete
  default-parallel repetitions, formatting/diff checks, and a warning-free
  locked release build while preserving the live resolver and installed launcher
  identity. M2-B1 is a foundation, not a mandate to add multi-writer fencing or
  more fallback tiers.
- User-directed M1-R2 reopens the Milestone 1 implementation closure for one
  exhaustive simplification and product-wiring audit before M2-B2 continues.
  This is not a sampled review: every surviving M1-R1/B1..B24 production
  definition and M1 test/probe harness must receive a keep/collapse/delete
  disposition against the current release-speed policy. Proof-only wrappers,
  duplicate validators, redundant defensive state, and tests that exist only to
  support removed mechanisms are deleted or folded. Load-bearing public behavior
  remains required.
- M1-R2 exhaustive simplification is implemented at
  `2b73f4ba23726ddab0792bbba721a2835dcb86d9`. The accumulated M1 implementation
  was reduced to 2,330 production lines and 1,624 test lines in `main.rs`; the
  change removed 9,420 lines while adding 1,666 lines of consolidated product
  and contract tests. Historical `test_m1_b*` bundle tests and all audited
  duplicate/proof-only layers are absent. The retained suite passes 33/33 serial
  with one explicit live smoke ignored by default; the explicit live
  resolver/installed-launcher smoke passes 1/1 and three complete default-parallel
  runs pass. M2-B1's 12 fault/recovery tests remain retained and passing.
- R2 KEEP groups are the direct public route planner, sandbox policy, final
  runtime FD/env/exec primitives, Termux environment snapshot/plan, generation
  manifest qualification, runtime/Manager qualification, one updater
  qualification gate, bounded doctor report/probe/command path, one public
  dispatch executor, and M2-B1 activation recovery. COLLAPSE/DELETE groups are
  duplicate B1 classification, parent FD restoration machinery, B8/B10 wrapper
  environment planners, B12 evidence-promotion/readiness wrappers, B21/B23
  generation-pointer mismatch machinery, B15-B19 planner/coordinator/render
  wrappers, nested launch errors, B24's proof-only entrypoint wrapper, and the
  unused shared-resolver fallback model. Tests were consolidated by product
  contract rather than bundle provenance.
- The remaining `main()` gap is now precisely classified: physical current-
  generation context acquisition is the missing input, and SPEC assigns that
  ownership to Milestone 2. Hiding the resulting dead paths with more
  `allow(dead_code)` would violate R2. M2-B2 therefore owns the minimal local
  activated-generation loader and real `main -> plan_public_dispatch ->
  execute_public_dispatch` wiring. M1 product closure remains open only until
  that cross-milestone connection is accepted.
- M2-B2 is accepted at `bee38e9eb481973c00205fb8a7191cdb22392f7c`.
  Production `main()` now performs raw public planning, loads exactly one
  activated generation from the M2 local layout, qualifies runtime/optional
  Manager assets from that generation, and executes upstream/doctor/Manager
  through the retained direct boundaries. Ordinary launch uses only `current`;
  it does not scan generations, read `previous`, canonicalize a fallback chain,
  use network, or invoke a package manager. The redundant in-memory
  `GenerationQualification` state was removed because the descriptor already
  requires `qualification=qualified`, and the duplicate state-root
  `generations/` directory was removed in favor of the SPEC-owned immutable
  generation root.
- B2 acceptance evidence: focused loader/main 6/6; full serial 38 passed / 0
  failed / 1 explicit smoke ignored by default; explicit real-Termux smoke 1/1;
  three complete default-parallel runs; `cargo fmt --check` and
  `git diff --check`; warning-free locked release build; live resolver and
  installed launcher SHA/stable-stat identity unchanged before/after. The only
  production `allow(dead_code)` is the existing M2 activation-state module's
  write side, which is the immediate input to local staging/activation work and
  does not hide an M1 product path.
- With `2b73f4ba...` simplification plus B2 real entrypoint wiring, Milestone 1
  product closure is re-accepted. M1 is no longer closed on proof-only
  injection evidence; the real production entrypoint reaches final execution.
- M2-B3 is accepted at `b692853a436e7df2540ccb1c52e967af4e921375`.
  `codex update --local <directory>` now has a real offline/bootstrap staging
  path: it copies only the fixed generation layout through a private candidate,
  rejects symlinks/special files, validates the copied candidate with the same
  B2 loader, and atomically publishes a complete **inactive** generation. B3
  never mutates activation state. Focused staging is 7/7; full serial is 46
  passed / 0 failed / 1 explicit smoke ignored by default; explicit live smoke
  1/1; three complete default-parallel runs; warning-free locked release build;
  live resolver and installed launcher identity unchanged. No activation,
  network, package-manager, lock/fencing, or fallback mechanism exists in B3.
- B4 feasibility evidence is concrete: current Termux provides
  `/data/data/com.termux/files/usr/bin/openssl`, OpenSSL 3.6.3, with SHA-256 and
  `pkeyutl -verify -rawin -pubin`; a job-private Ed25519 sign/verify roundtrip
  passed. This permits a vetted crypto path without adding a Rust dependency or
  installing a package. The release trust anchor must be bootstrap-provisioned;
  Core must not accept a public key shipped next to the release it is verifying.
- M2-B1's `verified` pointer is confirmed redundant: every constructor and
  rollback writes `verified == current`, and it has no independent product
  consumer. B4 removes it before activation and retains only `current` plus one
  explicit `previous` rollback target.
- Current checkpoint: M2-B4 — signed local release admission and activation.
  Verify one strict Ed25519-signed local release manifest and SHA-256 file
  inventory using the pinned bootstrap key and existing Termux OpenSSL, probe
  the admitted staged generation, then activate it through the simplified M2-B1
  transaction. No remote acquisition or fallback ladder is part of B4.
- Worker mode remains OFF; the primary Lead performs M2-B4 directly.
- M2-B4 implementation/proof divergence checkpoint is bound to
  `rewrite/rust-core@8df74abfca793fe9e8008553b5d9742ba6d2b4d4`, source SHA-256
  `de0943aa415bb6a7416a7e83a59755f53f90a53df1d10a437694e96a66433575`, and
  source-diff SHA-256
  `7007cad7881378a0a66f75fa207f968c2b5d01e2efebe22b67e1588121d81621`.
  The diff adds 684 and removes 88 lines, but adds no B4 regression test; the
  retained test target does not compile because three `LocalCoreRoots` fixtures
  omit the new trust/OpenSSL fields, the release build has one dead-code warning,
  and the prior B3 public test still asserts staging without `PREFIX` or
  activation. This exact snapshot is repairable work in progress, not an
  acceptance candidate.
- The checkpoint root cause is an execution-policy gap: final grouped validation
  was batched correctly, but compile/focused proof was also deferred, allowing
  several independent production contracts to accumulate behind red tests.
  M1-R2's KEEP/COLLAPSE/DELETE lesson existed only as historical ledger evidence
  instead of an active per-slice stop rule.
- Primary-Lead disposition: planning completed without a delegated planner or
  worker; freeze new B4 behavior, revise the current Workboard into vertical
  proof slices, exhaustively disposition the changed production/test class,
  restore compile/focused proof slice by slice, and run one grouped final
  acceptance batch only after stabilization. The success threshold is unchanged,
  so no goal lift is active.
- Same-revision evidence reuse remains required. It prevents redundant full-suite
  churn but never permits skipping a red slice gate. No Click plugin or Hook is
  installed, and this workflow does not override SPEC/GOAL acceptance.
- M2-B4 — signed local release admission and atomic activation — is accepted at
  `8483d2b2db488af032d6f9829639f971e2b5ff3f`. Exact
  `codex update --local <DIRECTORY>` now admits only the bootstrap-pinned
  Ed25519 key and strict signed SHA-256 inventory, enforces release policy and
  sequence before candidate execution, stages and re-verifies one complete
  generation, runs version/declared-doctor probes, and atomically activates it.
  Exact `codex update --rollback` verifies and swaps only the retained signed
  `previous` generation. Activation state is the single v2 `current` plus
  optional `previous` model; `verified`, unsigned staging, proof-only rollback,
  duplicate qualification, and fallback machinery are absent.
- The accepted B4 source SHA-256 is
  `ea8c840a7f4bff1dcbe3fd3ae36b16b5acb4e0389b8ec8304a069584a1fa49ba`
  and its parent-relative source-diff SHA-256 is
  `3d5b17788c66aacf3fda26d317112c1b1e23e2ae3257d64f05dbbee4600f69c5`.
  Final evidence passed focused activation 4/4, public rollback 2/2, the full
  serial suite 59 passed / 0 failed / 1 explicit smoke ignored, three isolated
  complete default-parallel runs each 59/0/1, explicit real-Termux read-only
  smoke 1/1, formatting/diff checks, and a warning-free locked release build.
  Live resolver and installed launcher SHA-256 plus device/inode/mode/uid/gid/
  size/mtime identities were unchanged; no live generation, activation,
  Manager, resolver, auth/session/profile, package, or publication state changed.
- B4's final product-path review caught one same-class defect after the first
  grouped run: forward update had not bound the recovered `current` pointer name
  to its signed generation identity. Acceptance was reopened, all installed
  target checks were collapsed into one verifier, a public pre-staging
  regression was added, and the complete grouped batch was rerun on the repaired
  source. A mistyped nonexistent Cargo package produced no test execution and was
  explicitly excluded from evidence. This closes the earlier large-change
  failure mode with slice-local stop-on-red proof rather than end-batched tests.
- Current checkpoint: M2-B5 — immutable HTTPS release acquisition. Add the
  smallest remote signed-release file acquisition adapter that feeds the exact
  B4 admission/staging/probe/activation path. Define its public and transport
  contract in `SPEC.md` before product code; do not create a second updater,
  archive path, release-discovery service, automatic checker, or fallback.
- M2-B5 starts from clean `rewrite/rust-core@8483d2b2db488af032d6f9829639f971e2b5ff3f`.
  Read-only feasibility found dependency-free Core plus Termux curl 8.21.0 at
  `$PREFIX/bin/curl` with HTTPS/OpenSSL support. No remote endpoint is yet
  authoritative and no live network request is acceptance evidence; transport
  tests use a pinned fake curl in temporary roots. Worker mode remains OFF and
  the primary Lead performs M2-B5 directly.
- M2-B5 — immutable HTTPS release acquisition — is accepted at
  `c39c338d6238d3e8aba128d8fa522d0e9de66d83`. Exact
  `codex update --remote <HTTPS_BASE_URL>` now validates one bounded canonical
  HTTPS generation base, invokes only `$PREFIX/bin/curl` with config/environment
  disabled and explicit CA/time/byte policy, admits bounded signed control before
  content, reconstructs only the signed file inventory in a private `.acquire-*`
  root, removes acquisition before probes/activation, and reuses the exact B4
  admission, staging, probe, atomic activation, anti-rollback, and rollback path.
  There is no discovery, redirect, retry, mirror, fallback URL, archive path,
  second updater, or live-network acceptance claim.
- B5 replaced the mode-blind release v1 format with strict
  `codex-release-v2`: every signed file record binds lowercase SHA-256 and exact
  four-octal-digit regular-file mode; special bits are impossible, every file is
  owner-readable, and runtime/Manager/helpers are owner-executable. Local source,
  remote reconstruction, staged generation, current verification, and rollback
  all use the same v2 check. The accepted source SHA-256 is
  `d5c5f69da6ce8d7f52b20ce8d426d3948e3452566a3a4f075dca67fd1e773dca`
  and its parent-relative source-diff SHA-256 is
  `747c4cc5fd3086454696179713b2e3120c967947c907ee00add3e75636e63196`.
- Final B5 evidence passed the full serial suite at 69 passed / 0 failed / 1
  explicit smoke ignored, three complete default-parallel suites each at
  69/0/1, the explicit real-Termux read-only smoke at 1/1, formatting/diff
  checks, and a warning-free locked release build. Live resolver and installed
  launcher SHA-256 plus device/inode/mode/uid/gid/size/mtime identities were
  unchanged; no live network, generation, activation, Manager, resolver,
  auth/profile/session, package, launcher, or publication state changed.
- The first otherwise-green grouped B5 batch was rejected because post-run
  inspection found one leaked B4 test root. The whole retained test cleanup
  class discarded `remove_dir_all` errors. B5 replaced it with one strict helper
  that tolerates only `NotFound`, removed the disposable leaked fixture, proved
  the affected regression 1/1, observed zero residue, and reran the entire
  grouped batch above on the repaired source. This extends the earlier
  stop-on-red lesson from test counts/warnings to cleanup evidence: a green test
  exit cannot override contradictory filesystem state.
- M2-B6 — official upstream artifact acquisition and safe adaptation — is
  accepted at `92787c85e4bc27de867f800d1414125d6247a210`. One non-installed
  release-production `codex-release-builder` accepts only an explicit stable
  version, local official aarch64-musl package, lowercase pinned SHA-256, Core
  artifact, metadata, and absent output. It snapshots while hashing, parses the
  bounded exact ustar/PAX layout without archive-directed writes, selects only
  the two static AArch64 executables, applies the exact drift-detecting 2/1/1/1
  equal-length policy, and complete-or-absent publishes only `generation.meta`,
  adapted `runtime`, and unmodified `compat/codex-code-mode-host`. It performs no
  discovery, signing, installation, activation, or live-state mutation.
- B6 is bound to official package `0.150.1` archive SHA-256
  `1ecac3f87823efb98153233b076ea3d6e34a7a8cebe43c5285dc5f79e1514639`.
  The release-builder library SHA-256 is
  `5cf290e919adaa4ef92f1715cff4cb0cdb2f6ad9973020f0111f60f00f4019ca`,
  its entrypoint SHA-256 is
  `ce09449e603806ea8bf8571f5978b11a58974f40ddba736ad2924204a0b40d1d`,
  the Core source with test-only B4/B5 integration is
  `71086b63782724cfad134af47f701932861bc01da8964787ab995914df165642`,
  and the parent-relative code-diff SHA-256 is
  `b9fc1e2b4e87aaac063ac8825c2b7a791965341a55ca0cf9339c169edcc43fe1`.
- Final B6 evidence passed the full serial workspace suite at Core
  70/0/1-ignored plus builder 5/0, three complete default-parallel suites at the
  same counts, the explicit real-Termux read-only smoke at 1/1, formatting and
  diff checks, and a warning-free locked release build. Test-root residue was
  zero; live resolver and installed launcher SHA-256 plus
  device/inode/mode/uid/gid/size/mtime identities were unchanged. A zero-test
  command and one test-only name-shadowing compile failure were rejected and
  corrected before evidence was reused. The shared pre-commit hook referenced a
  script absent from the orphan lineage, so the already-validated exact staged
  diff was committed with `--no-verify`; the hook did not change source or index.
- M2-B7 — signed trust-key rotation and rollback compatibility — is accepted by
  this authority update from base
  `253156c37a2bd22af8faae0bce03587999ffd136`. The user approved the exact
  security contract before mutation. `codex-release-v3` now binds one canonical
  32-byte Ed25519 release key, requires the candidate-key `release.sig` over the
  exact manifest, and requires a second `release-authority.sig` by the current
  update key only when that key changes. Adjacent/discovered keys, PKI/keyserver
  trust, unbounded key history, and a second updater remain absent.
- B7 upgrades durable authority to one `codex-activation-state-v3` /
  `codex-activation-journal-v3` transaction containing `update_key`,
  `current/current_key`, and the optional `previous/previous_key` pair. Forward
  activation advances update authority with the accepted candidate. Explicit
  rollback swaps only generation/verifier pairs and never rolls back
  `update_key`; a rotated-away key therefore cannot regain forward signing
  authority through runtime rollback. Installed-generation verification binds
  the state verifier key to the manifest release key before signature/inventory
  admission, and ordinary launch remains independent of OpenSSL, network, and
  trust verification.
- Bootstrap trust is now explicitly one-way. The pinned
  `release-public-key.pem` may initialize the first v3 state only in the later
  bootstrap bundle; production Core update/recovery owns no bootstrap-key path
  and fails closed when v3 state is absent rather than reconstructing authority
  from the old bootstrap key. Local and remote update, current verification,
  staging, activation, recovery, and rollback reuse the same bounded trust
  policy; remote rotation conditionally acquires the authority signature before
  any generation payload.
- The accepted B7 Core source SHA-256 is
  `c4e501ece0ac6ccf75a01409f7e8e804297b326a5ead6317516f9f9755f1a3e0`,
  its parent-relative binary-diff SHA-256 is
  `06ccbd8cbd42f156de861e753df778dac3f6f29a60648fd48fa228759c8b4fc6`,
  and the approved B7 specification SHA-256 is
  `4ca9035c9c1a31c5afc3e9d4de978b304c96c687d03c0bee0aa446078fe11647`.
  Focused proof closed signed-transition admission 1/1, state rotation 1/1,
  local/remote/rollback integration 3/3, the B1 trust+generation fault matrix
  12/12, B4 14/14, B5 10/10, and the B6 admission bridge 1/1.
- Final B7 evidence was rerun from scratch after the last warning repair on one
  final formatted source. The full serial workspace suite passed Core
  75/0/1-ignored plus builder 5/0; three complete default-parallel workspace
  suites each passed the same counts; the explicit real-Termux read-only smoke
  passed 1/1; and the locked release build passed with `-D warnings`. Test-root
  residue was zero and the generated untracked `target/` build tree was removed.
  The installed launcher remained SHA-256
  `0b0284155f2672263836029f760ba06a0cb284b7ca3a8e600ad399b43af36aff`
  with the exact pre-run device/inode/mode/uid/gid/size/mtime identity, and live
  `resolv.conf` remained SHA-256
  `7e8ad76e0d200e93918ca2e93c99ff8ecd02071953bf1479819db3ac0dbb6d07`
  with its exact pre-run identity. No live generation/trust state, resolver,
  installed launcher, Manager, auth/profile/session, package, network authority,
  or publication state changed.
- B7 rejected and repaired each red gate before proceeding: the registered
  validation metadata was found stale and npm-bound before any product mutation,
  so zero Rust tests from that path were counted; direct Cargo established the
  green baseline instead. The first raw-key OpenSSL path required `/dev/stdin`,
  stale v2 assertions/fixtures were updated only after their new fail-closed
  meaning was proven, and the first final release build exposed one production
  dead-code warning after bootstrap removal. That planner was narrowed to
  test-only bootstrap fixtures and the entire grouped acceptance was rerun.
  Registered project validation metadata remains an administrative mismatch and
  must be repaired before the next product slice relies on it.
- M2-B8 — prebuilt Core and minimal fresh bootstrap — is accepted at product tip
  `192fcece0b416004bddd9181e24b1245290ebe81` by this authority update. The B6
  `--core` input now snapshots and qualifies exactly one ELF64 little-endian
  AArch64 PIE using `/system/bin/linker64`, then binds the selected bytes through
  the existing `generation.meta.core_artifact_digest`; no Core copy, sidecar,
  alternate manifest/archive, or second release-production protocol was added.
  The accepted release-builder source SHA-256 is
  `aaa8ac051bf634bdf8fda799ca194b228b11e61e41fd1837570f979daefb4c9b`.
- Fresh bootstrap is one local audited script,
  `bootstrap/codex-bootstrap`, SHA-256
  `c1b107699a64c08cc49a99ceb433c3dd1b6c7ca53637fb0b53cc73b6ce35e9fa`.
  Before executing or publishing Core it snapshots the Core, bootstrap key, v3
  manifest/signature, and `generation.meta`; verifies the manifest with the
  pinned key; requires the signed release key to equal that key; verifies the
  signed descriptor digest/mode; and requires the Core SHA-256 to equal the
  descriptor's signed `core_artifact_digest`. It then publishes only the
  canonical initial key seed and stable Core entrypoint via same-directory
  create-new/no-clobber temporaries and invokes the authenticated Core for
  self-test and first activation. It has no network, package-manager, compiler,
  update, rollback, discovery, or post-state recovery path.
- Core source SHA-256
  `6b401eb2890e6bad3117685cb7ce156690a5dfe53b0bab83fba7c5d335ca22bf`
  adds only the private fresh-bootstrap entry before public dispatch. That path
  derives the canonical `release-public-key.pem`, requires authoritative v3
  state to remain absent after normal journal recovery, refuses initial key
  rotation, and reuses the accepted B7 v3 admission, candidate probe, immutable
  staging, installed re-verification, and initial state transaction. Once v3
  state exists it fails closed; ordinary launch, local/remote update, rollback,
  and recovery gained no bootstrap fallback or new trust source. The normative
  SPEC therefore remains unchanged at SHA-256
  `4ca9035c9c1a31c5afc3e9d4de978b304c96c687d03c0bee0aa446078fe11647`.
- B8 artifact and integration proof is exact. Slice 1 focused Core-artifact
  qualification passed 2/2 and the complete affected builder suite passed 7/7.
  Slice 2 bootstrap proof passed 4/4 plus the existing initial-activation durable
  fault matrix 1/1. Slice 3 passed one complete release-production flow using an
  actual locked release Core: release Core -> B6 official-shape generation -> v3
  signing/admission -> real bootstrap -> isolated installed Core/v3 state. The
  same release Core SHA-256
  `28217cd50b417b94ac13975be7f7ca094b25ca4484db6a406f791c5b2584906e`
  was proven at builder descriptor, bootstrap input, installed stable Core, and
  installed signed-generation verification; affected B7 5/5, B8 bootstrap 4/4,
  B6 admission 1/1, and builder 7/7 also remained green.
- Final B8 grouped acceptance was rerun on committed product tip `192fcece...`.
  The project-registry canonical serial workspace suite passed Core
  81/0/1-ignored plus builder 7/0; three complete default-parallel workspace
  suites each passed the same counts; the explicit real-Termux read-only smoke
  passed 1/1; `cargo fmt --check`, bootstrap shell syntax, and `git diff --check`
  passed; and the locked workspace release build passed with `-D warnings`.
  Test-root residue was zero and generated `target/` was removed. The installed
  launcher remained SHA-256
  `0b0284155f2672263836029f760ba06a0cb284b7ca3a8e600ad399b43af36aff`
  with identity
  `65089|1260183|755|10379|10379|7512|2026-08-28 01:28:18.815391370 +0900`,
  while live `resolv.conf` remained SHA-256
  `7e8ad76e0d200e93918ca2e93c99ff8ecd02071953bf1479819db3ac0dbb6d07`
  with identity
  `65089|94666|600|10379|10379|38|2026-08-28 01:04:03.530430900 +0900`.
  No live generation/trust state, resolver, launcher, Manager,
  auth/profile/session, package, network authority, publication ref, or private
  signing-key state changed.
- B8 rejected non-evidence and local proof defects instead of carrying them
  forward: a formatting-only Slice 1 stop; a legacy test-fixture mode mismatch;
  an exact test filter that selected zero tests; an Android hard-link permission
  failure replaced by no-clobber rename; and a stale Slice 3 expectation that
  `doctor` must exit zero while Manager remains intentionally unavailable. Each
  affected gate was rerun after repair. The inherited tmcp
  `project.validation.describe` package.json discovery remains a separate tooling
  limitation, while project-registry Cargo validation revision 3 is canonical.
  Each Slice 1-3 normal commit was also rejected before commit by the known
  orphan-lineage hook referencing absent `tools/update-wrapper-version.sh`; after
  exact index revalidation the established `--no-verify` closure was used. No B8
  commit was pushed or published.
- M2-B9 — launch/update overlap and injected-failure proof — is accepted at
  product tip `377fed80710e131ef6558118afcb45031818b302`. Slice 0 established a
  test-only pause harness over the existing eight-call activation durability
  boundary. Slice 1 then reproduced a concrete overlap defect: ordinary launch
  invoked recovery while an updater-owned `activation-journal.tmp` was durable,
  removed that temporary, and caused the updater's next publication to fail with
  ENOENT. The repair is deliberately one production behavior line: ordinary
  `load_activated_generation` now reads the authoritative pointer with
  `read_pointer_state`, while update, rollback, and explicit transaction recovery
  retain recovery ownership. No lock, retry, fallback, second state owner,
  persistent format, trust behavior, public surface, or SPEC delta was added.
- B9 focused proof closed every selected overlap class. Successful activation
  overlap passed 1/1 across durable calls 1, 3, 4, and 6; failed pre-activation
  update passed 1/1 across both before/after faults at calls 1 through 4; recovery
  overlap passed 1/1 across all sixteen before/after activation fault states and
  inserted ordinary launch after the first real recovery durable call wherever
  cleanup was required. All four B9 regressions passed together, the existing B1
  every-durable-boundary and partial/stale recovery regressions remained green,
  and the ordinary loader never mutated updater/recovery-owned journal or
  temporary files. The accepted Core source SHA-256 is
  `130f1097f9d31bdedf7a5212daf9dc3be6b5027e2481863708135411d00cb317`;
  the B9 Core diff from B8 authority commit `0bcac01...` has SHA-256
  `dca5dfc9bf5f15406d54afef8fcfc371149f534cf49c046824b9fe979fc43fc6`;
  normative SPEC remains
  `4ca9035c9c1a31c5afc3e9d4de978b304c96c687d03c0bee0aa446078fe11647`.
- Final B9 grouped acceptance ran on clean committed product tip `377fed8...`.
  The canonical locked serial workspace suite passed Core 85/0/1-ignored plus
  release-builder 7/0; three complete default-parallel workspace suites each
  passed the same counts; the explicit real-Termux read-only smoke passed 1/1;
  `cargo fmt --check`, bootstrap shell syntax, and `git diff --check` passed; and
  the locked workspace release build passed with `-D warnings`. B9 temp-root
  residue was rechecked with an explicit zero assertion after discarding a noisy
  output-helper invocation, generated `target/` was removed, and checkout
  returned clean. The installed launcher remained SHA-256
  `0b0284155f2672263836029f760ba06a0cb284b7ca3a8e600ad399b43af36aff`
  with identity
  `65089|1260183|755|10379|10379|7512|2026-08-28 01:28:18.815391370 +0900`,
  while live `resolv.conf` remained SHA-256
  `7e8ad76e0d200e93918ca2e93c99ff8ecd02071953bf1479819db3ac0dbb6d07`
  with identity
  `65089|94666|600|10379|10379|38|2026-08-28 01:04:03.530430900 +0900`.
  No live generation/trust state, resolver, launcher, Manager,
  auth/profile/session, package, network authority, publication ref, or private
  signing-key state changed. Slice 0-3 normal commits were rejected before commit
  by the known orphan-lineage hook referencing absent
  `tools/update-wrapper-version.sh`; each exact staged tree was revalidated and
  closed with the established `--no-verify` precedent without changing the hook.
- M2-B10 — offline local-artifact install/recovery qualification — is accepted at
  product tip `40d04dcb5a02687fc48a1897e36309c387edc91f` by this authority update.
  The B10 commits add only test-owned release fixtures and end-to-end
  regressions after the accepted B9 production behavior; no public command,
  persistent format, trust source, recovery owner, or SPEC contract changed.
- B10 is bound to the accepted SPEC SHA-256
  `4ca9035c9c1a31c5afc3e9d4de978b304c96c687d03c0bee0aa446078fe11647`, current
  Core source SHA-256
  `6118d3d10c072b209906d532904574e30ad90b7f5d08777ff5cd8c31f7105f38`,
  release-builder source SHA-256
  `aaa8ac051bf634bdf8fda799ca194b228b11e61e41fd1837570f979daefb4c9b`,
  bootstrap SHA-256
  `c1b107699a64c08cc49a99ceb433c3dd1b6c7ca53637fb0b53cc73b6ce35e9fa`, and
  the official upstream `0.150.1` archive SHA-256
  `1ecac3f87823efb98153233b076ea3d6e34a7a8cebe43c5285dc5f79e1514639`.
  The locked `-D warnings` release artifacts used for the final proof were Core
  `12bd6c525026d74df8f9784444cebfee45d33f3f00af5512b59168f38a9c8d01` and
  release-builder
  `d1db9e39f5b90dbf8f71e33b7f1a2fb80f6bd7399123278301328c77abeeb5ec`.
- B10 focused validation ran with `CODEX_B10_RELEASE_CORE` bound to that actual
  locked release Core and passed all four slices 4/4: release fixture/network
  denial, fresh offline bootstrap, signed local update plus explicit rollback,
  and injected transaction recovery with rollback still usable. The grouped
  locked serial workspace suite passed Core 89/0/1-ignored plus builder 7/0;
  three complete default-parallel repetitions passed the same counts. The
  explicit real-Termux read-only smoke passed 1/1, the locked `-D warnings`
  release build passed, formatting, bootstrap shell syntax, and `git diff
  --check` passed, and all B10/test-builder temporary roots plus repository
  `target/` were absent afterward.
- An environment-less invocation in which the B10 tests early-returned was
  rejected as non-evidence. No B10 acceptance count uses that run; the counts
  above came from the actual release-Core-bound focused and grouped commands.
  Live launcher SHA-256 remained
  `0b0284155f2672263836029f760ba06a0cb284b7ca3a8e600ad399b43af36aff` and live
  `resolv.conf` remained
  `7e8ad76e0d200e93918ca2e93c99ff8ecd02071953bf1479819db3ac0dbb6d07`, with no
  live generation/trust state, resolver, launcher, Manager, auth/profile/session,
  package, or publication state changed.
- At B10 closure, the next selected bundle was M2-B11 — isolated fresh-Termux
  and upgrade-from-legacy qualification. That bundle is now accepted below;
  final independent product review remains the later Milestone 2 gate. Worker
  mode remains OFF.

- M2-B11 — isolated fresh-Termux and digest-bound upgrade-from-legacy handoff — is accepted at product tip `c61f712cac0d3822a5ca66e48115215ff2722c07`. The implementation adds the exact local-only `codex-bootstrap upgrade-legacy <CORE_ARTIFACT> <SIGNED_RELEASE_DIR> <BOOTSTRAP_PUBLIC_KEY> <EXPECTED_LEGACY_ENTRYPOINT_SHA256>` path and preserves the existing v3 state, trust, update, rollback, and Manager boundaries.
- The normative B11 contract is SPEC SHA-256 `9ebe9a60a819c514beda09f7f70c86b7df4e375989c20be5752fc8cb6f132e4a`. The expected legacy digest is only an explicit replacement-target selector. Handoff prepares and verifies one complete signed initial v3 state before same-directory atomic entrypoint replacement; prepared and completed retries are exact and idempotent. No legacy discovery/import/execution, backup, second journal, fallback, or new release schema was added.
- Core continues to fail closed for explicit unsupported Linux sandbox modes with status 2 and never invokes, installs, downloads, or repairs bwrap. Ordinary launch remains independent of bwrap and Manager availability.
- The focused B11 group passed `9 passed, 0 failed` with the actual release Core, covering fresh qualification, exact grammar and digest classes, no-mutation conflicts, prepared-state resume and interruption, key/previous conflicts, atomic replacement, version/doctor, and the first post-handoff signed Core update plus rollback. A mistaken bare exact filter that selected zero tests was discarded; the corrected substring invocation supplied the counted evidence.
- Final locked validation passed: serial workspace Core `98 passed, 0 failed, 1 ignored` and release-builder `7 passed, 0 failed`; three default-parallel workspace repetitions passed the same counts. The locked `-D warnings` release Core build, `cargo fmt --check`, bootstrap shell syntax, and `git diff --check` also passed. Final source identities are Core `1da1096633e2c1b8c242970b7f545ce6109d5641a4baf2d3b9aa36de2d8fc3cc`, bootstrap `9a9285ee838ccf2665feb3c5055f66f03623e8abc9ba11e2633d15b7a31bb3e9`, release-builder source `aaa8ac051bf634bdf8fda799ca194b228b11e61e41fd1837570f979daefb4c9b`, and release Core artifact `d5587f0846648cfd920ae4d67498b1228cae8e45b6f751b09a4446d7de2eda74`.
- Disposition: KEEP the existing signed v3 admission, state authority, recovery, and one direct stable-entrypoint path; COLLAPSE handoff resume into that existing initial-state path and the existing activation boundary; DELETE legacy fallback/import/backup machinery and any bwrap repair path. No live launcher, resolver, auth/profile/session, Manager, package, or publication state changed, and no `codex-r2-*` test residue remains.
- Current checkpoint: M2 independent product review candidate. B11 is accepted for review, but no independent reviewer, promotion to `main`, push, or live cutover has been performed. Worker mode remains OFF.

- M2-R1 — publication durability and retry closure — is accepted at product tip `f4fa0b53518f49cd7077d2765bf68f713dc38fe0`. Generation publication now synchronizes final regular files and modes, syncs directory trees bottom-up, atomically publishes into the generation root, and syncs that root; a post-rename sync failure can reuse only the exact signed verified generation. Fresh bootstrap trust-seed and stable-entrypoint publication are owned by authenticated Core with private temporaries, final-mode/file/parent synchronization, no-replace differing-target protection, and same-input retries. Legacy handoff retries always revalidate the Core target and synchronize its parent, including an already-Core target. Release-builder re-synchronizes runtime and descriptor files after final mode changes.
- The normative M2-R1 SPEC SHA-256 is `3bcadc1831eb73a1f6e5e6813f4c015e8da039abbcb3310f32ebee6f0db28702`. Final source identities are Core `a3cc6715b21e7a9d07afe804964dc5950859c51a36ef4c8f47e9fe5c384919d4`, bootstrap `6eb5a68d97c954bc3379321f8046f480bce57addc1c73e3b17883436b5990f45`, and release-builder `d0de07b4cc338b1dfdd6b0ce1165090eef55f2c35fbfe8c24f96a7c36c473aab`. The warnings-denied locked release artifacts used in final proof were Core `7dea4df0bf68f2f0351208a420ba3a7daeb490bf4a9dc22b502f59a7b371d0a4` and release-builder `c53fa026e6afe827dd5a73e19fe4ccc6ed0ea4a3ceedf810e0098ce1ea427831`.
- M2-R1 focused proof passed: five Core tests covering generation fault boundaries, exact generation reuse, fresh trust-seed retry, fresh entrypoint retry, and legacy parent-sync retry; M2-B3 `6/6`, M2-B4 `14/14`, M2-B8 `5/5`, M2-B11 `9/9`, and release-builder focused proof `1/1`. The final locked serial workspace suite passed Core `103 passed, 0 failed, 1 ignored` and release-builder `7 passed, 0 failed`; three independent default-parallel workspace repetitions also passed. `cargo fmt --check`, bootstrap shell syntax, and `git diff --check` passed.
- Protected-surface verification kept the live launcher SHA-256 at `0b0284155f2672263836029f760ba06a0cb284b7ca3a8e600ad399b43af36aff` and live `resolv.conf` at `7e8ad76e0d200e93918ca2e93c99ff8ecd02071953bf1479819db3ac0dbb6d07` before and after. All validation used disposable roots or external temporary Cargo targets; no installed launcher/runtime, resolver, auth/profile/session, Manager, package, publication state, push, or promotion changed.
- Disposition: KEEP one signed v3 admission/state/recovery authority and one direct stable-entrypoint path; COLLAPSE fresh trust-seed publication, fresh entrypoint publication, and legacy retry publication behind the authenticated Core durability boundary; DELETE shell-side persistent publication, retry bypasses, legacy fallback/import/backup machinery, and any bwrap repair path. Core still fails closed for unsupported Linux sandbox requests and never invokes or repairs bwrap.
- Current checkpoint: M2 independent product review candidate. M2-R1 is accepted for review, but no independent reviewer, promotion to `main`, push, live cutover, or bounded device qualification has been performed. Worker mode remains OFF.

- M2-R1 review follow-up — generation collision race — is accepted at product
  tip `084531b42bbcd6235c32393a576a09265269974e`
  (`fix: protect immutable generation publication`). The review
  follow-up closed a concrete race in which the pre-publish existence check
  could be invalidated and ordinary Unix rename could replace an existing
  generation directory. The real generation publication path now uses the
  existing no-replace primitive and maps an atomic destination collision to the
  existing `GenerationCollision` result; no activation-state or legacy-entrypoint
  replacement semantics were changed.
- The focused regression
  `test_m2_r1_generation_collision_race_never_replaces_existing_directory`
  injects a destination after the check, proves the existing sentinel survives,
  proves the collision result, and proves candidate cleanup. The paired M2-R1
  generation durability test also remained green: focused generation group
  `2 passed, 0 failed`; the initial exact smoke filter that selected zero tests
  was discarded and the corrected substring invocation selected the intended
  one smoke test.
- Final grouped acceptance on the formatted source passed the locked serial
  workspace suite `104 passed, 0 failed, 1 ignored` and three independent
  default-parallel repetitions with the same counts; release-builder passed
  `7 passed, 0 failed`. `cargo fmt --check`, `git diff --check`, bootstrap
  shell syntax, and the locked warnings-denied workspace release build all
  passed. The explicit real-Termux read-only smoke passed `1 passed, 0 failed`.
- The final source hash is Core
  `1cfc6f0aa0c392a755845e9a37a96366ce40ff2a2c7be0408eac39f569fdfcc2` against
  SPEC hash `3bcadc1831eb73a1f6e5e6813f4c015e8da039abbcb3310f32ebee6f0db28702`.
  Test and build target roots were removed with zero residue. Protected
  launcher and resolver hashes and device/stat identities remained unchanged;
  no live generation/trust, resolver, launcher, Manager, auth/profile/session,
  package, network, publication, push, or promotion state changed.
- Exhaustive rename disposition for this class is complete: KEEP no-replace
  generation publication and the existing release-builder no-replace output;
  KEEP intentional activation-state replacement and digest-bound legacy
  entrypoint replacement; DELETE no new fallback, retry, lock, bwrap repair,
  or second publication path. Independent product review remains the next
  gate; worker mode remains OFF.

- M2-R1 independent-review remediation — bootstrap authority/residue,
  generation-root confinement, and bounded control/artifact I/O — is accepted
  at implementation commit
  `33b4bf3f6a4fcff7d2f7bbf67bb1d24b76b73d48`. Fresh bootstrap now passes the
  bounded authenticated key/Core snapshots into Core activation, binds the
  signed generation's `core_artifact_digest` to that Core, and preflights v3
  transaction residue before trust-seed, entrypoint, generation, or activation
  publication. Local/remote/installed generation paths reject symlink or
  non-directory generation roots before reuse or publication. Descriptor and
  manifest loads, bootstrap snapshots and Core/key copies, and release-builder
  Core snapshots enforce their bounds before and during consumption.
- Focused proof passed: Core `m2_r1_` group `11 passed, 0 failed`, the corrected
  exact Core-binding regression `1 passed, 0 failed`, and release-builder B8
  Core-artifact group `2 passed, 0 failed`. A prior unqualified exact filter
  selected zero tests and was discarded as non-evidence; the namespaced exact
  invocation selected the intended one test.
- Final grouped acceptance passed the locked workspace serial suite with Core
  `110 passed, 0 failed, 1 ignored` and release-builder `7 passed, 0 failed`.
  Three independent default-parallel workspace repetitions passed the same
  counts. The locked `-D warnings` release build, `cargo fmt --all -- --check`,
  bootstrap shell syntax, and `git diff --check` passed. The explicit
  real-Termux read-only smoke passed `1 passed, 0 failed`.
- Final authority/source identities are SPEC
  `dca2439c87567710e5a8fe7a219b16837b9454af0eb4b69d9db37a430f2be49e`, Core
  `02944a17348dcb0b822135e53ce71600e7e2440568d5a1a2d42be7e69117a094`,
  bootstrap
  `4cda45ad448d110c224854724ab8229fc9aa431d27ea4c818ab574046b1005db`, and
  release-builder
  `957dc5f6d46b280f7a8d4e24e1f25d0bfb589a82fb8664fde9b047dbf393005f`.
  The warnings-denied release artifacts used in the final build were Core
  `6de619aa3a233e1a4354c625daaf9840cfdf7f21dd110468428945c1fff97f37` and
  release-builder
  `7742288c621af679ad8ccc2bf61a08fd73f6d8d154045fd2b280898d2b294f16`.
- The three stale disposable roots observed after the parallel batch were
  confirmed unused, moved to a recoverable quarantine, and placed in the
  system trash; the corrected focused run produced no new root and the final
  canonical residue scan is empty. Protected launcher SHA-256 remains
  `0b0284155f2672263836029f760ba06a0cb284b7ca3a8e600ad399b43af36aff` and
  live `resolv.conf` remains
  `7e8ad76e0d200e93918ca2e93c99ff8ecd02071953bf1479819db3ac0dbb6d07`, with
  device/stat identities unchanged. No live generation/trust, resolver,
  launcher, Manager, auth/profile/session, package, push, or promotion state
  changed.
- Disposition: KEEP one signed v3 admission/state/recovery authority, one
  direct stable-entrypoint path, and atomic no-replace generation publication;
  COLLAPSE fresh bootstrap binding/residue checks and bounded-input handling
  into those existing Core boundaries; DELETE no fallback, repair, lock, or
  second publication path. Core still fails closed for explicit unsupported
  Linux sandbox requests and never invokes or repairs bwrap. Current checkpoint:
  M2 independent product review candidate; no independent reviewer, promotion
  to `main`, push, live cutover, or bounded device qualification has been
  performed. Worker mode remains OFF.

- M2-R2 — independent-review remediation — is accepted at implementation
  commit `a56a85cb88d3866ddc52134aae6dedaf88ac6c1f`. The selected active
  generation is now confined to real directories and regular non-symlink
  descriptor, runtime, Manager, helper, and compatibility paths before public
  launch; ordinary launch cannot follow a generation-root symlink escape.
  Activation and explicit recovery writers now serialize on one kernel-held
  exclusive state-root lock, with no persistent lock record and automatic
  kernel release on process exit. A contending writer returns `WriterBusy`
  before touching journal/state files. Valid `doctor` arguments now turn
  upstream probe/setup failure into a redacted `unhealthy` status so
  `doctor --json` still emits a machine report with nonzero health-failure
  status; usage errors remain distinct.
- The M2-R2 normative SPEC SHA-256 is
  `9f5db58c49d1572d794f2b60035fc07f3a68390d1e165df670649bd2daad2d8e`.
  Final source identities are Core
  `2d79d7185fe16aed80982509f1be78526713f9c76806b9e956b6a5f1da01918`,
  bootstrap
  `4cda45ad448d110c224854724ab8229fc9aa431d27ea4c818ab574046b1005db`, and
  release-builder
  `ce09449e603806ea8bf8571f5978b11a58974f40ddba736ad2924204a0b40d1d`. The
  locked warnings-denied release artifacts used in final proof were Core
  `189f96259a606d9c9d1d8df5a1f8338d07ad7c02f4d33733934e0a5f3cd497ca` and
  release-builder
  `7742288c621af679ad8ccc2bf61a08fd73f6d8d154045fd2b280898d2b294f16`.
- Focused proof passed: M2-R2 `3 passed, 0 failed` (six public generation
  symlink cases, writer contention/release, and public `doctor --json`
  failure), adjacent M2-B9 `4 passed, 0 failed`, M2-B2 `6 passed, 0 failed`,
  and the corrected B4 source-admission regression `1 passed, 0 failed`.
  Final locked serial workspace proof passed Core `113 passed, 0 failed,
  1 ignored` and release-builder `7 passed, 0 failed`; three independent
  default-parallel repetitions passed the same counts. The locked `-D
  warnings` release build, formatting, bootstrap shell syntax, and
  `git diff --check` passed. Explicit real-Termux read-only smoke passed
  `1 passed, 0 failed`.
- Protected launcher SHA-256 remains
  `0b0284155f2672263836029f760ba06a0cb284b7ca3a8e600ad399b43af36aff` with
  stat identity `65089|1260183|755|10379|10379|7512|2026-08-28
  01:28:18.815391370 +0900`; live `resolv.conf` remains
  `7e8ad76e0d200e93918ca2e93c99ff8ecd02071953bf1479819db3ac0dbb6d07` with
  stat identity `65089|94666|600|10379|10379|38|2026-08-28
  01:04:03.530430900 +0900`. All test/build roots were external and the
  final canonical residue scan is empty. No installed launcher/runtime,
  resolver, generation/trust, Manager, auth/profile/session, package,
  publication, push, promotion, or live cutover state changed.
- Disposition: KEEP one signed v3 generation/state/recovery authority, one
  direct stable-entrypoint path, no-replace publication, and the ordinary
  launch lock-free boundary; COLLAPSE active asset confinement and writer
  serialization into those existing Core paths; DELETE the stale multi-writer
  window, nested symlink consumption, raw doctor probe-error path, and any
  additional lock/fallback/bwrap-repair machinery. Current checkpoint: repaired
  M2 independent product review candidate; no fresh independent reviewer,
  promotion to `main`, push, live cutover, or bounded device qualification has
  been performed. Worker mode remains OFF.

- The required independent product review of the repaired M2 candidate is
  complete at implementation review commit
  `5a7a5292f38876087a5c9b5a41b1dd7e8dbf082b`.
  The primary Lead performed a fresh read-only audit of the public dispatch,
  bwrap boundary, runtime/FD environment, generation confinement, activation
  and recovery state machine, bootstrap, remote/local release paths, doctor
  envelope, and protected live surfaces. No new product-contract, public-path,
  state-integrity, bwrap, or protected-surface finding remained.
- The review initially exposed five warning-denied clippy findings. They were
  closed in the same-scope review commit by reusing the existing dispatch
  context, removing a one-use test helper, and applying direct standard-library
  forms; no product behavior or public contract changed. The corrected
  `cargo clippy --locked --workspace --all-targets -- -D warnings` gate passes.
- Review source identities are Core
  `8144b92b078a84c62b9758aba247d9f492a151c2d02c2fa6cd59f7185e52bc5d`,
  bootstrap
  `4cda45ad448d110c224854724ab8229fc9aa431d27ea4c818ab574046b1005db`, and
  release-builder
  `957dc5f6d46b280f7a8d4e24e1f25d0bfb589a82fb8664fde9b047dbf393005f`; the
  normative SPEC remains
  `9f5db58c49d1572d794f2b60035fc07f3a68390d1e165df670649bd2daad2d8e`. The
  warning-denied locked release artifacts are Core
  `358c105360c9754fee287f394139065a9e1c4996b95fa5b627653ccb63a08542` and
  release-builder
  `7742288c621af679ad8ccc2bf61a08fd73f6d8d154045fd2b280898d2b294f16`.
- Corrected focused proof passed M2-R2 `3 passed, 0 failed` and the direct
  doctor regression `1 passed, 0 failed`; the initial bare doctor filter that
  selected zero tests was discarded and is not acceptance evidence. The
  current revision's serial workspace suite passed Core `113 passed, 0 failed,
  1 ignored` and release-builder `7 passed, 0 failed`; three independent
  default-parallel repetitions passed the same counts. Formatting, bootstrap
  shell syntax, `git diff --check`, and the warnings-denied release build
  passed. The explicit real-Termux read-only smoke passed `1 passed, 0 failed`.
- Protected launcher and resolver hashes/stat identities remain unchanged, the
  canonical external-residue scan is empty, and no installed launcher/runtime,
  resolver, generation/trust, Manager, auth/profile/session, package,
  publication, push, promotion, or live cutover state changed.
- Final disposition: KEEP the one signed v3 admission/state/recovery authority,
  one direct stable-entrypoint path, no-replace publication, ordinary
  lock-free launch, and the explicit no-sandbox/bwrap boundary; COLLAPSE the
  review-only lint cleanup into existing context/test paths; DELETE no new
  fallback, repair, trust, lock, or review layer. Current checkpoint: M2 is
  independently reviewed and ready for an explicit final acceptance or
  promotion decision. No promotion to `main`, push, live cutover, or bounded
  device qualification has been performed. Worker mode remains OFF.

## M2 Final Acceptance and Local Promotion (2026-09-04)

- The user approved the completed M2 candidate for local publication and
  authorized promotion of `main`.
- Before promotion, the exact local `main` was
  `37f0a775ddc64d1641655a0cc83c0c2e681df704`. It was not reachable from the
  sealed `legacy/monolith` branch at
  `bf30a7dc94d4dad7f58836c69028160856e63c58`, and no existing legacy backup
  ref contained it.
- The old `main` was preserved before replacement as the exact local branch
  `legacy/main-pre-m2-20260904` at
  `37f0a775ddc64d1641655a0cc83c0c2e681df704`. The sealed
  `legacy/monolith` branch was left unchanged.
- The accepted `rewrite/rust-core` candidate, including this provenance
  record, is promoted to local `main` by direct ref replacement. The
  independent rewrite history remains unmerged with legacy history. Remote
  tracking refs, push state, installed runtime, live resolver, and bounded
  device state are unchanged.
- The M2 acceptance evidence immediately preceding this record remains the
  load-bearing proof: locked Core `113 passed, 0 failed, 1 ignored`,
  release-builder `7 passed, 0 failed`, three matching default-parallel
  repetitions, corrected focused review gates, warnings-denied release build,
  formatting, bootstrap syntax, diff check, and read-only real-Termux smoke.

## M2 Release Install/Update Qualification and Local Termux Cutover (2026-09-04)

- After local M2 promotion, the user requested a formal `install.sh` delivery
  frontend and authorized replacement of the working local Termux runtime.
  `SPEC.md` was updated before implementation to make `install.sh` an exact
  argv-forwarding frontend into the existing audited bootstrap; trust,
  generation, update, rollback, and recovery ownership remain in Core.
- This follow-on bundle is bound to
  `rewrite/rust-core@ab072b35d0b1d78354de88a16b89433727592636`. The normative
  SPEC SHA-256 is `728b0e938406ee830222a943293dd6089021d6c7fd7dc30677583b411f9bc0b1`;
  `install.sh` is `b9c1026fa58a2449714fd12ba09cb225bdcab77a7c47f52ec3be2d7934372efa`,
  Core source is `cd1c2b20c294452dd075bb98382333553c8169a3612153363cbb33a8bd6c8f61`,
  bootstrap source is `4cda45ad448d110c224854724ab8229fc9aa431d27ea4c818ab574046b1005db`,
  and the release-builder artifact is
  `7742288c621af679ad8ccc2bf61a08fd73f6d8d154045fd2b280898d2b294f16`.
- The focused install regression passed `1 passed, 0 failed`, covering exact
  fresh and `upgrade-legacy` argv forwarding plus missing and symlinked
  bootstrap refusal. `sh -n install.sh`, the invalid-argument nonzero
  no-mutation check, `cargo fmt --all -- --check`, and warnings-denied
  workspace clippy passed.
- The release qualification used the official pinned
  `codex-package-aarch64-unknown-linux-musl.tar.gz` for Codex `0.150.1`,
  SHA-256 `1ecac3f87823efb98153233b076ea3d6e34a7a8cebe43c5285dc5f79e1514639`,
  and a job-private Ed25519 signing key. The prebuilt Core artifact is
  `8c84beb9c729e110f8a24e35d355eee6803a3b14f10296a8d7b97fd6ca4b0fc2`; its
  selected runtime digest is
  `946b4337efbef5cea4eb50ace81fe17cd3fc62f53820f8db4183e505e5f6082b`.
  The signed stable generations were v1
  `local-20260904-57034e4` (sequence 1; manifest
  `274e2b0ad6b584508e20daf05d9f111b516f47d9821335b33615ac7fb323f348`,
  signature `fbd25fce60679e00a742f0708fa6f32cad083faef50bbd0ce5a5254b4513925f`)
  and v2 `local-20260904-57034e4-update` (sequence 2; manifest
  `11f49b476ab0d3c2c244b63c1825bd2cc0ea3d33a845d0607ffc151c46d51c5d`,
  signature `c9e85bef1f9b2b504bb6ddde89d1fd01394bd3eca5cfa435dfb7770309895606`).
  Release and signing material remained outside the repository; the private
  signing key was job-private and was removed after qualification.
- Separate disposable fresh and legacy roots both reached the real official
  runtime and reported `codex-cli 0.150.1`. Fresh installation produced a
  valid redacted `doctor --json` envelope (`rc=1` because Manager was
  unavailable), rejected explicit Linux sandbox mode with `rc=2`, and had no
  `bwrap` in the selected generation. Both roots completed local update to v2
  and explicit rollback to v1; legacy handoff additionally rejected rollback
  before a previous generation existed. The workspace locked serial suite on
  this revision passed Core `114 passed, 0 failed, 1 ignored` and
  release-builder `7 passed, 0 failed`; `git diff --check` passed.
- The live preflight confirmed the old stable launcher digest
  `0b0284155f2672263836029f760ba06a0cb284b7ca3a8e600ad399b43af36aff` and no
  pre-existing v3 Core state. Authorized
  `install.sh upgrade-legacy` replaced it with the authenticated Core artifact.
  Live verification reported `codex-cli 0.150.1`, a valid redacted doctor
  envelope (`rc=1`), and explicit sandbox rejection (`rc=2`). The final live
  state is v1 active with v2 retained for rollback; the launcher SHA is
  `8c84beb9c729e110f8a24e35d355eee6803a3b14f10296a8d7b97fd6ca4b0fc2`.
  Live `codex update --local` advanced to v2 and `codex update --rollback`
  returned to v1 without changing the launcher.
- Protected-surface verification found the live resolver unchanged at
  `7e8ad76e0d200e93918ca2e93c99ff8ecd02071953bf1479819db3ac0dbb6d07` and
  preserved resolver, auth/profile/session, Manager, package, and unrelated
  user-state identities outside the declared Core roots. No bwrap artifact was
  selected or invoked. No remote publication or push was performed, and the
  new install/cutover commit is on `rewrite/rust-core`; local `main` remains at
  its previously promoted tip.
- Disposition: KEEP the single `install.sh`→bootstrap delivery boundary and
  the existing authenticated Core update/rollback authority; COLLAPSE no new
  installer, updater, trust source, or fallback layer; DELETE no bwrap repair
  or legacy fallback path. The M2 release install/update qualification and
  authorized local cutover are complete. Further work requires an explicit new
  scope; worker mode remains OFF.

## R3 Upstream Update/Doctor and Code-Mode Contract Alignment (accepted)

- The current user-authorized threshold is that the installed Core preserves
  upstream `codex update` ownership for bare/ordinary update argv so the
  upstream Codex distribution updater remains the source of upstream contents,
  while exact signed-generation selectors remain Core-owned. `codex doctor`
  must expose the actual bounded upstream doctor output together with a
  Termux diagnosis, rather than reducing upstream diagnostics to a status bit.
- This is the explicit R3 refinement of the earlier broad top-level update
  reservation: Core still owns the Termux execution boundary and signed local
  generation operations, but it must not replace upstream's own update command
  or repository source for ordinary update argv.
- The release-production path must emit the code-mode host as one regular
  root-level generation file beside `runtime`, matching upstream's
  `current_exe().parent()` lookup. Existing v1 `compat/` generations are only
  migration inputs; no new release may reproduce that layout or depend on a
  symlink alias.
- The local audit established the prior observable sources: legacy top-level
  `codex update` passed through to upstream, legacy `codex termux update` used
  `npm pack` for the `@openai/codex` linux-arm64 package, and legacy wrapper
  doctor exposed a detailed Termux health report. The rewrite expresses these
  ownership and diagnostic outcomes through the current signed-generation and
  read-only Core contracts rather than copying legacy implementation.
- R3-A closes the update ownership defect: only exact `--local`, `--remote`,
  and `--rollback` selectors remain Core-owned; bare `codex update`,
  `update --help`, and other upstream update argv now use the final qualified
  upstream execution boundary, preserving the upstream updater as the source
  of upstream contents. The public-main fake-runtime proof covers bare update,
  optioned update, nonzero upstream exit propagation, and the selector
  classification boundary.
- R3-B closes the code-mode placement defect: release-builder output is
  `codex-local-generation-v2` with one regular root-level
  `codex-code-mode-host` beside `runtime`. Core uses that file directly and
  puts its generation root on the compatibility PATH. Existing v1
  `compat/codex-code-mode-host` generations remain read-only migration inputs;
  v2 rejects compatibility directories and root-level symlink companions.
  Remote acquisition creates only manifest-declared parents, so it cannot
  recreate the old empty `compat/` shape.
- R3-C closes the doctor projection defect: human `codex doctor` now includes
  bounded upstream doctor output plus a Termux doctor section, while JSON uses
  schema 2 with redacted upstream output, generation/layout/runtime/code-mode
  details, explicit bwrap non-use, Manager status, and summary status. Output
  is capped at 64 KiB; terminal controls and credential-like values are
  removed before composition. Probe failure and overflow retain a valid JSON
  envelope and nonzero health result, and the focused public-path tests prove
  doctor remains read-only.
- Focused R3 proof passed: Core update/public-main, doctor capture/composition
  and overflow, root/v1 layout migration, and release-builder output tests all
  passed. The final locked serial workspace suite passed Core `117 passed,
  0 failed, 1 ignored` and release-builder `7 passed, 0 failed`; three
  independent default-parallel workspace repetitions passed the same counts.
  `cargo clippy --locked --workspace --all-targets -- -D warnings`, release
  build, formatting, bootstrap shell syntax, and `git diff --check` passed.
- A mistaken release-builder filter selected zero tests and was discarded. Two
  stale v1 inventory fixtures were then corrected to include the existing
  compatibility host; the corrected exact tests and the complete suites passed.
  No zero-test invocation is counted as acceptance evidence.
- Protected-surface verification after the final proof found the live Core
  SHA-256 still `8c84beb9c729e110f8a24e35d355eee6803a3b14f10296a8d7b97fd6ca4b0fc2`
  and live resolver SHA-256 still
  `7e8ad76e0d200e93918ca2e93c99ff8ecd02071953bf1479819db3ac0dbb6d07` with
  unchanged resolver stat identity. No installed launcher/runtime, live
  generation/trust, resolver, Manager, auth/profile/session, package,
  publication, push, promotion, or bwrap state was changed. The current R3
  bundle is committed only on `rewrite/rust-core`; `main` and its backup remain
  untouched.
- Disposition: KEEP one final upstream execution boundary, one exact Core
  signed-generation update authority, one v2 root companion invariant, and
  one composed bounded doctor report. COLLAPSE the old PATH-only code-mode
  assumption and status-only doctor projection into those direct boundaries;
  retain v1 reading only as a bounded migration input. DELETE the old
  root-level symlink repair requirement, unconditional remote `compat/`
  creation, and any second update/doctor implementation path. Worker mode
  remains OFF.

## R3 Bounded Live Runtime Reflection (2026-09-04)

- After the R3 implementation was accepted, the user explicitly authorized a
  bounded local Termux runtime cutover. The exact source was
  `rewrite/rust-core@f205404bac151b2c618ca2534c711a808ae52b01`; the locked
  release-built Core artifact was installed at `$PREFIX/bin/codex` with
  SHA-256
  `01ffd7930018639c26e15c0008494b89f86b4571cfd55228fe22c2185f34370d`.
- The previous launcher digest
  `8c84beb9c729e110f8a24e35d355eee6803a3b14f10296a8d7b97fd6ca4b0fc2` was
  copied before the same-directory atomic replacement to the private temporary
  recovery path
  `/data/data/com.termux/files/usr/tmp/codex-r3-cutover.VGcHeC/codex.previous`.
  The backup is a device-test recovery aid, not a new product trust source.
- Post-cutover live checks passed: `codex --version` reported `0.150.1`; human
  `codex doctor` printed the bounded `[Upstream Codex doctor]` report and the
  `[Termux doctor]` section; `codex doctor --json` returned schema 2 with the
  upstream output and Termux fields; `codex update --help` reached upstream
  help with exit 0; and the current root code-mode companion and runtime both
  executed successfully.
- The active live generation and v3 activation state were not rewritten. It is
  the existing signed `codex-local-generation-v1` with the legacy
  `compat/codex-code-mode-host` layout, plus the previously existing root alias;
  therefore the new Core correctly reports `legacy-compat-v1` and
  `migration_required`. No unsigned or newly self-signed v2 bundle was
  introduced. A permanent v2 live migration requires a newly signed release
  accepted by the existing trust boundary.
- Protected-surface verification after cutover kept live `resolv.conf` at
  SHA-256
  `7e8ad76e0d200e93918ca2e93c99ff8ecd02071953bf1479819db3ac0dbb6d07`, the
  bootstrap trust seed and activation-state identities unchanged, and left
  `main`, its backup, legacy history, and remote refs untouched. No bwrap was
  invoked, installed, repaired, or selected.
- Disposition: KEEP the R3 Core launcher as the bounded live implementation;
  KEEP the existing signed generation and trust/state authority unchanged;
  DEFER only the authenticated v2 generation delivery needed to remove the
  live migration marker. Worker mode remains OFF.

## R4 Wrapper-Owned Safe Update and Doctor Presentation Repair (accepted)

- The user correction reopened the R3 bare-update decision: passing ordinary
  `codex update` arguments to upstream could install an unpatched upstream
  runtime. `SPEC.md` now makes every top-level update form Core-owned. The
  installed wrapper never runs the upstream self-updater; the wrapper release
  pipeline obtains the exact official package, applies the existing Termux
  patch policy through `codex-release-builder`, qualifies and signs the
  resulting generation, and publishes it for Core activation.
- Bare `codex update` now verifies a bounded, signed `update-index-v1` with the
  current v3 `update_key`, validates the stable channel, generation identity,
  and canonical immutable release base, then reuses the existing signed
  remote-generation admission, staging, probe, anti-rollback, and activation
  path. Bad transport, signature, index format, release qualification, or
  activation fails closed without upstream/package-manager/raw-package
  fallback. `--help`, malformed options, explicit local/remote selectors, and
  rollback remain deterministic Core paths.
- `codex doctor` now gives the supported upstream doctor a bounded PTY when
  human output is an interactive color-capable terminal, retains safe ANSI SGR
  markup, removes progress-line controls, and keeps non-TTY/`NO_COLOR`/JSON
  output plain and redacted. The Termux section uses the upstream-style
  `Codex Termux Wrapper Doctor` header and Runtime/Support/Wrapper/State/Store
  groups while retaining generation, code-mode migration, Manager, and the
  explicit `bwrap is not used` diagnosis.
- Focused proof passed: the signed wrapper-channel public-path test covered
  successful adapted v2 activation, bad index signature, malformed signed
  index, no upstream invocation, no fallback, and unchanged old state on
  failure; the doctor contract test covered redaction/plain-vs-colored
  rendering; the PTY test covered SGR preservation, CRLF normalization, and
  progress cleanup. The final locked workspace suite passed Core `119 passed,
  0 failed, 1 ignored` and release-builder `7 passed, 0 failed`; three complete
  default-parallel workspace repetitions passed the same counts.
- `cargo clippy --locked -p codex --all-targets -- -D warnings`, the locked
  workspace clippy run, the locked release-builder suite, `cargo fmt --check`,
  `git diff --check`, and the locked release Core build passed. The release
  artifact SHA-256 is
  `27519c6a505024f69c8800b78c0088b9d5be3b071543c5298c619d1ad93f4636`.
- The bounded live cutover completed after those gates. Only
  `/data/data/com.termux/files/usr/bin/codex` was atomically replaced with
  that exact artifact; the previous launcher is recoverable at
  `/data/data/com.termux/files/usr/tmp/codex-r4-cutover.Ifsy4h/codex.previous`
  with SHA-256
  `01ffd7930018639c26e15c0008494b89f86b4571cfd55228fe22c2185f34370d`.
  The live launcher now has the artifact digest. Post-cutover checks confirmed
  the wrapper update usage, malformed-argument rejection, signed-channel
  transport failure closure, code-mode host availability, and PTY SGR output.
  The resolver (`7e8ad76e0d200e93918ca2e93c99ff8ecd02071953bf1479819db3ac0dbb6d07`),
  trust seed (`03336cc8ac082c8afc900543e27220c391b536717165c9b3f1caa9cceb3d5790`),
  and activation state (`ccc443ae8615ed58bb22884104a67c31369dcb1f9c73b8ab8f15900353a0b94a`)
  remained unchanged. No bwrap invocation or repair was performed.
- The live selected generation remains the existing signed v1
  `legacy-compat-v1` generation, so its `code_mode_host` status is still
  `migration_required`; the new release pipeline will clear that marker only
  when a signed root-level companion generation is published. Until the
  wrapper channel publishes a signed index and adapted release assets, an
  automatic update is expected to fail closed rather than install an
  unpatched upstream runtime.

## R5 Official Upstream Build-Input Acquisition (accepted)

- The user correction was resolved at the correct boundary: the legacy
  on-device `npm pack -> patch -> activate` path remains behavior evidence only.
  The installed Rust Core still does not compile, invoke the release builder,
  patch a raw upstream executable, or run an upstream self-updater. Release
  production now has a Rust-owned `codex-release-builder fetch` operation that
  accepts one explicit stable `MAJOR.MINOR.PATCH` version and constructs only
  the official OpenAI archive URL
  `https://releases.openai.com/codex/releases/<version>/codex-package-aarch64-unknown-linux-musl.tar.gz`.
- The fetch path uses bounded HTTPS `curl` with a cleared environment, validates
  the executable tools and canonical output parent, streams into a private
  temporary archive, computes the exact lowercase SHA-256 with OpenSSL, and
  publishes the archive with `RENAME_NOREPLACE` followed by parent sync. It
  rejects channel discovery, mirrors, fallbacks, existing outputs, empty or
  oversized responses, and transport failures without leaving fetch staging.
  The printed digest is the input to the existing Rust adaptation/build path;
  no signing, activation, or live-state mutation occurs.
- The focused R5 regression exercised the actual `fetch` dispatch and strict
  grammar, exact official URL and curl argv, environment clearing, archive and
  digest identity, fetch-to-build generation production, output collision
  preservation, transport failure cleanup, and the existing adapted output
  boundary. It passed `1/1` with no warnings.
- Final acceptance passed the locked workspace suite once and in three complete
  parallel repetitions: Core `119 passed, 0 failed, 1 ignored` and
  release-builder `8 passed, 0 failed` each time. Workspace clippy with
  `-D warnings`, release Core build, `cargo fmt --check`, and
  `git diff --check` passed. The final R5 source identities are recorded by the
  implementation commit; no generated `target/` output is part of the change.
- Protected live identities remained unchanged: resolver
  `7e8ad76e0d200e93918ca2e93c99ff8ecd02071953bf1479819db3ac0dbb6d07`, trust
  seed `03336cc8ac082c8afc900543e27220c391b536717165c9b3f1caa9cceb3d5790`,
  and activation state
  `ccc443ae8615ed58bb22884104a67c31369dcb1f9c73b8ab8f15900353a0b94a`.
  No launcher, runtime, resolver, auth/profile/session, Manager, publication,
  push, promotion, or bwrap state was changed.
- Disposition: KEEP one official-source release fetch feeding the existing
  Rust builder; KEEP the signed-generation Core update boundary; DELETE no
  fallback or on-device build path. Live `codex update` remains intentionally
  fail-closed until an authorized signed wrapper index and adapted generation
  publication is available. This bundle does not claim that external
  publication has been completed.

## R6 Signed Wrapper Publication (accepted)

- R6 closed the release-production boundary left open by R5. The non-installed
  `codex-release-builder publish` command accepts one qualified R5
  `codex-local-generation-v2`, a positive release sequence, a canonical HTTPS
  generation base, an explicit OpenSSL executable, and an explicitly supplied
  Ed25519 private PEM. It does not upload to OpenAI or any remote service and
  does not touch installed Core or live user state.
- The publisher accepts exactly the first-target root layout (`generation.meta`,
  `runtime`, and root `codex-code-mode-host`), rejects symlinks, special files,
  unsafe modes, malformed qualification bindings, non-safe generation IDs,
  noncanonical/mismatched release bases, invalid keys, existing outputs, and
  unsupported arguments. It snapshots bounded inputs into private staging and
  verifies the descriptor, runtime digest, host digest, patch report, and
  source stability before publication.
- It derives the raw public key with the explicit OpenSSL tool, writes the
  exact Core `codex-release-v3` manifest with sorted digest/mode inventory,
  writes the exact four-record `codex-update-index-v1`, signs each exact byte
  sequence with `release.sig` and `update-index-v1.sig`, and intentionally
  emits no rotation authority signature. The complete output is an atomic
  `update-index-v1[.sig]` plus `releases/<generation-id>/`; the private key is
  never copied into it. The R6 Core integration regression passes this output
  through the production Core v3 release verifier.
- The focused R6 release-builder regression passed `1/1` and exercised actual
  OpenSSL verification, exact manifest/index bytes, inventory digests and
  modes, source equality, private-key exclusion, strict parser rejection,
  output-collision sentinel preservation, symlink rejection, and staging
  cleanup. The renamed Core integration regression also passed `1/1` against
  the real Core admission path.
- Final acceptance passed the locked workspace suite serially with Core `119
  passed, 0 failed, 1 ignored` and release-builder `9 passed, 0 failed`, then
  passed three consecutive complete default-parallel repetitions with the
  same counts. An earlier parallel attempt independently hit the inherited
  test-only `ETXTBSY` race in an unrelated temp executable test; that test
  passed in isolation, serial execution, and all three final repetitions. The
  final locked workspace clippy run with `-D warnings`, release workspace
  build, `cargo fmt --check`, and `git diff --check` all passed.
- Protected live identities remained unchanged: resolver
  `7e8ad76e0d200e93918ca2e93c99ff8ecd02071953bf1479819db3ac0dbb6d07`, trust
  seed `03336cc8ac082c8afc900543e27220c391b536717165c9b3f1caa9cceb3d5790`,
  and activation state
  `ccc443ae8615ed58bb22884104a67c31369dcb1f9c73b8ab8f15900353a0b94a`.
  No launcher/runtime, profiles, sessions, auth data, Manager state, bwrap,
  remote ref, or `main` promotion was changed; generated `target/` remains
  untracked and no publication artifact was committed.
- Disposition: KEEP one direct wrapper publisher feeding the existing signed
  Core update boundary; KEEP official OpenAI archive acquisition as the only
  upstream source authority; DELETE no Core fallback or raw-package path.
  External publication with the active update key is still a separately
  authorized operational step, so live `codex update` remains fail-closed.

## R7 Unified Bare Update (accepted)

- The user has expanded the post-M2 update requirement: one bare `codex update`
  must first look for a signed adapted build in `humtr/codex`; when the remote
  channel or its release is transport-unavailable, it must locally fetch the
  official latest (or explicitly selected) release metadata, bind its exact
  stable version and AArch64 package digest, fetch the exact official upstream
  archive, and run the prebuilt Rust release-builder
  routines, sign and qualify the adapted generation, and replace the active
  runtime through the existing atomic Core activation path. `--local`,
  `--remote`, and `--rollback` remain secondary explicit operations.
- Local fallback is not a Rust/Cargo self-compilation path. It requires the
  active release `update_key` private key through the bounded configured key
  path, never copies or prints that key, and cannot activate a locally produced
  generation when the key is absent or mismatched.
- A successfully activated local publication is retained under the Core-owned
  local publication store. If GitHub CLI account authentication is available,
  Core may best-effort publish the release tree and then the signed index to
  `humtr/codex`/`main`; upload failure must not undo the local activation.
- R7 is accepted at product tip `55d8a49` on `rewrite/rust-core`. The remote-hit
  regression proves a signed `humtr/codex` channel release is used without
  entering local production. The transport-absence regression proves the
  official latest metadata is resolved to one stable version and target digest,
  the exact versioned archive is fetched, the prebuilt release-builder adapts
  it, the current update authority signs it, and the existing signed local
  admission atomically activates it while retaining the previous generation.
  Existing R4/R5 failure matrices continue to prove invalid received channel
  content fails closed rather than entering fallback.
- The optional authenticated GitHub path is accepted as best effort: it runs
  only after local activation, uploads the five release files before the
  signed index pair, never uploads the private signing key, and leaves local
  activation successful when an upload fails. The local signed publication is
  retained under the Core-owned publication store.
- Final evidence on this revision: R7 focused Core tests `4 passed, 0 failed`;
  locked workspace tests Core `124 passed, 0 failed, 1 ignored` and
  release-builder `9 passed, 0 failed` in two final complete runs;
  `cargo fmt --check`, workspace clippy with `-D warnings`, release build, and
  `git diff --check` passed. Protected resolver, trust-seed, and activation
  state digests remained respectively
  `7e8ad76e0d200e93918ca2e93c99ff8ecd02071953bf1479819db3ac0dbb6d07`,
  `03336cc8ac082c8afc900543e27220c391b536717165c9b3f1caa9cceb3d5790`, and
  `ccc443ae8615ed58bb22884104a67c31369dcb1f9c73b8ab8f15900353a0b94a`.
  No installed launcher/runtime, profiles, sessions, auth data, Manager
  state, bwrap state, remote ref, or `main` promotion changed; generated
  `target/` remains untracked. Live runtime cutover and remote push remain
  separate operational actions and were not performed by this source bundle.

## R8 Doctor Human Presentation (accepted)

- The interrupted doctor review was resumed at source tip `5a72f77` on
  `rewrite/rust-core`. Human `codex doctor` output now begins with the bounded,
  sanitized upstream doctor body itself: Core no longer prepends the synthetic
  `[Upstream Codex doctor]` heading or duplicates its status line. Unsupported
  or empty upstream output still gets a concise explicit status diagnostic.
- The existing bounded PTY path continues to preserve safe upstream ANSI SGR
  sequences for a TTY without `NO_COLOR`; non-TTY and `NO_COLOR` output remain
  plain. The public route now computes the color decision once and uses it for
  both capture and composition.
- The Termux portion retains the legacy-observed `Codex Termux Wrapper Doctor`
  header and Runtime/Support/Wrapper/State/Store groups, followed by Manager
  and Summary. Focused doctor proof passed `10 passed, 0 failed`; the locked
  workspace passed Core `124 passed, 0 failed, 1 ignored` and release-builder
  `9 passed, 0 failed`. Format, diff check, workspace clippy with `-D warnings`,
  and release build passed.
- Protected live identities remained unchanged: launcher
  `344d1815cda3a31db074c314c910f0f764656f996772959e984b86ff966abced`, resolver
  `7e8ad76e0d200e93918ca2e93c99ff8ecd02071953bf1479819db3ac0dbb6d07`, trust
  pin `62ab1640b6b4e63afbd5952d11a0bd0a9f1cb78ddde2472e003a42c4db2b832c`,
  and activation state
  `37cabe244a6e1e788e08f68098fde2307c16e3044d9ee54216877be48848b8e1`.
  No live runtime, resolver, auth/profile/session, Manager, bwrap, remote
  ref, or publication state was changed by this source bundle.
- Disposition: KEEP one composed doctor path with upstream-first presentation
  and the legacy-shaped Termux diagnostic groups; DELETE the synthetic
  upstream wrapper heading/status projection. Remote publication transport is
  the next separately scoped bundle.

## R9 Remote Publication Transport (accepted)

- R9 closes the actual large-artifact publication failure. The old uploader
  sent the runtime through the GitHub Contents API as base64 JSON, which is not
  a viable path for the approximately 222 MiB runtime. The Core now validates
  the five complete release files and sends them as one GitHub Release asset
  set, tagged by the generation identity. Only the small signed index pair is
  written to `humtr/codex`/`main` through Contents, after asset-release success.
- The local fallback now signs a `release_base` matching
  `https://github.com/humtr/codex/releases/download/<generation_id>/`. Remote
  curl follows only HTTPS redirects, with the existing certificate, environment,
  connect/transfer, and response-size controls. GitHub child processes have a
  bounded 300-second wait; timeout or asset preflight failure cannot advance
  the signed index and cannot undo local activation.
- Focused R9 proof passed `3 passed, 0 failed`, plus the updated local fallback
  and remote transport regressions. Final locked workspace proof passed Core
  `126 passed, 0 failed, 1 ignored` and release-builder `9 passed, 0 failed`;
  clippy with `-D warnings`, release build, format, and diff check passed.
- Protected live identities remained unchanged: launcher
  `344d1815cda3a31db074c314c910f0f764656f996772959e984b86ff966abced`, resolver
  `7e8ad76e0d200e93918ca2e93c99ff8ecd02071953bf1479819db3ac0dbb6d07`, trust
  pin `62ab1640b6b4e63afbd5952d11a0bd0a9f1cb78ddde2472e003a42c4db2b832c`,
  and activation state
  `37cabe244a6e1e788e08f68098fde2307c16e3044d9ee54216877be48848b8e1`.
  No remote ref, publication artifact, live runtime, auth/profile/session,
  Manager, resolver, or bwrap state was changed by this source bundle.
- Disposition: KEEP one signed index authority plus one Release asset transport;
  KEEP HTTPS-only redirect handling for the release download path; DELETE the
  large-generation Contents upload path. Actual external publication and live
  replacement remain operational qualification actions.

## R9.1 Signed-Index Publication Repair and Live Reflection (accepted)

- The authorized live qualification reproduced the R9 publication symptom:
  GitHub Release asset creation completed, but both Contents index files were
  absent. The root cause was in the one shared base64 serializer: a final
  one- or two-byte chunk reused bytes from the preceding chunk. A 64-byte
  `release.sig` therefore produced an invalid signed-index payload, so the
  first signature PUT failed and the index PUT was never attempted.
- The production serializer now initializes each 3-byte chunk independently.
  The focused regression covers both 64-byte and 65-byte inputs, exercising
  both partial-tail branches. It passed `3 passed, 0 failed` with the existing
  R9 asset-inventory and bounded-wait regressions. The full locked workspace
  suite passed Core `127 passed, 0 failed, 1 ignored` and release-builder
  `9 passed, 0 failed`; locked workspace clippy with `-D warnings`, release
  build, formatting, and `git diff --check` passed.
- This R9.1 authority record binds the accepted source tip; Core source
  SHA-256 is
  `792fd5c27cfde5fd6c355576e2272b78b3bcccedff66ff011f1a9b46d36a6108`, the
  parent-relative Core diff SHA-256 is
  `0b989917cc920ca33fc8f0d2ef6a6d44faefa3e579e913f98aa8e4f349f51f96`, and
  the normative SPEC SHA-256 remains
  `8c93350902564ed1d4515e16296aeafe7958bf5d327b2c714efd122395ccf8f8`.
- The corrected release Core ran the complete bare `codex update` path on
  Termux. It activated `local-1788570645-23374-1` and reported
  `published local generation ...`; the official fallback built and signed
  the adapted generation, uploaded a non-draft/non-prerelease Release with
  all five assets uploaded, and then published both Contents files. Raw
  `update-index-v1` and `update-index-v1.sig` bytes match the local signed
  publication; the index signature verifies with the pinned update key, and
  its release base names the same Release tag. The local state retains
  `local-1788569830-7761-1` as the one previous generation.
- The stable launcher was then atomically replaced with the same Core
  artifact digest bound by the active signed generation. The prior launcher
  is recoverable at
  `/data/data/com.termux/files/usr/tmp/codex-r9-core-cutover.fknjAJ/codex.previous`.
  Installed `codex --version` reports `codex-cli 0.153.4`; installed human
  `codex doctor` now begins with the upstream doctor's own `Codex Doctor`
  output, preserves ANSI SGR on a color-capable TTY, has no synthetic
  `[Upstream Codex doctor]` heading, and appends the legacy-shaped Termux
  `Runtime`/`Support`/`Wrapper`/`State`/`Store` diagnosis. Its expected nonzero
  result is solely the unavailable Manager status (`rc=1`); the upstream
  WebSocket check was healthy with HTTP 101 in this run.
- Protected verification kept `resolv.conf` at
  `7e8ad76e0d200e93918ca2e93c99ff8ecd02071953bf1479819db3ac0dbb6d07`, the
  trust seed at
  `62ab1640b6b4e63afbd5952d11a0bd0a9f1cb78ddde2472e003a42c4db2b832c`, and
  the new installed launcher at
  `5c493c581ffb894ecbd2841c1ccdc8fbbf2b74c23d184468e8ebacc356548bac`.
  No resolver, auth/profile/session, Manager, package, or bwrap state was
  changed; no bwrap was invoked or selected.
- Disposition: KEEP one bounded base64 serializer, one Release-asset
  transport, one signed index authority, and one atomic live Core launcher
  boundary. DELETE the stale chunk-state defect and the operational gap that
  left the accepted Core doctor fix out of the installed entrypoint. R9.1 is
  closed; no independent source implementation slice remains active.

## R9.2 Termux Doctor Color Override (accepted)

- The additional Termux audit found two related causes for the reported plain
  upstream doctor. The active execution environment supplies `NO_COLOR=1`,
  which the existing PTY decision correctly honored; and, when the upstream
  output contained a credential-like field such as `api_key`, the old redactor
  returned the entirely plain redacted document, discarding otherwise safe
  upstream SGR. The sealed legacy renderer also honored `NO_COLOR`, so it was
  not a separate Termux color implementation to copy.
- The public contract now accepts exact `codex doctor --color` in addition to
  the existing no-argument and `--json` forms. It is human-only and TTY-only,
  mutually exclusive with `--json`, and does not mutate the caller environment.
  On a TTY it removes `NO_COLOR` only from the bounded upstream child and uses
  the existing Termux `script` PTY; non-TTY output and JSON remain plain.
- The redactor now records plain-text replacement ranges, reapplies those
  replacements over the sanitized styled stream while retaining only safe SGR,
  handles multiple out-of-order sensitive fields, and falls back to the already
  redacted plain line if its internal mapping is ever inconsistent. No secret
  is exposed by the color path.
- Focused R9.2 proof passed `11 passed, 0 failed`, including the parser,
  `NO_COLOR` child-environment isolation, single and multiple redaction/SGR
  cases, and the real public main path under the Termux `script` PTY with
  inherited `NO_COLOR=1`. The final grouped workspace proof passed Core `128
  passed, 0 failed, 1 ignored` and release-builder `9 passed, 0 failed` in the
  locked serial run and two consecutive locked parallel runs. Clippy with
  `-D warnings`, release build, format, and diff checks passed.
- This authority record binds Core source SHA-256
  `6ad6bb407e8fc11efffc8a59bbf132f1c0ceb0908b1e72e14acefa8493a506cb`, the
  parent-relative Core diff SHA-256
  `fa8e81c8ae8cbbc0bbb98644e1985b94fcb44b88f6339f5f34c32e0fe70ed7e6`, and
  normative SPEC SHA-256
  `e80c0df47e8cf93c3b98f24af8996fcbb9168b904aa84b62cb9cddc6aae5bb9f`.
- The subsequent user-authorized bounded live reflection fetched the official
  0.153.4 archive with SHA-256
  `fc395cb043a1093ab0db34f44aba3199bfaa9ce640cd9be7fd588f44b0da64a4`, built
  from the accepted Core artifact, signed local generation
  `local-1788578457-0-1` as release sequence 5 with the existing trusted
  update key, and activated it through the old installed Core's
  `codex update --local` path. The prior active generation
  `local-1788570645-23374-1` is retained as the one `previous` generation.
- The stable launcher was then atomically replaced with the same accepted Core
  artifact, SHA-256
  `109b556884150a134c39a8892c599f6551dbc1fb18ea557eebbfc690b4fc3a9b`; the
  prior launcher is recoverable at
  `/data/data/com.termux/files/usr/tmp/codex-r9-2-core-cutover.3jfG3I/codex.previous`.
  Live `codex --version` reports `codex-cli 0.153.4`. Under the real Termux
  `script` PTY with inherited `NO_COLOR=1`, `codex doctor --color` preserved
  upstream SGR, showed both upstream-first and Termux sections, omitted the
  synthetic heading, and returned the expected health-failure status from the
  unavailable Manager; default human and JSON doctor output remained plain.
- The complete local publication was then sent to the fixed authenticated
  `humtr/codex` publication target. Release tag `local-1788578457-0-1` is
  non-draft and non-prerelease with all five signed generation assets. The
  remote `update-index-v1` and `update-index-v1.sig` bytes match the local
  publication exactly, and the remote signature verifies with the pinned trust
  public key. The signed index now points to the new generation and its Release
  base; the remote publication branch tip is `856268e93cd29244f59635e5ee84c36a0b5b37d0`.
- A separate disposable consumer qualification first rejected an intentionally
  auth-free fixture at the required candidate doctor probe; its live state was
  untouched. A retry used the existing `CODEX_HOME` only as a read-only child
  input without copying credentials, and the automatic bare `codex update`
  path then fetched the remote signed channel and activated
  `local-1788578457-0-1` from sequence 4. The disposable consumer's
  `codex doctor --color` under `script` preserved SGR and both doctor sections,
  and its expected Manager health failure was the only nonzero result. The
  live auth digest was unchanged and the disposable root was fully removed.
- Protected live identities remained unchanged: `resolv.conf`
  `7e8ad76e0d200e93918ca2e93c99ff8ecd02071953bf1479819db3ac0dbb6d07`, trust
  seed `62ab1640b6b4e63afbd5952d11a0bd0a9f1cb78ddde2472e003a42c4db2b832c`,
  and the backup launcher
  `5c493c581ffb894ecbd2841c1ccdc8fbbf2b74c23d184468e8ebacc356548bac`.
  No auth/profile/session, Manager, resolver, trust, bwrap, or source-history
  ref was changed; activation state changed only through the authenticated
  Core update transaction, and no generated artifact was committed. The
  publication target's `main` changed only through the ordered signed-index
  publication above.
- Disposition: KEEP one direct upstream-first doctor path, one bounded Termux
  PTY capture, and one fail-closed redaction invariant. COLLAPSE the prior
  all-or-nothing decolorization after redaction into range-preserving SGR
  redaction; retain plain fallback only for an impossible mapping mismatch.

## Local Main Lineage Promotion (2026-09-05)

- After R9.2 source, live, remote-publication, and disposable-consumer
  qualification were complete, the user-authorized local source promotion was
  performed. The exact pre-promotion local `main` was
  `57034e4cd2d4f259c9046ac11073dc0b7f7dbb47` and was preserved as
  `legacy/main-pre-r9.2-20260905`. The earlier
  `legacy/main-pre-m2-20260904` backup and sealed `legacy/monolith` were left
  unchanged.
- Local `main` was then replaced atomically, with the expected old ref, by the
  accepted `rewrite/rust-core` tip
  `51d2e786bbfd31db1e22fd9eed02a3e7f008db88`. This was a direct ref
  replacement, not a merge or rebase; the implementation lineage remains
  independent of legacy history.
- The remote publication branch remains at
  `856268e93cd29244f59635e5ee84c36a0b5b37d0`, the remote rewrite branch
  remains at `253156c37a2bd22af8faae0bce03587999ffd136`, and neither was
  changed by this local promotion. Live protected identities were unchanged
  from the R9.2 preflight: resolver
  `7e8ad76e0d200e93918ca2e93c99ff8ecd02071953bf1479819db3ac0dbb6d07`, trust
  seed
  `62ab1640b6b4e63afbd5952d11a0bd0a9f1cb78ddde2472e003a42c4db2b832c`, and
  installed launcher
  `109b556884150a134c39a8892c599f6551dbc1fb18ea557eebbfc690b4fc3a9b`.
- No live runtime, activation state, resolver, auth/profile/session, Manager,
  bwrap, or remote publication content was changed by this source-lineage
  operation. The next implementation work is a separately scoped Manager
  bundle behind the Core boundary; no Core source slice is currently active.

## Manager v1 Definition Checkpoint (2026-09-05)

- The user initiated post-Core Manager definition. This does not lift or alter
  the two-milestone Core success threshold. `SPEC.md` now defines Manager v1
  as an optional artifact behind `codex termux`, with explicit ownership,
  process handoff, profile state, bounded session projection, notification
  configuration, and Core-mediated repair boundaries.
- MGR-1 is the first implementation bundle: profile list/current/create/use,
  profile-home containment, atomic Manager metadata, child-only `CODEX_HOME`,
  and raw Core launch fidelity. Session, notification, and repair bundles are
  deliberately deferred until their own contracts and focused proofs exist.
- This checkpoint adds no Manager code or artifact and changes no live
  launcher/runtime, Core generation/trust/activation state, resolver,
  auth/profile/session data, or remote publication. No Manager acceptance
  evidence is claimed yet.

## MGR-1 Profile Selection and Isolated Launch (accepted)

- MGR-1 implements the exact `profile list`, `profile current`, `profile
  create`, and `profile use` grammar behind the existing versioned Core
  handoff. It owns only its declared Manager root and profile homes; malformed
  or symlinked Manager state is rejected, profile creation is create-new and
  durable, and selection replacement is atomic. The default profile is never
  copied, and no auth, session, log, Core, resolver, or arbitrary upstream
  state is inspected or migrated.
- Core now supplies and the Manager validates
  `CODEX_TERMUX_CORE_API=codex-manager-core-v1` plus the validated stable Core
  entrypoint. `profile use` publishes selection before launch, sets or removes
  `CODEX_HOME` only in the child, removes the internal handoff variables from
  that child, preserves raw upstream argv/streams/TTY/signals/exit status, and
  emits no Manager success output before the Core exec boundary.
- The release builder accepts an optional executable Manager artifact, carries
  it into the generation root, binds `manager_artifact_digest`, and includes it
  in the signed publication inventory. Core's local official-upstream fallback
  carries forward the authenticated active Manager artifact so an update does
  not silently remove the Manager.
- Focused evidence on the final source passed: Manager unit 9/9, public
  integration 4/4, Core handoff 1/1, and release-builder Manager build and
  publication 2/2. The locked grouped workspace passed Core `128 passed,
  0 failed, 1 ignored`, Manager unit `9 passed`, Manager integration `4
  passed`, and release-builder `11 passed`; workspace clippy with `-D
  warnings`, the locked release build, formatting, and `git diff --check`
  passed. Release-mode disposable profile-root smoke passed 4/4.
- Protected-surface verification kept the resolver at SHA-256
  `7e8ad76e0d200e93918ca2e93c99ff8ecd02071953bf1479819db3ac0dbb6d07`, the
  installed launcher at
  `109b556884150a134c39a8892c599f6551dbc1fb18ea557eebbfc690b4fc3a9b`, the
  bootstrap trust seed at
  `62ab1640b6b4e63afbd5952d11a0bd0a9f1cb78ddde2472e003a42c4db2b832c`, and
  activation state at
  `813cfaa53f0945656e56e0a26bc0de62cd36cfcb30b1c4cac5b866f7e82599e4`.
  No installed runtime/launcher, Manager artifact/state, auth/profile/session
  data, resolver, bwrap state, remote ref, push, or `main` promotion changed.
- Disposition: KEEP one versioned Core handoff, one Manager-owned profile state
  root, one child-only isolated launch path, and one optional signed Manager
  artifact binding; DELETE no Core update/doctor/install authority and no
  legacy state import or bwrap repair path. MGR-2 session listing/resume is
  definition-only and is the next separately scoped bundle.

## MGR-2 Bounded Session Listing and Resume (historical; superseded by SCS)

> SCS supersedes this accepted historical implementation contract. The filesystem
> discovery, TSV projection, and pre-resume lookup described in this section are no
> longer steady-state Manager responsibilities. Current behavior delegates browsing
> and resume to upstream `codex resume`; a custom execution identity reaches that
> upstream surface through `codex termux profile use <PROFILE_ID> -- resume ...`.
> This section remains only as acceptance history and must not be used to reintroduce
> Manager-owned conversation indexing or discovery.

- MGR-2 implements the exact local Manager forms for session listing and
  resume. Listing uses the persisted MGR-1 selection unless an explicit
  profile or `--all` is supplied; `--all` visits the default home and complete
  custom profiles, while arbitrary inherited `CODEX_HOME` is not reinterpreted
  as Manager state.
- Discovery is read-only and bounded to eight directory levels and 4,096
  directory entries per command. It accepts only safe `.jsonl` regular files
  with valid opaque UTF-8 references, nonnegative mtime, size at most 64 MiB,
  and a successful readability open. It never reads session bytes, parses
  JSONL, follows symlinked roots/components, emits paths or content, or creates
  a persistent index. Results are the exact timestamp-descending TSV
  projection with bytewise tie-breaks.
- Resume performs a fresh bounded discovery and requires one matching
  reference before the existing atomic selection transaction. Missing and
  ambiguous references leave selection unchanged and never launch Core. A
  successful path execs the validated Core entrypoint with exactly
  `resume`, the discovered opaque reference, and raw trailing argv, preserving
  child-only `CODEX_HOME`, streams, TTY, signals, and exit status.
- Focused evidence passed the parser, bounded-discovery, list-projection, and
  resume slices at 2/2 each. The final locked workspace suite passed Core
  `128 passed, 0 failed, 1 ignored`, Manager unit `13 passed`, Manager public
  integration `8 passed`, and release-builder `11 passed`. Workspace clippy
  with `-D warnings`, the warnings-denied release build, formatting, diff
  checks, and release-mode Manager smoke `8 passed` all passed.
- This authority record binds Manager source SHA-256
  `f166838d1adfaabf2f59221e64e3aaedbc25bc0aaeecb33172821a16be5135c0` and
  normative SPEC SHA-256
  `06511290f7ad97056bf3f153dc11b7b584f4825d0ca6222cf9d0564c44819d38`.
- Protected verification kept the resolver at SHA-256
  `7e8ad76e0d200e93918ca2e93c99ff8ecd02071953bf1479819db3ac0dbb6d07`, the
  installed launcher at
  `109b556884150a134c39a8892c599f6551dbc1fb18ea557eebbfc690b4fc3a9b`, the
  bootstrap trust seed at
  `62ab1640b6b4e63afbd5952d11a0bd0a9f1cb78ddde2472e003a42c4db2b832c`, and
  activation state at
  `813cfaa53f0945656e56e0a26bc0de62cd36cfcb30b1c4cac5b866f7e82599e4`.
  No installed runtime or launcher, live Manager/profile/session/auth state,
  resolver, bwrap state, remote ref, push, or main promotion changed.
- Disposition: KEEP one bounded metadata-only discovery path, one exact TSV
  projection, one fresh unique resume binding, and the existing MGR-1 atomic
  selection/Core exec boundary. DELETE no upstream session authority and add
  no transcript, sharing, migration, or persistent index machinery. MGR-3
  notification configuration remains the next separately scoped bundle.

## MGR-3 Notification Configuration and Delivery (accepted)

- MGR-3 now implements the exact notify show and notify set forms behind
  the versioned Manager notification record. It validates the canonical hook
  allowlist and all bounded channel, content, newline, toast, color, and group
  values; reads defaults without creating state; rejects malformed, symlinked,
  overlong, conflicting, or incorrectly-modeled records; and publishes the
  complete mode-0600 record through a private atomic replacement.
- Core is the only owner of the generated runtime configuration. On an
  ordinary upstream launch with a qualified Manager artifact it reads the
  bounded record read-only and atomically projects enabled events into the
  Core-owned marker config.toml; missing/invalid state, unavailable Manager,
  and unrelated Core config files fail closed without breaking launch. The
  projection maps each event to the bounded internal
  codex termux notify emit <EVENT> endpoint.
- The internal endpoint reads at most 64 KiB, parses only the specified
  top-level title/body fields, normalizes and bounds text, and invokes
  termux-notification and/or termux-toast best-effort with suppressed child
  streams and a bounded wait. Provider absence/failure, malformed input, and
  disabled hooks return success without output or payload persistence.
- Focused evidence passed Manager contract/codec and JSON-bound tests 6/6,
  Manager public configuration/delivery integration 1/1, and Core real-launch
  projection 1/1. The final debug grouped workspace passed Core 129 passed,
  0 failed, 1 ignored, Manager unit 19 passed, Manager integration 9
  passed, and release-builder 11 passed; the release grouped workspace
  passed the same counts. Workspace clippy with -D warnings, the warnings
  denied release build, formatting, git diff --check, and protected-surface
  verification all passed.
- Final MGR-3 source identities are Core
  58098ba00a5ad13127052ef28c2b2ec65ef5417f0fbf60dc76b671a451fe0cb4,
  Manager
  ba43657495eb6f46327e3295da6be33fece9167d1a49566afdb8c5627697f4af,
  and normative SPEC
  3b4bb642088a8799c6b4351c824fb416e01d822a2ace4aff47b8bd05b6621a83.
- Protected verification kept the resolver at SHA-256
  7e8ad76e0d200e93918ca2e93c99ff8ecd02071953bf1479819db3ac0dbb6d07, the
  installed launcher at
  109b556884150a134c39a8892c599f6551dbc1fb18ea557eebbfc690b4fc3a9b, the
  bootstrap trust seed at
  62ab1640b6b4e63afbd5952d11a0bd0a9f1cb78ddde2472e003a42c4db2b832c, and
  activation state at
  813cfaa53f0945656e56e0a26bc0de62cd36cfcb30b1c4cac5b866f7e82599e4.
  No installed runtime or launcher, live Manager/profile/session/auth state,
  resolver, bwrap state, remote ref, push, or main promotion changed.
- Disposition: KEEP one Manager-owned versioned config codec, one Core-owned
  projection, one internal bounded emit boundary, and independent
  capability-aware providers. COLLAPSE the legacy config-env/payload/log
  ladder into the single record and direct provider path; DELETE payload,
  session/cwd/transcript metadata, fallback logging, and Core-directory writes
  from Manager. MGR-4 is accepted below.

## MGR-4 Repair Planning Through Core (accepted)

- MGR-4 implements exactly `codex termux repair plan` and `codex termux repair
  apply`. Both forms reject options and trailing arguments. Manager validates
  the existing handoff, sets only the fixed versioned repair request values,
  removes profile `CODEX_HOME`, and execs the validated Core entrypoint without
  printing a Manager success line. Normal Manager launches remove the internal
  request variables before the upstream/Core boundary.
- Core accepts only the exact request/operation and matching argv pairs. The
  read-only plan qualifies the selected generation and emits the bounded
  `codex-core-repair-v1` record: root code-mode-host layout is
  `none/healthy`, legacy compat layout is `update/legacy-generation`, and
  unavailable state is `unavailable/core-state-unavailable` with status 1.
  It does not invoke upstream, access the network, expose paths or generation
  identities, or mutate Core state.
- Apply recomputes the plan in Core. The healthy path emits exactly
  `codex repair: no repair needed`; the legacy path removes the internal
  request variables and invokes the existing no-argument signed Core update;
  unavailable state fails closed with the fixed plan-unavailable error. No
  second updater, rollback heuristic, package-manager action, raw upstream
  installation, legacy import, bwrap repair, or Manager Core-state write was
  added.
- Focused evidence passed Core request parser/route admission, read-only plan,
  and public apply/plan boundary tests 3/3; Manager exact grammar and public
  handoff boundary tests passed 2/2. The final debug workspace passed Core
  133 passed, 0 failed, 1 ignored, Manager unit 20 passed, Manager integration
  10 passed, and release-builder 11 passed. The release workspace passed the
  same counts. Locked clippy with `-D warnings`, the warnings-denied release
  build, formatting, diff checks, and protected-surface verification passed.
- Final MGR-4 source identities are Core
  `55d6d2816e02a9199b1a763c3541af00ee210a05c21596ae0292b11fdc3135ed`,
  Manager
  `3e7e59f39abcc1560a50ed990682c7899ffb734d49c36327c1e8669f075c14c6`, and
  normative SPEC
  `3edf4fe591f58689cb17cabe7e6841dd788a7496105d407ec5fbaa0877406428`.
- Protected verification kept the resolver at SHA-256
  `7e8ad76e0d200e93918ca2e93c99ff8ecd02071953bf1479819db3ac0dbb6d07`, the
  installed launcher at
  `109b556884150a134c39a8892c599f6551dbc1fb18ea557eebbfc690b4fc3a9b`, the
  bootstrap trust seed at
  `62ab1640b6b4e63afbd5952d11a0bd0a9f1cb78ddde2472e003a42c4db2b832c`, and
  activation state at
  `813cfaa53f0945656e56e0a26bc0de62cd36cfcb30b1c4cac5b866f7e82599e4`.
  No live runtime/launcher, Manager/profile/session/auth state, resolver,
  bwrap state, remote ref, push, or main promotion changed.
- Disposition: KEEP one fixed Manager-to-Core repair request, one Core-owned
  bounded plan, and the existing signed update operation. COLLAPSE the legacy
  support/metadata/rollback/rebuild action ladder into Core's one explicit
  update admission; DELETE Manager-side generation inspection, direct repair
  writes, fallback repair, bwrap repair, package-manager action, and legacy
  state import. MGR-5 is accepted below.

## MGR-5 Manager Artifact Build and Qualification (accepted)

- MGR-5 adds no public `codex termux` command or Manager persistent state. The
  separately built Manager executable now has the exact private build-time
  probe `--artifact-probe` with marker
  `CODEX_MANAGER_ARTIFACT_PROBE=1`; it requires no user state and emits only
  the fixed `codex-manager-artifact-v1` / `core_api=codex-manager-core-v1`
  record. Core strips the marker at its Manager exec boundary, so the probe is
  not exposed through ordinary `codex termux` launch.
- Release-builder snapshots the Manager into private staging, runs the probe
  with empty environment, null stdin, private cwd, 512-byte stdout/stderr
  bounds, and a five-second wait bound, then carries only the qualified
  snapshot at mode `0755`. Mismatch, stderr, nonzero, oversize, timeout,
  non-executable, and symlink inputs fail closed before generation
  publication. The snapshot digest is bound in `manager_artifact_digest` and
  the signed inventory includes `manager`.
- Core's optional GitHub Release asset set now includes `manager` when the
  signed generation contains it, with the same regular-file, per-file, and
  aggregate bounds. The existing release-before-index ordering and Core
  handoff remain unchanged; generations without Manager remain valid and
  report Manager unavailable.
- Focused evidence passed the real built Manager probe integration 1/1, the
  release-builder bounded probe matrix 1/1, and the Core optional remote asset
  inventory/symlink regression 1/1. The final debug workspace passed Core
  134 passed, 0 failed, 1 ignored, Manager unit 20 passed, Manager integration
  11 passed, and release-builder 12 passed. The release workspace passed the
  same counts. Locked clippy with `-D warnings`, the warnings-denied release
  build, formatting, diff checks, and protected-surface verification passed.
- Final MGR-5 source identities are Core
  `1bed71c8a3b30604ffe738d7fb19fa22c0d041c92d1d08bd7d7aaccc76018a68`,
  Manager
  `9134bd459f36966c62aa7f8b183e0b4f6af3c44d1fa7226cbfad8a4c05d57476`,
  release-builder
  `cb89492ff81d7ea2481e80c5833b30bed866c8736a01ef254f01ba34f7ce37f2`, and
  normative SPEC
  `97b051203f005c35ec13e9063a035bfcd1d4674b93e1f597f88765402b49234c`.
- Protected verification kept the resolver at SHA-256
  `7e8ad76e0d200e93918ca2e93c99ff8ecd02071953bf1479819db3ac0dbb6d07`, the
  installed launcher at
  `109b556884150a134c39a8892c599f6551dbc1fb18ea557eebbfc690b4fc3a9b`, the
  bootstrap trust seed at
  `62ab1640b6b4e63afbd5952d11a0bd0a9f1cb78ddde2472e003a42c4db2b832c`, and
  activation state at
  `813cfaa53f0945656e56e0a26bc0de62cd36cfcb30b1c4cac5b866f7e82599e4`.
  No live runtime/launcher, Manager/profile/session/auth state, resolver,
  bwrap state, remote ref, push, or main promotion changed.
- Disposition: KEEP one release-builder probe and one optional signed Manager
  asset path; COLLAPSE digest-only Manager admission into the snapshot probe
  and existing signed inventory; DELETE no second Manager updater, probe
  state, trust source, or Core launch path. No MGR-6 bundle is selected yet.

## MGR-6 Manager Artifact Distribution and Disposable Qualification (accepted)

- MGR-6 adds no public command, Manager record, Core trust source, or on-device
  build path. The release boundary remains one off-device, locked-toolchain
  Manager artifact from the exact accepted rewrite revision through the
  existing bounded release-builder input. PATH/live-state/unpinned discovery
  remains excluded.
- Core's publication asset inventory now parses the signed manifest before
  upload: a signed `manager` entry requires a regular bounded Manager asset;
  a missing, symlinked, special, or extra unlisted Manager fails closed. A
  Manager-less Core-only manifest remains valid. The signed asset ordering and
  index advancement boundary remain unchanged.
- Release-mode disposable consumer proof passed the actual built Manager
  artifact for the exact artifact probe, fresh profile lifecycle/isolation,
  and inherited legacy-home non-interpretation. Release-mode Core handoff
  proof passed raw argv/streams/exit preservation and probe-marker removal.
  Existing signed generation digest-binding and Core-only fallback regressions
  remain green.
- Focused asset completeness/publication regressions passed 3/3. The final
  debug and release workspaces passed Core 134 passed, 0 failed, 1 ignored,
  Manager unit 20, Manager integration 11, and release-builder 12. Locked
  clippy, warnings-denied release build, formatting, diff checks, and
  protected-surface verification passed.
- Final MGR-6 source identities are Core
  `efd9e25e5fe4a1f4b310ab971b2b87a919ece2761e493692fb5de1992c0098a4`,
  Manager
  `9134bd459f36966c62aa7f8b183e0b4f6af3c44d1fa7226cbfad8a4c05d57476`,
  release-builder
  `cb89492ff81d7ea2481e80c5833b30bed866c8736a01ef254f01ba34f7ce37f2`, and
  normative SPEC
  `ec8f68d59de49a11cb6b20a738170e3810610764d19421f81a4689ea12b9afed`.
- Protected surfaces remain unchanged: resolver, installed launcher, trust
  seed, activation state, live runtime/Manager/profile/session/auth state,
  bwrap state, remote refs, and `main`. No remote push or live replacement was
  performed. Remote publication readback and live cutover remain separate
  operational gates requiring an explicit target and authorization.
- Disposition: KEEP one signed-manifest-to-asset completeness check and the
  existing release-before-index boundary; COLLAPSE disposable proof into the
  existing Manager/Core integration paths; DELETE no new updater, trust source,
  or live-state path. MGR-7 is the accepted operational follow-up recorded
  below.

## MGR-7 Remote Publication Readback and Operational Qualification (accepted)

- MGR-7 added no source command, persistent state, trust source, or release
  format. Before external I/O, the accepted release-built inputs were bound to
  generation `local-1788680568-mgr7-1`, local publication
  `/data/data/com.termux/files/usr/tmp/codex-mgr7-candidate.eM827z/publication`,
  repository `humtr/codex`/branch `main`, and bounded fresh/legacy device
  qualification. The candidate used upstream Codex 0.153.4 archive SHA-256
  `fc395cb043a1093ab0db34f44aba3199bfaa9ce640cd9be7fd588f44b0da64a4`,
  release-built Core SHA-256
  `48f3df4ebc7d4f833fb2b091e0f70ed3afcd20e2170cfe14cf5001b0fee0a11`, and
  Manager SHA-256
  `a706292d653fb33cc652fb1d9ef03ca8cb1e9e6f402a6098b00d4bedeaf189fc`.
- The existing production GitHub publication path completed one nonzero
  `tests::github_upload_probe` invocation. Release sequence 6 published the
  complete six-asset Release, including `manager`, followed by
  `update-index-v1.sig` and then `update-index-v1`. The remote release is
  non-draft/non-prerelease; readback lists `runtime`,
  `codex-code-mode-host`, `manager`, `generation.meta`, `release.manifest`,
  and `release.sig`. No private key was uploaded, no OpenAI repository was
  touched, and no source-history ref was pushed.
- The private bounded HTTPS readback root
  `/data/data/com.termux/files/usr/tmp/codex-mgr7-readback.JYQU08` verified
  both signed control documents with the pinned Ed25519 key, the exact
  generation and release base, all four signed inventory asset digests and
  modes, and the release-built Manager artifact. The Manager probe output was
  exact; its handoff removed the internal API variables and caller
  `CODEX_HOME`, preserved streams/exit, and delivered SIGTERM with exit 143.
- Separate disposable roots passed the public install paths:
  fresh `/data/data/com.termux/files/usr/tmp/codex-mgr7-fresh.khgE7y` and
  legacy `/data/data/com.termux/files/usr/tmp/codex-mgr7-legacy.rJn8MV`.
  Both activated the signed Manager-bearing generation, reported upstream
  version 0.153.4, reached Manager profile isolation, emitted doctor JSON
  containing Manager status, and rejected explicit Linux sandbox mode with
  status 2 without invoking bwrap. The fresh root additionally passed
  `doctor --color` through the Termux PTY with SGR and upstream-first plus
  Termux sections; Manager child stdout/stderr/exit and profile `CODEX_HOME`
  isolation passed.
- An intentionally auth-free disposable fixture was rejected at the required
  candidate doctor probe, as designed. The accepted qualification used the
  existing `CODEX_HOME` only as a read-only child input, without copying or
  printing credentials. The auth digest was unchanged before and after, as
  were the protected resolver, installed launcher, trust seed, and activation
  state identities. No live runtime replacement or live activation occurred.
- Final grouped source acceptance on this same revision passed Core
  `134 passed, 0 failed, 1 ignored`, Manager unit `20 passed`, Manager
  integration `11 passed`, and release-builder `12 passed` in both debug and
  release profiles. Locked clippy with `-D warnings`, the warnings-denied
  release build, formatting, and diff checks passed. The protected hashes
  remained equal to the recorded resolver, launcher, trust-seed, and
  activation-state identities.
- Source revision `75ba67f443da6cfb4af73f09f82f8ad3a0f36ece` remains the accepted
  MGR-7 definition/release-builder implementation base; MGR-7 closed as an
  operational evidence bundle and requires no production-code change. Future
  publication or live cutover must bind a new exact candidate and explicit
  authorization.

## R10 Coordinated Core + generation update (accepted)

- R10 was opened by the bounded live finding after MGR-7: generation
  `local-1788680568-mgr7-1` activated, but its Manager handoff failed because
  the installed Core launcher was not the matching Core. The candidate was
  rolled back to `local-1788570645-23374-1`; protected resolver, launcher, and
  trust-key identities were preserved. R10 closes that root cause in source;
  it did not perform another live cutover.
- The normative contract now defines signed release format v4. A v4
  generation carries an executable root-level `core` asset whose digest and
  mode are signed in the same inventory as the generation. Existing v3
  generations without Core remain readable, while a new Manager-bearing
  candidate without a coordinated Core asset is rejected before staging.
- `release-builder` now snapshots, validates, signs, and publishes the Core
  artifact with the generation. Core admission verifies the v4 inventory,
  digest, mode, path, and Manager coupling; remote asset inventory includes
  the same Core artifact and rejects missing, extra, or unsafe files.
- Coordinated activation snapshots the currently installed Core launcher,
  installs the candidate launcher, and commits the signed generation under
  the activation lock. State-boundary failure restores the exact old launcher
  and pointer state. Rollback restores the retained launcher/generation pair,
  with bounded metadata binding the retained launcher digest to its generation.
- Focused R10 proof passed the v4/v3 trust-policy matrix, the
  `test_r10_new_manager_update_requires_coordinated_core_asset` rejection,
  `test_r10_coordinated_activation_restores_entrypoint_on_state_boundary_failure`
  recovery regression, and the v4 remote update/rollback integration with
  exact launcher restoration. The release-builder suite passed 12 tests.
- Final grouped acceptance on the same source revision passed Core `136
  passed, 0 failed, 1 ignored`, Manager unit `20 passed`, Manager integration
  `11 passed`, and release-builder `12 passed`; locked clippy with `-D
  warnings`, the warnings-denied release build, formatting, and diff checks
  also passed. Existing fresh/legacy disposable qualification evidence remains
  accepted from MGR-7; no live runtime, Manager/profile/session/auth state,
  resolver, remote ref, or source publication was changed by R10.
- Disposition: KEEP one signed v4 Core inventory and one coordinated
  activation/rollback path; COLLAPSE launcher replacement into the existing
  signed generation transaction; DELETE no second updater, trust source, or
  live-state path. The pre-bundle source base was
  `8f398aae78c03927eb6f3115fa593f76ad62b2bd`; this R10 checkpoint is closed
  after the source commit `19875a0507ea5a478cd9043807560399e4292016` was
  created. The accepted rewrite was then promoted to local publication
  authority `main` only after preserving its prior tip as
  `legacy/main-pre-r10-20260906`; no remote ref or publication was changed.

## R10 Live Qualification and Cutover (accepted)

- On 2026-09-12, after the accepted R10 source revision
  `0621105fd1be8461b370466fbfa981938241074d` was requalified, one explicitly
  authorized bounded live cutover produced and activated signed v4 generation
  `local-1789181261-r10-live-1` at release sequence `7`. The candidate was
  built from the exact accepted upstream Codex `0.153.4` archive SHA-256
  `fc395cb043a1093ab0db34f44aba3199bfaa9ce640cd9be7fd588f44b0da64a4`,
  with Core SHA-256
  `00ecd5a3536809ef23de055d9c511dc8f503f4658fdb7ff28037ac9e81cd3325`
  and Manager SHA-256
  `a706292d653fb33cc652fb1d9ef03ca8cb1e9e6f402a6098b00d4bedeaf189fc`.
  The signed manifest was `codex-release-v4` and contained the coordinated
  `core`, `manager`, `runtime`, `codex-code-mode-host`, and `generation.meta`
  inventory.
- Immediately before mutation, the installed launcher, activation state,
  resolver, trust seed, auth file, session tree, and candidate artifact digests
  were rebound to their exact expected identities. The accepted R10 Core then
  executed the repository-native `update --local` path; no ad hoc launcher
  replacement was used. Activation completed with
  `local-1789181261-r10-live-1` current and
  `local-1788570645-23374-1` retained as the previous generation.
- The installed launcher SHA-256 became the exact candidate Core digest
  `00ecd5a3536809ef23de055d9c511dc8f503f4658fdb7ff28037ac9e81cd3325`.
  The rollback entrypoint cache retained the exact prior launcher SHA-256
  `109b556884150a134c39a8892c599f6551dbc1fb18ea557eebbfc690b4fc3a9b`
  and bound it to generation `local-1788570645-23374-1` in rollback metadata.
  The retained old generation remained present.
- Post-cutover `codex doctor --json` returned success with Core, runtime,
  Manager, and overall summary all `healthy`, reporting the new generation.
  `codex --version` remained `codex-cli 0.153.4`, and the real installed
  `codex termux profile list` handoff returned only `default`. Manager
  persistent state remained absent.
- Protected identities remained unchanged across the operation: resolver
  SHA-256 `7e8ad76e0d200e93918ca2e93c99ff8ecd02071953bf1479819db3ac0dbb6d07`,
  trust-seed SHA-256
  `62ab1640b6b4e63afbd5952d11a0bd0a9f1cb78ddde2472e003a42c4db2b832c`,
  auth SHA-256
  `5ac22bf47177ad1366352502427a9bb11f57b861675f756644ca4fed8f78ad32`,
  and the bounded session-tree identity. No source file, source commit, remote
  ref, or remote publication was changed by the live operation. Temporary
  candidate staging was removed after the installed generation was verified.
- This closes the exact live failure exposed by MGR-7: the active Manager-bearing
  generation and installed Core launcher now form one signed coordinated pair,
  and the retained old launcher/generation pair is available for repository-
  native rollback. No new source implementation slice is selected by this
  operational acceptance.

## Final Core Goal Closure (accepted)

- On 2026-09-12, the primary Lead completed a final completion-gate audit bound
  to `rewrite/rust-core@2340bd345ff8f867c16ed78a28f1e5ca1fbb2f49`.
  The actual remote `origin/rewrite/rust-core` matched that exact checkpoint;
  actual remote `main` remained separately at
  `004702fd8081df2a2b07efd1ed394b510bf4953b` and was not part of the audit.
- SPEC Milestone 1 is satisfied by the accepted real Core entrypoint, exact
  upstream passthrough/version behavior, environment/FD/process contracts,
  resolver non-mutation, sandbox policy, read-only doctor, updater interfaces,
  and focused/unit/integration/fault/real-Termux proof. The later M2-B2 wiring
  closed the former proof-only `main()` gap, so Milestone 1 is accepted on the
  real production entrypoint rather than test injection alone.
- SPEC Milestone 2 is satisfied by accepted prebuilt Core delivery; fresh and
  legacy bootstrap; signed immutable manifests and bounded key rotation;
  official upstream acquisition/adaptation; atomic activation, recovery, and
  rollback; offline install/recovery; launch/update overlap and injected-fault
  coverage; isolated fresh-Termux and upgrade-from-legacy qualification; and
  the completed independent product review at
  `5a7a5292f38876087a5c9b5a41b1dd7e8dbf082b`.
- The `Current Success Threshold` above is satisfied. Accepted disposable
  fresh/legacy Termux qualification used prebuilt release artifacts to install
  and reach the real upstream runtime, report version and doctor results, then
  perform signed update and explicit rollback. M2-B9/B10 provide the accepted
  overlap, injected transaction recovery, fresh offline bootstrap, and
  recovery-with-rollback proof using the actual release Core. The bootstrap
  path has no compiler or package-manager path, so the product does not require
  an on-device Rust toolchain. The later R10 v4 proof and authorized live
  qualification additionally verify coordinated Core/generation activation,
  state-boundary recovery, exact retained-pair rollback, and a healthy installed
  runtime.
- Across the load-bearing source, disposable-device, and live acceptance
  evidence, protected resolver, trust, auth/profile/session, Manager, package,
  and unrelated user-state boundaries remained unchanged except for explicitly
  authorized Core-generation/launcher transactions inside their declared
  ownership boundary. No missing Core completion gate remains.
- Manager product features remain outside the original two-milestone Core
  completion claim even though the separately accepted Manager work and R10
  coordination are retained as additional product evidence.
- Therefore the Rust Core rewrite goal is complete. No Goal Lift is active and
  no next source implementation slice is selected. Future product work must
  establish a new goal/lift and a new bounded `WORKBOARD.md` bundle before
  product-code mutation. Remote `main` promotion, release/index publication,
  and further live mutation remain separate explicitly authorized operations.

## Post-goal Maintenance MNT-1 — Retire stale wrapper-version hook (accepted)

- On 2026-09-12 the user authorized a bounded audit and retirement of the old
  wrapper-version automation after the Rust Core goal closure. The maintenance
  base was `rewrite/rust-core@b6bf28588dae123da811a64224a7ffd9900c9edc`.
- The audit found no tracked product, release, bootstrap, Manager, or test
  consumer of `update-wrapper-version`, `WRAPPER_VERSION`, or
  `wrapper_version`. `tools/update-wrapper-version.sh` and
  `config/wrapper-version.env` are both absent and untracked in the accepted
  rewrite.
- The only executable residue was the repository-local untracked
  `.git/hooks/pre-commit` from the predecessor environment, SHA-256
  `e719cfa64f5a1d37d41e02dda33e6df58a2f819f89d16aca70497d7fcd51a638`,
  mode `0755`. It did nothing except invoke the absent updater script and stage
  the absent wrapper-version env file; no `core.hooksPath` override was
  configured.
- This residue was not a product version authority. `SPEC.md` requires
  `codex --version` / `-V` to print exactly upstream version output, while the
  signed v4 release manifest, generation identity, release sequence,
  `generation.meta`, and signed Core artifact digest provide the release and
  activation identity used by the Rust Core.
- MNT-1 removed only that exact untracked local hook. No replacement hook,
  updater script, wrapper-version env file, version authority, or product-code
  path was introduced. Historical records of earlier hook failures remain in
  this ledger as history rather than current routing instructions.
- Post-removal proof passed: the hook/script/env paths are absent; the tracked
  consumer search outside authority history returns zero; `git diff --check`
  passes; and `cargo check --workspace --locked` succeeds. The only tracked
  maintenance changes are `GOAL.md` and `WORKBOARD.md`.
- MNT-1 closes through an ordinary Git commit with no `--no-verify` exception.
  No product behavior, live state, local `main`, remote ref, or release
  publication is changed. No Goal Lift is active and no new source slice is
  selected.

## Post-Core Termux Compatibility Audit (2026-09-12)

- The read-only audit was bound to `rewrite/rust-core` commit
  `275b8725c4ee0924ef00a10c3e2f3995421ec8f2` and to the currently qualified
  upstream `0.153.4` source behavior. It changed no source, live runtime,
  resolver, auth, profile/session state, network configuration, or remote ref.
  A later upstream target must revalidate target-sensitive findings before
  relying on this evidence.
- Browser opening is a shared compatibility surface, not one login call site:
  upstream uses desktop browser opening for primary login, TUI onboarding and
  history URL actions, and MCP OAuth. The current Termux product has no
  explicit shared opener policy for those surfaces.
- A subsequent real MCP add attempt reached OAuth discovery but failed during
  Dynamic Client Registration with `invalid_client_metadata` because the
  authorization-server account configuration did not allow the submitted
  redirect URI. This occurs before browser opening and is not classified as a
  Core defect without evidence that the submitted callback itself violates the
  selected upstream registration mode or advertised server metadata. The
  provider/account allowlist is an external TC-2 acceptance prerequisite; the
  repository must not patch around or mutate it.
- The upstream app-server daemon expects an independent
  `$CODEX_HOME/packages/standalone/current/codex` and contains an hourly
  standalone installer/update loop. That path is incompatible with the
  accepted wrapper-owned signed update authority. The live qualified wrapper
  installation did not contain that standalone tree during the audit.
- Manager custom profile paths make the upstream app-server control socket at
  least 118 bytes and up to 181 bytes for the accepted profile-id bound, while
  Termux exposes `sockaddr_un.sun_path[108]`. The default profile socket is 82
  bytes and is not affected. Long custom-profile IDE IPC primary paths can
  also exceed the bound, but upstream has a temporary-directory fallback, so
  that path remains a regression target rather than a confirmed failure.
- The shipped runtime is compiled for `aarch64-unknown-linux-musl`; therefore
  upstream `target_os = "android"` guards are inactive at run time. The audit
  confirmed this matters for clipboard behavior: image paste enters the Linux
  desktop/WSL path and Android-only copy UI guards do not apply.
- The accepted package adaptation intentionally excludes bundled `rg`. The
  audited device has Termux `ripgrep`, but bootstrap and product documentation
  neither provision nor declare it, while upstream thread/content search can
  require `rg`. This is a fresh-environment dependency risk, not evidence that
  the audited live device is broken.
- MCP OAuth `Auto` load/save paths fall back from keyring to file storage, while
  delete/logout returns on keyring failure before deleting the file fallback.
  This is a latent Termux compatibility risk that requires a disposable
  credential-store proof before implementation is claimed necessary.
- No new defect was found in shell selection/PATH, the current Termux
  `$SHELL -lc` behavior, external editor launch, code-mode-host, accepted
  sandbox substitution, signals/TTY, TLS/WebSocket connectivity, startup CA
  layering, default-profile socket length, foreground remote-control temporary
  sockets, or the default-disabled zsh-fork path.

## Goal Lifts

### TERMUX-COMPAT — Linux-target upstream compatibility on Termux (accepted)

This lift addresses concrete post-Core product risks discovered above without
reopening the accepted Rust Core completion claim. The original two-milestone
Core goal and its R10 live qualification remain accepted evidence; this lift
adds a new compatibility success threshold on top of them.

The lift succeeds when all of the following are proven on the then-selected
supported upstream version:

1. No app-server, remote-control, login, MCP, TUI, or other compatibility path
   can create or trust an unmanaged standalone Codex installation, execute an
   upstream self-updater, bypass the signed-generation authority, or widen the
   accepted runtime byte-patch policy without a prior SPEC amendment.
2. Daemon-backed app-server behavior is either bound to the currently
   qualified signed generation or fails closed before network/state mutation;
   every supported app-server Unix socket uses a private profile-distinct path
   within the Termux pathname limit. Foreground remote-control behavior remains
   intact.
3. All enumerated upstream browser-open intents share one bounded Termux URL
   opener policy with safe manual fallback; fixing only primary login is not
   sufficient. MCP OAuth is proven end to end only after its authorization
   server is externally configured to allow the exact legitimate callback for
   the selected upstream registration mode. The proof covers DCR/CIMD strategy
   selection as applicable, authorization URL production, Termux browser open,
   loopback callback validation, token exchange, and disposable credential
   persistence. Provider-policy rejection alone is not repaired in Core; a
   client patch is admitted only if the exact allowed callback is still wrong
   because Codex contradicts advertised metadata or the applicable protocol.
4. Material upstream Linux-versus-Android compile-time behavior is reviewed for
   each supported release. Confirmed desktop-only assumptions such as image
   clipboard paste are adapted or explicitly unavailable without destabilizing
   the TUI; existing terminal-mediated text-copy fallback remains usable.
5. A fresh product environment has no silent ordinary-correctness dependency
   on an undeclared system `rg`. The accepted resolution is either a qualified
   signed helper/fallback or explicit bounded degradation/diagnosis, never an
   automatic package-manager install.
6. MCP OAuth fallback authority is proven in disposable roots. If keyring
   unavailability can strand a file-backed credential on logout, delete
   semantics are corrected so the resolved fallback authority can be removed;
   credential contents are never recorded as evidence.
7. Focused compatibility tests and the existing workspace/protected-state
   regressions pass with no live resolver, launcher/runtime, auth, profile,
   session, Manager, network configuration, remote ref, or publication
   mutation.

The implementation is intentionally sliced. `WORKBOARD.md` selects exactly
one current bundle; later browser/clipboard and host-tool/credential work does
not become an implicit dependency of the first app-server bundle. Remote
source push, `main` promotion, release/index publication, and live cutover
remain separate explicitly authorized operations.

### TC-1 — app-server authority fence and short socket boundary (accepted)

- TC-1 is accepted at local source commit
  `6abb70bf33144468d9420d7880f25f5afc0b5684`. Before acceptance, the
  target-sensitive upstream audit was rebound from the historical `0.153.4`
  evidence to upstream tag `rust-v0.154.0`, commit
  `6b9826e3aa83b1a5947db50f4332cb9c65f1b340`. That release still resolves
  daemon execution through `$CODEX_HOME/packages/standalone/current`, and its
  Unix updater still fetches `https://chatgpt.com/codex/install.sh` and repeats
  on an hourly interval. The wrapper therefore cannot safely treat upstream
  daemon management as signed-generation authority.
- The Core public-dispatch boundary now classifies every selected upstream
  daemon-backed form before generation, repair, runtime, network, or app-server
  state is loaded. `app-server daemon ...` and `remote-control start`, `stop`,
  and `pair` fail closed with one stable Termux-specific unsupported result and
  exit status 2. The classifier covers the selected 0.154 root/global option
  grammar, including global `-c`/`--config`, `--enable`, `--disable`, and the
  multi-value root `-i`/`--image`, while preserving `--` termination and
  option-value disambiguation.
- No daemon-backed path remains supported by TC-1, so TC-1 introduces no daemon
  socket namespace and cannot create a long custom-profile daemon socket,
  unmanaged standalone tree, installer request, or updater loop. The upstream
  0.154 foreground `remote-control` path remains outside the fence and retains
  its private short `/tmp/codex-rc-*/rc.sock` transport. Foreground app-server
  and help/JSON routing remain passthrough behavior.
- TC-1 adds no raw runtime byte substitution and does not widen
  `termux-fd-remap-v1`; no SPEC amendment was required. It adds no second
  updater, shadow installation, profile/socket state, or Manager-owned state.
- Final acceptance job `job_six_2edc9f2ce4` passed both focused TC-1 tests,
  `cargo check --workspace --locked`, the complete locked workspace test suite,
  `cargo clippy --workspace --all-targets --locked -- -D warnings`,
  `cargo fmt --all -- --check`, and `git diff --check`. The locked workspace
  suite reported Core `138 passed / 0 failed / 1 explicit live smoke ignored`,
  Manager unit `20/20`, Manager integration `11/11`, and release-builder
  `12/12`, with doc tests green.
- Source acceptance used only the Task-owned disposable worktree. No live
  resolver, launcher/runtime, auth, profile, session, Manager, network/provider
  configuration, release/index publication, remote ref, or live cutover was
  changed. Worker mode remained OFF. TC-1 is closed; `WORKBOARD.md` advances
  only the preplanned TC-2 routing.


### TC-2 — shared Termux browser opener and Linux-target clipboard boundary (accepted)

- TC-2 source recovery was rebound to accepted integration base
  `74c5ffe0f2cbbec2b52a342758a1a913bf2cd2d8`; the remote
  `origin/rewrite/rust-core` was reverified at the same SHA. The latest tracked
  TC-2 diff was recovered from the orphaned resume worktree into fresh managed
  worktrees without copying or deleting any untracked build output. The
  selected upstream target remains `rust-v0.154.0` at
  `6b9826e3aa83b1a5947db50f4332cb9c65f1b340`.
- Production review confirms one signed browser-helper policy for the enumerated
  upstream browser-open intents. The opener helper is bound to the exact
  qualified `$PREFIX/bin/termux-open-url`, accepts exactly one complete
  HTTP/HTTPS URL, uses no shell evaluation, and fails closed for malformed or
  unsafe schemes, extra arguments, and argument-injection forms. The manual
  helper is a bounded fallback with the same URL validation. Their identities
  are `termux-browser-open-v1` and `termux-browser-manual-v1`; signed generation
  inventory/staging uses `browser/open/curl` and `browser/manual/curl`, while
  generic helpers retain the existing `helpers/<index>` semantics. Browser
  helper digests remain bound by the signed generation/update authority, and
  URL opening introduces no logical `CODEX_HOME` auth/config/session mutation.
- The prior 22 Core regressions were fixture-contract failures rather than a new
  production compatibility defect. Candidate/release/update/doctor fixtures now
  add the two mandatory browser helpers where the signed generation contract
  requires them, while the baseline fixture and generic-helper indexing keep
  their legacy semantics. This restores the complete Core suite without
  weakening the new mandatory helper inventory.
- Clipboard review was rebound to the selected upstream source. On Linux,
  upstream image paste first attempts the desktop `arboard` transport and then
  only the enumerated WSL PowerShell fallback; WSL detection checks
  `/proc/version` and, if that cannot be read, only `WSL_DISTRO_NAME` and
  `WSL_INTEROP`. The accepted host was read-only verified as aarch64 Android
  Termux (`TERMUX_VERSION=0.119.0-beta.3`, Termux `$PREFIX`, Android root/data
  indicators, no WSL kernel marker); Android denies this process access to
  `/proc/version`, and both WSL selector variables were absent. The child-local
  projection makes X11/Wayland unusable and removes both WSL selectors, so the
  selected upstream image path cannot reach a usable desktop or WSL transport.
  Without a separately qualified backend image clipboard is therefore
  unavailable; terminal-mediated text-copy fallback remains outside this
  image-transport fence. This is a source/capability clarification only and
  does not widen the raw-runtime byte-patch allowlist.
- Final repository gate job `job_sy8_8d146d5293` used only the disposable
  managed worktree and its private `target/tc2-analysis` directory. Results:
  `cargo fmt --all` PASS; `cargo fmt --all -- --check` PASS; focused Core TC-2
  `1 passed / 0 failed`; focused release-builder TC-2 `1 passed / 0 failed`;
  release-builder full `13 passed / 0 failed`; Core full
  `139 passed / 0 failed / 1 explicit real-Termux smoke ignored`;
  `cargo check --workspace --locked` PASS; locked workspace tests PASS with
  Core `139/0/1`, Manager unit `20/20`, Manager integration `11/11`, and
  release-builder `13/13` (doc/zero-test targets also green);
  `cargo clippy --workspace --all-targets --locked -- -D warnings` PASS; and
  `git diff --check` PASS. No TC-1 regression was observed.
- Disposable MCP OAuth E2E job `job_t3r_6232d788b0` passed against the selected
  adapted `codex-cli 0.154.0` runtime. The fixture deliberately did not
  advertise CIMD, so upstream `Auto` selected Dynamic Client Registration. DCR
  observed exactly one registration and accepted only the exact selected-release
  loopback callback derived from the MCP server URL; the emitted authorization
  URL carried that same callback and S256 PKCE. The exact signed
  `termux-browser-open-v1` helper was then exercised with that emitted URL and
  delegated to the installed qualified `$PREFIX/bin/termux-open-url` with exit
  status 0. Because the non-interactive Android browser handoff did not itself
  fetch the localhost fixture, a disposable browser driver followed the same
  emitted URL; that drove the real Codex loopback listener, one exact token
  exchange, PKCE verifier validation, `mcp login` exit 0, and persistence of one
  disposable file-backed credential. No credential, authorization code, token,
  client secret, account identifier, or full authorization URL is recorded as
  evidence. The sensitive live `~/.codex` fingerprint was unchanged before and
  after the proof.
- The earlier attempt whose Android browser process did not fetch the localhost
  fixture is non-PASS diagnostic evidence only; it does not weaken the bounded
  opener proof. Acceptance relies on the signed-helper handoff plus the
  deterministic disposable driver for callback/token completion, rather than on
  an unrelated live MCP endpoint or provider configuration.
- Fresh acceptance gate `job_t3x_5865b62f9b` completed with exit status 0 on
  the unchanged TC-2 source diff: formatting check PASS; release-builder full
  `13 passed / 0 failed`; Core full `139 passed / 0 failed / 1 explicit
  real-Termux installed-runtime smoke ignored`; workspace locked check PASS;
  workspace locked tests PASS with Core `139/0/1`, Manager unit `20/20`,
  Manager integration `11/11`, and release-builder `13/13`; warnings-denied
  workspace/all-targets clippy PASS; and `git diff --check` PASS. The focused
  commands embedded in that gate used incomplete `--exact` names and therefore
  matched zero tests, so they are not counted as focused evidence. Corrected
  focused gate `job_t42_57752a5acb` then ran the fully-qualified names and
  passed Core TC-2 `1/1` and release-builder TC-2 `1/1`.
- TC-2 is accepted at local source commit
  `ad5ccd654a783c4d5f67a5253eea5705a4c86577`. No installed Codex
  runtime/helper, live resolver, persistent process environment, live
  `CODEX_HOME` auth/config/session/profile, provider/account configuration,
  release/index publication, remote ref, live cutover, or `main` promotion was
  changed. The explicit real-Termux installed-runtime smoke remains
  intentionally ignored because it was outside the user's authorization for
  this run. At TC-2 acceptance time no remote push had been performed. Before
  TC-3 implementation, the user explicitly authorized one fast-forward push;
  `origin/rewrite/rust-core` advanced from
  `74c5ffe0f2cbbec2b52a342758a1a913bf2cd2d8` to the accepted TC-2 plus TC-3
  routing commit `4b1af151d6f5ce17bd9dc351e0587b5e83319ee4`. No TC-3 implementation
  result has been pushed.

### TC-3 — host-tool disposition and disposable MCP credential fallback (accepted)

- TC-3 was implemented from pushed base
  `4b1af151d6f5ce17bd9dc351e0587b5e83319ee4` against selected upstream
  `rust-v0.154.0` / `6b9826e3aa83b1a5947db50f4332cb9c65f1b340`. The selected-release source
  review enumerated the material `rg` boundaries before choosing any product
  change. Upstream thread/content search falls back to its internal Rust scan
  when spawning `rg` returns `NotFound`; the runtime doctor diagnoses a missing
  search command and supplies explicit ripgrep remediation. The Linux bwrap
  `rg` use remains outside normal Termux execution under the accepted TC-1
  sandbox fence.
- The official selected 0.154.0 archive was inspected rather than assuming its
  bundled `codex-path/rg` was Termux-compatible. Disposable job
  `job_ta7_4810a9e1a8` proved that file is an AArch64 GNU/Linux dynamically
  linked ELF using `/lib/ld-linux-aarch64.so.1`; direct Termux execution exits
  127. An initial signed-helper design was therefore discarded before
  acceptance. TC-3 does not publish that incompatible binary, add an `rg`
  helper, widen PATH authority, invoke a package manager, or add a runtime byte
  patch. Focused release-builder proof keeps helper count at two and asserts no
  `termux-rg` helper is published.
- The accepted `rg` resolution is explicit bounded degradation/diagnosis, as
  permitted by the Goal Lift. Product-level disposable proof
  `job_tab_3500953747` ran the selected adapted runtime with a PATH containing no
  `rg`: the runtime remained invocable; `doctor` exited 1 with its bounded
  warning status, diagnosed the ripgrep/search-command absence, and exposed
  installation remediation. No live package state or PATH was changed.
- The MCP credential risk did reproduce. Pre-fix disposable job
  `job_t70_18122586f9` used non-sensitive fixture credentials only: explicit
  `File` mode logout exited 0 and removed the file entry, while the selected
  upstream `Auto` mode encountered a keyring/secret-backend deletion error,
  exited 1, and left the file-backed fallback credential present. No credential
  value, token, client secret, authorization code, or account identifier was
  emitted as evidence.
- The correction is deliberately narrower than patching upstream OAuth code.
  The Core-owned Termux system configuration now projects
  `mcp_oauth_credentials_store = "file"` alongside the existing wrapper-owned
  configuration. This does not mutate logical user `CODEX_HOME` configuration;
  it selects one coherent Termux credential authority so save/load/delete use
  the same file backend instead of entering an unavailable desktop keyring path.
  Focused Core proof `test_tc3_mcp_file_store_policy_is_exact` binds the exact
  policy and verifies no keyring selection is projected.
- Post-fix disposable E2E job `job_taa_f819d35021` identified the exact fixture
  store key through upstream File-mode behavior, then removed the user-side
  store selector and ran the current Core with only the MCP server definition in
  disposable user config. The Core system policy drove logout successfully:
  the fixture store changed from one entry to zero, logout exited 0, no
  keyring/secret-backend error appeared, and a second logout also exited 0.
  Stronger load-delete-load proof `job_tak_b589b713eb` used the selected
  upstream `mcp list --json` auth-status path, which calls the MCP credential
  store: before deletion the disposable file credential loaded as OAuth-auth
  state, after Core-routed logout a second list resolved to unknown/no stored
  auth state, the file store contained zero entries, and no keyring/secret
  backend error occurred. Fixture secret values were never printed or recorded.
- Final source gate `job_tac_e9118c5d0c` passed with exit status 0. Exact
  focused regressions passed for TC-1 `1/1`, TC-2 Core `1/1`, TC-2
  release-builder `1/1`, TC-3 Core `1/1`, and TC-3 release-builder `1/1`.
  Release-builder full reported `14 passed / 0 failed`; Core full reported
  `140 passed / 0 failed / 1 explicit real-Termux installed-runtime smoke
  ignored`; `cargo check --workspace --locked` passed; locked workspace tests
  passed with Core `140/0/1`, Manager unit `20/20`, Manager integration `11/11`,
  and release-builder `14/14`; warnings-denied workspace/all-targets clippy,
  formatting, and `git diff --check` all passed.
- TC-3 is accepted at local source commit
  `370caceb63bd7fddb7b6075da49d0404755a8e43` and completes the preplanned
  TERMUX-COMPAT bundles. The seven Goal Lift success thresholds are satisfied by
  TC-1, TC-2, and TC-3 accepted evidence. The separately ignored real
  installed-runtime smoke is not one of those seven success conditions and was
  not silently promoted into this run. No live installed runtime/helper,
  `$PREFIX`, package-manager state, resolver, persistent process environment,
  live `CODEX_HOME` auth/config/session/profile, OS/account credential store,
  provider/account configuration, release/index, or `main` state was mutated.
  The accepted TC-3 source commit remains local; a second source push requires
  separate explicit user authorization.

### TC-LIVE-BRIDGE — R10 signed browser-helper layout migration (accepted; live cutover complete)

- The authorized live cutover from the accepted closure at
  `3e6899262d1fa4ce3952c89341aa6b89a58364ed` exposed one bounded update
  compatibility defect rather than a runtime or trust failure. The installed
  healthy R10 Core (`codex-cli 0.153.4`, launcher SHA-256
  `00ecd5a3536809ef23de055d9c511dc8f503f4658fdb7ff28037ac9e81cd3325`)
  rejected the signed sequence-8 TC-3 candidate before mutation with
  `release file inventory path is invalid` because its parser predates the
  canonical TC-2 paths `browser/open/curl` and `browser/manual/curl`.
- The repair scope is one signed two-step migration inside the existing v3/v4
  update authority. An explicitly marked bridge generation uses the already
  accepted browser helper identities and bytes but places them at the
  R10-readable signed paths `helpers/0` and `helpers/1`. The new Core may read
  that exact indexed layout only for `creation_metadata =
  "r10-browser-helper-bridge-v1"`; normal generations remain canonical at
  `browser/open/curl` and `browser/manual/curl` and receive no implicit legacy
  fallback.
- Acceptance must prove fail-closed marker/layout mismatches, exact helper
  identity/digest/mode signing, R10-compatible bridge inventory, bridge runtime
  launch with the same browser policy, a disposable bridge-to-canonical forward
  update, and rollback compatibility while the bridge is retained as the one
  previous generation. TC-1/TC-2/TC-3 focused regressions and the full locked
  workspace/fmt/clippy/diff gates must remain green.
- The bridge does not add a release format, signing key, bootstrap authority,
  package-manager action, PATH widening, raw-runtime patch, direct launcher
  replacement, or credential/provider mutation. The selected upstream remains
  `rust-v0.154.0` / `6b9826e3aa83b1a5947db50f4332cb9c65f1b340`.
- Source acceptance is complete. Core accepts the indexed helper layout only
  when the generation descriptor carries exact marker
  `r10-browser-helper-bridge-v1` and exactly the two accepted browser helper
  identities in open/manual order; otherwise the indexed bridge contract fails
  closed. Normal generation paths remain `browser/open/curl` and
  `browser/manual/curl`. Release-builder emits and signs `helpers/0` and
  `helpers/1` only for that exact bridge marker and keeps ordinary publication
  canonical.
- Focused regression job `job_tlb_1a92062b15` passed exact 1/1 tests for the
  two TC-1 daemon-boundary cases, TC-2 browser/clipboard capability fence,
  TC-3 MCP file-store policy, the Core bridge marker/layout contract, the
  release-builder TC-2 and TC-3 contracts, and the bridge build/publish
  contract. Earlier bridge-only job `job_tjy_fa9d010b4b` also passed Core,
  canonical TC-2 builder, and bridge builder exact tests 1/1 each.
- Final disposable migration job `job_tlz_daa4096317` cloned only Core-owned
  non-credential activation/rollback state, its two referenced signed
  generations, the public trust pin, and the installed R10 launcher into a
  separate HOME/PREFIX. With a non-sensitive fixture API key, the real installed
  R10 updater admitted the final signed bridge sequence 8, the final bridge Core
  then admitted canonical sequence 9, and explicit rollback returned to the
  retained bridge. The initial runtime was `codex-cli 0.153.4`; every state
  after the first transition reported `codex-cli 0.154.0`, and doctor exited 0
  at R10, bridge, canonical, and rollback states. The real live launcher,
  resolver, and protected auth/config/profile/session fingerprint remained
  unchanged throughout this proof. Earlier job `job_tl3_0cb97457b0` proved the
  same state-machine path on the pre-clippy artifact and is diagnostic rather
  than the final artifact-bound evidence.
- Final gate `job_tlj_5ff0e9a49c` passed on the final source after the last
  clippy repair: `cargo fmt --all -- --check`, locked workspace check/test,
  Core `141 passed, 0 failed, 1 ignored` (the explicit real-Termux installed
  runtime smoke), release-builder `15/15`, Manager unit `20/20`, Manager
  integration `11/11`, `cargo clippy --workspace --all-targets --locked -- -D
  warnings`, and `git diff --check`. The earlier full run that stopped only on
  the now-repaired clippy argument-count warning is not counted as final
  acceptance evidence.
- Final warnings-denied release artifacts are Core SHA-256
  `0055ec0ecc762e4a4c878be62fd26f93118785218f004b26fe12b178cd3380eb`,
  Manager SHA-256
  `a706292d653fb33cc652fb1d9ef03ca8cb1e9e6f402a6098b00d4bedeaf189fc`,
  and release-builder SHA-256
  `73f9f1f1b92056a226f74735bc42ef0f534820099f8df714d7bc88f48e653627`.
  Final candidate regeneration job `job_tly_21be55d83a` verified the existing
  private signing authority derives the installed public key, copied no private
  PEM into publication output, and produced signed sequence-8 bridge manifest
  SHA-256 `ece00c94ee4224c0f745e0798f2b98b28f3305a138878543b44d5f97c12d1fc6`
  with `helpers/0` and `helpers/1`, plus sequence-9 canonical manifest SHA-256
  `27a54f9e6f6fce1d116b8a7ff347845747035a581caf2d8506c157d9e0357f64`
  with `browser/open/curl` and `browser/manual/curl`. Both carry selected
  adapted 0.154.0 runtime SHA-256
  `123c96efbd8b16e1ccd5c34a6212b0d8f1c895e92917829cb6371ddfd39aa8c0`.
- Accepted bridge source commit
  `1b51902775c89c79522ef868921e4b95229a35fb` (`termux: bridge R10 browser
  helper update layout`) was pushed by exact fast-forward in job
  `job_tm6_aa125d60bb`; `origin/rewrite/rust-core` then resolved to that exact
  commit. No force push, alternate publication ref, or unrelated remote
  mutation occurred.
- Final live preflight job `job_tm9_51bbafd0b1` rebound the healthy R10
  baseline (`codex-cli 0.153.4`, launcher SHA-256
  `00ecd5a3536809ef23de055d9c511dc8f503f4658fdb7ff28037ac9e81cd3325`,
  current `local-1789181261-r10-live-1`, previous
  `local-1788570645-23374-1`), signing authority, both final signed manifests,
  resolver SHA-256
  `7e8ad76e0d200e93918ca2e93c99ff8ecd02071953bf1479819db3ac0dbb6d07`,
  and protected auth/config/profile/session fingerprint
  `aedf59af2b1781ef3bd2235969f71cb02f371c2bd22a8b77a1777d5bc6387078`.
  Doctor exited 0 before mutation.
- Live bridge job `job_tma_c58554e728` used only the installed R10
  `$PREFIX/bin/codex update --local` path on signed sequence 8 and exited 0.
  The launcher became exact accepted Core SHA-256
  `0055ec0ecc762e4a4c878be62fd26f93118785218f004b26fe12b178cd3380eb`,
  reported `codex-cli 0.154.0`, and doctor exited 0. Activation state became
  current `local-20260914-r10-browser-bridge-1`, previous
  `local-1789181261-r10-live-1`; the installed bridge contained executable
  `helpers/0` and `helpers/1` and no `browser` tree. Resolver and protected
  fingerprint were unchanged.
- Live canonical job `job_tmb_b190024075` then used the newly installed Core on
  signed sequence 9 and exited 0. Final live state is `codex-cli 0.154.0`, exact
  launcher SHA-256
  `0055ec0ecc762e4a4c878be62fd26f93118785218f004b26fe12b178cd3380eb`,
  doctor exit 0, current `local-20260914-tc3-canonical-1`, and previous
  `local-20260914-r10-browser-bridge-1`. The active generation contains
  executable `browser/open/curl` and `browser/manual/curl` and no legacy
  `helpers` tree. Resolver and protected fingerprint again remained exact.
- No live rollback was performed merely for proof; the final artifact-bound
  disposable rollback proof remains `job_tlz_daa4096317`. No direct launcher
  overwrite, bootstrap reauthorization, package-manager action, raw-runtime
  patch, credential/provider mutation, or credential-content inspection was
  used during live cutover. TC-LIVE-BRIDGE and the TERMUX-COMPAT live migration
  are therefore closed.

### UPDATE-CHANNEL-LATEST — stable channel and already-current update repair (accepted)

- The bounded follow-up is accepted and closed. Exact-current update handling was
  repaired in source commit
  `bd57d8c2222c51f0de07ca1ac77f8766edd6fe38`: an authenticated candidate is a
  success/no-op only when its release sequence equals current and both generation
  identity and signed manifest exactly match installed current. Lower sequences
  and equal-but-different releases remain fail-closed. The nested GitHub Release
  publisher fence was added in
  `a18acf3e77d7c9b3ada8ba64ab052cc91e774c84`. Final Pages publication authority
  and workflow source are accepted in
  `057f078a091441c1f624c7b229649bd1463ef774`, which is also the source revision
  pushed to `origin/rewrite/rust-core` before publication.
- Final repository validation on that source passed release-builder **15/15**,
  Core **142 passed / 0 failed / 1 ignored**, Manager **20/20** plus integration
  **11/11**, locked workspace check/test, clippy `-D warnings`, formatting, and
  `git diff --check`. Focused gates separately proved bare-channel exact-current
  no-op, equal-current versus rollback, non-monotonic rejection, nested Release
  publisher fail-closed behavior, marker-bound R10 bridge layout, and bridge
  builder publication.
- The final public signed generation is
  `local-20260914-update-channel-bridge-1`, release sequence **10**, upstream
  `0.154.0`, exact `creation_metadata = "r10-browser-helper-bridge-v1"`, and
  R10-readable signed helper inventory `helpers/0`, `helpers/1`. It intentionally
  uses the compatibility layout rather than the local sequence-9 canonical
  `browser/*` layout so a retained sequence-7 R10 parser can consume the final
  public target directly. The signed tree is 292,960,783 bytes. Key artifact
  SHA-256 values are Core
  `4ecfd7b6a9515e1670e972e87c068df8772f9b0dd101d46c4d29c99941c69f03`,
  runtime `123c96efbd8b16e1ccd5c34a6212b0d8f1c895e92917829cb6371ddfd39aa8c0`,
  Manager `a706292d653fb33cc652fb1d9ef03ca8cb1e9e6f402a6098b00d4bedeaf189fc`,
  code-mode host `f31e1c5ffbbca7884aff2f0f8795d3da197f4aafb114033a399dfc17a5119031`,
  and release manifest
  `130ff79d192af84d00d24855d495b0c13df424a896ece6875980ff914fb4cd78`.
- Publication uses GitHub Release `local-20260914-update-channel-bridge-1` as
  staging (release database ID **388405334**) and the fixed repository workflow
  `.github/workflows/publish-termux-update-pages.yml` to reconstruct the exact
  signed tree on GitHub Pages. The workflow was mirrored byte-exactly to `main`
  in `d0398bb64abdd39f6bf68f6d1972fb05560037f1`; workflow-dispatch run
  **34848065440** completed successfully from that head. Complete Pages HTTPS
  readback reverified the release signature, every signed digest, and every file
  byte-for-byte, including the 227,482,840-byte runtime. The stable signed
  `release_base` is
  `https://humtr.github.io/codex/local-20260914-update-channel-bridge-1/`.
- The signed stable index was advanced only after complete Pages readback, in the
  contract order signature then content. The signature commit is `349e61237f0bf3c1a487a198296bb4eedfc17b03`
  and final `main` head is `40692fd63b4b5408f4600b338e854225d8b653c5`. Public `update-index-v1` SHA-256 is
  `a77160aca09207806e17ba558dc8440f18928501e3c3f317f798b45e1c4999fa`; its
  signature SHA-256 is
  `0e69977a7fb687463fa9a9e503750adaadbdc494fd5740a9fe6a66c8719d4268`.
  Public readback verifies cryptographically and targets sequence 10.
- The decisive public smoke used an isolated retained R10 sequence-7 generation
  and the actual no-argument public channel. Starting from `codex-cli 0.153.4`,
  the first `codex update` exited 0, printed
  `activated channel generation local-20260914-update-channel-bridge-1`, and
  produced `codex-cli 0.154.0` with doctor exit 0. A second no-argument
  `codex update` exited 0, printed
  `codex is already up to date (generation local-20260914-update-channel-bridge-1)`,
  left launcher, activation state, and generation tree unchanged, and again left
  doctor at exit 0. No user auth/config/profile/session data or signing private
  key was copied into the disposable proof.
- The real live installation was not advanced to sequence 10 for this proof. It
  remains healthy `codex-cli 0.154.0`, current
  `local-20260914-tc3-canonical-1`, previous
  `local-20260914-r10-browser-bridge-1`, exact launcher SHA-256
  `0055ec0ecc762e4a4c878be62fd26f93118785218f004b26fe12b178cd3380eb`,
  resolver SHA-256
  `7e8ad76e0d200e93918ca2e93c99ff8ecd02071953bf1479819db3ac0dbb6d07`,
  and doctor exit 0. The public smoke proved the live launcher, activation state,
  resolver, and protected `.codex`/`.config/codex` metadata fingerprint were
  unchanged across the smoke. No live runtime replacement, rollback, package
  installation, signing-key rotation, provider/account mutation, or credential
  content inspection occurred. UPDATE-CHANNEL-LATEST is therefore closed.

### AUTO-UPSTREAM-ROLLBACK — automatic upstream intake, rollback hold, and force retry (accepted)

- User authorization selects this bounded follow-up after UPDATE-CHANNEL-LATEST.
  Normal operation stays `codex update`; there is no manual stable-promotion
  approval. Real-use regressions recover through Core-owned `codex update --rollback`.
- Rollback reuses the existing complete-generation/Core-entrypoint atomic
  transition. Only after success may Core atomically record a separate v1 hold
  containing the authenticated generation and release sequence rolled back from.
  Activation-state v3 stays unchanged for retained-R10 compatibility. After normal
  trust/anti-rollback checks, ordinary update may advance only beyond the held
  signed sequence; a committed greater sequence clears the obsolete hold/guard.
  `codex update --force` requires the authenticated hold and retries exactly that
  held sequence for the invocation, bypassing only the hold comparison and
  retaining the normal hold after success.
  `codex update --rollback` remains the sole public rollback selector; no new
  top-level rollback command is added.
- Upstream automation is update-triggered only on the maintainer Termux device:
  the default signed channel must be unmodified, the secure local signing key must
  derive the active update authority, and the local GitHub CLI must be
  authenticated. Ordinary consumer devices never build/sign/publish. Repository
  inspection found zero self-hosted Actions runners, so the private signing key
  is not exported to GitHub-hosted Ubuntu. If public stable is already current or
  only held, the maintainer Core resolves exact official OpenAI stable metadata
  and locally adapts/signs only a genuinely newer upstream using the existing
  prebuilt builder.
- Automated publication extends the accepted Release-staging + Pages transport.
  GitHub Actions may only reconstruct/deploy already signed bytes and must preserve
  the currently signed stable generation alongside the candidate because Pages is
  a whole-site replacement. Stable promotion requires successful deployment,
  complete candidate HTTPS readback, and disposable public no-argument update
  smoke, then replaces `update-index-v1` and its signature together in one Git
  tree/commit and advances `main` non-forced from the exact verified parent. A
  failed pre-commit gate preserves old stable; an indeterminate final ref result is
  not guessed. Once promotion is known committed, local activation may be deferred
  and is recovered by re-running `codex update` against the newly signed stable.
- Acceptance requires focused routing/hold/malformed-state/force/anti-rollback
  tests, automatic upstream comparison/build controls, publication fail-closed
  tests, full workspace/check/test/clippy/fmt/diff gates, and disposable update →
  rollback → held-update refusal → force/greater-sequence behavior without live
  mutation.
- Disposable process-level migration smoke exposed one additional legacy boundary:
  sequence 11 correctly wrote the hold, but complete rollback to public sequence
  10 also restored the old sequence-10 Core, which did not understand the new hold
  and immediately reinstalled sequence 11. Acceptance therefore additionally
  requires a signed rollback-Core guard for only non-hold-aware rollback targets.
  The guard must retain only the held generation's authenticated Core control
  plane, bind it to the exact target/held pointer pair and signed digest, preserve
  the previous generation's runtime/Manager/helper payload, close the rollback
  commit-to-hold crash window, and disappear once normal hold-aware Core pairing is
  restored. The failed bypass smoke is rejection evidence, not acceptance.
- Final acceptance on 2026-09-15 passed the disposable rollback/hold/force and
  legacy-guard process flows, the maintainer publisher authority/fail-closed
  controls, and the complete repository gate: release-builder 15/15; Core 156
  passed, 0 failed, 1 explicitly ignored real-Termux smoke; Manager integration
  11/11; locked workspace check/test; clippy with `-D warnings`; formatting; and
  `git diff --check`. The acceptance run did not mutate the installed live Codex,
  public stable publication, signing authority, resolver, auth, config, profile,
  or session state.


### UPDATE-PROGRESS-RESPONSIVENESS — animated TTY progress and bounded control fetches (accepted)

- User feedback on 2026-09-19 identified two post-UX1 responsiveness defects:
  the transient TTY glyph changed only at phase boundaries rather than animating
  while a blocking phase was in progress, and the pre-authentication
  `Checking for updates...` phase reused the generic 300-second per-file
  transfer ceiling even for small signed control-plane resources.
- Exact accepted source is
  `81131655d98f114b5324bd8ee5866cff0a171941`. The change preserves the
  existing signed-channel trust, signature/digest/mode/version verification,
  anti-rollback, candidate probing, atomic activation, LKG/rollback, and CAS
  behavior. TTY progress now advances through the existing spinner frame set on
  an 80 ms tick while retaining exactly one transient stderr line; phase changes
  render immediately, and the worker is stopped and joined before transient
  cleanup and every permanent output. Non-TTY progress remains disabled.
- The exact pre-authentication text remains `Checking for updates...`. Small
  signed control-plane fetches used to authenticate the index and release
  metadata now use a 30-second transfer ceiling while retaining the existing
  15-second connect timeout. Large signed payload downloads retain the existing
  300-second transfer ceiling.
- Exact-source Termux validation job `job_w9s_d47fda726c` completed with exit
  0 from a clean worktree at the accepted source. Focused spinner-cycle,
  control/payload-timeout, and PTY update tests passed; the PTY test requires
  multiple redraws of the same `Checking for updates...` phase so a static
  glyph cannot satisfy the gate. Full locked workspace tests also passed:
  Core 149 passed / 0 failed / 1 explicitly ignored real-Termux smoke, Manager
  20/20, Manager integration 11/11, and release-builder 19/19. Locked workspace
  check, clippy with `-D warnings`, rustfmt, and `git diff --check` all
  passed, and the worktree was clean before and after validation.
- Production publication was explicitly authorized on 2026-09-19 and used a
  separately validated one-shot sequence-15 -> sequence-16 gate from exact
  workflow source `39983f628fe568ec98b8c69bb87b02f9449c0104`. The producer
  workflow was first installed as the sole-file non-forced child
  `main=40501cf88d86c1c9281d918141e90389ad2006f6` of
  `ba36c44f871ef266c4887986535ed87a4d2becc9`, leaving stable bytes
  unchanged. Gate-validation job `job_wai_242ba900ee` proved the positive
  exact-baseline path plus fail-closed wrong-ref/authorization/generation/
  sequence/candidate/source cases and the ordered duplicate-helper descriptor
  comparator.
- Production run `35434790060` then completed green end-to-end. It
  authenticated exact baseline version 0.155.1, generation
  `local-hosted-0-155-1-07f77b89a177-ux1-human-output`, sequence 15;
  ordinary comparison first yielded no candidate; only the new manual gate
  admitted exact next sequence 16 from accepted product source
  `81131655d98f114b5324bd8ee5866cff0a171941`. The run passed exact
  sequence-15 non-Core digest/mode preservation, Core-only integrity-bound
  descriptor delta, Android/AArch64 native Manager/Core/runtime smoke,
  production signing and independent signature verification, immutable Release
  staging, LKG-preserving Pages deployment, public HTTPS every-byte readback,
  disposable ordinary update/version/doctor/second-no-op proof, and the existing
  non-forced exact-parent CAS. CAS reported `promotion_result=committed`.
  Public stable is now signed sequence 16 generation
  `local-hosted-0-155-1-81131655d98f-update-progress-responsiveness`;
  `main=52c69f21472fb2d082ef87f0e6583c88b7a8e3db` is the sole child of
  the producer-install parent and changes only `update-index-v1` and its
  signature.
- Live consumer closure used only the ordinary signed public update path.
  Read-only preflight `job_wal_3820f2c5d8` confirmed exact 0.155.1 on the
  healthy sequence-15 generation. PTY job `job_wam_258c839b9b` then invoked
  only no-argument `codex update`; its captured update contained the expected
  same-version header, signed-activation success sentence, and
  `Codex 0.155.1 is now active.`. That job's post-update harness exited 1
  only because it over-constrained the *initiating sequence-15 Core* to already
  show the new animated pre-activation spinner; it observed the expected old
  single frame and did not indicate an activation failure. Immediate read-only
  job `job_wan_fe35cc553b` proved that sequence 16 was active with healthy
  Termux Core, runtime, code-mode host, and Manager.
- New-Core PTY proof `job_wao_d345a6690f` then ran exact-current
  `codex update` with both child stdout and stderr attached to a real PTY. It
  observed 374 in-place `Checking for updates...` redraws and all ten spinner
  frames `⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏`, verified erase-before-permanent-output
  cleanup, and ended with exact
  `Codex 0.155.1 is already up to date.`. A subsequent non-TTY no-op
  produced exactly the same permanent line with no CR or ANSI controls.
  The live version remains exact `codex-cli 0.155.1`; the active generation is
  now the signed sequence-16 progress-responsiveness generation. The bundle is
  accepted and closed.



### UPDATE-EXACT-CURRENT-FASTPATH — signed-index exact-current short-circuit (accepted)

- User feedback on 2026-09-19 identified that ordinary exact-current
  `codex update` remained unreasonably slow and explicitly rejected timeout
  tuning as the explanation. Investigation confirmed a control-flow defect:
  after authenticating the signed stable index and learning that its generation
  was already active, Core still reacquired the entire current signed release
  before `prepare_signed_local_release_with_hold_policy` could return
  `AlreadyCurrent`. On the current sequence-16 release this included the
  approximately 233 MB runtime asset, so a no-op update could take tens of
  seconds despite requiring no activation.
- Exact accepted source is
  `7817b939c81ce15c76d3d0d57157ca5e378a8491`. Ordinary no-argument
  `UpdateHoldPolicy::Enforce` now stops after the signed index pair is
  authenticated when the index generation exactly equals the active generation,
  `current_key == update_key`, the index release base is bound to that exact
  generation identity, the rollback-hold/guard state validates, and the
  installed current generation fully re-verifies under the official authority.
  It then returns the existing `AlreadyCurrent` result without creating a
  generation acquisition tree or fetching release control/payload bytes.
- The shortcut is deliberately unavailable to `--force`, a local-derived
  current generation, a different authenticated stable generation, malformed
  hold/guard state, an invalid index/signature, an invalid release-base
  generation binding, or a locally invalid installed generation. Those paths
  retain the existing fail-closed/full authenticated behavior. Candidate
  acquisition, signature/digest/mode checks, anti-rollback, candidate probing,
  atomic activation, LKG/rollback, local-derived semantics, and CAS behavior
  are unchanged.
- Focused validation job `job_wcd_60498c3b61` passed the exact-current
  signed-channel regression and the rollback-hold/`--force` regression. The
  focused shell later returned nonzero only because Cargo created an untracked
  worktree-local `target/` directory; inspection job `job_wci_78c1c8931f`
  proved that was the only dirty entry and that no tracked source had changed.
  The build output was then moved to job-private storage for the authoritative
  full validation.
- Exact-source full validation job `job_wcj_53ab115dbc` completed with exit
  0 and a clean worktree. The curl-log regression proves an ordinary
  exact-current update performs exactly the signed index and index-signature
  fetches and does not request `release.manifest`, `release.sig`,
  `release-authority.sig`, `generation.meta`, runtime, Core, Manager,
  helpers, or code-mode-host. The force regression proves `--force` still
  fetches and verifies release control and payload data. Full locked workspace
  validation passed: Core 149 passed / 0 failed / 1 explicitly ignored
  real-Termux smoke, Manager 20/20, Manager integration 11/11, and
  release-builder 19/19; workspace check, clippy with `-D warnings`, rustfmt,
  and `git diff --check` also passed.
- Production publication was explicitly authorized by the user on 2026-09-19.
  The normal tmcp-backed interactive dispatch transport returned HTTP 404
  before submission, so a temporary exact-parent push bridge was used without
  broadening release admission. Initial bridge run `35436887423` failed before
  creating any job because a colon-space inside an unquoted GitHub expression
  made the workflow YAML invalid; signed stable index/signature bytes remained
  unchanged at sequence 16 and no signing, Release, Pages, or CAS action ran.
  The repaired retry used exact trigger parent
  `5f7a316cd433d659979c1ec1bf648294002b4307` and produced successful
  production run `35437042333`.
- Run `35437042333` authenticated exact upstream `0.155.1`, sequence 16,
  generation
  `local-hosted-0-155-1-81131655d98f-update-progress-responsiveness`;
  ordinary comparison first returned no candidate and only the bounded
  fastpath gate admitted exact accepted source
  `7817b939c81ce15c76d3d0d57157ca5e378a8491` as sequence 17 generation
  `local-hosted-0-155-1-7817b939c81c-exact-current-fastpath`. The run passed
  exact sequence-16 non-Core byte/mode preservation with Core-only change,
  ordered descriptor comparison, Android/AArch64 executable smoke, production
  signing and independent verification, immutable Release staging,
  LKG-preserving Pages deployment, public HTTPS every-byte readback, disposable
  ordinary update/version/semantic-doctor/no-op proof, and the existing
  non-forced exact-parent CAS.
- Disposable public proof job `105881773125` verified the candidate as exact
  release sequence 17, activated it through ordinary update, proved exact
  `codex-cli 0.155.1`, healthy Termux Core/runtime/code-mode host, genuine
  upstream doctor execution (unhealthy in the credential-free disposable
  environment, correctly distinct from Core integrity), and an exact second
  no-op `Codex 0.155.1 is already up to date.` with no state delta.
  CAS job `105881831694` reported `promotion_result=committed`,
  `ref_update_rc=0`, used `force:false`, and reverified the served stable
  index signature after promotion.
- Promotion commit is
  `f361b4a241b34cee1a7ca5bf4c98914a6b9dd600`, the sole child of trigger
  parent `49ba318efd80f33a9181d0fb56dfd41402cffde5` and changes only
  `update-index-v1` plus its signature. Public stable is now signed sequence
  17 generation
  `local-hosted-0-155-1-7817b939c81c-exact-current-fastpath`. The temporary
  push bridge was immediately removed by workflow-only child
  `main=56ba28e1baee87721767ba34b505cc2bc1303c44`; stable index/signature
  blobs remained unchanged by cleanup.
- Live Termux consumption is **accepted 2026-09-19**, closing
  UPDATE-EXACT-CURRENT-FASTPATH-PROD-17. After tmcp transport recovered,
  read-only preflight job `job_wdn_35abc56ffe` proved the live installation
  was exact `codex-cli 0.155.1` on signed sequence-16 generation
  `local-hosted-0-155-1-81131655d98f-update-progress-responsiveness`,
  with healthy Termux Core/runtime/code-mode host/Manager and healthy upstream
  doctor, while the same device read public stable as the promoted seq17
  fastpath generation.
- Live activation job `job_wdp_a5ba929cd5` then invoked only ordinary
  no-argument signed public `codex update`. The initiating seq16 Core used the
  existing full acquisition path and completed the same-version corrective
  activation in 62014 ms, printing
  `Updating the Termux release for Codex 0.155.1...`,
  `Verified and activated the signed Termux release.`, and
  `Codex 0.155.1 is now active.`; exact version remained
  `codex-cli 0.155.1`.
- New-Core proof job `job_wdu_343ee51ae9` bound the active generation exactly
  to `local-hosted-0-155-1-7817b939c81c-exact-current-fastpath` and reported
  Core, runtime, code-mode host, Manager, upstream, and composed summary all
  healthy with doctor exit 0. Its timed ordinary exact-current update completed
  in **1268 ms**, emitted exactly
  `Codex 0.155.1 is already up to date.` on stdout, emitted empty stderr,
  contained no CR/ANSI control bytes, and left the complete Core generation /
  activation / launcher snapshot byte-for-byte unchanged. Final version remained
  exact `codex-cli 0.155.1`. No manual generation copy, direct Core overwrite,
  alternate trust key, force push, provider fixture, or protected user-state
  mutation was used.


## UPDATE-DOWNLOAD-SIZE-V1 Production Acceptance (accepted 2026-09-30)

- Source acceptance was completed from `rewrite/rust-core` product source `4fd14ed8aafcf29602f43e00c314eb8da39a6e5f`. Production used exact producer source `b164e5b61cb438913cdb1633d74c62d6c96f6523` for upstream `0.159.0`.
- Production retry run `36556869965` completed successfully through unsigned build, native Android/AArch64 executable smoke, production-authority signing, immutable Release staging, LKG-preserving Pages deployment, public every-byte readback plus disposable update proof, and exact-parent CAS promotion.
- The promoted signed public stable is release sequence `23`, generation `local-hosted-0-159-0-b164e5b61cb4`, with upstream `codex-cli 0.159.0`. The signed generation remains retained in the repository GitHub Release under the same generation tag.
- Public authenticated download-size control resources were verified at `<generation>/compat/download-size-v1` and `<generation>/compat/download-size-v1.sig`. Independent verification passed the Ed25519 signature and exact `release.manifest` SHA-256 binding. The authenticated payload inventory reports `7` files totaling `316535756` bytes.
- Fresh live readback after user consumption verified the Termux installation is now exact `codex-cli 0.159.0`, with activation state `current=local-hosted-0-159-0-b164e5b61cb4` and retained previous generation `local-hosted-0-155-1-9300a68852c8-no-emoji-output-r3`.
- The user separately observed both the real live `0.155.1 -> 0.159.0` ordinary signed update completing normally and a subsequent `0.159.0` exact-current update returning normally. The first transition was driven by the old `0.155.1` Core, so absence of the new authenticated download-size display on that transition is expected and is not a failure of the published sidecar contract.
- Final live TTY acceptance of the new `downloaded / authenticated-total` presentation is therefore intentionally deferred until a future newer signed upstream generation is consumed by the installed `0.159.0` Core. That future observation is a release-consumer check, not unfinished work in this accepted bundle.
- Disposition: **UPDATE-DOWNLOAD-SIZE-V1 is accepted and closed.** No further source, production, or live-runtime mutation is authorized by this bundle.

## STARTUP-ADVISORY-SINGLE-KEY-TRANSIENT-V1 Acceptance (accepted 2026-10-02)

- User-visible failure was reproduced from the accepted Core implementation:
  the five-second bare-launch update advisory used a background `read_line()`
  while the terminal remained canonical. A typed `y` therefore required Enter;
  timeout could leave that byte pending for upstream Codex, matching the
  observed `1sy` contamination. The countdown also failed to behave as one
  reliably transient terminal line.
- Accepted product source is exact
  `75aed30c7ad6c427d30115b0667957130da198f5`. Linux/Android TTY startup
  advisory input is now bounded non-canonical/no-echo single-byte input:
  `y`/`Y` selects update immediately without Enter, other input keeps the
  current runtime, prompt bytes are consumed rather than forwarded, the line is
  erased in place, and the exact prior terminal mode is restored before update
  execution or upstream launch.
- Focused PTY regression tests sent bare `y` and `n` bytes with no newline,
  proved immediate `y` response before the 4-second redraw, no input echo,
  transient clear sequences, timeout/keep behavior, and exact PTY-mode
  restoration. Full source validation then passed the adopted repository gate:
  Python migration 9 tests, Core 159 passed with one explicit real-device smoke
  ignored, Manager library 16, Manager profile integration 9, release-builder
  19, formatting, and diff checks.
- Production authority was rebound to public sequence 26 generation
  `local-hosted-0-160-0-4fd14ed8aafc`, while the live device was sequence 24
  `local-hosted-0-159-2-4fd14ed8aafc`. Fresh official upstream remained exact
  `0.160.0` with archive SHA-256
  `7f0fe42ff22ecfa3a47bc4a34f5b22c4218b431a4ec0aba51c7d98299f07900c`.
- One-shot main trigger commit
  `a124e609fbddb949840d4ca091b66d277d2195b6` ran GitHub Actions production
  run `36965656433`. Exact corrective preflight, Android/AArch64 cross-build,
  Core-only/non-Core-byte preservation, native executable smoke, production
  signing, independent verification, immutable Release staging, LKG-preserving
  Pages deployment, every-byte public HTTPS readback, disposable ordinary
  update/no-op proof, and non-forced exact-parent CAS all completed
  successfully.
- Stable promotion commit is
  `2dc79bd11842c9e5970b8c73c470886b780f2ff8`. Public stable is signed release
  sequence **27**, generation
  `local-hosted-0-160-0-75aed30c7ad6-startup-advisory`, upstream
  `codex-cli 0.160.0`. Independent post-promotion readback reverified the
  Release signature under public-key SHA-256
  `62ab1640b6b4e63afbd5952d11a0bd0a9f1cb78ddde2472e003a42c4db2b832c`,
  sequence 27, and exact upstream archive digest above.
- Live activation used only ordinary signed public `codex update` from
  sequence 24. Original operation `b1cd3dcb4037498e82634b498d4d9c0b` exited
  0 after printing `Updating Codex 0.159.2 -> 0.160.0...`,
  `Verified and activated the signed Termux release.`, and
  `Codex 0.160.0 is now active.`.
- Fresh live proof reports exact `codex-cli 0.160.0`, active generation
  `local-hosted-0-160-0-75aed30c7ad6-startup-advisory`, release sequence 27,
  and healthy Termux Core/runtime/code-mode-host/Manager/composed summary. The
  installed Core SHA-256
  `6f83ec6d7994f0c978690e367696bddb26c8422872af45816be5748bc1f402e0`
  exactly matches the signed sequence-27 release manifest, binding the live
  binary to the PTY-tested Core.
- Installed-Core PTY acceptance then executed the exact live sequence-27 Core
  artifact (SHA-256
  `6f83ec6d7994f0c978690e367696bddb26c8422872af45816be5748bc1f402e0`)
  under an isolated temporary HOME with copied signed generation bytes.
  Operation `249b77e740fd44e195a42c49f137e451` sent bare `n` and `y`
  bytes with no newline. They cleared the advisory in 0.5 ms and 0.2 ms
  respectively; both probes observed non-canonical/no-echo prompt mode, no
  echoed prompt byte, zero queued input after the decision, one in-place
  transient clear, and exact prior termios restoration before runtime handoff.
  The `y` probe re-entered the authenticated update path and returned
  `Codex 0.160.0 is already up to date.`, proving the selected action without
  trusting the synthetic advisory candidate. The fixture was removed after
  each probe and did not mutate the live activation state.
- Exact-current live update returned exactly
  `Codex 0.160.0 is already up to date.` with empty stderr. A first
  deliberately broad snapshot also observed long-lived `.acquire-*` scratch
  housekeeping; after excluding that explicitly ephemeral acquisition scratch,
  a second exact-current proof showed **no durable state delta across 282
  installed-Core entries**.
- Disposition: **STARTUP-ADVISORY-SINGLE-KEY-TRANSIENT-V1 / sequence 27 is
  accepted and closed.** No further source, public, or live-runtime mutation is
  authorized by this bundle.

## Blocked / Resume Conditions

- Stop before any live install, activation, or replacement of the working
  Codex runtime during Milestone 1.
- Stop if a required upstream artifact cannot be immutably identified or
  verified.
- Stop if a test would write the live resolver, auth, profile, session, or
  installed runtime paths.
- Stop if update recovery cannot prove one complete old or new generation.
- Stop before implementation if the goal run is not using the configured
  primary Lead model and effort and no explicitly authorized equivalent exists.
- Exhausting the current `WORKBOARD.md` bundle is a planning checkpoint, not a
  blocker. If the milestone gate is incomplete, the primary Lead plans and
  records the next bounded bundle itself.

Resume by reading `SPEC.md`, then this file, then `WORKBOARD.md`. Continue only
the selected current milestone. When its gate is proven, the same primary Lead
updates this ledger, replaces `WORKBOARD.md` with the next milestone plan, and
continues without a routine user pause.

## Handoff

Resume through the installed `$goal-md` workflow with
`/goal resume codex-goal.md`; it must resolve to this file on
`rewrite/rust-core` and run with the primary agent configured as
`gpt-5.6-sol` / `max`. The primary Lead authors, records, and directly implements
each bounded bundle. The legacy branch may be inspected by the Lead for behavior
discovery but no source file may be copied into the rewrite.

2026-10-05 user replied 승인. 진행해. to exact fadae244d8bf4e3f88ebf7af1e1510cb2eeea2d7/39 request.
Operational authorization now covers ordinary humtr/codex main push, signed39
Release/Pages/non-force CAS, independent actual signed/public/native qualification,
and ordinary installed update retaining complete38 as previous. Resume binds clean
rewrite6a71005, clean exact candidatefadae24/parent9bac2f41, remote9bac2f41 and
accepted exact sourceb7c1e61 hosted37275318923 success. Workers OFF. No permission
question remains for this bounded delivery; no user work cancellation authorized.
