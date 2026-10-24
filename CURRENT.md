# Operating state and product completion

Reviewed 2026-10-24 UTC. `rewrite/rust-core` owns implementation; `main` pins
accepted exact sources for signed publication. SPEC owns product contracts,
README user instructions, and this file the current operating state. Superseded
incident/release narratives remain in Git; original private evidence remains in
`~/.cache/codex/acceptance`.

## Tool-call interoperability: Provider10 delivered

The research session01a16cf1-847a-7b30-9868-28c74e5cef1b selected
native gpt-6.1-sol/high at2026-10-24T09:15:34Z, but its09:15:53Z follow-up
failed with HTTP400: a retained68-character call_id exceeded64 characters.
Selection succeeded; history admission failed.

Responsibility/data flow: gateway_calls produces portable native Call IDs while
retaining the app host ID for ACK/results. Broker dispatch IDs also use64-hex
digests because Operator directly correlates those IDs with native results;
logical retry keys and the journal format are unchanged. Native and Chat continue
through the existing upstream transcript/executor, without an upstream patch,
transport proxy, migration flag or second history store. The user explicitly
excludes retroactive repair; old oversized historical IDs are left intact.

The first773c937 candidate missed the direct Operator consumer: one command
executed, but its result failed with mcp_result_invalid. It was rejected before
signing; its effect and failed native turn remain preserved without replay.
Source6e26cee includes both the Broker and gateway corrections. Local full gates
(369 Rust/141 Python), Provider gates (182 Rust plus Bun/Python), targeted faults
and direct diff/producer-consumer review pass. Mutations detect old Broker IDs,
unbounded app IDs and oversized batch IDs; disposable source is restored.
Independent-agent and GPT Pro reviews were not performed.

Signed Provider10 `chatgpt-6e26cee6d342-s10`, source6e26cee, is fully publicly
read back, Android-consumed, stable-promoted and activated for the ID repair.
Provider12 below is current, with Provider11 retained for rollback.
Binary SHA256 is `c742fdd1ba66ba4315d8bb3fe839601b70ad633223785eee6ab7bc2dc1d6a64a`.
Source40096385599, Provider/Android40096385723 and signing40097313205 pass.
The12 public assets, installed compositions, signed package lifecycle, exact
hosted native execution and public install/use/rollback pass. Non-forced stable
promotion changes only Provider index/signature/source; installed Full107 and
its frontend/runtime bytes remain unchanged.

After activation, three real primary turns pass on one new UUID: Chat reads one
owned file through one native command; gpt-6.1-sol/high answers from its result;
Chat then quotes that native answer. The tool ID is64 bytes, prior canonical
turns survive and the workspace is unchanged. Both Chat turns select none effort.
Own work Stop confirms native and host stop. This actual logged-in-app/native
inference proof is separate from synthetic Android admission tests.17 protected
config/auth/launcher inputs, eight pre-existing work identities/phases,18
registered native server process identities,16 MCP tools and fresh tunnel
polling are positively read back. Physical second-client display was not observed.

Host-only Stop initially refused an old reconciliation_required record. Its
thread was unloaded on all18 verified servers, its last native turn failed,
and it had no driver/pending calls or active app lane. Normal Provider maintenance
shutdown then released its package lease; that record was not repaired or marked
completed/stopped. A separate observer overcounted unregistered runtime PID4843,
which exited without captured command provenance. Its purpose/cause is unknown;
the incidental all-runtime-PID survival condition is rejected. Original failures
remain evidence. Read-only verification covers the registered native owners,
protected inputs, work inventory, signed selection, actual service/stdio images,
app readiness and tunnel polling; no native server was stopped by the update.

Evidence: `~/.cache/codex/acceptance/call-id-compat-kdt41vdr/`, especially
`delivery-final/{device-activation-proof,live-device-smoke,consumer-proof}.json`,
package/composition records, targeted-fault logs and the original failed observers.
Implementation, qualification, signed publication and device activation are complete.

