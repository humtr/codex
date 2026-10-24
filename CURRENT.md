# Operating state and product completion

Reviewed 2026-10-24 UTC. `rewrite/rust-core` owns implementation; `main` pins
accepted exact sources for signed publication. SPEC owns product contracts,
README user instructions, and this file the current operating state. Superseded
incident/release narratives remain in Git; original private evidence remains in
`~/.cache/codex/acceptance`.

## Reasoning-effort incident and redo boundary

The original thread's `thread_settings_applied` records establish the actual
selection change on2026-10-24: high→medium at02:30:32.045Z, then low at
02:30:32.055Z. The user restored high at02:41:02.946Z. These are KST11:30:32
and11:41:02. Current configuration and the earlier high `turn_context` did not
disprove that change; the earlier denial used the wrong evidence. The preserved
17 sampling-configuration log records in the low-selection interval still report
High for the already-running turn. They establish that recorded request scope,
not the absence of a TUI selection change or unrecorded requests' settings.

Upstream0.164.0 binds both Shift+Down and Alt+comma to immediate effort reduction,
accepts key repeats, and emits the user's exact lowest-level warning from that
shortcut handler. It updates TUI state, then sends `thread/settings/update`;
Core persists each resulting snapshot. Two such submissions precede the two
recorded reductions. This explains a reachable accidental navigation/selection
path without a model change, config rewrite or Provider round. Actual installed
Android TUI reproduction yields high→medium→low and the identical warning.
The precise original key is not recoverable: retained TUI logs start after that
event and there is no original input recording. Do not attribute that key to the
user, the assistant or a terminal defect as an established fact. Other producers
checked include accepted model migration, the usage model-switch prompt and
explicit model selection; none is established as this event's source.

The user identified accidental shortcut input as likely and explicitly cancelled
keymap prevention. The temporary opt-out in all6 existing execution paths
(4 distinct config files) was removed; each file again exactly matches its
pre-change bytes. README prevention guidance was also removed. No model/effort
was forced, no user login or credential was changed, and no running user job or
TUI was terminated. The shortcuts retain their original behavior.

Actual Android Core/installed Full106 TUI, isolated signed fixture, inert auth and
loopback Responses passed the following producer-to-consumer checks. Each case's
own canonical thread and emitted follow-up request were checked independently:

| Input/configuration | Canonical turn efforts | Result |
|---|---|---|
| Default Shift+Down, three presses | high, low | Two reductions and exact lowest-level warning reproduced |
| Disabled shortcuts, Shift+Down | high, high | Navigation does not reduce effort |
| Disabled shortcuts, Alt+comma | high, high | Alternate reduction binding disabled |
| Explicit F6 remap | high, low | User-configured shortcuts still work |
| Disabled shortcuts, explicit `/model --reasoning=medium` | high, medium | Manual model settings remain available |

The successful fixture made15 local synthetic Responses requests across5 primary
threads,0 real-model dispatches. Original failed fixtures remain preserved: one
exceeded the Unix-socket path budget; one omitted the config-level endpoint and
received401 from the official endpoint with an inert fixture key; two subsequent
observers wrongly assumed a single request/thread despite native auxiliary work.
The corrected observer uses each primary thread's canonical completion and
effort, alongside the emitted request. No user auth was copied. These failed
attempts are not accepted proof and are not hidden by a longer timeout or weaker
product assertion.

Retained change: an AGENTS rule distinguishing settings history from current
config and request evidence, addressing the incorrect earlier diagnosis.
There is no remaining keymap/configuration behavior change or temporary product
code. The isolated keymap controls above are reproducibility evidence, not an
active prevention policy. Local docs gate and final diff review passed; Iron
Laws, independent-agent and GPT Pro review were not performed.

Evidence: `~/.cache/codex/acceptance/bridge-incident-20261024/redo-high/` contains
`effort-settings-timeline.json`, `sampling-effort-summary.json`,
`effort-shortcut-android.json`, the diagnostic driver and original logs, and
`effort-keymap-{application,reversion}.json`. The earlier13-file unshipped continuity candidate
was preserved as `superseded-candidate.patch`/`superseded-source` and reverted to
HEAD for the requested redo; none of its checks admits it as the final repair.
The broader continuity repairs below remain open; this investigation does not
claim their deployment or runtime replacement.

## Open bidirectional-continuity findings

The installed Full106/Provider8 acceptance below covers its recorded scenarios;
it does not establish ordinary cold resume while already using Chat, automatic
Chat-to-native follow-up, or live display in a second ChatGPT client. The user's
reported failures expose remaining defects. Source repair is in progress;
installed selection, account login and existing user work remain unchanged.

Responsibility/data flow: native Core owns canonical turns and execution; the
frontend must restore the selected model's Provider route and metadata together.
Provider owns effective tool declarations, work/controller reconciliation and
the app stream. A saved Chat read proves persistence, not another client's live
display or acquisition of the same native controller.

- **Cold Chat resume loses model metadata.** Original work8437349b / native
  thread01a16ae9 initially used a catalog-backed server in janmori101; its failed
  resume used a general server in wrlab. `add_installed_provider` restores only
  the Provider definition, whereas explicit model entry also supplies
  `model_catalog_json`. An isolated installed-Core replay of the original two
  completed turns reproduces this boundary: with the generated Chat catalog,
  the first47 canonical items exactly match the stored Provider prefix. Without
  it, the same model ID gets a different first developer instruction
  (184 versus21,546 characters), invalidating that prefix. This explains the
  reported `native_context_prefix_changed` without corruption of the saved turns.
  The native models manager owns its catalog at backend startup: adding a
  thread-scoped path alone is insufficient. The current source resolves saved
  Chat metadata in the existing startup/reconnection path before attachment;
  explicit catalogs and model/effort choices remain authoritative. Sourceedd8151
  builds and its55 focused native regressions pass. An actual Android isolated
  same-thread native→Chat→cold Chat resume→native run passes without Manager,
  using4 local synthetic Responses requests and0 real Chat requests. Both saved
  history and saved effort survive plain resume without model/catalog overrides.
  The native mutation correctly fails with Ok(false) versus Ok(true), but run
  40062934666's observer incorrectly expected plural `1 tests`; that observer was
  corrected and exact-source full native qualification is running again.
  The prefix guard remains intact; no replacement Chat hides a failed resume.
- **Historical/current tool declarations collide.** The native tool scope has15
  tools; collected `tool_search` descriptions differ across revisions (13,185
  versus13,856 characters). Installed Provider rejects those differences as
  `ambiguous_tool_declaration`. The corrected source validates each declaration
  batch independently, folds chronological incremental changes, and gives the
  current request's explicit top-level complete catalog precedence. Native input
  history with AdditionalTools remains incremental, even if the current feature
  toggle is disabled: prompt.tools is empty and the client adds no rebuilt prefix.
  A genuinely fresh nonincremental request carries the explicit current catalog.
  No UUID-version or first-position heuristic remains. Original two-turn replay
  contains a full catalog followed by a one-tool update; chronology preserves
  the later revision without rejecting legitimate updates. Seven focused parser tests pass; a
  disposable source mutation selecting the first history revision fails with
  the expected old/current-contract mismatch and was restored. Offline parsing
  of original installed-Core captures succeeds with0 app dispatches: the
  catalog-backed capture uses its later475-character search declaration; the
  non-Lite current catalog correctly omits historical search capability. A
  helper initially assumed that capability existed in both captures; that failed
  observer is retained and its corrected result is not a product fallback.
  All4 actual-wire captures parse offline, including fresh nonincremental input;
  chronological first-wins mutation fails and disposable source is restored.
  Full local Provider acceptance passes on the final event-observer implementation.
