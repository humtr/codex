# Operating state and next work

Implementation: rewrite/rust-core. Publication: main's immutable source caller.
AGENTS.md owns development rules; SPEC.md owns product behavior and boundaries.
This repository carries no external workflow skill or model configuration.

## Operating baseline

- Public and installed stable52: local-hosted-0-161-0-e8dc2d76195f-s52,
  CLI0.161.0, accepted implementation e8dc2d7. Publication38733863985 passes all
  eight release stages, including Android execution, signing, complete public
  readback/disposable update and non-forced promotion. main caller ad3e723 pins
  that source; promotion149b30b preserves the caller and advances the signed index.
- Ordinary signed `codex update` activates stable52; repeat update is a no-op.
  Core/Manager doctor is healthy with no failures. All eight installed hashes,
  modes and the manifest signature verify; fourteen existing config/auth/provider
  files and seven current PID/start/image identities remain unchanged. Stable51
  is retained for rollback. Existing clients retain their loaded images until
  normal restart or frontend re-entry. Private proof: stable52-device-activation.json.
- The resolved-catalog Core/Manager path preserves the current production
  permission/profile/window behavior. Local full acceptance passes Core198/one
  existing ignored, Manager42+56+three path gates, builder25, Python18+59 and
  fmt/Clippy. Exact-source hosted acceptance38731009205 passes. Corrected native
  frontend38726029331 passes build/execution, all41 actual TUI tests, lint and
  pristine upstream Core/dependency proof. Raw frontend23e34867 is bound to
  immutable archive045a7055 under manager-tui-native-v0.161.0-c2db8f7; its installed
  FD-adapted SHA256 is7cfb36562d44777f6b68ee425eac7cdf3cec631a4e8f93912eb1a7ce0e556beb.
- Actual Android legacy/named/--profile menu round trips pass with that frontend
  and current Core, preserving PID/PTY/home/thread, permissions, config and sibling
  defaults. Catalog proof separately passes12-tool admission, byte-bound reuse/
  isolation, old-thread reads and Manager discovery. Both permission fixtures pass
  six real Android commands and six guardian reviews through launch/cold resume
  and /btw. These fixtures submit no Chat inference. Private aggregate:
  stable52-native-device-qualified.json. The failed first native compile exposed
  duplicate declarations, corrected in c2db8f7; the old permission observer queried
  only core/servers and now locates its exact owned fixture in both server families.
  Retain those negative results without treating observer failure as product proof.
- Installed opt-in ChatGPT provider is9014deaf810ff40a583d/sourcee27ab09,
  with durable `returnTransport=local`; all590 payload files verify and older
  payloads remain available for recovery. Actual installed signed-app + stable52
  Core passes tool discovery, exact namespaced MCP admission, real auto_review,
  one isolated stdio MCP read, result return, initial/intermediate/final reports
  and cold Core/TUI restore. GPT-6 Instant:2 actual user messages (main + guardian),
  2 tool-role returns,4 model starts,26.071s total, no Pro/retry. These are observed
  submissions, not Chat allowance deductions. Auth/endpoint and13 base config/
  auth/state files plus seven process identities are unchanged. The trial budget
  was removed and normal operation is ready. Private proof:
  chat-owned-mcp-installed-live.json and chat-tool-recovery-operating-final.json.
  Earlier stable51 two-command/cold-bootstrap proofs remain narrow history. The
  first-start failure and original user-trial rejected name remain unconfirmed.
- The independent MCP candidate remains unselected: the host still caches its
  old connector schema and has not qualified the new tools. Long cross-provider
  compaction is also unqualified; opaque native checkpoints are rejected before
  placeholder conversion rather than silently losing context. These are separate
  follow-ups, not completed by the catalog release. Same-Chat model changes still
  re-enter the frontend. Upstream0.162+ remains unqualified; this release intentionally
  retains0.161. Existing upstream title/history doctor warnings remain separate;
  user title/history data is not edited to conceal them.

## Chat provider work ownership