## Project selection and app recovery: Provider12 delivered

Provider12 `chatgpt-5d2204a6c664-s12`, source5d2204a6, is signed, publicly
read back, Android-consumed, activated and stable-promoted; Provider11 is rollback.
Installed Core remains Full107 with its qualified c55a5c9 frontend. Source
40122894013, Provider/Android40122894169 and signing40123806220 pass. Public
promotion0eadaced changes only Provider index/signature/source, without forcing
history or modifying Base/Full heads.

The newly reported thread01a16ee2-04c0-7a91-afa5-9de91bde8189 executed12
native calls with64-byte IDs and all Broker results settled. Its final public
answer is retained locally and in the authenticated saved Chat. The app logged
404 `Resume stream unavailable` at17:11:00Z and successful polling recovery at
17:14:45Z (attempt40). Provider10 missed that recovered completion, timed out at
17:24:29Z and rejected the next input with `lane_reconciliation_required`.

Responsibility/data flow: the existing bridge now observes the app submission's
higher-level `onCompletion` as well as raw stream completion, using the same lane
epoch and cancellation fences. Failed/interrupted completion remains failure;
displayed text alone cannot settle a turn. The SDK single-tool-result helper
discards completion callbacks, so its separate submission branch/capability gate
is removed. User input and local tool results share the existing submission
implementation. First transport-error and resume metadata reaches content-free
Provider logs; no timeout extension, replacement inference or upstream patch
was added. Direct affected-flow review and four disposable faults detect missing
completion, the obsolete helper, retain-mode selection and late old completion.
Disposable source is restored; independent-agent/GPT Pro review was not performed.

A final same-epoch approval-stream audit reproduced a late original submission
callback settling the newer host reply. The additional source correction shares
one completion observer between ordinary input/results and delegated approval,
and fences it by both epoch and stream serial. The failing original audit and
its passing regression are retained. Provider12 includes this correction and
retains the verified project and original creation without another project.

The initiating Chat transport loss remains unestablished. The404 belongs to the
app's own resume endpoint, which the Provider new-dispatch hook does not intercept.
A separate local native app-server websocket1006/pong timeout does not prove why
Chat's first stream was lost. The missed completion is a verified repository
code defect; the original initiating fault is not attributed without evidence.

The project request previously used `history.mode=retain` and invoked no creation.
`projects list|use` CLI and the `codex_projects` MCP control now use the common
account-bound create/reuse/readback resolver, then atomically save project mode,
name and exact ID. Remote failure preserves local policy; concurrent config edits
preserve the remote binding for retry without duplicate creation. Creation locking
moves from individual AppHosts into that resolver. Existing conversations retain
their destination. Only the new project tool declares a new write effect; other
qualified tool annotations remain unchanged. Broad setup UX remains deferred.

The actual logged-in app created one owned private `Codex` project under
Provider11; Provider12 reuses that same verified destination. Readback and
a second use of its exact ID pass without inference or another creation, and
history policy is now project. Four real primary turns on one new native UUID
pass: Chat performs one native file read, Chat answers a follow-up, native
gpt-6.1-sol/high continues, then Chat returns. Both saved Chat conversations are
positively read back in that project. The one tool ID is64 bytes, previous turns
and workspace bytes survive, and own native/host Stop are confirmed. This was
normal live inference; missing-terminal recovery is independently exercised by
source faults with the pinned-app fixture, not claimed as an induced live outage.
Local184 Rust/77 Bun plus Python/lint/release and the armed actual Android native
command/patch/same-thread return fixture pass; its app/model replies are synthetic.
The exact hosted executable, compositions, signed package lifecycle with owned
TLS,12 public assets and disposable public install/use/rollback/stable update pass.
The first Provider12 live fixture submitted0 primary prompts: model re-entry
showed the native read-only/other-app ownership guard. Its terminal and native
records are preserved, with no inference replay or lock bypass. A new fixture
waited for the frontend's actual ready composer before model input and completed
the four turns. The original lock holder is unverified; no Core code or timeout
was changed, and the first failure is not accepted proof.