- **Workspace admission depends on stale phase and observation order.** Managed
  exchange leaves a successful round marked running. `codex_status` can reconcile
  an idle native completed turn and idle app lane to completed; installed create
  checks stored phases without that observation. Candidate source moves that one
  completion rule into native-event observation, status and workspace admission;
  an old exchange finisher cannot overwrite completed or stopped. One observer
  per work is cancelled on replacement/Stop/service close. Actual Android proof
  explicitly waits for persisted completed before making any status request and
  passes. A same-thread claim→transfer→accept from settled managed work also
  passes actual Android MCP→Core execution: one native file write and final
  reach the attached TUI with0 new managed model submissions. The first driver
  failed to use start's rotated controller; its original failed observer is
  retained and corrected to consume that response. A completed idle claim now
  retains its route-transfer boundary, and managed transfer releases its passive
  app hooks. Work listing supplies execution home and saved source ID so a
  follow-up can select the existing work. Recovery routing now leaves saved
  model/effort untouched; only
  explicit initial Chat selection supplies Provider defaults. Mode-only starts
  read the thread's current settings instead of applying those defaults. Actual
  Android Plan-start metadata and the new canonical turn_context both retain
  selected high effort against the Provider's none default. The actual saved Chat first requested
  list, then create in prj/codex, then created a separate temporary work after
  refusal. It did not continue the existing work. Reconcile the owning native
  turn/jobs and app lane at admission, retain one writer, and use the existing
  same-thread transfer/accept contract for current-Chat control. An ordinary
  saved-Chat reply alone does not transfer control. Automatic same-Chat follow-up
  still needs verified source/work routing rather than creating another work.
- **MCP transport did experience disruption.** Tunnel logs show the stdio MCP
  child exit at2026-10-23T22:45:27Z, a failed restart during a Core activation
  transaction, and a successful new child at22:45:33Z. The initial exit's cause
  is not established by those logs. At the later Chat retry, 2026-10-24T01:00:21–
  01:00:28Z, polling timeouts occur and none of those three tool requests appears
  in the local dispatcher log. Polling reports recovery at01:01:54Z. The same
  tunnel process remains alive; current external host/list/status/read requests
  and healthz/readyz succeed. `Session terminated` is not emitted by our MCP
  server and does not prove native termination. Its precise remote session cause
  remains unverified; neither permanent tunnel failure nor guaranteed continuous
  connectivity follows from these observations. Recovery should reinitialize
  the observation transport and reconcile the original request/work, never replay
  an uncertain mutation or require a blind service restart.
  Candidate stdio keeps one framing process and verifies the current service
  endpoint/credential for each RPC. Actual CLI integration passes service loss,
  one explicit error, replacement-service recovery and EOF without desktop/Core
  launch. A disposable startup-binding-cache mutation makes that regression fail;
  restored source passes. Native websocket cache also discards a closed transport
  for the next new request; in-flight mutations retain unknown-delivery errors.
  An owned real Unix websocket peer proves a lost mutation reply remains unknown,
  its next read reconnects to the same process and the effect is not replayed.
  This local proof does not establish remote MCP session recovery.
- **Projection and client visibility are distinct.** The original saved Chat
  read enumerates105 source message entries across five user messages; hidden,
  unsupported and redacted contents retain the reader's explicit limitations. Only
  its first two user requests correspond to the original completed native turns;
  later direct Chat inputs and its temporary-work audit do not automatically
  become turns in that native thread. AppHost projects its owned stream into
  native output; the bridge has no delivery acknowledgment from another ChatGPT
  client's visible conversation. Saved original responses are present, so
  missing persistence is not established. A simultaneous physical second-client
  live-display reproduction was not performed. Candidate hooks no longer own a
  settled conversation's ordinary new user/tool stream just because its Chat ID
  matches. They retain positive lane/request/host-reply ownership and Stop fences.
  The SDK submission is reserved before invoking it, covering synchronous result
  continuation as well as asynchronous starts. All56 app-wire contract tests pass;
  actual app ABI/second-client refresh remains a separate acceptance boundary.

User queue capability now uses the existing native queue, not a second scheduler.
`codex_sessions action=queue` / `work queue` preserve model/effort, require explicit
thread/text/stable call ID, and recover from the upstream queue or canonical
client-message item without resend. Actual Android native gpt-6.1-sol proof passes
warm enqueue/consumption, lost-receipt recovery after commit, owner-unavailable
receipt readback, changed-input rejection, cold/unloaded explicit-account enqueue
without inference, queued lost-receipt recovery, and consumption after plain
resume. Six local synthetic Responses requests,0 real Chat requests; same UUID
and original turns preserved. The first fixture tried to prepare over its own
non-generated transition profile; the production preservation guard correctly
refused. The corrected fixture retains that file before explicit preparation.
The original failed diagnostic is retained. Queue ACK remains distinct from
receiving-turn proof.

Local Provider, Python and Rust acceptance pass. Final Provider gate exit0 is
retained in `redo-high/final-provider-check-result.json`; all runtime tests,
clippy, release build, Bun and Python gates pass. Clippy found an unnecessary
borrow introduced while moving the completion rule; it was fixed at that call,
without suppressing the diagnostic. Direct diff/production-flow review removed
a redundant immediate completion RPC after every managed response: only the
native terminal observer owns proactive completion, using the app lane committed
before its Completed event. Final Android proof still passes with no status call.
Omitting that observer in disposable source makes the first completion assertion
fail with0 status observations; service close subsequently fences the work as
reconciliation_required. An initial mutation driver picked a stale Cargo artifact;
that rejected observer is retained and the corrected driver uses Cargo's emitted
exact executable path. App-hook ownership and closed-native-cache mutations also
fail their respective contracts, with disposable source restored. Existing app
permission/Stop tests were corrected to identify their owned request explicitly;
ordinary unowned user and tool streams are now covered independently. No tests
were skipped or acceptance tolerances weakened. Independent-agent/GPT Pro review
was not performed; this workspace requests direct execution and no Pro review
facility is provisioned.

Additional native investigation reproduced in-app `/resume` after a model-picker
re-entry: generated CLI flags made an unrelated saved Chat use the current native
model. Source0387c7c reuses the existing re-entry coordinator, with an internal
launch-choice versus saved-resume marker; later in-app resume restores saved
settings instead of treating generated flags as user CLI intent. It extracts the
existing source guard/request construction, removes their old inline copy and
adds no history conversion or summary. Cold and warm routes still use one Core
launcher/backend binding implementation. Original user CLI model overrides keep
upstream behavior. Native source build/mutation/full acceptance is running at
40069163154; older40066060334 was cancelled as a superseded exact-source run.
Android warm-resume acceptance is pending the new executable. Original failed
probe remains `redo-high/warm-resume-probe.py` and ct.wbp2i211; no real model was
called.

Unsigned source candidates and synthetic integrations are not publication or
device activation. The runit
stdio wrapper still invokes Core and the tunnel keeper still restarts on service
PID change; their documented target is direct admitted immutable stdio execution
and per-RPC binding recovery. Apply only during bounded device maintenance after
qualification, preserving user work/config and existing remote registration.