The [experiment current state and complete backlog](https://github.com/humtr/codex/blob/experiment/web-termux/CURRENT.md#active-remaining-work)
owns the twelve remaining Chat-provider work areas, deferred models/media/phone
presentation and their evidence boundaries. The current user-trial MCP tool-name
failure is reproduced at the adapter admission boundary. Canonical catalog,
old-round observation and completed-sample recovery fixes are now installed in
the opt-in provider, with native MCP live proof above. This remains experimental,
not a public stable Chat-provider promotion or full lifecycle qualification.
Core stable52 remains independently accepted; unrelated upstream/update/doctor
maintenance is separate from Chat acceptance.

## Core / Manager responsibility and update plan

Requested2026-10-16. This is a proposed implementation sequence, not an amendment
of SPEC, a new public command contract, or permission to silently remove existing
features. No runtime, generation, account or conversation has been changed by
writing this plan. Amend the affected SPEC sections before each implementation.

Priority confirmed2026-10-16: responsibility realignment precedes the other
feature/failure improvements. Complete the ownership/interface and relevant
implementation separation first, preserving the accepted baseline; a document-only
rename does not satisfy this priority. Do not grow the current coupling by first
adding provider setup tools, retention machinery, selection fixes or error recovery
to components that will relinquish that responsibility. Within a separation slice,
make only the behavior changes needed to preserve its existing public contract.
Then implement the deferred improvements in their established owners. The
experiment CURRENT owns that follow-up backlog and the new live failure evidence;
do not lose those tasks or treat their deferral as completion.

### Outcome and observed coupling

Core must support qualified upstream execution and updates without depending on
optional convenience development. Manager must retain account, task and Android
conveniences. The opt-in Chat provider must evolve without adding Chat-specific
branches to Core. Keep one public entrypoint and one signed activation/rollback
authority; no second updater, supervisor, conversation database or plugin framework.

First implementation slice separates qualification from publication. The read-only
qualify-termux-runtime.yml owns existing unsigned construction and Android smoke;
auto-release-termux.yml calls it only in integrated mode before signing. The new
core-runtime-qualification.yml requests base-only mode with no publication secrets
or write permissions. It defaults to the authenticated current upstream archive;
manual latest discovery still requires the existing exact runtime qualification.
Base construction skips Manager compilation and frontend acquisition/pins, validates
all six base assets, and cannot enter the integrated signing/promotion path.
The normal feature-preserving stable admission is unchanged. This is independent
qualification, not completion of Core policy extraction or independent default
upgrades. Main's published caller and device remain stable52 until an accepted
source is separately pinned and activated. Manager configuration extraction follows
below; account/default routing, managed-home safety and server/catalog ownership
still require their remaining alignment slices.

Local acceptance for this slice passes full workspace checks: Core198 with one
pre-existing ignored case, Manager42+56+three path gates, builder25, Python18+66,
fmt/Clippy and actionlint1.7.7. Final Python acceptance also passes after the
optional-frontend decision arguments were removed from base mode. Real Android
Core-only construction from the exact official0.161 archive passes all six base
asset checks, --version and exec --help; Manager is absent and unsupported sandbox
argv still exits2. Doctor reports healthy Core/runtime/Code Mode host and Manager
unavailable; its overall failure is the deliberately credential-free fixture's
auth check. No user credential was imported to make that probe green. An initial
fixture helper correctly rejected the base-only identity through its integrated
publication validator; the disposable device fixture uses its own signing key
and does not relax that validator. Proof/logs: responsibility-base-{device.json,
full.log,python.log} in the private acceptance cache. Exact-source implementation
80b9562 also passes hosted source acceptance38839831045 (workflow/diff, full
workspace and Android focused proofs) and independent base qualification38839831198
(unsigned Android/AArch64 construction and executable smoke). These hosted results
are separate from the physical-device proof above. No public release or device
activation has been performed by this slice.

### Manager configuration preparation slice

Core no longer reads notification/config-v1, names notification events, renders
hook policy or walks the shared configuration-profile registry. The admitted
Manager's private __runtime-config-v1 endpoint renders existing native callbacks
and matchers and owns no-replace shared profile projection. Core retains only a
bounded local envelope exchange and atomic publication after its execution
defaults; shared-server snapshots consume those same bytes. The response limit
is8 KiB within the existing16-KiB snapshot bound, deadline2s, no inherited stdin or
stderr, no inference, workload arguments or alternate execution. Failed responses
warn once and leave the base runtime usable; absent Manager needs no query.
Previously registered/native local profiles remain usable without Manager.

Managed-home mutation leases, shared conversation layout, default-account routing,
frontend/server selection and catalog decisions have not been extracted by this
slice. Existing filesystem layout, public entrypoints, notification delivery and
window/title handling stay in place. No installed runtime or account state has
been edited. Core and Manager have no new compile-time dependency on each other.

Implementation01e2f50 and test-lint correction6e4f419 pass exact-source hosted
acceptance38845552319. Independent base qualification38845156837 also passes;
its Core/runtime inputs are unchanged by the test-only lint correction. Local
workspace tests pass Core196/one pre-existing ignored, Manager46+58+three path
gates, builder25 and Python18+66. Final focused cleanup observation, fmt and Clippy
pass after the lint correction. Native Android signed fixtures reuse the accepted
frontend bytes with the new Core/Manager: saved/inherited account identity, live
FD36 protection, rename inode retention, delete without shared-history changes,
Manager absence and native --profile/mcp-list loading all pass. Shared profiles
are created by Manager, local overrides win, and existing registrations still
load without Manager. Signed ordinary/memory notification qualification also
passes using only a loopback Responses fixture; actual Chat inference is zero.
Physical notification tapping/window return was not re-exercised by this slice;
the delivery and window/title implementations are unchanged.

Failed observers are retained, not counted as product passes: libtest buffered
stderr disappeared at exec (--nocapture fixes it); descendant SIGKILL observation
needed a bounded wait; native fixture PREFIX lacked OpenSSL; a nested TMPDIR made
Android socket names too long. Final native profile proof also found real retained
FD36 owners: native shell-environment helpers can outlive the server in independent
process groups. Deletion correctly refused. The fixture cleanup now verifies its
exact home inode on FD36 before retiring only its own remaining helpers, while
production mutation protection is unchanged. Both native qualifiers now use
signed fixtures and the real verifier. These corrections improve proof quality;
they do not authorize stopping user processes or relaxing profile protection.

Private aggregate: responsibility-manager-qualified.json. Native proof logs:
responsibility-manager-native-{profiles-final,notifications-signed}.log. The
stable52 pointer and all eight installed payload hashes/modes remain unchanged;
no publication, device activation or real credentials were used. Remaining work
is the account/default delegation and managed-home policy/safety split, followed
by server/catalog and frontend coupling reduction in the sequence below.

Source baseline is rewrite/rust-core6bf9f6b; device/public baseline is stable52,
upstream0.161.0. Relevant observed coupling:

- SPEC generation admission and auto-release-termux.yml require an exact paired
  frontend for every new candidate. A frontend-bearing local generation cannot
  construct an unpaired newer runtime. Core-only execution exists, but the normal
  publication contract does not provide independent Core advancement.
- frontend/manager-tui/native.patch touches36 upstream TUI files including tests,
  snapshots and fixtures. It does not patch the upstream execution-engine source.
  Its source APIs still require revision-specific maintenance and compilation.
- At that baseline Core launch read Manager notification policy, generated
  event-specific hooks and projected shared configuration profiles; the slice
  above extracts those policies. It still recognizes managed profile paths and
  prepares shared conversation layout; their policy/safety split remains open.
- shared_server.rs combines necessary signed-runtime/socket binding with catalog
  snapshots and reuse decisions needed by the enhanced frontend. Release-builder
  also maintains exact-version binary permission/UDS patches. Moving files between
  crates would not remove these maintenance costs.

### Target responsibility matrix

| Capability | Policy / state owner | Execution boundary and planned change |
| --- | --- | --- |
| Upstream inference, tools, approvals/reviewer, threads, resume, compaction and writer locks | Upstream | Use upstream commands/protocol; no local replacement or transcript parser in Core. |
| Android loader, DNS, certificates, filesystem publication, browser opener and safe socket paths | Core; release-builder constructs artifacts | Apply to bare and Manager launches, including exec/resume/fork/side chat. Requalify actual upstream revisions. |
| Sandbox availability | Core capability enforcement; upstream approval engine | Enforce unavailable sandbox consistently without changing reviewer/approval intent. Manager presents supported choices only. |
| Artifact trust, generation retention, activation and rollback | Core; existing producer signs/publishes | One authenticated inventory and transaction. Manager/provider never select arbitrary runtime binaries or write pointers. |
| Account create/rename/delete/default/select and /switch | Manager | Manager owns registry and handoff UX. Core receives the selected execution home; it does not select accounts or parse the registry. |
| Profile mutation protection | Manager owns mutation; minimal execution lease protects live homes | Preserve protection through bare launches, exec replacement and persistent servers. Generalize the existing lease only after proving all entrypoints; never delete it merely to remove a Manager path reference. |
| Installation-wide conversation sharing | Manager-selected product policy; upstream owns all records/DB/locks | Manager prepares the declared layout; Core retains only the generic launch binding needed for already-managed homes. Preserve bare access to existing shared history and explicit isolated homes. |
| Shared --profile configurations | Manager owns convenience registration; upstream resolves settings | Move registry/projection policy out of Core. Preserve global chatgpt fallback, local overrides and native --profile meaning without copying credentials. |
| Server process and connection | Upstream owns server/turn lifecycle; Core binds qualified runtime and compatible endpoints | Keep bounded signed-runtime spawn, necessary environment/FDs and peer validation. Manager requests optional configuration; never create a second executor or competing supervisor. |
| Model catalog, provider selection and frontend server reuse requirements | Manager/frontend; provider owns its account catalog | Core accepts only bounded opaque startup input if the pinned upstream API cannot carry it. It must not know Chat model names, resolve provider auth or own model-selection policy. |
| Notification preferences, event selection, question text and delivery | Manager | Move policy parsing/hook rendering out of Core. Use upstream hooks/config/events, retaining one question notification and exact existing-window return. |
| Android window registration, identity, title forwarding and tmux | Manager; Termux app owns native handles; upstream composes /title | Core provides only optional delegation. Bare mode stays usable, tmux stays optional, titles never become route identity. |
| ChatGPT request/return transport, login context, retries and host lifetime | Optional Chat provider | Expose qualified Responses behavior through upstream provider configuration. Core remains provider-neutral; native tools and canonical transcripts remain upstream-owned. |
| Diagnosis | Each component owns its checks; Core composes bounded results | Distinguish base failure, optional feature unavailability and upstream warnings. No Manager repair authority or duplicated recovery engine. |

The minimal adapter is determined by the execution invariant, not just by whether
the code is labelled Android-specific. Generic lease/config/FD validation may
remain in Core where removing it would make direct launches unsafe. Conversely,
bare-codex convenience does not justify embedding the convenience's policy there.

### Interfaces and functional improvements

1. **Launch boundary.** Keep codex termux and existing validated Core handoff.
   Separate Manager preparation from Core environment/exec. Reuse current bounded
   argv/FD/private-file contracts; do not add a universal launch-plan protocol.
   An optional capability query must be local, bounded and free of inference,
   credentials and update discovery. A failed query must not start work twice.
2. **Configuration boundary.** Manager owns notification and convenience fragments;
   upstream owns precedence and parsing. First verify the exact pinned upstream
   loader/API can combine base requirements with Manager input. Do not invent a
   config-file flag or solve this by editing account config.toml. If generic Core
   projection is necessary, it validates/applies bounded input without recognizing
   notification events, Manager registry formats or Chat model names. One effective
   configuration must cover initial launch, resume, /switch and /btw.
3. **State boundary.** Reuse current roots and inode/lock protections. Move code
   ownership before considering physical data relocation. Registration/migration
   establishes shared layout; launch verifies required bindings without repeatedly
   repairing user state. Missing optional state is distinct from an unsafe declared
   managed home, which must fail explicitly. Never reinterpret an unsafe managed
   account as a fresh isolated account or reset its selected identity.
4. **Server boundary.** First compare the pinned native server surface with the
   custom coordinator. Remove custom reuse/retirement only if native behavior
   preserves exact runtime/environment, account isolation and live-client retention.
   Otherwise retain that minimum in Core. Move frontend/catalog policy to Manager;
   a common runtime-binding primitive may remain, with opaque catalog input only
   where required. Do not duplicate servers in Manager to make Core look smaller.
5. **Frontend boundary.** Keep one upstream-derived UI with a narrow maintained
   integration module. Consolidate /switch, window callbacks and provider catalog
   access behind small call sites. Move domain logic out of upstream app/chatwidget
   internals. Preserve upstream command inventory, selected permissions, drafts,
   active-work guards, same-thread re-entry, /title and native transcript rendering.
   Upstream has no proven general slash-extension hook in this source; an external
   Manager alone cannot magically add /switch. Some TUI revision work remains.
6. **Provider boundary.** Keep the existing working local-function return and its
   independent MCP option outside Core. Provider capability/catalog changes must
   not require a Core release. A generic frontend handoff may still be necessary
   where upstream cannot change providers in place; do not promise reload removal
   by moving code. App-free transport research is a separate provider task.

   Fix same-provider selection separately from cross-provider re-entry. Current
   needs_reentry returns true whenever the source is Chat or the target is a Chat
   model, so even effort-only selections take the exec/resume path. The local
   provider also closes its owned app conversation and resets projection/context
   whenever the full route changes, including effort. Classify exact no-op,
   same-provider model/effort change and actual provider/account/transport change.
   Use upstream in-session model/effort updates for the second case and preserve
   the owned conversation where the host supports the requested transition.
   Changing only the frontend predicate is insufficient: catalog admission,
   context-boundary handling and the provider reset must be qualified together.
   Prove unchanged frontend PID/TTY/thread, actual next-request settings, retained
   tool history and no inference on selection itself; reject pending-call changes.

### Update and release design

- Separate qualification into base runtime/compatibility, Manager CLI, enhanced
  frontend and optional provider. Each records exact inputs and applicable public
  behavior. A base candidate does not wait for Chat inference or MCP-return trials.
- Prefer reuse of an unchanged admitted Manager artifact with a newly qualified
  runtime when its interface tests pass. Backend/frontend compatibility is established
  by real protocol/behavior proof; do not simply delete the current version check.
  An upstream-derived frontend still needs a new build when its own source changes.
- Keep one signed generation inventory, stable index and atomic activation. A
  qualified producer assembles compatible independently built inputs. No unsigned
  Manager replacement, mutable external helper pointer or second stable channel.
- Independently qualify a base candidate even while the enhanced UI is unfinished.
  Publish/activate a base-only choice only through an explicit supported product
  mode; do not silently remove /switch, window return or supported permissions from
  the ordinary update path. This choice is not an existing public command.
- Prefer an integrated update preserving all accepted functions. If no compatible
  frontend exists, retain the current default generation and report precisely that
  the base candidate passed but enhanced integration is blocked. The default-update
  bottleneck is only fully removed when the integration works across the candidate
  or an explicit base-mode downgrade policy is accepted. Honest separation of gates
  alone is not a claim that all feature-preserving upgrades are independent.
- Keep current trust, complete readback, Android execution, disposable update and
  non-forced promotion. Capability checks narrow the affected tests; they do not
  waive security, data or cross-version migration tests. Rollback restores the whole
  admitted combination while existing processes retain their loaded images.

### Execution slices and acceptance

| Order | Changes and source targets | Required evidence / exit condition |
| --- | --- | --- |
| 1. Establish contracts and a small baseline | Amend SPEC4/4.5, generation admission, Manager and9A sections in the relevant implementation slice. Record direct managed-home launch, UI and update behaviors using current public entrypoints. | Separate the three Core meanings: our launcher, upstream engine and modified frontend. Baseline covers absent Manager, isolated/managed home, /switch, /btw, notifications, writer ownership and failed updates. No live Chat use needed. |
| 2. Split release qualification | auto-release-termux.yml, release_decision.py, manager_tui_artifact.py, producer/inventory tests and release-builder admission. | An optional frontend/provider failure does not prevent constructing and testing a base candidate. Complete candidate inventories still reject missing declared assets. An incompatible enhanced/default update preserves current pointer and user features; no incompatible helper is executed. |
| 3. Extract Manager launch policy | Core main.rs notification/projection paths; Manager lib.rs/profile launch; shared_profiles.rs; existing handoff tests. | Manager chooses accounts/hooks/config; Core does not parse its policy records. Fresh/resume/switch/side-chat use the same effective permissions/config. Manager absence/failure cannot block a plain independent launch. Existing auth/config bytes stay unchanged. |
| 4. Separate managed-home policy from execution safety | shared_layout.rs, profile_lease.rs, Manager profile lifecycle and shared-state transition scripts. | Bare launch of an existing managed home still holds the mutation lease and sees canonical history; isolated homes remain isolated. Deletion/rename races, inherited FD lifetime, server retention and interrupted migration preserve data. No second history index or new mandatory daemon. |
| 5. Reduce server/catalog coupling | shared_server.rs, manager_tui.rs, Manager task discovery and frontend termux_backend_catalog integration. | Determine the native capability gap on the actual candidate; eliminate only duplicated behavior. Different identities/catalogs do not cross, changed catalog bytes cannot reuse stale state, current writers stay observable, old jobs survive updates, and no upstream unmanaged installer starts. |
| 6. Minimize frontend and binary changes | native.patch, command-boundary checks, native workflow and release-builder/runtime_patch.rs. | Classify every hunk as compatibility, integration or obsolete. Remove duplicate/dead patches. Compare supported upstream config, small source patch and current binary adaptation by real build/rebase cost; keep the smallest qualified method. Preserve all supported approval modes without bwrap and all36-file-patch behaviors that remain required. |
| 7. Qualify a real next-upstream candidate | Current producer/signing/readback/update scripts, Android fixtures and current-generation rollback. | Independently build base and integrated candidates; verify unaffected component reuse. Feature-preserving update, failed/missing optional asset, rollback, local reconstruction and retained running clients pass. Physical notification/window observations are reported separately. |
| 8. Simplify after proof | Remove replaced branches, projections and obsolete assumptions; update SPEC/CURRENT/README. | One preparation owner per mutation, one installed entrypoint, no extra management service, no stranded user state. Publish only the proven scope; Chat provider acceptance remains independently tracked. |

Perform each slice as a reviewable change with its own relevant gate; do not hold
all changes for a single rewrite. Release-admission changes and filesystem/process
changes need full affected acceptance, including Android; documentation alone needs
scripts/check.sh docs. Unchanged proof is reusable only for the same inputs/scope.

### Verification matrix and cost controls

- Exercise base execution with Manager/frontend/provider absent, incompatible and
  unavailable, offline and without optional Termux APIs. Check raw argv, cwd, exit,
  signals, TTY and explicitly selected account/configuration; no hidden fallback.
- Exercise managed/external homes, local overrides, shared --profile chatgpt,
  /new, resume, /switch, /btw and cold restart. Check auth separation, canonical
  history and current kernel writer ownership, never original message authorship.
- Exercise Ask/Approve for me/Full Access with real native commands and deterministic
  model/reviewer fixtures. No bwrap attempts, permissions escalation by fallback or
  loss of selected approval/reviewer policy. Cover both frontend and upstream routes.
- Exercise two independent Android windows with tmux off/on. One-tap notifications
  select the correct existing window without extra sessions, nested tmux or replay;
  /title and profile/model switches preserve route identity. Physical taps may need
  the user only after automated checks pass.
- Inject failed startup, malformed Manager input, update interruption, changed
  optional inventory, old/new clients and rollback. Verify authenticated runtime
  binding, no double dispatch and unchanged user files/process identities.
- Measure version-specific binary sites and production TUI hunks separately from
  tests; count required edits, build/qualification wall time, reused inputs and
  blocking gates on a real upstream upgrade. Acceptance is reduced coupling with
  preserved function, not a lower line count or a crate rename.
- Initial investigation and deterministic tests consume no Chat messages. Explain
  necessity and estimated submissions before any later real-provider test; do not
  repeat live Chat tests for unchanged transport. Do not spend Pro usage.

First implement slices1/2; then3/4,5/6 and7/8. This scope does not restart broader
Termux cleanup, introduce Manager repair, or move the Chat provider into Core.
Provider feature expansion and broad autonomous-development requalification follow
the responsibility realignment; the new invalid_tool_input failure is retained as
a required recovery/regression task, not used to expand Core during this phase.

## Termux permission resume correction

- Source795a064 fixes native local resume dropping Core's no-sandbox default and
  restoring obsolete saved workspace permissions. Approval policy/reviewer remain
  independent; explicit restricted settings fail before local thread/turn requests.
- Native38530530528 passes exact ARM64 build,29 actual TUI tests, lint and pristine
  upstream Core/dependencies. Raw frontend SHA256c2a75d4c594c3008001bd71c5fc5308c75d7e79a7dbd9b8a4aa42296a6b64523.
  This is the historical stable50 input; stable51 admits the combined39-test set.
- Android legacy and named-profile fixtures each pass three actual commands and
  three guardian requests through fresh launch, cold resume and /btw. Seeded saved
  workspace settings become the current supported selection; named identity and
  on-request/auto_review remain intact. An older frontend negative control restores
  workspace settings instead. The deterministic model tests approval plumbing,
  not model judgment, and submits no real inference or account credentials.
  Private aggregate proofs: ~/.cache/codex/acceptance/permission-lifecycle-{native-model,named,stale}.json.
  The reusable script owns its signed disposable roots and needs no Chat provider
  installation or external Python package. Local full checks pass; changed producer
  admission and all Python18+59 tests also pass. Signed stable50 is published and
  installed; the running old client needs a normal restart to apply it. `/switch`
  uses same-conversation native resume in the destination account home, so this
  correction also covers its qualified local cold-resume path. Existing live
  writer reattachment keeps that writer's permissions and does not take it over.

## Independent window return and optional tmux

- `codex termux launch` selects an existing profile with Up/Down and Enter;
  Esc/Ctrl-C cancels, the saved default is unchanged, --profile is explicit and
  upstream bytes after -- are preserved. Default is tmux off. Ordinary interactive
  `codex` uses the same optional window routing without a profile picker.
- Native-capable Termux runs in the invoking terminal. Confirmed unsupported native
  API uses one random named dedicated terminal per launch without requiring tmux.
  Inconclusive AM failure never becomes a constructor fallback. Stock manual rename
  or close races may create an inert empty terminal, never restart/resume work.
- Bare foreground FD bindings and tmux pane bindings share notification focus.
  UUID changes use the live FD rather than labels/titles or stored conversations.
  `/switch`, /new, /resume and /title do not change the Android origin.
- Hidden/status requests inside tmux refuse before picker, launch descriptors,
  Android commands or work. Both Manager and AI removed their inside-session
  new-window branch; no nested session, extra pane or replacement workload is
  created. Independent Android terminals can each own a separate container.
  `codex termux attach SESSION` rebinds only detached Manager work without replay.
- tmux and Android starters use private single-consumption bounded descriptors for
  raw argv/caller environment; terminal identity stays new-terminal owned. The
  descriptors are removed before exec. Lazy cleanup requires verified departure.
  Temporary descriptors can contain caller auth environment; permanent bindings,
  Android intents, notifications and logs contain none of it.
- Native Termux extension on the existing authenticated AM socket captures exact
  handle/PID/start from peer ancestry and rechecks the app's live table on focus.
  It stops at the app boundary and introduces no new daemon/socket/registry.
  Installed package com.termux/code118, 0.118.0+8629e63.origin2, base8629e632.
  Existing APK acceptance/build38099833533 remains applicable; app inputs unchanged.
  APK SHA256d32973c555ff9d8d726306948a0e0af1c2b41ccd2938a22edcd848bf7a278453.
  Original and accepted APK remain in Download/termux-origin-candidate.
- User tapped all four fresh Manager return notifications. A/B bare and hidden tmux C with synthetic native workloads recorded exit0, selected=true and count5; D's seeded stock named
  route returned exit0/count5 on the custom APK. D proves named-service reuse,
  not an installation of an official APK. All four owned workloads exited normally;
  user-requested cleanup restored baseline count1 and removed the fake install trees.
  Private proof: ~/.cache/codex/acceptance/window-device-88b0a09.json. This return
  proof remains applicable to8ba5997: the follow-up only removes nested construction;
  fresh launch, binding and focus code is unchanged. Actual Android signed installed
  Manager also refused all three nested entrypoints before any state creation.
- A fresh notification is required for each trial: Android consumes it on one tap.
  Never ask for a second tap on an already consumed notification.
- The owned direct signed-stable46 title trial has no PTY intermediary or copied
  authentication. The user confirmed normal rendering and Termux title propagation;
  upstream persisted the selected activity/app-name/model/run-state/current-dir
  order. `/title` configures title fields/order, separately from route identity.
  A discarded intermediary trial rendered badly; it is not product acceptance.
- Bare same-PID frontend re-entry needs replacement of the old qualified FD
  binding. The affected real-exec path now passes native and named-route return,
  UUID changes and preservation of unknown bindings. Local composed full acceptance
  passed Core193/one pre-existing ignored, Manager42+55+three path gates, builder25,
  Python18+59, fmt/Clippy/diff. Two startup fixtures now clear inherited window-ready
  state, so a connected caller cannot silently bypass the first-launch advisory
  test. Hosted acceptance and signed stable47 installation passed. A fresh real frontend
  trial exposed a separate missing bare-registration call; stable48 corrects it.
  The user confirmed the fresh return selected `title switch48` after `/switch`.
  Two notification actions exited0, changed selected=false to true and kept
  terminal count3; the latest fresh tap has explicit user confirmation. The first
  tap's screen observation was inconclusive. PID/start/TTY and Android origin stayed unchanged while the FD
  binding changed across re-entry. This is actual signed frontend/Manager proof,
  with tmux off, not merely the synthetic FD fixture.
  An owned version-only reproduction also identifies a trial-only PATH-alias
  warning: upstream refuses a CODEX_HOME beneath TMPDIR. A separate scratch
  directory removes that warning without weakening the upstream check. Future
  trial launches use this separation; the user's fleeting text was not captured.

## Question notification consolidation

- Manager now presents the allowlisted sync/async question tools' actual text,
  never their options or other arguments. Empty question reminders are no-ops.
  A validated conversation ID selects one Android notification per group/thread;
  completion updates that notification rather than leaving a generic input alert.
  Separate conversations and existing exact window return remain independent.
- Candidate native Android Manager delivery passed against the real Termux:API:
  sync/async question replacement, native completion replacement, two separate
  conversations, empty-reminder suppression and removal of all owned alerts.
  The reusable `scripts/qualify_notification_delivery.py` uses disposable state
  and synthetic content only. Local full acceptance passes Core193/one pre-existing
  ignored, Manager42+56 and three actual path gates, builder25, Python18+59,
  fmt/Clippy/diff. Hosted acceptance38218520434 and publication38218954233 passed.
  Ordinary signed `codex update` activated stable49; installed Manager's real
  Android delivery rerun passed and doctor exited0. Twelve protected file hashes
  and fourteen preexisting live PID/start/image identities stayed unchanged;
  stable48 is the rollback target. No running conversation needed restarting.
  The live structured plugin-installation question also produced exactly one body
  notification, with the current conversation tag and no generic input reminder.
  Proof: ~/.cache/codex/acceptance/notification-consolidation-device.json.

## Source and installed AI

- AI main e8d49c3 formally includes the installed profile/auth/Manager adapter and
  migration support; the approved normal push completed, with a clean source tree.
  Enhanced full verification passes50/warn0/fail0, including TUI166 and isolated
  installed Manager profile lifecycle. Migration stages complete accounts before
  publication, restores the source on failure and refuses unreadable live owners;
  unique AGY profile state is preserved. No real account migration was performed.
  All thirteen installed ai_* modules match the published source; only three
  modules needed atomic replacement, and seventeen other runtime/config hashes
  remained unchanged. Private proof: ~/.cache/codex/acceptance/ai-formal-source.json.
- App source remains the separate formal ~/prj/termux repository, with official Git
  history and read-only upstream. AI has no competing Codex profile/window authority.
- AI follow-up1f958e5 removes its AGY shim/installation path; enhanced full verification
  passes48/warn0/fail0. AGY main ac228cf publishes the formerly installed native Android
  support. Its full suite passed and the formal setup installed wrapper261011.01,
  upstream1.2.17, with one $PREFIX/bin/agy entrypoint. The raw input and all9302
  protected profile/authentication files retain their hashes; installed library and
  builder match published source. Native allocator/syscall patterns stay unchanged,
  while explicit Linux compatibility remains available. Default installation does
  not require glibc/proot. Private proof: agy-formal-source.json in the same cache.

## Reusable acceptance

- Exact source6c5819c acceptance38087812283 passed every job, including actual
  Android/AArch64 Core execution and locked workspace tests/Clippy/fmt.
  Local Core192 passed/one pre-existing ignored; Manager40 unit +55 integration
  +two public tmux/native-FD paths passed; release-builder25 and Python18+59 pass.
  Two superseded AI-action assertions were updated before the final Manager gate.
- Scoped d18f796 acceptance38100063268 covers documentation; runtime jobs skipped.
  Its unchanged6c runtime proof is reused rather than described as a fresh build.
- Repository-local development rules are accepted; main is a thin source caller.
  Standard checks are scripts/check.sh docs, python, rust or full, chosen by scope.

## Remaining limits

- The former native frontend gated FD registration on TMUX/TMUX_PANE despite
  Manager supporting bare bindings. Synthetic native workload path proofs did
  not cover this real-frontend call. The admitted stable48 frontend removes
  that condition, and its native child gate exercises both bare and tmux processes.
  Native compilation/all27 regressions/lint/pristine Core proof38135821930 and
  qualification binding passed at73f77de. Both bare and tmux children exercise the
  call. Qualified raw frontend00842f35 and input archive805a71f9 have immutable
  public readback proof; the producer now pins this exact source/run/input pair.
  Signed publication/activation passed; the real phone frontend publishes its
  bare FD binding with the matching original Android route. The fresh physical
  return across `/switch` is accepted by the user and has selected/count proof in
  stable48-device.json. `/title` rendering itself is also accepted by the user.
- The user separately confirmed `/switch` to the owned other profile rendered
  normally in the same Android terminal on stable47. This confirms screen/profile
  transition, not missing bare FD registration or notification return. Private
  proof: title-switch-device.json in the acceptance cache.
- Official unmodified Termux8629e632 production inputs, real Android/x86_64 APK,
  accepted Manager24ad60d and real AM client1.5.0 passed hosted38137172382. Two bare
  launches retain their caller terminals and each construct one dedicated terminal;
  six A/B existing returns preserve count and workload count. A dead binding refuses
  before constructor and the other still returns. Only the model frontend is a
  synthetic FD fixture; an emulator-only libc shim exposes the same renameat2 kernel
  syscall already used by production arm64. This is actual stock APK/Manager proof,
  separately from phone/native Codex frontend acceptance. The reproducible manual
  workflow and fixture are retained under .github/stock-window.

- Web trial tunnel DNS is now qualified separately from startup health. The
  standalone global Linux/CGO-disabled CLI selected /etc/resolv.conf and retried
  localhost53; Tdev's own native Android/CGO binary was already correct and is
  unchanged. Exact OpenAI source0f870e5 built on-phone with Android/arm64 CGO
  resolves api.openai.com and its owned trial records successful HTTP204 long
  polls. The existing key reference/tunnel ID are reused, with env-i/private HOME
  and no active TurnBroker. Local ready/healthy alone is insufficient. The same
  alias/profile/source remain a temporary experiment, not an installed service.
  Native artifact/proof are retained under XDG cache. The user confirmed plugin
  creation/installation; real model/tool-loop acceptance remains open and the
  execution broker is not yet connected.
  Proof: ~/.cache/codex/acceptance/web-native-tunnel.json; experiment8fce479.

- Broader authorized normalization continues separately: independent AGY/OpenCode
  notification hooks, isolated real ChatGPT Web acceptance and the final home/usr
  authority audit. Existing synthetic browser proofs do not establish account,
  model, tool-loop, cancel, compact or resume acceptance. Photos v3 cleanup90b8546
  is published: retired snapshots remain recoverable in Git, active data/source
  bytes are preserved and clean-tree packaging/service-smoke passed.
- Termux source/runtime audit: Tdev's CLI uses ~/.local/share/tdev/active, the old
  composition root is absent, actual authenticated local MCP tools/list returns13
  tools and both default/janmori connections are enabled with control-plane polling.
  Controller/retained House databases pass read-only SQLite quick_check. Runit root
  and15-second monitor preserve their original PIDs; service/AM installer ownership
  checks pass. Package-owned inactive GUI/Cloudflare service entries remain down.
  Four inactive home source/recovery trees were relocated intact to the private
  ~/.local/share/termux-recovery/20261011/original-trees after425 files were extracted
  and hash-verified from a private recovery archive. Automatic review rejected
  deletion; the reversible relocation preserves every original byte instead.
  Legacy credential configuration is unchanged. Empty prj/artex was removed.
- Historical stable49 Web trial in experiment/web-termux preserves the failed native-host trial;
  the restored mobile viewer ise72f1e8 with user-accepted023c263 controls. Original
  viewer/Tunnel connectivity and Native2 installation are confirmed. The rejected
  dca030e width behavior is inactive. Fresh navigation uses --refresh; browser
  login/profile and owned normal-window480x785 fitting are preserved.
  The stale cookie-only seed had24 account cookies versus27 in the current
  experimental profile. A read-only SQLite snapshot excludes challenge cookies
  and other domains and reaches an authenticated unique composer. One prepared
  empty Temporary Chat is handed off by exact owned CDP target ID and consumed
  once; stale targets, drafts, user turns and ambiguous composers are refused.
  This removes the redundant navigation between inspection and actual Core work.
  Actual signed stable49 Core,6.1.8/7cc8b8d server and TurnBroker in disposable
  roots reach authenticated surface, model/effort, exact Native2 selection, full
  prompt/files and Send. The conversation POST returns403 plus
  cf-mitigated:challenge; the adapter reports chatgpt_security_check. The same
  result occurs using the unchanged full experimental browser profile in place.
  This is the model-submission failure, not a disconnected viewer or Tunnel.
  Real reply, two model-initiated tools, cancel, compact and resume remain open.
  Native TypeScript passes. Six affected suites pass177/0 with1005 assertions,
  including five actual native Chromium DOM tests. Account-free private-pipe
  proof passes descriptor/auth/origin/concurrent-owner refusal, prepared-page
  readback/consumption, stale-target/draft/user-turn refusal, persistent context,
  reconnect/network and owned-process/descriptor cleanup. These gates do not
  establish real service success. No private screenshots/conversations/keys are
  published and no CAPTCHA solution or browser-property override is used.
  Automatic approval review rejected an indefinite native-host hold of the live
  experimental profile; it was not executed. The bounded human-view variant was
  allowed: max120 seconds, no model requests while waiting, owner verification,
  private recovery checkpoint, automatic ordinary-browser/marker restoration and
  stale-alert removal. The latest timed view displayed a challenge title without
  a composer, expired and restored ordinary Chrome. The user confirmed clicking verification returns to Verify repeatedly. This
  fails the native-host human gate; it is not missing user input. DNS/public
  validation JavaScript access work (HTTP200). An account-free public diagnostic
  shows JS/cookies/WebCrypto available but no WebGL without unsafe fallback.
  Verified apt154 was compared in TMPDIR with installed152: both support WebGL
  with Playwright's unsafe fallback and both lack it when the flag is excluded.
  An upgrade is therefore not proven to fix verification. No package/operating
  graphics flag was changed; downloaded154 files were removed. The particular
  triggering bot signal remains unproven. Keep ordinary Chrome/Native2 Tunnel
  running; require a new technical basis before another production check.
  Private aggregates: ~/.cache/codex/acceptance/web-current-login.json,
  web-same-profile.json, web-human-check.json and web-warm-page.json.
  Experimental browser data belongs to ~/.local/share/codex-web-experiment;
  current isolated sources/checkouts remain in TMPDIR. There is no new boot
  service, merged provider or production installer admission. Operating Tdev
  config/key hashes and the native trial Tunnel remain unchanged.

- Public update currently fetches complete payloads with a300-second per-payload
  budget. Slow large downloads can hit that limit and trigger local-derived
  fallback; the existing same-version frontend rule can then refuse fallback.
  A verified inventory assembled from identical installed bytes and immutable
  Release assets is accepted by the ordinary `codex update --local` path. Efficient
  public acquisition reuse and clearer transport/fallback reporting remain separate
  Core work; do not change signing/admission to work around network speed.
- Existing old tmux work does not infer a detached original Android ancestor.
  New registered launch or explicit detached Manager attach supplies the origin;
  no notification creates replacement work to retrofit old jobs.
- Switching a live bare PTY into/out of tmux is not supported; choose a new safe
  launch. No user job was stopped to change execution mode.

- Two independent official alternatives are prepared in experiment/web-termux
  at6bbb383.
  Signed Linux ARM64 desktop 26.1007.21434 installs in a Debian13/PRoot candidate
  under XDG cache; account data stays separately in
  ~/.local/share/codex-web-experiment/linux-desktop/home. The login page rendered
  in bounded trials. Candidate HTTP(S) MIME defaults now route the app's sign-in
  to its scoped Android handler rather than back into chatgpt.desktop. Guest GIO
  dispatch reached that handler using an unrecognized account-free target, which
  was refused. Signed-app URL construction revealed a separate refusal: it uses
  /oauth/authorize. An actual guest button click also identified the default
  chatgpt.com/codex/desktop-auth wrapper carrying authorize_url. Both paths are
  accepted now, with exactly one nested OpenAI target and no URL rewriting.
  Five Android MIME/routing/target cases pass, including the panel helper under
  candidate HOME and owned browser window selection. A private tint2 panel shows
  Browser, ChatGPT and clock. Browser now opens candidate Debian Chromium in the
  same Linux display, separate from the Android authentication handoff. Actual
  panel clicks open one browser, reuse its same window and fit 480x743. Its
  /root/browser-profile is separate from the app and existing browser accounts;
  controller-owned browser processes are included in shutdown cleanup.
  Linux user login completed and candidate auth is ChatGPT with a Business-family
  team claim. The bundled Linux Core passed a real-service ephemeral turn in an
  isolated guest workspace: two successful shell calls wrote/read a correlated
  marker and the final model reply matched it. The owner-only aggregate is
  linux-desktop/guest-core-proof.json; test files/logs/processes were removed.
  No ordinary Termux profile was imported. Actual native GUI input and visually
  inspected distinct uppercase reply now pass, including on the current detached
  controller. The ChatGPT task button returns from Chromium to the response.
  Clipboard readback remains unaccepted. Desktop Work parity and runtime manifest
  404 remain separate unresolved concerns; successful Core/GUI replies do not
  establish complete product parity. SIGTERM now enters
  owned-process cleanup; four older orphan groups were safely stopped.
  Fresh notifications resolve the current endpoint at tap time and verify owner
  PID/start/executable. Default seconds=0 now keeps the authorized interactive
  desktop open until the app/controller closes; positive bounds up to 3600 still
  support disposable trials. The start action detaches the same controller from
  tool-command lifetime after a previous foreground owner vanished and left its
  children alive. Old owned orphan groups and unused private screens were safely
  cleaned. No new system service or global launcher was added. Browser aggregate
  proof is linux-desktop/browser-panel-qualification.json; viewer HTTP 200 passes.
  Native SIWC public-client qualification has a separate loopback consent page
  and owner-only records in ~/.local/share/codex-web-experiment/siwc. It uses the
  unmodified official devkit OAuth/SSE modules, with the official guide's 0600
  file storage rather than the devkit's encrypted high-level store. Current
  Termux Keystore supports signatures, not encryption; no new key service or
  existing credential reuse is introduced. Its fresh notification starts manual
  OpenAI consent. Native Node compilation/vendor 27, Android persistence/privacy/UI 6
  and repository Python 18+59 pass. Signed stable49's account-free app-server gate
  passes two correlated actual shell calls and direct-route body-shape checks.
  Two new actual Android MIME/routing cases also pass; current viewer HTTP and
  delivered page JavaScript pass. That Core fixture is synthetic service proof.
  The user's external sign-in returned 3p_login_tenant_policy_denied, preserved
  separately from verified credentials. ChatGPT workspace Apps settings do not
  govern this tenant policy; Global admin External access does. Official plan
  usage eligibility documents Plus/Pro, so policy access alone is not proof of
  Business entitlement. No organization setting or selected account was changed.
  The user now selects their separate Plus account for the SIWC qualification;
  a fresh alert requests OpenAI login and plan-use consent, with no Admin/API key.
  SIWC real account/model/tool-loop gates remain unpassed. Linux first-party
  login and guest Core now pass independently of the external identity rejection.
  Production remains stable49; no global launcher or boot service changed.

  Public Responses research: SIWC uses api.openai.com/v1/responses and currently
  documents Plus/Pro plan eligibility. Business Codex access tokens are official
  CLI/app-server credentials, not general public API credentials. Workspace
  Agents offers Business preview with an independent api.chatgpt.com trigger,
  scoped token and asynchronous status polling; final answer retrieval is not
  supported. An MCP result callback is a possible bridge, not implemented proof.
  Ordinary API-key inference has separate API charges; no paid request or
  organization-policy change was made. See experiment README's sourced comparison.


## ChatGPT model pilot

- Ownership correction: the active Chat provider reuses miuuyy6.1.5 request/SSE,
  tool mapping, queue, turn identity and summary helpers. Our signed-app
  transport/lifecycle and Core/Manager integration do not make it an independently
  written Responses implementation. Full-mode MCP source is present but not wired
  into the active text relay; tool results currently cause another Chat user
  submission. `Codex Native2` was an upstream browser default copied into an old
  probe, not an MCP wire requirement. Experimental browser entrypoints now require
  the existing connector's explicit display name without a fixed brand/prefix;
  active app routing needs no such name. Existing connections, installed payload
  and dependency licenses remain unchanged. Three selector/CLI tests, five browser
  helper tests and the installed6.1.5 provider mapping pass with zero Chat submits.
- Current work minimizes debt and message submissions in the standalone ChatGPT
  provider. tdev progress/control proposals are separate-session work, not a
  dependency. On2026-10-15 experimentdcf286c's independently written `local` return
  candidate passed actual signed-app GPT-6 Instant + Android Core stable51 proof:
  two real dependent shell commands, genuine model-authored initial/intermediate/
  final reports, canonical persistence, fresh Core resume and actual TUI rendering.
  It observed1 initial user submission,2 tool-role returns and3 model starts;
  those counts do not establish Chat message-limit deductions. Public reports
  accompany gateway calls, adding no report-only inference round. Two prior
  three-start tool-loop trials failed the report gate and remain recorded; current
  source fixes that omission. Core remains the only executor/history owner.
  Experiment615c9f5 adds actual native-command interruption and resume with the
  original result plus genuine new input in one completion. Core's interruption
  notice and new instruction share one ordered user message. The signed-app/Core
  trial passes with2 user messages,2 tool messages and3 starts; the file's content,
  inode, mtime and size remain unchanged. All three public reports persist and
  cold Core/TUI restore them without inference. `turn/interrupt` alone can leave
  unified-exec running: explicit exact-thread background-terminal termination
  confirms exit with no provider request. A further real-Core offline proof
  confirms removal from Core's terminal registry. Ordinary Ctrl-C was not changed.
  Source tests pass52 local-return Bun cases,2 metadata hook cases and34
  experiment Python cases. Unchanged Rust acceptance remains323 tests/one
  pre-existing ignored, fmt/Clippy. These are candidate gates.
  During that source qualification provider46db05e/bridge92a356f stayed
  unchanged, with561 manifest hashes verified. The authorized local installation
  is now recorded in Operating baseline. MCP settlement, generation-time
  stop/observation recovery, approvals, independent-owner and long-history/context
  acceptance remain open; device experimental activation is not their full
  qualification or a public promotion. Detailed current state and private proof names:
  [experiment CURRENT](https://github.com/humtr/codex/blob/experiment/web-termux/CURRENT.md#native-local-return-candidate-current-implementation-and-proof).
  Experimentec9f1f0's bounded ordinary-model MCP metadata proof passes:1 outbound
  GPT-6 Instant app dispatch/1 user message, exact-token inventory call through the real trial
  tunnel, exact-ID error return and final response, without another adapter submit.
  Metadata-only predecessors used0 Chat. This proves connector/error continuation,
  not successful Core settlement or Chat allowance deductions. App-dispatch count
  does not measure hidden host inference rounds. RPC ID0 is reused across distinct
  calls; pinned tunnel0f870e5 serializes shared stdio and tombstones/aliases expired
  IDs, so global RPC-ID deduplication is invalid. Its deadline does not establish
  stopped Core work. The official hosted120-second absolute deadline also includes
  queue time and is not extended by progress; long approval/execution needs a
  separately qualified continuation. Existing Tdev clients/profiles remain intact;
  the owned desktop/tunnel stop after the test. Independent retained MCP settlement
  shares the existing native projection/executor as described next.
  Experimentc140859 implements that independent return path as three small modules:
  ephemeral owner/call/result binding, MCP stdio endpoint and signed-app transport.
  It exposes native_catalog/native_tool and reuses DesktopToolSession; Core remains
  the only executor/history owner. Tokens bind backend/thread/turn, logical call_id
  survives RPC-ID reuse, duplicate calls share one result and changed input refuses.
  Disconnected IPC observers retain results; explicit retirement rejects late work.
  Terminal host completion cannot admit queued Core actions. App-observer failure
  still closes its remote lane and blocks replay; full recovery is not qualified.
  Actual Android stable51 Core + independent stdio/Unix/HTTP candidate passes two
  commands, exact result returns, all three report phases, canonical persistence,
  cold TUI restore and owned socket/root cleanup. The host is deterministic; this
  is zero-Chat integration proof. The signed-app pre-send check also passes0 starts
  with local Work/handoff declarations absent. Local-return regression remains
  green. Bun70 full plus11 affected tests after the final guard, Python34 and18+59
  pass; unchanged Rust323 and native41 proofs are reused. Installed561 files and
  existing Tdev processes remain unchanged. The historical six-tool connector
  catalog still needs new-endpoint discovery/positive host qualification; approvals,
  long waits, actual host receipt, app-observer recovery and media remain open.
  Private proof: chat-independent-mcp-core-qualified.json,
  chat-independent-mcp-app-dry.json and chat-local-after-mcp-offline.json.
  Zero-model discovery against the new endpoint confirms the remaining gate:
  the host still returns6 historical definitions and sends no stdio request.
  Readiness/polling pass but fresh-catalog acceptance correctly fails. The helper
  now requires both app-visible native_catalog/native_tool and an actual2-tool
  tools/list response;3 affected Python tests reject cached/stale evidence. Keep
  the raw failure chat-independent-mcp-discovery.json. Identify the supported
  custom-app schema refresh before a live positive trial; no Chat prompt, account
  reconnect or operating provider change was used to work around the cache.
  The following earlier reviews are background, not a competing implementation
  order. Source-only ordered packing reduces an owned 50,410-character
  fixture from27 submissions to2 while preserving schemas/history; no Chat quota
  deduction is measured. Admission errors and context/work submission accounting
  are now distinct on the experiment branch. First repair actual model-use boundaries.
  The pinned bridge contains full-mode MCP result continuation, which our adapter
  has not connected. At the user's direction, independently write our continuation
  using miuuyy's flow and AgentDock's lifecycle as references, adapted to Core's
  exact turn APIs; do not import their executors or another management plane.
  Ten further source comparisons support app-native submission plus targeted MCP
  continuation as the current candidate, not wholesale launcher/runtime adoption.
  Browserless Codex proxies and official SIWC use a different inference/allowance
  route; no superior ordinary-Chat protocol or quota-saving measurement is proved.
  AgentDock/CCCC add execution-lifetime and delivery/completion references. Dots
  is excluded from current work at the user's request; its historical source
  comparison does not create a provider dependency or implementation follow-up.
  Then qualify native Chat tool-result continuation through the existing adapter
  before expanding its architecture. Detailed scope and the no-inference stage-cost
  measurement live in experiment/web-termux CURRENT.md. Source implementation and
  explicitly budgeted live proof were subsequently authorized; no operating
  provider/runtime activation follows from these experiment results.
  The experiment source now repairs bounded identity/zstd HTTP decoding and
  pre-parser rejection of unreadable compaction. Actual Android offline HTTP,
  merged-catalog and existing-broker IPC proofs pass. Experiment678dbaa adds
  upstream-resolved catalog transfer through Core's existing server coordinator;
  ab35549 records actual Android admission with12 tools, content-bound reuse,
  changed-catalog isolation, retained thread access and Manager owner discovery.
  Native siblings/default sockets remain distinct, without duplicating the config
  loader or routing native inference through the optional adapter. Local77 Python
  and323 Rust tests pass (one existing ignored resolver smoke), plus fmt/Clippy.
  Release frontend38654727667 builds; actual Android legacy/named and upstream
  --profile chatgpt round trips pass with backend catalog binding and no inference.
  That first CI failed compiling a new test before execution; a20c58d fixes its
  error conversion. Run38657053854 then failed the socket-output rejection test:
  upstream's path conversion resolves relative paths. Fixf5e7304 checks the raw
  absolute path and uses the checked constructor; relative/tilde/invalid UTF-8
  cases refuse. The exact helper passes2 Android-host tests using the real upstream
  path implementation; the original parser fails the negative control. Pinned
  native frontend qualification38699918319 passes all jobs: ARM64 CLI build/
  execution,41 actual TUI tests, lint and pristine upstream Core checks. Its exact
  raw frontend4fa1a4f0 passes a fresh actual Android
  /model Chat/native round trip in disposable signed state, preserving PID/PTY,
  home/thread, resolved backend catalog, configuration and sibling defaults.
  No inference or operating activation occurs. Private proof and exact-input
  binding: chat-catalog-menu-38699918319-device{,-binding}.json. This is not
  acceptance of long-history cross-provider compaction or real model-use repair.
  A second read-only native connection also observes the exact persisted turn
  become interrupted during an owned local command, whose process exits; stop
  sends no additional provider HTTP request. This control candidate is not wired
  to MCP, and ephemeral threads reject the tested history APIs. Keep cancellation
  behavior until those gaps are resolved. Those source tests performed no runtime
  activation or Chat inference. Current device provider and unchanged signed
  stable51 baseline are recorded in Operating baseline.
  Review-only CoS reference: totec448-spec/chat-on-steroids2.1.37 at2d1c86e adds
  precise caller attribution, retained result-delivery and ambiguous-dispatch
  invariants. It is its own MCP executor, not a Responses provider. Automatic
  Continue/Goal may add user submissions; session_finish's HELD is a model
  instruction, not a provider finalization lock. Detailed source paths, limits
  and independent implementation consequences are in experiment/web-termux
  CURRENT.md. Reference code/tests were inspected but not executed; no Chat
  submissions or runtime changes occurred.
  Additional output-silence review preserves execution when rendering/observation
  fails: internal app-store reads reduce visible-DOM dependence but still require
  the app renderer/Scope/CDP. Current finished-only reads, generic abort-on-error
  cleanup and provider-independent heartbeat need separate lifecycle/visibility
  treatment. Responses-specific review narrows display to existing native Core
  events first; extra status/viewer features are optional, not a provider gate.
  Sampling completion is distinct from a Core turn ending. HTTP cancellation can
  mean genuine Ctrl-C, while during tool execution no Responses request may be
  open. Retained MCP mode therefore requires qualified exact-turn control evidence;
  never detach all HTTP aborts or assume arbitrary stream replay is supported.
  Observer/display recovery must add no user prompts, restart no actions and never
  equate heartbeat with progress. Actual rendering,
  observer-loss, heartbeat-only, stop and resume gates remain unqualified; detailed
  findings and fault matrix are in experiment/web-termux CURRENT.md.
  Clarified acceptance requires ChatGPT's genuine intermediate reports and native
  tool execution/results in the normal Codex transcript, persisted for resume;
  a spinner, final-only reply or separate viewer is insufficient. Current adapter
  emits only tool/final envelopes. Pinned upstream already parses commentary
  items; independently bridge actual remote reports/tool calls into ordered
  Responses items without additional user prompts, duplicate text or a second
  executor. Remote report capture, render/persistence and reconnect remain gates.
  Non-MCP source investigation inspected 19 exact-revision repositories and matched
  three JS assets to the installed 26.1007.21434 app.asar. The app has an actual
  tool-role result constructor (Nwi/Hwi); Temporary Chat declares no local
  functions and normal default declaration is handoff. A subsequent authorized
  live probe on GPT-6 Instant appended one arbitrary echo signature: a real server
  call, exact call-ID tool-role return through bRt and an unpredictable exact final
  proof all passed. Subsequent dependent chains of 2/4/8 results each used one
  client user submission and 2/4/8 exact tool-only returns. All 17 actual request
  starts and terminal reason=done events passed, including complete unpredictable
  final receipts; generation retries were disabled and wrappers restored.
  The two-result harness's completed-vs-done classifier was corrected from recorded
  events without repeating inference. Each chain kept its server conversation;
  working_turn_id/turn_exchange_id had 3/5/9 distinct values. Single host
  turn retention and Chat-limit savings are unproved. Full Core integration,
  schemas, approval, stop/resume and transcript remain gates. GPT Actions
  is a real non-MCP REST path, but belongs to Custom GPTs; official plugin migration
  does not carry custom actions over. It is not a recommended new long-term base.
  Source tracing explains the boundary: bRt creates a fresh tool message without
  original working/exchange metadata and starts action=next. Correct call-ID
  pairing does not imply a retained host exchange. The app uses those IDs for
  grouping/stream correlation; no ID-to-ordinary-Chat-limit mapping was established.
  Scoped approval/async ID inheritance is not a generic callback guarantee.
  Native remains a low-debt candidate, not a demonstrated quota-saving winner;
  compare a true awaited callback and a properly scoped Chat counter before that
  choice. Follow-up ID review used zero inference and changed no runtime.
  The subsequent complete tool-lifecycle review supersedes local-function-first
  implementation: declaration, Core execution, MCP settlement, approval/input,
  projection and cancellation must be understood before the next trial. The app's
  direct bRt import callers publish accepted/rejected handoff results; ordinary
  remote MCP transcript parsing and widget RPCs are separate. Reference full mode
  resolves the pending MCP Promise from Core's result without a new Chat submit;
  the installed adapter bundles only parser/SSE helpers, not that full mode.
  Version matters:2026 MCP adds requestState/inputResponses with fresh RPC IDs
  and request-SSE-close cancellation, unlike2025 resumable HTTP. These RPC IDs
  are distinct from Chat working/exchange IDs and prove no allowance deduction.
  Official tunnel stdio also serializes calls and retires late response IDs;
  broker parallel proof is not through-tunnel proof. No inference, app launch,
  provider change or implementation followed this review. The next gates are
  actual channel/version/capabilities, then a separately budgeted two-call Chat
  proof and native transcript/stop/resume acceptance. Detailed call graphs,
  unknowns and evidence order are on experiment/web-termux CURRENT.md;
  private manifest: chat-tool-system-source-review.json in the acceptance cache.
  Detailed live/source evidence is on experiment/web-termux;
  private proofs are ~/.cache/codex/acceptance/chat-local-function-live.json and
  chat-local-function-chain.json. Two app streamPost entries were observed in
  the first successful probe; an earlier browser
  interception attempt may have sent one additional request and was disclosed.
  No installed provider activation, credential extraction, reference program
  execution or quota-limit experiment occurred; temporary app wrappers were restored.
- `/model` is the primary opt-in selection; `--profile chatgpt` is its fallback.
  stable51 includes the qualified native menu and generic shared config projection.
  The provider remains an opt-in experiment, not a bundled Core dependency.
  One private core/config/profiles/chatgpt.config.toml owns the registration;
  six existing homes have aliases. An empty new home on installed stable51
  automatically receives its alias and completes two native commands/exact reply
  without creating base config or copying authentication. Manager is unnecessary
  for projection; local profile overrides win. Native model/account defaults stay
  unchanged, and app Chat login remains independent of Codex account selection.
- Actual Android `/model` round trips in legacy and named-permission fixtures
  enter Chat GPT-6/Instant and return to OpenAI. Same PID/PTY/home/thread,
  immutable frontend, official backend and configuration/permissions are retained;
  thread/read verifies both routes and a sibling retains server defaults. No model
  inference or Pro selection occurs in this menu gate. Combined frontend fresh,
  cold-resume and /btw fixtures pass six native commands and six guardian requests,
  retaining no-sandbox and independent auto_review selection. The lifecycle proof
  waits for the delivered fork response; an earlier unsynchronized sample failed.
- Real-use failures on 2026-10-14 remain unresolved. The shared backend logs an
  unknown Chat model; a fresh request reaches the local adapter but returns 400
  `unsupported_desktop_request`. Switching the existing long thread attempts
  previous-model `gpt-6.1-sol` compaction through the newly selected Chat endpoint
  and fails before sampling. The adapter's 2 MiB body ceiling independently
  returns 413 in a no-inference local probe; retained active-history serialization
  exceeds that ceiling, but the failing HTTP body's actual size is unmeasured.
  These are integration gaps, not evidence of a Chat message-limit rejection.
  Prior menu-only and direct-profile proofs do not qualify long-thread switching
  with actual inference. Detailed evidence, uncertainty and required regression
  scope are maintained in [experiment follow-up](https://github.com/humtr/codex/blob/experiment/web-termux/CURRENT.md#chatgpt-model-errors-next-work).
- Provider0de70e3072a4b8e87a31 is installed from experiment source46db05e.
  The signed Linux app26.1007.21434 owns actual
  Chat submit/cancel/state, login and token refresh. Independent temporary Chat
  conversations/ledgers/cursors serve native threads and guardian children through
  one private FD/CDP connection; no UI Send clicks or extracted first-party tokens
  are required. App/display remain dependencies. UI fallback is serialized and
  allowed only before submission; ambiguous failures never replay a native action.
- Real overlapping GPT-6 replies, cancelled-lane/surviving-sibling proof, two native
  Android clients/four commands and two actual guardian reviews pass. The initial
  incomplete guardian sibling remains failed evidence; its bounded retry passes.
  Final two-client/four-command concurrency and two guardian reviews pass on the
  corrected transport. Installed guardian and native catalog-search proofs pass.
  App/adapter restart
  followed by actual submit/cancel, cold same-thread Core resume and two native
  commands/exact reply also passes. Native Core owns local execution, approval,
  history and checkpoints; the adapter converts only advertised text/tool wire.
- Instant/Medium/High/Extra High each pass two actual native commands and exact
  final reply through the new internal transport. Pro entries stay execution
  unverified: automated Pro inference is prohibited and was not performed.
  GPT-5.6 remains deferred after earlier continuation failures. Internal Medium
  compaction now passes four commands, actual summary/secret persistence and both
  Core and whole-app restart/resume. The candidate's shortened execution/context
  instructions were restored to the qualified shared UI prompt; reply parenting
  is explicit and startup waits for the actual app scope before choosing fallback.
  GitHub catalog search and the actual discovered read-only repository call pass
  in wrlab. Compared szob returns no GitHub matches for the same native query;
  a no-inference controlled comparison returns0 GitHub matches there and8 in
  wrlab. Native account tool availability is separate from shared Chat app login.
  Failed/timeout trials stay retained as evidence. Automatic/fault compaction,
  multimodal context, hours-long load and physical phone reboot remain separate
  internal qualification. Phone horizontal width remains deferred. Opaque agent
  payloads/routes refuse, but a newly reproduced bundled-parser gap replaces an
  unreadable native compaction checkpoint with a note. Readable canonical handoff
  or explicit refusal remains required; this is not qualified lossless resume.
- `/v1/responses` is the wire interface, not an API entitlement. Requests use the
  signed app's Chat mode; measured native context usage is an estimate, not a
  billing/KV-cache proof. Earlier quota comparison observed no increment even in
  the positive Codex control, so it cannot prove zero Codex deduction. Plus SIWC
  public Responses access is a separate unqualified track; desktop Chat acceptance
  does not grant it. No paid API-key inference or organization-policy change occurred.
- Maintained setup and detailed contracts live on experiment/web-termux. Private
  evidence in ~/.cache/codex/acceptance includes chat-model-shared-device-{legacy,
  named}.json, chat-combined-permissions-{synchronized,named}.json,
  chat-internal-{cancel-resume-retry,installed-guardian,installed-search,
  installed-medium,installed-high,installed-xhigh}.json, chat-stable51-new-home.json
  and chat-{app-concurrency-cancel,native-dual-shared-profile,
  native-dual-guardian-retry}.json. Final internal proofs include
  chat-internal-checkpoint-startup-wait.json, chat-internal-discovered-tool-wrlab.json,
  native-account-catalog-comparison.json, chat-final-concurrent-guardian.json
  chat-final-app-acceptance-timing.json, chat-final-medium-overlap-cancel.json
  and chat-final-installed-inventory.json. App submission timestamps now come
  from response acceptance inside the app; polling timestamps were too late
  to prove overlap for very short replies. Tiny Instant responses can also finish
  before the sibling is accepted; those overlap trials remain false rather than
  becoming proof of a transport defect. Medium proves actual overlapping app
  acceptance, and the independent native guardian gate proves concurrent requests.
  Cancellation and surviving sibling succeeded in the earlier short-reply trials. No user credentials or
  conversation bodies are published with the proofs.