For activation, the reported failed work's native Stop and zero jobs are confirmed,
but its legacy app Stop remains unconfirmed. Its saved committed final is freshly
read back. Normal Provider maintenance shutdown fences MCP and closes its owned
app; the failed record remains reconciliation_required, never relabeled completed
or replayed.19 registered native server process identities survive. All17 protected
inputs are unchanged during the final activation; the earlier authorized Provider
history update is verified separately. The sole intended policy change is Provider
history selection. The same app identity,17-tool MCP inventory, shared generated
routing and final ready/idle app state are verified. Physical second-client UI
was not observed.

Evidence: `~/.cache/codex/acceptance/post-answer-recovery-20261024/`, especially
`delivery12/{final-device-proof,project-selection-proof,live-project-membership,
live-device-smoke,stable-consumer-proof}.json`, original project creation in
`delivery/project-selection-proof.json`, original failed observers and
mutation logs. Completed inactive fixture payloads are archived through the
existing cleaner with restore records; current/rollback artifacts and the unknown
repository target are preserved. Reusable compiler caches remain below15GiB.
KST is enabled in the user's ~/.bashrc and login ~/.bash_profile. Both fresh
interactive and login shells report TZ=Asia/Seoul and KST/+0900; originals or
absence records are retained privately. Running processes keep their environment;
canonical UTC records remain UTC.

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
The continuity repairs and their actual delivery boundaries are recorded below;
the shortcut investigation does not itself qualify those repairs.

## Bidirectional continuity: repair and qualification

Responsibility/data flow: Core owns canonical turns, settings and native execution;
the frontend restores saved model metadata before backend startup and preserves
saved settings across in-app resume. Provider owns the effective tool catalog,
work/controller reconciliation and its positively identified app stream. The
existing upstream native queue owns deferred user messages. There is no second
scheduler, transcript or execution route.

The requested redo replaced the superseded candidate. Current Provider source is
aeaefa7; frontend source is c55a5c9, whose recipe is unchanged by the later Provider
fixes. Behavior changes repair resume, declaration updates, completion observation,
hook ownership, transport recovery and queue admission; these are not described
as behavior-preserving refactoring.

| Reported boundary | Cause and resulting behavior |
| --- | --- |
| `native_context_prefix_changed` | Cold resume restored the Provider definition without its model catalog, changing the developer prefix. The existing startup path now restores saved Chat metadata before the models manager starts. Explicit catalogs/model overrides remain authoritative; prefix validation remains intact. |
| Warm `/resume` selected the wrong model | Model-picker re-entry's generated CLI flags became sticky overrides for another saved thread. The existing coordinator now distinguishes explicit selection from saved resume; saved settings are restored at Core-owned Chat/native boundaries. Unknown/custom-provider resume keeps upstream behavior. |
| `ambiguous_tool_declaration` | Historical revisions were treated as one conflicting declaration batch. Each batch is validated independently; chronological incremental updates replace older declarations and an explicit complete current catalog takes precedence. Same-batch conflicts remain errors. UUID/position heuristics were removed. |
| Completed work blocked the workspace | Completion depended on a later status poll; a late exchange finisher could restore running. One rule observes the actual completed native turn, idle app lane and no owned jobs, with matching owner/turn identity. Its terminal-event observer, status and admission share that rule. Replaced watchers are cancelled; no successful Stop is fabricated. |
| Ordinary messages in the same Chat were intercepted | Conversation ID alone retained hook ownership after settlement. Hooks now require positive stream/request/lane ownership. Ordinary user/tool streams pass through; owned permission replies, late requests and Stop fences remain protected. SDK pending submission is reserved before invocation, including synchronous continuation. |
| Managed-to-current follow-up retained old ownership | Settled claim preserves its transfer boundary; transfer detaches only passive owned hooks. Missing app ownership releases idempotently without launching an app. List supplies the existing work's home and source conversation so the caller can reuse it. Ordinary Chat text alone does not steal an active writer. |
| MCP/service replacement broke observation | One stdio framing process verifies the current authenticated service binding per RPC. A closed native websocket is discarded for the next new request. Uncertain mutations are never replayed. Device stdio invokes the admitted immutable Provider directly; the tunnel no longer restarts merely because Provider PID changes. |
| Queue messages were test-only | `codex_sessions action=queue` and `work queue` use the existing native queue. Explicit thread/text/stable call ID are required; lost receipts reconcile from upstream queue/canonical history without resend. Explicit execution home supports an unloaded saved thread. ACK proves enqueue, not consumption. |