Original native completion is2026-10-23T22:41:30Z (KST next day07:41:30), not a
new execution at the next resume. The temporary Chat-origin audit finishes around
22:50Z (KST07:50). The user's "Codex closed" retry is01:00Z (KST10:00). Saved
message creation timestamps can precede the stream's completion; compare native
task completion and correlated calls rather than sorting creation times alone.

The earlier completion notice uses upstream `codex queue --thread ... --message`
and verifies a new canonical receiving turn in the bound native supervisor. It
does not select a Chat model or mirror an idle ChatGPT conversation. That proof
and native→Chat→native model entry are different workflows; neither establishes
the complete bidirectional behavior now requested.

Evidence: `~/.cache/codex/acceptance/bridge-incident-20261024/` contains
`correlation-summary.json`, `catalog-prefix-reproduction.json`,
`installed-parser-reproduction.json`, `tunnel-{timeline,lifecycle}.json`,
`health-current.json` and private loopback request captures. The first capture
attempt rejected an incomplete fixture model schema before sending any request;
its original failure is retained. Corrected catalog/no-catalog captures each
made one loopback request, deliberately rejected by the receiver. Actual parser
probes each contained a guaranteed invalid final input. Total real Chat/model
dispatches for this investigation:0. Existing user conversations and jobs were
not stopped. Candidate changes replace strict cross-revision tool merging and startup metadata
loss; they add no account-specific branches. Queue reuses existing mutation
receipts because its consumer must distinguish a lost ACK from a new enqueue.
Signed publication/device activation and actual app/live-client qualification
remain separate unfinished boundaries.

## Current status and next action

The deployed baseline below retains its original scoped evidence. Address the
open continuity findings above before accepting the requested bidirectional flow.

Signed Provider8 is published, promoted and installed:
`chatgpt-a5cd3e20c683-s8`, sourcea5cd3e2, Android SHA256
`c6851da66fbee5fd3df9ce6a1cf0a1d7a804aadd8aee333443756c8673610e09`.
Provider7 is the rollback generation. Signed Core Full106/source72b6ad4,
upstream0.164.0, is published and installed; Full102 is the rollback generation.
Its native frontend source18ef075 uses the saved thread's model for handoff,
independent of a missing/different account default. Real populated native→Chat
response→native return passes on the installed generation. Another account's
MCP registration needs that workspace's administrator credential and the matching
Linux app login in the current workspace-scoped setup. Those prerequisites are
not present; registration absence is expected, not an outstanding wrapper defect.

Account-specific identifiers are not compiled into the production Provider,
Core or Manager source audit. Native admission discovers the uniquely verified
loaded thread across execution homes and follows its canonical home/workspace;
`execution.home` and `execution.workspace` are defaults for explicit new work.
All six existing execution homes use shared generated Provider routing, now8.

The Linux app account comes from its current authenticated scope. Policy is
configurable in `~/.config/codex/providers/chatgpt.toml` as
`history.account_change` (`new_chat` default, `refuse` optional). The redundant
explicit default was removed after checking Provider7 rollback compatibility;
the effective behavior stays `new_chat`, and Provider7 validates the current file.
After a settled round, a genuinely
new native user input opens a new chat under the selected app account. `refuse`
retains the original-account requirement. Pending/unknown/old-turn requests and
results cannot move into another account. Existing settled response fingerprints
remain compatible with Provider7, so upgrades do not replay completed requests.

A replaced login scope/client retires only passive owned hooks, then binds the
current context. Allocation and the actual conversation POST validate identity;
a login change between those steps cannot dispatch across accounts. Engine and
app-wire fixtures cover account/user/workspace changes and return, configured
refusal, active/pending guards, same-renderer scope/client replacement and a
pre-POST account race with zero network requests. These are simulated login
identities; no physical login switch to another real ChatGPT account is claimed.

Actual installed Provider8 returns the exact requested marker for one fresh
native Chat-model prompt from an owned external execution home; workspace remains
empty, the owned work is stopped and backend kernel exit confirmed. Provider7's
original real completed turn and all failed/aborted witnesses remain preserved.
The actual app uses its existing login; native fixtures contain only inert auth,
never copied user credentials. Authenticated installed MCP initialize/tools-list
returns16 tools with0 model requests. Provider8's startup observer first timed out
at12 seconds; a later read-only status proves readiness without repeating install
or app acquisition. All13 recorded protected native auth/config/Core/tunnel files
and the three pre-existing work records remain unchanged through activation.

Full102 ordinary cold resume without `--profile` reads the original completed Chat
turn and returns to native with0 new inference. Its fixture copies public model
metadata only, because inert native auth cannot fetch a live native catalog.
A further actual nonempty native→Chat attempt initially submits0 prompts and
refuses: TUI checkpoint preparation incorrectly reads optional `config.model`
instead of the verified thread's restored model. Direct Provider encoding of
the same snapshot succeeds. Source18ef075 removes that dependency; its regression
covers missing/different defaults and retains the child-stdin EOF requirement.

Native40007659045 passes55 focused and94 retained upstream tests plus lint.
Source40007647072/Core40008201400 pass; the first Core attempt's public download
502 is retained, followed by a successful failed-gate rerun. Exact binding72b6ad4
passes Source40013225767/Core40013228235. The changed Android cold-resume regression
passes without Manager/default model, preserving history through native→Chat→native,
with3 synthetic local Responses requests and0 real Chat. Signed publication
40013805699 passes Android build/smoke, complete public readback, disposable update
and non-forced stable promotion. Physical102→106 preserves15 protected files and
all three Provider service PIDs, stopping0 existing native jobs.

Installed Full106/Provider8 now resumes the same original real populated thread
without `--profile` or a configured model, enters Chat, completes one distinct new
prompt with the exact requested marker, and returns to native with both completed
turns preserved and an empty workspace. Its post-return work cleanup command
correctly refuses `native_workspace_or_route_changed`; do not count that command
as a successful Stop. A separate0-input reconciliation passes: it re-enters the owned Chat
route, stops only that qualification work and returns to native with both turns
unchanged. The first reconciliation observer encountered a transient removed
socket during re-entry; the corrected observer retries only that read. The
original failed cleanup/result remains preserved beside the reconciliation proof.

The broader Android cohort passes two native execution accounts, workspace
changes, cold backend/Provider replacement, Stop, first/existing failure, readable
history/native return and ordinary cold resume:10 native inputs,8 synthetic app
submissions,0 real Chat. The additional Full106 populated real re-entry proof covers the missing
default-model case separately. The account-owner and Android hidepid fixes use
verified process identity/stat evidence, retain all records and do not signal
unrelated runtimes. Failed history is an allowed re-entry boundary only with full
public-history and input/tool/activity guards. Ordinary resume adds only a
non-selecting installed Provider definition; upstream remains the sole backend.

For upstream0.164, retain inline model/reasoning/speed/permission arguments,
saved-thread settings and compressed-history resume, repeated CLI overrides and
compaction corrections. The wrapper adaptations above resolve discovery, routing
and handoff assumptions around those upstream behaviors. Official release:
https://github.com/openai/codex/releases/tag/rust-v0.164.0 . Upstream reviewer
eligibility remains unchanged; the discarded Untrusted experiment stays removed.

Provider8 passes177 Rust tests/Clippy (28 existing opt-in tests ignored),71 JS and
the CI Python gates. Exact Source40005181142, Core40005270102 and independent
Android Provider40005181160 pass. Actual Android composition84/package83 steps,
signing40006385857, complete12-asset public readback,10 immutable consumer steps,
7 default-stable/config-policy/rollback steps and non-forced promotion pass.
Base/Full heads were preserved by Provider promotion. Full102 publication
40000802027 and physical101→102 update retain their original protected-state proof.
Current existing tunnel health/control-plane polling also returns `ok`.

Termux Bridge has no account/plugin-ID whitelist. Connector names remain
`transport.connector_name`; tunnel IDs/private credential references remain in
the tunnel-client profile, with remote organization/workspace association config.
The existing tunnel has one organization and one workspace. The user clarifies
that the available administrator credential is not for the additional account's
workspace (`szob`), and using that workspace's Secure MCP connection also requires
logging into the Linux app as that account in the current setup. The missing
registration is therefore an expected consequence of absent workspace credentials
and the unmatched app login, not evidence of hardcoded ownership or a wrapper bug.
Account-independent code does not grant cross-workspace permissions.

Retain the original additive association attempt's403
`tunnel_principal_association_unverified`, request
`req_82da4b5f380f41ea8887ee5c323c0ae2`, and unchanged-association readback as
observations under the available credential. That response does not establish
that a reviewed override is the only route with the correct workspace credential
and matching login. No further provisioning, login change, additional tunnel or
credential change is requested by this clarification.

Evidence root: `~/.cache/codex/acceptance/account-independent-20261023T145152Z/`.
Key records: `app-account-provider8-final/{plan,delivery,consumer-proof,stable-consumer-proof,device-mcp-metadata-proof}.json`,
`app-account-provider8-final/activation-2/device-activation-proof.json`,
`live-fresh-provider8/live-first-prompt-result.json`,
`live-plain-resume-return-3/result.json`, `combined-final-acceptance.json`,
`live-existing-prompt-8/{result,direct-checkpoint-private}.json`,
`frontend-saved-model-plan.json`, `saved-model-android-2.json`,
`core-saved-model-{public-plan,device-proof}.json`,
`live-existing-full106/result.json`, `live-full106-reconcile-2/result.json`,
`app-account-provider8-final/rollback-config-compatibility-proof.json`,
`tunnel-complete-pair-proof.json`,
`final-account-source-audit.json`.
Original failures are retained and never counted as successful gates.

Completed owned composition/package/consumer payloads were archived using
`clean_development.py` with distinct restore records. Preserve original native
records, source, current/rollback public packages and unknown
`providers/chatgpt/target/`. Reusable local compiler targets total about5GiB.

## Operating baseline

| Plane | Verified state |
| --- | --- |
| Implementation | Provider8 account/config/context repair and saved-model frontend18ef075 bound in72b6ad4 accepted, published and applied |
| Operating Core | Signed Full106/source72b6ad4, previous Full102, upstream0.164.0; existing clients retain their pinned executables |
| Public/selected Provider | Sequence8/sourcea5cd3e2, previous7, signed/read back/Android-consumed/promoted/activated; native defaults/auth retained |
| Host/profile | App26.1007.21434 under the Rust-owned private-pipe host; authenticated readiness and actual fresh inference accepted, original login/history retained |
| MCP | Authenticated local16-tool inventory accepted; additional workspace registration awaits its administrator credential and matching Linux app login, an acknowledged setup prerequisite |
| Retired source | Experiment refs/history, unique user changes, original snapshots, licenses and restore records remain preserved |
| Retained dependencies | Immutable c3642b19a87cdace158f JS/Bun/Python installation, rootfs, optional noVNC, current/rollback payloads and original proofs remain |

The following retained baseline records describe earlier qualified deliveries;
they do not supersede the current selections above. Read the original
`binding-release-20261023T030234Z` before maintaining its tunnel/transport scope.

Device evidence: `~/.cache/codex/acceptance/device-runtime-update-full93-provider-s3`.
`device-update-review.json` and `followup-full94/result.json` accept the actual
Core/Provider selections, kernel owner plus immutable Provider hash, same-account host readiness, authenticated
MCP initialize/list/read (16 tools), existing tunnel readiness and8 CLI/config
smokes. All ordinary account config/auth bytes remain unchanged. Only the known
generated shared ChatGPT profile was explicitly rebound; account-local links and
native defaults remain intact. The native socket budget was checked at84 bytes.

The idle legacy desktop was closed only after all5 retained works were stopped,
no active lane/control owner, and no UI draft/stream were verified. Its exact
source profile/history remain in the original location. The new guest preserves
1,865 app-profile entries and1,758 guest native login/history entries; IPC/tmp/
writer-lock directories were excluded from the copy. Guest `.codex/auth.json`
is required in addition to Chromium's `profile`; omitting it caused the preserved
first app_auth_context_unavailable observation. The corrected same-account read
passes without re-login or model work. The initial cold app_scope_unavailable,
relay-rejected Browser.close (no effect) and unsupported `--profile features`
observer command remain original failures; they were not retried as effects.
Supported `--profile chatgpt mcp list` loads the generated profile successfully.

H source admission subsequently promoted Full94 automatically. The final93→94
update uses that new immutable stable input, not a replay of the completed86→93
cycle. The same-account host stays ready and account config/auth hashes remain
unchanged. Previous93 and the original86/88 backup remain.

The permanent MCP wrapper is `~/.local/state/codex/providers/chatgpt/mcp-stdio`;
the existing codex-provider-rust-trial alias now invokes it. Old experiment
launchers, sources, rootfs/noVNC and original conversations remain preserved;
no live/unknown user state was pruned. Native model inference and physical viewer
usability were not retested. Do not rerun the completed update/install/migration
or old qualification roots. A future update must inspect current selection,
package leases, actual owners and pending work first.

### Model configuration and session authority

The current primary Codex thread is01a165ad-9447-7ea1-a85c-5a7592af7bf9.
Its canonical turn_context confirms gpt-6.1-sol/high; ordinary config is unpinned.
Quota-driven profile switches and supervisor resumption are excluded from product
failure/recovery costs. The user's latest decision removes forced ordinary model/
effort defaults. All6 default/account config paths (4 distinct files; api/awrk
already alias the default file) now omit top-level model/model_reasoning_effort
and the unsupported `[models.new_thread]` table. That table was an erroneous prior
edit and caused the reported ignored-setting warning. Actual installed Core
`features list` succeeds for all4 distinct configurations without unknown-setting
warnings;0 model requests. Other parsed settings, account boundaries and existing
symlinks are preserved. Evidence/backups:
`~/.cache/codex/acceptance/model-config-correction-20261022T220051Z`.
Only qualification supervisor launches explicitly select Sol6.1/high, through
`~/.local/state/codex/g-qualification-sol61.sh`. ChatGPT trial models are separate. Runtime
and resumed/explicit session choices may differ from ordinary defaults; inspect
canonical turn_context before claiming the executing model. Do not add a global
model pin or auto-upgrade/model-routing policy during handoff.

Core's `~/.local/share/codex/core/config/config.toml` is generated system-layer
output, reconstructed by render_core_config/system_config::prepare. It is not a
user-editable common-preferences file. Account `CODEX_HOME/config.toml` files are
user settings and are not overwritten by that generation. A future common user
preferences surface would need an explicit input/merge contract, not manual edits
to generated output or symlinking all account settings. No such feature is added.