Added: saved-catalog restoration at the existing startup boundary, the internal
selection/resume distinction, one native completion observer, current-binding
stdio recovery and user queue admission. Removed: duplicate declaration schema,
UUID/position guesses, old inline re-entry construction, startup-cached stdio
authentication wrapper, closed-client reuse, conversation-ID-only ownership and
the redundant immediate completion RPC. The transition proof now uses the actual
supplied Provider executable instead of a synthetic queue service; its warm-resume
observer checks the actual resumed UUID in the re-entered frontend process.
The obsolete synthetic fixture marker has no remaining consumer and is removed.
Retained adapters cross actual Core/app/stdio/native process boundaries. No
temporary product path or removal condition remains.

Actual Android proof uses the hosted frontend and exact hosted Provider binary:

- Same-UUID native→Chat→cold resume→native and in-app warm saved-Chat resume,
  preserving existing turns, saved effort and prefix metadata, without Manager.
- Two native execution accounts and a changed workspace; Provider/native owner
  replacement; Stop; first/existing-turn failure with readable history and native
  recovery; mode-only Plan retains selected high effort against a none default.
- Persisted completed before any status poll, then same-thread settled
  claim→transfer→accept→current native file write/final in the attached TUI, with
  no new managed submission for the current-Chat follow-up.
- Warm and unloaded queue consumption, lost committed/queued receipt recovery,
  owner-unavailable receipt readback and changed-input rejection, without resend.
- Exact hosted executable performs native command, custom patch and final through
  the existing MCP→Core path. The first observer incorrectly compared contextual
  envelopes; its failed turn is preserved. The original effect occurred once;
  distinct follow-up patch/final completed without replaying that command.

These native/managed fixtures use inert auth and local synthetic Responses/app
replies, not real Chat reasoning. The actual logged-in Linux app/SDK construction
probe additionally passes same-identity/profile checks with zero conversation
POSTs; injected SDK-start failure is not real Stop proof. Its owned app context
was closed and services restored. Physical simultaneous display in a second
ChatGPT client was not observed.

The original `Session terminated` source remains unverified. Tunnel logs do prove
an earlier stdio exit/restart and later polling disruption; they do not prove the
native task terminated or the tunnel stayed continuously connected. Original
native completion was22:41:30Z (next-day KST07:41:30); the separate Chat audit
finished around22:50Z. Saved creation times and later replay display are not new
native execution timestamps. Persisted Chat history and live second-client UI
delivery remain distinct observations.

Local Provider/Python/Rust gates and direct affected-flow review pass. Hosted
Source40069962639, Core40070000629 and Provider/Android40069962667 pass foraeaefa7.
Exact-bound Source40073657638 and Core40073658822 also pass. Source fault
mutations detect chronological first-wins declarations, missing
completion observation with zero status polls, stale startup stdio binding,
closed native cache, conversation-ID hook capture and app acquisition during an
absent-owner release. Disposable mutations were restored. Native40069297349
passes build,55 focused tests, saved-catalog mutation,94 retained upstream tests
and lint. `native-admission.json` verifies its source/recipe/artifact and retained
proof. Iron Laws, independent-agent and GPT Pro reviews
were not performed: none is installed as a gate, direct execution is requested
and no Pro facility is provisioned.