## Components and approved architecture

Every Termux Codex execution passes through Core. Minimum operation is upstream
plus Core; Manager, TUI and Provider are independent optional components, not a
mandatory launch chain.

| Component | Owns | Does not own |
| --- | --- | --- |
| Upstream | Inference/tool/reviewer engine, canonical transcript, threads, turns, compaction, writer locks | Signed generation lifecycle or Android window policy |
| Core | Qualified executable/environment/FDs, home/shared-layout/server binding, install/update/rollback/provenance | Account/window/quota/model policy or Provider transport |
| Manager | Profiles, quota/rotation, notifications/windows/title/tmux and task discovery/handoff policy | Another executor/history store or runtime pointer authority |
| TUI | Upstream rendering and optional /switch, permissions and /model integration | Provider transport/host supervisor or another model catalog |
| Provider | Rust config/catalog, model/tool adaptation and shared Chat/account/CDP/host/lane management; common interfaces implemented and qualified at the bounded F integration scope | Native execution/approval authority, canonical history or Core runtime selection |

Core links no Manager/Provider implementation. `runtime-binding` is its small
execution-record contract consumed by optional tools. Core-only operation bypasses
optional preparation/config/TUI but retains execution-home/shared storage/runtime
validation. Per-launch effective settings and server FD34 snapshots are immutable;
optional fragments are admitted by the actual upstream loader before dispatch.
Malformed/hung optional components preserve Core defaults and confirmed home.
No fallback after handoff may replay work or change an explicitly selected account.

The current config-v1 execution record is the only codec. Unsupported records
permit no mutation; preserve their processes/data. Historical experimental codec
emulation and staged reader/writer migration were rejected. Core retains necessary
shared server/config isolation; removal of catalog namespaces is not required to
achieve optional-component independence.

Public naming is TUI. Existing frontend/manager-tui paths and signed markers retain
their spelling until a qualified migration. Catalog model/effort/context data
belongs to Provider configuration; neither Manager nor Core maintains its own list.

## Product completion scope and work order

| Stage | Disposition |
| --- | --- |
| A–D | Shared native correctness and bounded real development/comparison accepted at their exact inputs |
| E | Functional, ownership, context, project, host and idempotency decisions complete |
| F1–F9 | Common Rust bridge, delegation, reading, continuity and host integration boundedly accepted |
| G | Exact-source capability review, corrected native-final delivery, signing/readback, actual Android consumers and non-forced stable promotion accepted |
| H | Unique work preserved, duplicate source/worktrees retired, affected Provider/full gates independently reviewed, source committed/pushed e0f0927 |
| Device update | Initial G/H activation: Full94 / Provider sequence3. Binding correction: Full95 / Provider sequence4, same-account host, authenticated16-tool MCP and remote polling. Empty-session correction: Full96 / previous95, same Provider4/services/config/auth and exact qualified TUI hash;0 real model requests |

### G product qualification

G's corrected source skips only verified stale/nonruntime historical bindings;
selected-owner identity, private metadata and peer validation stay strict. The
original reviewer returned allow and its program executed exactly once, with3/3
original native returns, before the old source turn failed without a final. That
failure stays failed. The separate same-thread report-only turn used1 actual
GPT-6-thinking request,0 native tools,23.933s; CLI/native/saved final matched.
Both Stops, no jobs, candidate/native owner exit, bridge/input-observer cleanup
and original remote16-tool/read restoration were independently verified.

Useful exact-source proof (all roots below are private acceptance archives):

| Latest proof | Accepted scope and limits |
| --- | --- |
| `phase-g-source-installed-6aa2280` | Hosted Provider workflow39850738497 and source workflow39850738448 pass on exact6aa2280. Seven-file artifact/source readback and4 actual Android probes pass. Private installed composition84 steps plus signed-package113 steps pass; independent audit matches197 exits,185 noncredential stdout/113 stderr hashes and4 signatures. Twelve credential bodies are omitted.464.935s,0 Chat/native-account requests; same-primary canonical notification acknowledged. Archive manifest verifies513 original files |
| `phase-g-native-discovery-fix-5da0254d` | Correction's local Provider gate:167 Rust,162 JavaScript,111 Python, fmt/Clippy/release;28 opt-in Rust tests explicitly not run by that gate. Local Android reused-PID regression plus broader synthetic operator/cold-resume62.773s and managed lifecycle17.977s pass. The latter has5 synthetic inputs/78 observations/RSS growth3320KiB; its one cold native_socket_unavailable observation is retained.0 real Chat requests |
| `phase-g-native-discovery-fix-5da0254d/hosted-owner-reuse-6aa2280` | The exact hosted5ef3536c binary passes Android Status/Stop with an unrelated reused PID; unsafe metadata still rejects native_binding_invalid. PID bytes/mode restored, native owner/dispatch counts unchanged, stopped/no jobs, service exited and reaped.0 model/native inputs |
| `phase-g-native-discovery-fix-5da0254d/hosted-sessions-list-read-assertion-miss/reconciliation.json` | Original MCP list response succeeds and contains all3 works, including the retained stopped work54386f7c on thread01a16542. The harness incorrectly matched `id` instead of the documented `work_id`. Its failed assertion remains; offline audit accepts the original response without another call. The earlier claim that the work was omitted is withdrawn; there is no demonstrated list-discovery defect |
| `phase-g-source-installed-5da0254d` | Prior8d5d2056 hosted/private admission: source workflows,4 Android probes,84 composition/113 signed-package steps; no Chat. Exact earlier inputs and1387 archived hashes remain useful; they do not replace6aa2280 admission |
| `phase-g-tui-reentry-5da0254d` | Same original native thread retains prior turns and the explicit interruption, then completes one distinct follow-up on hosted8d5d2056.14 tests,5 new original calls/results; cohort10/10, no missing/replayed calls. Native final/original CLI/saved report match. One new GPT-6 request, cohort2, no automatic source summary.165.183s; supervisor-reconciled segments, not uninterrupted execution |
| `phase-g-review-timeout-handoff-5da0254d` and `phase-g-profile-switch-review-5da0254d` | grv.ns2g9obl completed its one development task and same-thread Chat handoff:11/11 original results,10 tests, unchanged protected files. Source generation1→successor2 and29 saved successor messages independently read; assistant finish matches native final. Original harness had a generation assumption and matched prompt text as completion; corrected audits preserve that distinction. Reviewer timed out before its program effect; no verdict is claimed for that run |
| `phase-g-review-policy-preparation-5da0254d` | gj.hnoqg7k6 completed its distinct task/effect with1 Chat request,3/3 native results and exact CLI/native/saved final. No reviewer ran: the new executable did not match the prompt rule. Keep this limitation and do not repeat the effect |
| `phase-g-native-review-owner-fault-5da0254d` | gk.hmaf439i used2 actual GPT-6 requests; unbound independent reviewer returned allow and the program executed once. All3 original calls/results returned; protected files unchanged. Old Provider5da0254d then hit the historical-PID discovery error and the source native turn ended systemError without a final. Reconciled native/host Stop, no jobs, candidate exit and original remote16-tool/read restoration pass. The original failed source result is still failed |
| `phase-g-actual-06062c04`, `phase-g-tui-first-and-failed-reentry-06062c04`, `phase-g-operator-cold-fix-local` | Earlier affected managed/direct development, first original-TUI task and real cold-owner failure/recovery evidence remain scoped to their inputs. Cold-owner/frontend fixes were admitted through5da0254d; accepted tasks are not repeated |