Failed observers and real guards remain evidence, not accepted successes: the
CI Unix-socket fixture needed explicit0600 under umask022; product socket checks
were retained. A mutation driver initially chose a stale Cargo artifact and was
corrected to the emitted executable. The queue proof no longer prepares over a
private fixture profile. No skip/xfail or weaker acceptance criteria were added.

Evidence: `~/.cache/codex/acceptance/bridge-incident-20261024/redo-high/`, especially
`warm-chat-final-android.json`, `exact-queue-cold-android.json`,
`account-bidirectional-android.json`, `exact-release-native.json`,
`actual-app-hook-probe3.log`, the `*-mutation.*` records, original failures and
the final gate logs. Read AGENTS.md again after recovery before editing/reviewing;
inspect this section, the actual diff and unresolved boundaries, not a summary
claim that newly added code is established architecture.

## Current status and next action

Signed Provider12 `chatgpt-5d2204a6c664-s12`, source5d2204a6, is fully publicly
read back, Android-consumed, activated and stable-promoted; Provider11 is rollback.
The project/recovery section above owns its proof and remaining cause boundary.
Core remains Full107; its native runtime/frontend are unchanged by this repair.

Current device readback proves the admitted12 service/stdio images, same logged-in
app identity/readiness,17 authenticated MCP tools,19 registered native owners and
account-local config/auth preservation. Shared routing is rebound to12. The new
Codex project is verified and selected for new lanes, and four real primary turns
including a consecutive Chat follow-up complete. The reported older failed work
remains recorded for reconciliation. The following activation/restart observations
are retained earlier boundaries, not a statement that an earlier Provider is selected.

The first activation attempt correctly refused an inherited package lease and
left selection8 unchanged; services were restored. The exact native catalog
holder had zero loaded threads and zero child processes. Its metadata was saved
before PID-bound graceful shutdown and kernel exit confirmation, then a fresh
exclusive-lease check preceded activation. `host stop` separately refused
`host_active_work`; that guard was not bypassed. Inherited package leases remain
the documented maintenance boundary; no active native work or unverified owner
was stopped.

Signed Core Full107 `local-relay-hosted-0-164-0-deaf199f7667-s107`, sourcedeaf199,
upstream0.164, is published, fully publicly read back, disposable-update qualified,
stable-promoted and activated. Full106 is rollback. Publication40073946157 passes
the existing exact-source build/Android smoke/signing/readback/consumer/CAS path.
The separate actual-device public consumer also passes; its installed frontend
equals the admitted c55a5c9 bytes with only the existing54 Termux substitutions.
Core remains byte-identical to Full106; the frontend is replaced. Device activation
preserves15 protected files, all three Provider service PIDs and27 pre-existing
native processes, with zero observed exits. No user client/job was stopped.
Existing clients still run their pinned executables; a newly opened frontend
uses107. Installed selection is not a hot swap of an already-open TUI.

Account/workspace IDs, installation and tunnel choices belong to config/runtime
discovery. No production account/plugin-ID whitelist was introduced. App identity
comes from the authenticated app scope; native admission follows the uniquely
verified thread home/workspace. `history.account_change` remains configurable
(`new_chat` default, `refuse` optional). Settled new input can bind the selected
account; pending/unknown input/results cannot migrate between accounts. Fixtures
cover identity/workspace changes; no physical second-account login switch is
claimed. Another workspace's Secure MCP still requires its administrator
credential and matching Linux app login, as the user explicitly acknowledged.
No additional registration, tunnel or login change is part of this maintenance.