Corrected public delivery is archived under `phase-g-public-delivery-6aa2280`:
immutable release chatgpt-6aa228034346-s3; signing run39865112885;12 public assets
read back;10 immutable and4 stable Android consumer commands pass; non-forced
Provider-only promotion main6e89e76.0 model requests. The failed main-registration
workflow39864984102 remains preserved and is not a signing failure of the payload.

Final native proof is `phase-g-final-followup-6aa2280` (154 original files).
Cancelled preturn preparation and zero-request stale-observer cleanup remain
separate immutable archives. Never arm/replay gf continuation5 or an old run.py.
No hours-long reliability, actual allowance debit, hidden reasoning access,
unlimited Chat capacity or universal performance winner is inferred.

### H preservation and verification

`phase-h-retirement-6aa2280` preserves both source trees, unique user edits,
34 retired repository files, refs/licenses/provenance, tests and independent
review. Restore instructions identify exact worktree refs and patches. Active
Rust source, embedded app/viewer assets, explicit local-function fallback,
app-bridge/CDP tests and reusable qualification tooling remain.

Provider/full pass counts are in the baseline above. A quota-driven profile
switch interrupted the original foreground full check; its no-exit log remains
interrupted. Verified141 Python/fmt and170 Core passes were reused. The remaining
40 Core cases (plus1 ignored), other154 Rust cases and Clippy completed exit0.
No passed Core case was replayed. H's exact reviewed diff and retained evidence
hashes were checked again before committing, without rerunning accepted tests.

## Proposed ChatGPT control through MCP

The implemented and boundedly qualified control workflow lets ChatGPT discover
and manage local Codex work through MCP. Codex remains the execution, approval
and canonical recording system; Provider transports/control-adapts that path.
There is no second planning agent or transcript database. Termux Bridge names
the MCP connection, not another product component.

### Agreed bridge structure and E work

Entrance and reasoning context are independent: native CLI/TUI or ChatGPT can
control a work item, and inference can stay native or use a replaceable Chat.
One active controller owns each work binding; independent children/reviewers
retain independent contexts. General delegation and direct entry use the same
native execution/history/approval path. Defaults remain explicit in Provider
config; neither an entrance nor a tool annotation authorizes replay/fallback.

### Same-thread continuity and Chat context handoff

The primary native thread retains durable history while Chat conversations are
replaceable reasoning contexts. Summaries combine the accepted summary, confirmed
history since its boundary and current instructions/work state. Fence old control,
settle/reconcile calls and jobs, record the handoff, then require the successor's
exact current receipt. Failed/uncertain transitions preserve a recoverable boundary.
Origin/work/original CLI return binding survives. No summary covers later events
implicitly. Native compaction and Chat renewal remain separate operations.

Ordinary model selection reuses readable native history without a mandatory fresh
summary. An explicit source summary is a separate capability. Never decode/drop
foreign opaque checkpoints or replay tools to manufacture continuity.

### Idempotency preliminary research and phased work

E/F common operation, exact duplicate/result recovery and restart logic are
accepted at their bounded scopes. The Rust16-tool catalog applies12 true/4 false
idempotency hints; other annotation fields stay unchanged. The observed ChatGPT
backend omits idempotentHint; its internal consumption and causal performance
benefit are unknown. Logical call IDs, actual effects and original receipts—not
hints or JSON-RPC IDs—establish exactly-once/recovery proof.

### Linux app and host capability research

E research and F implementation cover authenticated app identity/project/read/
input, exact owner/CDP/private-pipe management, shared host controls and optional
viewer handling. Rust owns production host lifetime; retained CDP helpers are
qualification dependencies. App binaries/rootfs/login are separately provisioned,
not bundled into the Provider package or inferred from its model catalog.

## Context and easy configuration

Provider config: `$XDG_CONFIG_HOME/codex/providers/chatgpt.toml`, default
`~/.config/codex/providers/chatgpt.toml`. Rust owns configuration/catalog data;
Core/Manager/TUI do not maintain a second model-family list. GPT-5.6 is excluded;
Pro stays manual-only and excluded from automated trials; the Instant alias is
hidden. Listing a model establishes neither entitlement nor inference readiness.

`codex provider chatgpt config show/effective/validate/set/unset/edit` uses atomic
validated edits. `prepare --home PATH` explicitly publishes immutable native
catalog/profile routing for the next startup, preserving account config/auth.
Host routes/history are read at request admission; existing bound native catalogs
do not mutate. `status` is read-only; a failed readiness check does not authorize
restarting a live owner. Default native/user model config remains unpinned.

The user1,000,000-token setting is policy, not measured host capacity. Native
compaction/hash triggers, HTTP byte bounds and true Chat remaining context differ.
No independently verified remaining-context sensor exists. Summary acknowledgment
requires the next canonical input; identical text in an unchanged prefix is not
acknowledgment. Public ocx1 checkpoints contain readable context, not private
reasoning or decrypted foreign state.

## ChatGPT projects and history

Project/account/destination identity is explicit. Reuse the exact owned private
project; ambiguous/inaccessible destinations fail. Reconcile an uncertain create
before retry. Safe-boundary changes affect new lanes; preserve existing chats,
files/instructions/memory/access settings. Independent native reviewers remain
outside the main configured project. F destination/reading and G actual project-
guided development/handoff are accepted at their recorded inputs. Conflicting
instruction priority is not inferred. Broad setup UX remains deferred.

## Confirmed operating Chat blocker

Retained Full60 native→Chat history failed unsupported_desktop_route when an old
model's compaction reached the Chat endpoint, and includes an unreadable opaque
checkpoint. Preserve that conversation. Readable-history model selection and later
actual TUI return/re-entry pass; they do not make the old opaque input readable.
Use supported upstream readable handoff at a settled boundary. Original negatives:
chat-full60-operating-diagnosis.json / chat-full60-cross-provider-admission.json.
Known box/MemoryCite formatting fixes have bounded stream-safe proof; they do not
reclassify the historical failed trial.

## Deferred setup and repository work

These are follow-ups, not reopened G/H gates or authorization to start their
implementation during the documentation task.

### Shared minimum setup and login