For upstream0.164, retain inline model/reasoning/speed/permission arguments,
saved-thread settings/compressed-history resume, repeated CLI overrides and
compaction corrections. Reviewer eligibility is unchanged; the discarded
Untrusted experiment stays removed. Original incident and earlier Full106/
Provider8 real-model evidence remain under
`~/.cache/codex/acceptance/account-independent-20261023T145152Z/`; they do not
establish a changed boundary's acceptance. Retained Provider9 additionally
completes one actual logged-in-app first prompt from an isolated execution home,
returning the exact marker through installed Full106. The empty workspace and
inert native auth remain unchanged; its own work Stop and backend kernel exit
are confirmed. Installed Full107 then passes ordinary cold resume without profile,
model or catalog flags and in-app warm resume from a different native thread,
with two distinct real Chat prompts. The same UUID retains all3 canonical turns,
each recorded Chat effort is none, native return preserves them and the workspace
stays empty. Managed completed is observed through list before any Provider status
poll. Own Stop and backend kernel exits are confirmed. Key records are
`redo-high/live-fresh-provider9/live-first-prompt-result.json`,
`live-full-resume/{result,saved-read-proof}.json`, `full-consumer-proof.json`
and `full-device-proof.json`.

Saved-source observation distinguishes explicit Stop from ordinary completion.
The first proof explicitly stopped its owned work before cold resume. The existing
`Phase::Stopped` contract starts a successor Chat for a new native input and keeps
the old source in `previous_chats`; both are positively read back. The two107
cold/warm finals are in the same current Chat; the first fresh final is in its
intact predecessor. The initial observer wrongly expected all3 finals in one Chat;
its mismatch and corrected boundary are retained. No prefix guard was bypassed
or native history replaced.

The additional actual native-only cold-loss proof passes without intervening work
Stop or Provider restart. It confirms completed work and an idle settled app lane,
closes only the owned TUI/idle backend, then plainly resumes the saved Chat and
submits one distinct next prompt. Work ID, native UUID and saved conversation ID
all remain identical. Both assistant finals are read from that same Chat; all5
canonical turns survive, native return preserves them and only the final cleanup
stops its owned work. All5 real Chat requests completed; no request was replayed.
`live-uninterrupted-cold-2/result.json` records this boundary. The first attempt
submitted0 model prompts: its read observer crossed the re-entry socket rename.
That original failure is retained; only read observation retries the demonstrated
socket replacement, without broader error swallowing or a longer timeout.

Remaining boundaries: physical live display in a simultaneous second ChatGPT
client and the original remote `Session terminated` cause are unverified. Local
stdio replacement, saved Chat persistence and native completion do not establish
those external client observations. No further source/runtime repair is pending
for the qualified boundaries above. The existing inherited-package-lease
maintenance requirement and workspace-scoped Secure MCP prerequisites remain.
Unknown `providers/chatgpt/target/` is preserved.
Reusable compiler caches remain below15GiB; manifest-matched inactive fixture
payloads are archived with restore records, retaining native records/source.
This completion batch reclaims2,722,209,792 allocated fixture bytes; it does not
prune user state, the unknown target or operating current/rollback generations.


## Operating baseline

| Plane | Verified state |
| --- | --- |
| Implementation | Provider12 verified project selection and stream-bound recovered submission completion, with portable IDs and retained continuity/queue repair; cold/warm saved-Chat frontendc55a5c9 remains qualified and installed |
| Operating Core | Signed Full107/sourcedeaf199, previous Full106, upstream0.164.0; existing clients retain their pinned executables |
| Public/selected Provider | Sequence12/source5d2204a6, previous11, signed/read back/Android-consumed/promoted/activated; native defaults/auth retained |
| Host/profile | App26.1007.21434 under the Rust-owned private-pipe host; authenticated readiness and actual fresh inference accepted, original login/history retained |
| MCP | Authenticated local17-tool inventory, including explicit project selection, accepted; additional workspace registration awaits its administrator credential and matching Linux app login, an acknowledged setup prerequisite |
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