Use one implementation behind an explicit setup entrypoint and fresh interactive
post-install invocation. Collect only missing inputs for the selected entrance;
reuse configuration, accounts and login. Native login, Linux app login and remote
MCP authentication remain distinct. Use existing installers/host/login controls
and model-free readiness checks. Preserve ordinary launch behavior and unpinned
native model settings. No new setup command exists yet. The manual procedure is
in the [Provider guide](providers/chatgpt/README.md#first-time-setup).

### ChatGPT entry and task management

Existing tools support listing/observing work, original result retrieval,
supported continuation and confirmed Stop. Users can select the installed Termux
Bridge MCP connection and ask short natural-language tasks; `reasoning=current`
and `reasoning=managed` are assistant-facing routing fields. Connection selection
and natural-language routing do not establish a universal automatic default.

The proposed convenience is a Termux Bridge plugin with one reusable workflow
skill for both routes, followed only as needed by a compact project/work panel.
It should reuse existing Provider controls and native history/approval, expose
readiness, bound workspace, reasoning source, work/thread identity, pending human
requests and original results, and reconcile uncertain operations without resend.
No bundled skill, UI resource, project picker or dashboard is implemented. Exact
packaging/selection behavior and actual client support need bounded proof before
becoming a public contract. See the [current/proposed split](providers/chatgpt/README.md#chatgpt-entry-and-task-management).

### Repository separation and cleanup

The user wants a clean development repository without accumulated PR/Actions
history; implementation is explicitly deferred. The intended layout is an old
repository retained as an archive, a fresh private development repository keeping
the `codex` name/source, and a separately named public distribution repository.
`codex-archive` and `codex-dist` are example names, not created destinations.
Preserve Git source/commit/tag provenance and the independent implementation and
publication histories; a fresh GitHub PR/Actions history does not require
squashing source history or deleting the old repository.

Only this owner's device is currently in use; the user accepts a brief install/
update outage. Zero-downtime migration is not required. Reusing the old repository
name removes its redirect, so explicitly reconnect bootstrap, Base/Full and
Provider indices, Release asset URLs, publication origin validators, source pins,
documentation and CI. Git source transfer does not transfer Release attachments,
Actions settings/secrets, app permissions or other repository metadata. Configure
scoped cross-repository publication rights; keep signing/auth credentials private.
Retain immutable original artifacts, signatures, proofs and rollback sources.
Future qualification must check fresh install and this device's Core/Provider
update paths against the new public distribution, with applicable signed release
and Android acceptance. Existing local execution can continue during the outage.
No rename, repository creation, visibility change, source move or endpoint change
has been performed for this task.

## Development artifact retention

Use TMPDIR for disposable roots and XDG cache for compiler outputs. Prune only
verified owned inactive payloads through `scripts/clean_development.py`, after
preserving source/diff/results/native records and current/rollback artifacts.
It verifies fixture key, exact manifest and absent process references; `gp.*/c`
uses the same guards. No age/name inference; unknown repo target stays untouched.

Latest two completed fixture batches archived/pruned12 native payload copies,
1,756,069,888 allocated bytes; original evidence/restore records remain. Last
compiler-cache observation was about4.7GiB, below15GiB. Retain needed executables
outside mutable outputs before pruning idle debug builds. Disposable qualification
used debug information=0, incremental=0, jobs=2. Accepted binaries need no rebuild
merely because their copies were archived. Pinned/rootfs/profile/user data is not
generated-fixture cleanup material.

### Verified execution pitfalls — check before continuing G/H

This is the user's requested pre-execution reference, reconciled against original
failure records on2026-10-22. Read the relevant row before forming a new command;
use the existing implementation/guard rather than reconstructing its procedure.
A failed prerequisite stops its dependent action. Never replay a completed effect
to fix an observer, packaging or reporting failure.

| Pitfall and original evidence | Required pre-execution check |
| --- | --- |
| Wrong signing ref: `phase-g-post-device-20261022T081337Z/dispatcher-registration/plan.json` explicitly says main always fails before signing. This session repeated it in39864984102 | Read the selected workflow body and compare remote rewrite/rust-core HEAD with the accepted source. Dispatch the implementation revision; main is registration/promotion only. Correct source run39865112885 already passed; never rerun its signing |
| Long native HOME: chat-entry.11uglt0w made a116-byte socket and consumed a Chat before native preparation failed; see its retained original acceptance failure | Call existing scripts/qualify_provider_chat_entry.py:check_native_socket_budget before fixture creation or model input. Budget is107 bytes including the actual absolute HOME layout. Retained gp.i3b9_52r/c/h is103; the copied gpub/consumer layout would be112. The operating HOME was checked at84 bytes |
| Downloaded Actions file modes differ from signed payload modes: this session's first raw-package check failed on Cargo.lock | Keep the raw download. Reconstruct a separate package with existing provider_publication.restore, then verify signature/inventory/modes. The earlier consume_public.py already uses this path; do not rebuild/re-sign or chmod the original evidence |
| Dependent shell commands continued after an earlier check failed: this session attempted consume.py after stage audit failed | Run dependent steps sequentially with checked exit/result predicates. Do not put the consumer, promotion or deletion after an unchecked shell command. Stage/result proof must exist and pass before its dependent action |
| Wrong runtime/interpreter/protocol and premature stdio EOF: gf continuation1–5 preparation records,0 model requests | Reuse real Termux sh/env paths, protocol2025-11-25 and request→response sequencing before EOF. Fixture PREFIX is a product fixture, not the interpreter authority. Check imports/assets and the actual tunnel child target before submission |
| Stale PID/URL/lease or incomplete cleanup: retained-owner discovery fault, cancelled gf continuation5, and gfinal.w8_8wm0n stale observer | Compare kernel start/executable, selected owner, work/history and pending calls. Verify native/host Stop separately; absent lane is an independently proven condition, not a fabricated ACK. Audit both bridge and input observer; release only an exact owned inactive hook and restore its original functions |
| False final/continuity assertions: grr generation assumptions, prompt-marker match and historical `id` vs `work_id` list check | Read the actual response schema. Require assistant final content and matching original CLI/native/saved records; do not match the prompt. Preserve original failed results and generation/controller boundaries. No model replay for an offline observer fix |
| Wrong receiving thread or model/config authority: prior-primary bindings and unsupported models.new_thread settings | Bind final notification to actual CODEX_THREAD_ID and verify canonical receipt/turn_context. Ordinary configs stay unpinned; only this development supervisor selects gpt-6.1-sol/high. Generated Core config is output, not the common preference file |
| Stale promotion parent and binary Contents API conversion: prior Full90 CAS rejection and binary-readback-red.log | Read the latest main only after prior mutations settle, preserve independent histories and use non-forced promotion. Reconcile unknown mutation once before any retry. Read signatures through canonical Git blobs/raw bytes, not the text Contents API |
| Retiring unique/user/current data as generic cleanup | Before H removal, preserve experiment refs, exact source/user1m patch and regression, licenses, original proofs and pinned/current/rollback dependencies. Check process references. Use clean_development.py only for owned generated payloads; never prune unknown target contents or an active/unknown worktree |
| Provider update stays busy after its services exit: binding-release-20261023T030234Z | Inspect kernel FD/lock holders as well as executable paths. A native catalog can inherit the Provider directory lease. Preserve metadata and prove no loaded threads, active jobs or children before gracefully stopping that exact idle owner; verify exclusive lease availability before a new update intent. Keep an unchanged-selection rejection distinct from an uncertain mutation |


## Verification and publication

Reuse green proof only for unchanged actual inputs/environment/scope. H changes
no compiled Rust or embedded runtime assets, so applicable accepted Android proof
is reused for those inputs. Provider/full source-removal gates pass separately.
Runtime/state/security/installer changes and releases still require affected/full
acceptance plus applicable Android execution proof. Failed/zero-test required
gates remain unfinished. Original private transcripts/account bindings stay private.

Public assets/history are immutable; release signing/readback/Android consumers
and non-forced CAS promotion retain exact-source bindings. Implementation and
publication histories stay independent. Public Provider source6aa2280 remains
accepted after H; H retirement does not relabel that release as a different source.
Device activation is a distinct user-authorized action with its own results.

### Remaining acceptance and delivery

This matrix records accepted G/H scopes and separately deferred follow-ups.
The A–H work is complete at those scopes. Future actual Chat trials need a declared scope/expected
submissions first. Do not turn completed fixtures/live proof into mandatory reruns
without changed inputs, failures or an unresolved boundary.
G applies to the paths/components retained after F integration, informed by D's
historical route evidence; replacement may retire an old
path-specific gate with its feature. Shared safety, supported upstream behavior,
protected state and signed lifecycle gates cannot be retired as path optimization.

| Area | Evidence already retained | Accepted disposition and separate follow-up |
| --- | --- | --- |
| Design and functional definition | E complete; F1–F9 bounded implementation/integration reviewed above | G retained-capability review accepted at6aa2280; E decisions remain closed |
| Idempotency contract and hints | Common operation/duplicate/recovery logic qualified; Rust12 true/4 false values applied on candidate40 with other hints unchanged; actual native23/23 plus retained loss/restart/summary faults | G packaged-path review retains these invariants. Backend consumption, causal speedup and billing remain unmeasured |
| Linux app/host management | E/F shared host CLI/MCP, private-pipe acquisition/Stop, endpoint reuse/RFB; G3 actual fresh-profile Android host tests and preserved-profile physical reboot recovery | G exact-input/ownership review accepted; current final followed fresh preflight; later device migration verifies the same authenticated account. New provisioning and physical viewer polish remain separate |
| Common bridge and usage routing | F shared managed MCP/local and current direct/general MCP; G four installed usage combinations and later affected managed/direct development | G exact-input review and distinct admitted-candidate native-final follow-up pass. No completed development or comparison is repeated |
| Coordinated agents and orchestration | F actual review/correction/dependencies, isolated writers/conflict and tree Stop; G independent real allow/effect plus separately accepted project/handoff development | Distinct same-thread native-final follow-up and retained composition/resource review pass. Original interrupted/failed cohorts stay failed; H retires duplicate source |
| Shared saved-Chat reading | F discovery/read of unrelated/work-linked sources, duplicate titles/branches/redactions and full ranges; G independently read29 successor messages and matched the assistant native final | G capability/readback review passes; current saved assistant final matches native/CLI. Hidden reasoning remains unavailable |
| Direct entry and delegated return | Earlier direct two-turn and F general delegation/question/final; G installed entrances and affected managed/direct development with original CLI/native/saved finals | G final review reuses these exact accepted scopes. Changed or uncovered future routes require targeted proof |
| User continuity and control | F existing-Chat input/current cursor/acknowledged transfers; G original-TUI same-thread return/re-entry, source generation1→successor2 and original Stop/history reconciliation | Distinct same-thread report-only final passes after the correction, with0 new tools or summaries. Ordinary readable-history model selection requests no fresh summary |
| Host lifecycle/visibility | F scoped Stop/failure fixtures; G actual3 owned fresh-profile host lifecycle tests, physical reboot/request-free recovery, private composition/restart/rollback and20 retained synthetic turns | G exact-owner, pending-call and Stop review passes. Current native/host Stop and restored remote16/read are independently verified; hours-long reliability remains unmeasured |
| Context and TUI | C/C2 actual development,5 same-Chat turns and actual successor development; F transfers; G actual project-guided handoff, original-TUI return/re-entry on hosted5da0254d, plus Android cold-owner and retained-turn fixtures | Installed TUI return/re-entry is closed at that scope. G final review retains applicable context/interruption guarantees. Old unreadable Full60 opaque history stays unsupported; capacity/compaction research remains deferred |
| Tools/approval/question | Full native catalog and retained function/custom/tool-return fixtures, actual command/patch/test development and original question answers; G independent real reviewer allow and exactly-once approved effect | G native-final follow-up and full-surface retained-proof mapping pass; no supported tool is omitted. Physical question presentation remains a separate observation; quoted executable prefix matching retains its upstream limitation |
| Existing MCP return | Actual R11/C/C2/F original call/result/final evidence, G current16-tool/read readiness and corrected list response with3 works; all3 approval-task native results returned | Hosted6aa2280 final delivery and route/Stop reconciliation pass. The old systemError source turn is preserved; program/reviewer/effect were never replayed |
| Cost/presentation | Stream-safe markup projection; D2/D3/F9 timing/request/byte/resource samples; G actual original CLI/native/saved reports and final-only same-primary notice acknowledgments | G result/presentation audit passes. Actual allowance debit and a universal performance winner remain unmeasured |
| Core update information/prompt | User-confirmed85→86 success; Core bytes unchanged; release-date/other display details observed as unknown | Low priority with the next substantive Core change: [deferred Core follow-up](#deferred-core-follow-up), including component information and a prompt on each eligible bare launch; preserve upstream/noninteractive behavior |
| ChatGPT project setup/management | Authenticated project operations and F exact destination/context contracts; G actual project-guided source/handoff and unbound independent reviewer | G retained project/identity mapping passes. Untested conflicting-instruction priority is not inferred; broad setup UX remains lower priority |
| Viewer and desktop | E app/host management research; F Rust private RFB/HTTP/control fixtures and actual owned fresh-profile RFB; existing notification/terminal Linux viewer opener; old desktop font/icon observations | noVNC usability retains lower priority: [development host management and viewer usability](#deferred-linux-chatgpt-development-host-management). Physical current viewer/keyboard/scroll/login claims require separate observation; old observations do not qualify a changed viewer. This polish is outside declared G and the viewer remains optional |
| Packaging/composition | Publicc65d3f8 Base89/Full90 and Provider release sequence 2 signing/readback and actual18-command Android consumer pass; latest6aa2280 hosted source/artifact,84 composition/113 private signed-package steps and Android owner regression pass | G corrected6aa2280 Provider release sequence 3 signing, complete12-asset public readback, Android10 immutable/4 stable consumer commands and non-forced main6e89e76 promotion pass. Old public assets remain immutable; activation is separate |
| Experiment retirement/publication | Unique source/user1m changes and immutable original proof preserved; earlier public delivery is accepted, latest correction is publicly admitted | G runtime/public admission accepted. H preserved unique source/user1m/licenses/refs and retired34 source files plus2 inactive worktrees; Provider/full verification and independent review pass. Pinned rootfs/viewer/current/rollback payloads and explicit Rust fallback remain |


### Deferred Core follow-up

With the next substantive Core change, improve installed/target component versions,
source/date information and missing metadata presentation. On every eligible TTY
zero-argument bare launch with a verified update, offer current→target details and
an update choice; decline/timeout must not hide the next eligible prompt. First
amend SPEC's display-acquisition/six-hour snooze contract. Arguments/non-TTY streams
must bypass discovery/prompts/advisory writes and preserve upstream behavior.
This remains deferred; runtime updating does not implement it.

<a id="deferred-linux-chatgpt-development-host-management"></a>

### Linux ChatGPT app management and deferred viewer work

Use Provider's `host status/ensure/open/stop`, `view`, saved-Chat controls and
shared authenticated facade. Rust ownership/status/Stop and optional RFB/viewer
have bounded proof. Accepted Android intent/HTTP responses do not establish
physical visibility, touch, Korean IME, scrolling, keyboard or login usability.
noVNC mobile polish, broad project setup UX, actual capacity/compaction policy,
optional TypeScript and app-free alternatives remain separate later work.

Native quoted-executable prefix-rule matching retains its upstream limitation;
reviewer allow/effect proof used a plain owned program path. Do not rewrite model
commands in Provider to hide that boundary. Actual costs, hidden reasoning and
long-duration reliability remain unmeasured. No optional follow-up reopens G/H.
