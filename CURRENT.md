# Operating state and product completion

Reviewed 2026-10-22. `rewrite/rust-core` owns implementation; `main` pins an accepted
exact source for publication. SPEC owns product behavior and trust boundaries,
README user instructions, this file current state and unfinished work. Superseded
audit narratives and plans remain in Git.

## Current status and next action

A–D are complete at their retained scopes. E design/source decisions and F1–F9
bounded implementation/integration are complete. G product qualification/delivery
is active: isolated Android composition and signed optional Provider package/
install/update/rollback checks have bounded acceptance. Physical reboot and actual
public wrapper update/use/rollback/use now pass independent review. Latest
Provider88257d2a passes84 composition and113 remote/signed-package steps; fresh
owned-profile host lifecycle passes3 actual Android tests without inference.
Exact source c65d3f8900bc77755407d137157890d71209c96e passes all three hosted
source/Provider/Core gates. Public Base89, Full90 and optional Provider2 are
signed, publicly read back and promoted non-forced. Actual Android consumption
of the Provider's public immutable and promoted stable paths passes18 commands,
including update/reinstall/rollback. This closes delivery, not all G runtime
boundaries. R11 proves one actual GPT-6 app dispatch and completed native task
from the original TUI after the accepted cold resume. TUI route downshift/re-entry,
broader host/approval/context and longer-duration retained-route proof remain.
Original failed batches stay failed; R11's original false-negative result also
remains immutable beside its separate success reconciliation.
H retirement follows
accepted replacement. The
[phase table](#product-completion-scope-and-work-order),
[F final review](#f-final-integration-review) and
[remaining acceptance matrix](#remaining-acceptance-and-delivery) define those
boundaries. F uses actual, synthetic and recovered-result evidence; it is not
whole-product release acceptance. Original failed runs remain failed.

The 2026-10-22 documentation review confirmed branch rewrite/rust-core and HEAD
2d81dd74b61e0469f2fe68a63404f70faf3c958f with existing uncommitted implementation,
tests and documentation preserved. All22 retained F proof hashes match. The
candidate40 manifest has186 files. Before this cleanup, differences were the
CURRENT/root/Provider README documents and the separately qualified test-only
owned_host.rs helper; this cleanup also updates SPEC and the TUI README.
Production inputs then matched candidate40. Subsequent G work adds the optional
Core/package interface and a model-free Provider artifact probe, qualified below.
Installed JS/Bun/Python adapter, Core/Full
generation86 and public/default selection were not changed by this review.
The historical F completion.json retains its pre-cleanup document hashes; no
retained proof artifact or original verdict was rewritten.

Passive observation at2026-10-22T00:14:09Z found candidate40's expected executable
and loopback listener alive, the shared Linux app supervisor at its recorded
process start, and the unrelated prior Provider executable/listener alive. This
does not re-establish login, model progress or functional readiness. Last retained
authenticated readiness/idle-state proof is2026-10-21T23:44:28Z below. No Chat
request, refresh, service restart, native work or new trial ran in this review.

Source1c58d1cb's ordinary same-thread model return without an automatic source
summary now passes both synthetic Android compositions and retained R11 history
selection. The new frontend/current Provider combination also passes combined
cold replacement/Stop and20 retained turns in two independently reviewed segments.
Next freeze/admit the changed exact Provider source/artifact, then close broader
host/context/approval and actual retained-route boundaries.
The previously published frontend requests a
fresh source summary for every nonempty Chat-to-native switch; source1c58d1cb
removes that picker policy at the user's direction and retains explicit summary
generation/completion handling for future context operations. Upstream still
owns model instructions, history reconstruction and conditional compaction.
R11's retained effective history is already readable. Exact native build,50 focused/
94 retained upstream tests, lint and both actual Android synthetic transition
cases pass; the retained-real-history selection case also passes, as recorded below. R11's
observer defects are corrected in an offline, unlaunched harness copy and covered
by nine local tests; do not spend another Chat request merely to replace R11's
false-negative receipt. The new Provider source requires its own affected
qualification and subsequent artifact admission; published Provider2 stays immutable. Delivery is independently accepted at
phase-g-post-device-20261022T081337Z/public-delivery-review.json. Source is
committed; do not rerun accepted signing, package, reboot or device development.
Accepted installed compositions, reboot and device update tasks must not be
replayed. Publication/default replacement and device
activation retain their separate gates. Do not rerun accepted F development just
to replace an old verifier failure. Billing/token debit and backend consumption
of idempotentHint remain unknown. The subsequent user instruction authorized G;
its current scope and evidence are recorded below.

## Development artifact retention

The user authorized reclaiming obsolete development outputs while preserving
everything needed through G/H. Cleanup is independent of product publication or
H's source retirement. Records are under
~/.cache/codex/acceptance/cleanup-20261022T014233Z.
The idle Provider incremental cache was removed first (32.643GiB filesystem free
space increase), then other idle debug compiler outputs; the release cache and
immutable accepted candidates remain. This causes a cold debug rebuild next time,
not a model request or a product/default change.

Cleanup completed2026-10-22T02:00:43Z: observed free space grew21.5 ->214.1GiB
(about192.6GiB). Provider target shrank94.2 ->1.3GiB.145 obsolete nested D3
compiler caches and three other obsolete debug caches were pruned.719 inactive
manifest-matched payload copies (98.2GiB allocated) were reduced to3 unique zstd
objects (about172MiB). One257,816,200-byte runtime was restored to its original
path and its hash/size/mode verified; that sample remains, with718 original
payload paths still archived. All3 objects independently decompress to their
original digests. Records: completion.json and restore-review.json in the cleanup
root. Original F22/G7 result-reference hashes and the existing F/prior-Provider/
shared-Linux process identities remain unchanged (preservation-review.json).
No Chat request, runtime-service stop, user-state deletion or activation occurred.
Unclassified and process-referenced environments remain for their later G/H review.

scripts/clean_development.py defaults to a dry run. It checks process executable,
cwd, argument/native-home references, open files and mappings; compiler cleanup
also takes Cargo's debug/build/artifact locks. It never removes fixture directories, journals,
auth/config, workspaces, source or result logs. Inactive generated fixtures are
identified by their private fixture key, exact generation marker and manifest.
Only manifest-matched runtime, codex-code-mode-host and helpers/2 executable
copies are archived, independently decompressed/hash-verified, recorded and then
removed. Active/unclassified environments remain. Hardlink/reflink sharing is
unavailable on this device; no symlink replacement or resolver edit is used.

Unique compressed payloads live in
~/.cache/codex/acceptance/fixture-payloads/objects. Historical original paths can
therefore lack archived executable copies; original verdicts/manifests remain
unchanged. Before executing/reinspecting an old full fixture inventory, restore
its required payloads with the exact original path and retained cleanup record:

```sh
python -B scripts/clean_development.py restore --apply \
  --record /absolute/path/to/fixtures-removed.jsonl \
  --path /absolute/original/TMPDIR/fixture/path/runtime
```

Restoration checks the original digest and does not start a service or inference.
An existing different file or active fixture refuses restoration. Do not rerun
accepted development merely because its executable copies were archived.
Keep current/rollback/accepted immutable candidate artifacts outside mutable Cargo
outputs. At each completed batch, reclaim archived inactive fixture payloads and
review a compiler cache above15GiB; retain anything needed by the next armed proof
before pruning. AGENTS records this maintenance rule. The G composition runner
now archives its own successful fixture payloads automatically unless explicitly
kept for debugging, retaining original results and all nonpayload files. Its
changed helper inputs do not rewrite the earlier84-step G proof. Cleanup/archive/
restore ran on real retained artifacts; CLI/import/syntax/whitespace checks pass.
The G automatic-retention hook was added without repeating the completed native
development/composition cases; its full integration remains part of the next
otherwise-needed G batch.

## Operating baseline

| Component | Current authority |
| --- | --- |
| Upstream | Qualified0.163.0, source83086188ae0614bd882818ff1aa23f3194962012; release/upstream.json |
| Installed Full | Actual activation-state selects local-relay-hosted-0-163-0-acf1bdb8f531-s86; upstream0.163.0; previous Full88 after accepted actual public86→88 update/use and supported88→86 rollback/use. Original86/85/settings are preserved in gdev.lvqnpcin/device-before.tar.gz. This existing public wrapper cycle does not activate the new G Provider |
| Earlier public Full proof | local-relay-hosted-0-163-0-54f926437ad5-s60; source54f926437ad5093bb11e8d53c3502b810e514071; published2026-10-17T12:42:39Z |
| Base used by Full60 | local-base-relay-hosted-0-163-0-b18cc2a2f5f4-s58, reused byte for byte; previous user Full57 retained |
| Earlier Full60 TUI proof | Source1c413cf, qualification39027733603; Full76 helper was a different signed inventory entry |
| Retained prior TUI candidate | Source310f0f7e5ce421e61e2c3158406f17e743584557, qualified39107948384; old binding invalidated by changed source recipe, artifact retained/not activated |
| Installed Chat adapter | Immutable c3642b19a87cdace158f; sourceBase9a5436a plus preserved user1m setting; JS/Bun/Python; app-local function returns |
| Rust Provider candidates | Latest G candidate88257d2a:156 Rust tests,39 affected helper tests, actual installed86 use,84 composition/113 remote-package steps,3 fresh-profile actual Android host tests and synthetic native boundary proof. Original restored G service uses134ea670 and retains its state/endpoint/tunnel. F candidate40/65c24d41 and earlier9fefba99 D2/D3 and2edc2215 C2 scopes are historical exact-input evidence. G exact-source freeze/CI records are in phase-g-post-device-20261022T081337Z; no public Rust activation |
| Experiment | experiment/web-termux at08e9856; user's uncommitted1m profile/test edits preserved, branch/worktree not closed |

Full60 acceptance39033120658, Core39033121990 and publication39033383447 passed.
Main pinb0702ba selects source54f; promotionb5e1d9f publishes Full60. The reusable
main caller retains its acceptedb18 input. The Base fingerprint is
43e0b888ec06af28e2e98d55cdd30adb9f01e5ef9d3e8f691cfab7c6e2a97a3c.
The pilot inventory was read against its actual runtime binding:526 immutable
files, zero mismatches. These observations do not activate the Rust candidate.
Source release/upstream.json now binds frontend15da596 via successful39129327503:
50 focused native tests,94 retained upstream-feature checks and lint passed.
The downloaded exact artifact/recipe was independently verified and an actual
Android no-Manager /model round trip passed. This does not activate a public
generation. Automatic source commits may invalidate obsolete bindings while
retaining published Base/Full and prior immutable artifacts.

The provisioned app remains26.1007.21434 with Chromium155.0.8059.40. The user
reestablished login after revocation/logout. Authenticated request construction
now passes against the expected private account with zero Chat submissions,
including after Electron creates auxiliary index-page targets. Read-only DOM
selection admits the sole main Chat/Work surface; multiple real main surfaces
still refuse. Its observed
app-initial-ed5807835d97.js/app-shared-2d77298f0100.js capabilities define this
bridge's qualified host boundary. App binaries, rootfs, login and account data are
not bundled into the repository or implied by a model listing.

## Documentation authority and review scope

The earlier repository-wide documentation audit covered 14 files: the seven maintained
Markdown documents (AGENTS, SPEC, CURRENT, root/TUI/Provider README and NOTICE),
the two frozen D3 Markdown fixtures, the native-summary prefix and four retained
license texts. Local links/heading anchors and fenced blocks were scanned across the full
inventory. Architecture/ownership, E–F–G–H order, current-versus-historical proof,
configuration/command descriptions and release/activation limits were reconciled.
The installed activation-state and release/upstream.json were read without
starting a runtime; original proof files retain their verdicts and exact inputs.

SPEC owns contract/target behavior; CURRENT owns current state, one sequence,
remaining acceptance and bounded evidence. README files summarize their component
and link here. NOTICE owns provenance; license/prompt/frozen-test content is
preserved. AGENTS' gate list now includes the already implemented provider scope
in scripts/check.sh. That audit and the2026-10-22 status cleanup change
documentation only: no implementation, tests, Chat submissions, live-service
mutations, default or activation changes. The status cleanup includes the passive
process/listener observations described above.
E owns design/functional decisions, F implementation/integration, G product/delivery
and H retirement. Earlier
operator/source labels remain compatible; source/full/synthetic/live/release
proofs are not interchangeable. Deferred Core display/prompt work remains low
priority. Earlier
documentation-agenda-audit.json and documentation-audit-migration-example.log
retain their historical scope, including the then-observed Full76; they do not
describe today's installed generation. Completed fixtures/live proofs are not a
new backlog. Historical evidence remains in Git/private acceptance logs.

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

## Proposed ChatGPT control through MCP

Status: Phase B's opt-in operator implementation and bounded synthetic Android
qualification are complete at their recorded exact-source scope. C has passed
actual initial Chat creation/attachment/receipt acceptance, one read-only command
in real ~/prj/ai and public CLI/canonical reports/final, through both background
conversation approval and the actual UI fallback. Both explicit Stops confirmed
in those successful trials. Their source fingerprints differ and remain separate.
Earlier annotation candidate eeaf9007 passes ten sequential actual TMPDIR
development cycles, tests and original MCP/CLI/history returns. Current2edc2215
also passes C2: five distinct sequential development user turns in one saved Chat,
source-authored handoff to a saved successor, receipt before actual continued
development and original sixth native CLI return on the same primary thread. The released-host
restart correction, direct AI-project development, non-user successor admission,
incremental handoff/continuation, active interruption/recovery and one fresh
matched comparison have bounded independent proof. The same primary Codex thread
persists while Chat contexts can be replaced.
D historically selected operator/MCP for fully bound work and a local-function fallback.
D2 qualifies all four Codex-entry combinations. The agreed bridge structure below
now separates entrance from reasoning context and reuses the Codex managed path.
Direct ChatGPT two-turn development and F's combined routing/general delegation
now have bounded integration acceptance. Actual, synthetic and recovered-result
evidence remain distinguished in the F review below. G delivery follows and remains
separate from operating activation. Bare configuration
and the installed adapter retain their existing defaults.

This proposal supersedes the earlier interpretation of dual entry as just a
ChatGPT-first bootstrap for the existing per-inference tool loop. The user wants
ChatGPT to operate local Codex through MCP: discover tools, inspect/manage work
and sessions, execute development, choose context/checkpoint/handoff operations,
publish public reports and finish the task. Codex is the local execution/recording
system; the Provider is its transport/control adapter, not a second planning agent.
The documentation review alone approved no implementation/default/activation change;
the subsequent user decision authorizes B. SPEC now includes the opt-in operator
contract. The model-adapter/local-function path remains the current default.

The operator control workflow supports two entrances; the proposed combined
Provider also retains Codex's model-adapter path as described below:

- Codex entrance: Provider applies selected model/effort, starts the Chat task and
  keeps the original native request connected to that task's output/completion.
- ChatGPT entrance: the user starts Chat directly; MCP opens/attaches local work.
  No recursive Chat initiation or second Codex model is required for primary work.
- Common path: ChatGPT controls local Codex through MCP, receives real execution
  results on original tool calls, emits reports and explicitly finishes. Native
  events/records remain available from either entrance; only a Codex-origin request
  has an original CLI response to settle. Another CLI may observe the same work.

### Origin and return routing without another Chat user turn

The user did not request an origin marker in the user prompt. Persist the binding
in the existing Provider journal: origin, a stable run/request identity, confirmed
execution home/account, native thread/turn associations, outstanding calls and the
local reply sink. The opt-in work record implements these routing references;
its exact execution owner and local return remain authoritative.
Keep the same primary thread and run identity across model-request rounds, Chat
conversation handoffs and restart; Chat renewal never forks the primary thread.
thread_id, response_id and JSON-RPC id alone are not the whole task identity.

If ChatGPT needs origin/control context, return a compact model-visible record on
an existing non-user return path: the original local-function return or MCP result,
preferably the already-needed attach/catalog response. This reuses the demonstrated
same-call return mechanism; B qualifies its new payload with a synthetic host.
Actual signed-app use remains a C gate.
Do not append a user-role environment block, resend previous answers, or post a
follow-up user message merely to supply routing information. If context is needed
before a first return exists, establish that bootstrap explicitly; do not invent a
server push into a nonexistent tool call. Avoid a separate model round just for
origin when an existing result can carry it.

Responses-compatible wire format is our local adapter interface, not proof that
arbitrary Responses metadata reaches ChatGPT. MCP content/structuredContent is
model-visible; _meta is UI-only. Keep private paths, credentials and the actual
reply channel local. Report/final calls resolve through the saved binding, not a
model-selected URL or guessed destination. Model-visible does not mean absent
from Chat history, free of context cost, or uncharged by Chat message limits.
Reference: https://developers.openai.com/plugins/reference#tool-results

### Responsibility and observable behavior

| Owner | Proposed responsibility | Remains outside it |
| --- | --- | --- |
| ChatGPT | Development decisions, discovery, session/context strategy, public reports and completion requests through MCP | Fabricating execution receipts or declaring an unconfirmed stop |
| Provider | Model/effort initiation for Codex entry, bound MCP/API controls, exact-call correlation, return routing, event projection and host lifetime | A competing planner, shell executor, canonical history store or silent account selection |
| Upstream Codex | Actual tools, configured approvals/reviewer, writer ownership, authoritative native records and execution/control primitives | Chat transport and UI-entry routing policy |
| Core | Verified runtime/environment/server binding and generation lifecycle | New ChatGPT control policy or Provider dependencies |
| Optional Manager/TUI | Profile/window selection and displaying/controlling the bound native work | A second work owner or duplicate transcript authority |

Inventory the full requested Codex control surface before freezing MCP tools.
Expose supported native operations, including necessary context/session controls;
existing native_catalog/native_tool alone do not constitute this capability. A
missing general native-tool/control API may require a Provider/TUI integration or
small upstream adapter; do not move orchestration into Termux Core to bypass it.
The installed0.163 experimental schema includes command/exec, thread/shellCommand,
thread/inject_items, thread/compact/start and turn/interrupt. Schema existence is
not runtime qualification: command/exec has no thread, thread/shellCommand is a
user command with different policy, and history injection alone does not update
a live TUI. Prefer reuse of actual native execution and existing Responses/event
projection over arbitrary rollout-file edits or an unverified second executor.
Reference: https://learn.chatgpt.com/docs/app-server

The context/handoff policy below distinguishes Chat renewal from native inference
compaction. A control command targeting an active turn can require a deferred or
settled transition, not a recursive wait while that turn awaits its MCP result.
Independent review and child agents need their own real contexts/transports;
do not replace auto_review
with self-review or silently weaken the configured approval mode. If a reviewer
still uses the Linux app, that capability retains the dependency even when the
primary ChatGPT-first path does not.

Execution/output events must stream to an attached CLI and durable native history.
Public report/final MCP calls enter that same output path; an optional host-text
observer can add token-level streaming. MCP alone does not copy all Chat UI prose.
An actual command receipt is recorded by native execution, not retroactively
invented by ChatGPT. Completion resolves once only after outstanding calls settle.
Neither a report stored on disk nor an HTTP200 proves the user saw live progress.

Do not adopt blind continuation after observer loss. Show actual native activity,
last-observed host state/freshness, unknown versus idle, and stop requested versus
confirmed. MCP-only observation cannot establish whether ChatGPT is thinking or
idle between calls, nor guarantee its Stop button stops local work. Explicit Stop
must fence new actions and confirm the affected native turn/jobs. Resume reads the
latest history and unresolved original calls; it must not replay uncertain effects.
One active controller owns each work binding even if both UIs observe it. Separate
work can run concurrently; a second entrance cannot silently become a second writer.

### Same-thread continuity and Chat context handoff

The primary Codex thread keeps the long-term canonical execution/history record.
Chat conversations are replaceable reasoning contexts; use each for substantial
MCP development, then prepare its successor in that same native thread. Independent
children/reviewers retain their own native contexts; they are not replacements for
the primary thread. Origin, work identity and original CLI reply binding survive.

ChatGPT supplies the next summary from the previous accepted summary, confirmed
native history since its source boundary and current actual work state. Include
current instructions/goals, decisions, changed files, tests, remaining issues and
references for fetching details through MCP. Local code validates/persists through
supported upstream recording, retaining full original history. The Provider journal
keeps transport/controller state and references, not a second transcript/summary DB.
Reconcile against original evidence so repeated summaries do not silently lose work.

Bind handoff to a definite native-history boundary and controller generation. Fence
new effects from the old Chat, settle or reconcile pending calls/approvals/jobs,
record the prepared handoff, then confirm the successor's attachment/receipt before
granting control. Attachment rereads later instructions/events and unresolved calls;
an acknowledged summary does not cover new events automatically. A failed or
uncertain transition preserves the last recoverable boundary without executing
effects twice. Handoff may span native turns, but does not complete the user's
overall task or discard its original return binding.

Native inference compaction and Chat renewal are separate operations. Keep native
compaction only where the chosen execution integration needs it; native token/hash
triggers must not dictate Chat renewal. No independently verified host-capacity
sensor exists. Initial/cold history import is projection, not automatic model
summarization; the old30k/32k or preserved1m settings are not measured Chat limits.
Use task milestones, observed context growth and headroom rather than a fixed MCP
call count; call output sizes differ. Do not renew on every native transport round.

The selected integration reuses native Responses tool dispatch for function/custom/
search execution, approvals, writer ownership, native events and durable recording.
A separate MCP operator Host retains work across transport rounds; it does not run
one Chat submission for each round. A separate upstream executor would duplicate
that engine and need new approval/event/history integration. The operator therefore
keeps the native path, with Chat handoff independent of native compaction. Native
summary requests use the same controller and are acknowledged against the next
canonical input; `new_context` is not a Chat-renewal mechanism.

`execution.mode=operator` is explicit and requires an explicit Core, execution
home and workspace. Its control return is MCP or the D2 app-injected local path;
ordinary direct ChatGPT entry uses MCP. These bindings require a service restart to change;
model_adapter/local_function remains the default. The operator consumes only the
existing dependency-free runtime-binding codec and authenticated Core server
preparation. No Core, Manager or TUI implementation change is required. The MCP
stdio bridge supplies the private local service credential; it is not model data.
Chat-first startup/control does not start a desktop or a second model. A Codex-
origin Chat start or independent native reviewer lazily acquires the provisioned
app; those capabilities still need it. No implicit rootfs download/login occurs.

The control surface covers sessions/create/resume/fork/start, native tool discovery,
execution and exact retries, paged native public history, live report/final commit,
status, question answers, stop, incremental Chat handoff, native summaries and
explicit recovery. Only original native question requests accept user answers;
this interface cannot grant approvals or replace auto_review with self-review.
Controller capabilities, original thread/turn and unresolved logical calls are
journaled; report/summary bodies remain in native history. Unknown dispatch never
replays. An uncertain successor stays fenced and permits explicit MCP takeover
only after old-host stop confirmation, current-history agreement and no unresolved
native effects. Provider restart rotates controllers and requires reconciliation.
A late old observer returns its original result without advancing a successor's read cursors;
native observer shutdown preserves an explicitly requested Stop state.

History uses upstream's paged items API, excluding private reasoning and unsettled
items. A narrow read-only projection of the exact owner-private native rollout
supplies public user/developer/system input omitted by that API. It validates
thread identity, excludes reasoning/internal metadata and has a separate source
cursor. Attachment receipts, handoff/recovery and later original tool returns
cover this context too. Upstream input queued during an active turn is not yet
committed history; the next native round delivers it. Candidate bounds are16MiB
per native frame,64MiB for a complete public-history or rollout read and4096
public-item pages; exceeding them refuses explicitly without truncation. These are implementation bounds, not Chat capacity measurements or
long-duration qualification. Initial real user-task Chat creation, attachment and
receipt acceptance now have bounded C proof. Actual non-user summary/successor
admission and continued development remain C gates; synthetic renewal is B proof.

C's first signed-app attempt made zero requests: the legacy MCP wire guard
rejected the system-only initial operator message before its before-request hook.
No user messages, model starts or native effects occurred. Native Stop was
confirmed; the stale app stream flag initially prevented host Stop confirmation.
The negative proof and an explicitly scoped zero-dispatch bridge release are
retained in phase-c-bootstrap-negative.json and phase-c-zero-dispatch-release.log.
This does not invalidate B's synthetic proof or establish a charged Chat debit.

The correction forwards the actual first Codex user request as that request,
without origin/return/history blocks. Only an explicit operator non-user start
may bootstrap a successor or request a native summary; it must not displace an
unsettled earlier stream. Known local rejection settles its stream state and
can confirm explicit Stop without inventing an abort terminal. Observed app errors
must reach the original CLI and native failure history while preserving recovery.
An explicit public final at a settled Chat boundary without a native finish is an
incomplete round, never successful completion. An app permission/authentication
wait is separately observable in CLI/history/status. The existing explicit reveal
command checks the bound account/host and navigates only to its owned Chat; it
does not submit inference or fabricate consent.

The real trial connector is `Termux Codex Provider`, bound to the independently
owned Rust tunnel. `Codex Native2` is a different older connection and must not be
used as this trial's name. The wrong-name attempt reached no local operator call.
Refreshing only the verified Rust connector now reads back all14 operator/reviewer
tools, with zero model starts. Local tunnel readiness/stdio identity was insufficient:
remote polling also needed the existing process-only CONNECT/DNS relay. Resolver
files and Tdev were unchanged; remote lookup and actual MCP probe were then observed.
Evidence: phase-c-connector-refresh.json and private transport preflight records.

The corrected connector trial reached a real signed-app `confirm_action` before
its first attachment; no native command or receipt acceptance occurred. Both
native and host Stop were explicitly confirmed, with no owned jobs remaining.
The first two completed actual trials each observed one primary app dispatch/one
explicit user object, zero host consent replies and zero local operator effects.
Negative evidence: phase-c-first-live-negative.json and
phase-c-host-consent-negative.json. A third actual trial received the app's tool
error that OpenAI safety checks blocked codex_attach, before any local operator
invocation. The app then reported failure; the corrected Provider surfaced the
incomplete round in actual native CLI/history, preserved the work and confirmed
both stops. No command, file edit, receipt acceptance or successor occurred.
The exact private negative proof is phase-c-safety-negative.json. Three primary
app dispatches/three explicit user objects and zero host replies were observed
across these trials; title/setup starts and charged allowance remain unmeasured.

Read-only app metadata confirms the correct private trial connection is ACTIVE,
all14 actions are linked and actions_changed_since_review=false. The returned
safety error supplies no specific reason; a missing action review is not an
established cause. Preserve the full tool surface and refusal evidence.

The user subsequently confirmed the real permission card was physically visible
in the Android browser viewer and delegated clicking Allow once. The fresh bounded
trial then passed actual attachment/receipt acceptance, Korean initial/intermediate
reports, exactly one read-only `git status --short` in real ~/prj/ai, its original
MCP return and `PERMISSION_CHECK_COMPLETE` as a canonical native final. CLI output
and canonical history are independently checked; the requested marker in echoed
user input is insufficient. It observed two app dispatches, one user object, one
real consent reply and zero native-result resubmissions. Both explicit Stops
confirmed, with no owned jobs. This establishes that the visible consent control
can resolve this permission wait; the earlier generic safety refusal remains a
separate negative result, not a permanent unsupported diagnosis. Private proof:
phase-c-fast-permission.json and phase-c-fast-permission-review.json.

The delegated approval policy is conversation scope first in both routes:
`approve --thread ID` uses the signed app's real action without a window;
`approve-ui --thread ID` opens the owned private viewer/Chat and actually clicks
its pending card's right-edge dropdown and offered conversation item. Only an
absent conversation option permits Allow once. A successful or uncertain approval
never permits another scope/path. Automatic visible fallback requires a known
capability rejection before approval intent. Account/connector/native-thread and
server-offered target fences apply; authentication, forms and passkeys remain
manual. This changes no native approval policy or account-wide permission.

Earlier background trials are retained at their actual scope. The first rejected
an unrecognized standard allow shape without sending a reply or making native
effects; both Stops confirmed (phase-c-background-shape-negative.json). The
corrected one-time action sent a real allow/remember=false reply with no viewer,
but the model ended without receipt acceptance or native execution. It correctly
remained incomplete and both Stops confirmed (phase-c-background-permission.json,
phase-c-background-approval-review.json). The old fixture's prohibition on repeating
any invocation could conflict with the required second codex_attach receipt
handshake; the model's exact reason was not established.
The corrected fixture explicitly permits that handshake while still forbidding
replay of native effects; this is no product protocol change. Historical
`approve-once` command names describe those earlier proofs only.

The user inspected and delegated the actual conversation menu item; its real
button click produced allow/remember=true. The integrated background conversation
route subsequently passed actual GPT-6 High attachment/receipt acceptance,
Korean initial/intermediate CLI and canonical reports, exactly one read-only
`git status --short` in real ~/prj/ai, original MCP return and canonical/CLI final.
Two app dispatches, one user object, one host approval reply and zero native-result
resubmissions were observed. Both Stops confirmed with no owned jobs. Exact
source/binary inputs and separate history/CLI review are retained in
phase-c-scoped-main.json and phase-c-scoped-main-review.json. This closes the
background receipt/command smoke gate, not long development or renewal.

Visible fallback qualification found that the signed app opens its Radix dropdown
on pointerdown and can queue its real approval handler beyond the old two-second
observer. Source now uses the actual card/menu attributes and offered label,
observes real wire dispatch asynchronously, and retains a queued-approval fence
through Stop/release. A late handler after Stop must not send; silence or a click
alone proves no approval or stop. Retained negative scopes:
phase-c-scoped-ui-pointer-negative.json and phase-c-scoped-ui-ack-negative.json.
The pre-fix host Stop acknowledgment in the queued-handler negative proof does
not establish that no late handler ran; later saved metadata showed its approval.
A later interrupted run retains unknown delivery/stop rather than a success;
saved app readback found one confirmation and no reply, and the native rollout
had no command/final (phase-c-scoped-ui-interrupted.json). The next run reached a
cold-render timeout before approval intent; both Stops confirmed and no host reply
was sent (phase-c-scoped-ui-card-negative.json). Read-only inspection then found
the exact card in the same bridge window/same client, with a distinct UI scope:
this was delayed rendering, not evidence for a second-window execution path.
The source now polls bounded pre-intent readiness separately from queued-handler
and actual wire observation. Main approval has no UI readiness wait; its existing
HTTP/client error bounds remain unchanged. Only explicit approve-ui gets the
longer UI bound.
The final immutable candidate subsequently passed the complete actual UI path:
conversation Allow selection, receipt acceptance, the same bounded read-only
~/prj/ai command, original MCP return, initial/intermediate reports and final in
both CLI and canonical history, on the original native thread. It observed two
app dispatches/one user object/one host approval reply/zero result resubmissions;
saved app readback confirms exactly one allow/remember=true reply and no once
approval. Both explicit Stops confirmed with no jobs. Separate
source, canonical-history and CLI review: phase-c-scoped-render-inputs.json,
phase-c-scoped-ui.json and phase-c-scoped-ui-review.json. This qualifies delegated
approval continuity, not actual long development, successor Chat renewal or
charged allowance.
Optional operator-owned lazy hosts now publish the private viewer capability only
after host startup; its empty-profile Android proof spends zero Chat.
C still must qualify non-user renewal and continued AI development. Default
selection, publication and operating activation remain separate decisions.

### Timing, fallback and proposed revised order

Phase A has frozen a useful, recoverable Rust baseline. Qualify this alternative
before expanding orchestration that it may replace. Finishing every existing
optimization and publishing a final legacy-shaped Provider first risks duplicate
development; replacing the working path immediately
risks losing its broader native/AI-development proof. A full public release is not
a prerequisite for the new bounded experiment. Base/Full automation and unrelated
Core/Manager fixes remain independent and may ship with their own acceptance.

The single proposed implementation sequence and decision gates are in
[Product completion scope and work order](#product-completion-scope-and-work-order).
They do not change current defaults or authorize the new implementation by themselves.

A baseline checkpoint is not a demand to polish every old feature. Fix shared
correctness defects first. Defer new Provider-side planning/context heuristics,
transport-specific micro-optimization and broad cosmetic work until the alternative
comparison, unless required for the baseline. Do not defer required failure/stop
visibility, exact-call correctness or account/data preservation as cosmetics.

Retain the existing Rust model-adapter/local-function path as an explicit fallback
candidate while qualifying the new operator path. Existing MCP return is a
transport on that same adapter, not the new full-control workflow. Do not overload
transport.return_path to pretend execution policy and result transport are the
same setting; final naming/schema waits for design. Keep only deliberately
supported combinations, not a general cross-product routing framework.
Fallback retention is not a permanent promise to maintain two orchestration
implementations. Keep it only if it offers distinct useful coverage, such as work
without an available Chat-side MCP connector, at acceptable shared-test cost.
Remove obsolete path-specific policy after replacement proof and an explicit
product decision; keep common codecs/execution/history instead of duplicating them.

No automatic path switch after uncertain dispatch, active tools or an unresolved
writer. Choose before starting; change paths only with settled/reconciled state
and an explicit handoff. Rollback preserves checkpoints/history; it never replays
a task. Installed JS/Bun/Python remains the protected current installation and
migration reference, not a newly supported fallback implementation. Never silently
select it because the Rust/new MCP route fails.

### Required proof before operating default replacement

D selects the delivery target below; G must close its remaining applicable matrix
gates before operating replacement. Completed B/C/C2 proof retains its exact scope.

- Zero-Chat fixtures: origin/context travels through non-user returns; no extra
  user-role environment/previous-answer block. Origin and reply binding survive
  reconnect/handoff without exposing private return destinations.
- Actual Android: discover tools, run multiple command/patch/test rounds, replace
  Chat with a source-bound incremental handoff in the same primary Codex thread,
  retain original native history and stream public
  initial/intermediate/final reports to the original CLI. A no-observer Chat entry
  still records execution without creating a dummy CLI-origin request.
- Interrupt and disconnect during model wait, execution, approval, checkpoint and
  handoff. Verify observed stale/unknown state, the exact stopped work, remaining
  owned jobs, result reconciliation and no new effects after confirmed Stop.
- Two entry clients observe/control one task without duplicate writers; independent
  tasks/reviewers remain isolated. Prove restart, unchanged primary thread/return
  binding, summary receipt plus post-boundary events, and safe failed/double/stale
  controller attachment. Synthetic Chat replacement is B evidence; actual Chat
  creation/continuation is C evidence. Qualify native compaction/cross-provider
  return only where the selected path still needs it; R11 proves neither handoff.
- A matched real development task, including direct ~/prj/ai under its AGENTS and
  preserved user changes, must show analysis, actual failure/correction, tests and
  same-task resume. Supervisor evaluates the real diff/tests and physical CLI
  transcript, not just Provider delivery logs. Existing AI proof is retained as
  baseline evidence, not reassigned to the new workflow.
  Compare repeated fixtures with equivalent inputs for latency/call overhead;
  use sequential legitimate AI changes for real development, not simultaneous
  writers or repeating an already-completed patch. Do not infer efficiency from
  the unmatched earlier AI-development and two-command MCP trials.
- Reuse prior acceptance only at its exact scope. Earlier non-user tool-return
  proof does not establish the broader controls or charged Chat message savings.
  B's new synthetic native/CLI proof is distinct from C's actual Chat proof. No Pro/GPT-5.6 trial or silent retry.

Phase B source, full local gates and bounded synthetic Android qualification are
accepted in the proof index below, including hosted exact-source packaging and
downloaded Android candidate readback/start/stop. This is not signed delivery.
That B proof alone establishes no real Chat behavior, new default or runtime
activation; the separately scoped C trials above retain their own evidence.

## Product completion scope and work order

This is the single work sequence. A's bounded baseline and B's opt-in source/
synthetic Android proof are accepted. C's actual consent/receipt/read-only command
smoke, ten-cycle TMPDIR development and one actual same-thread non-user Chat
handoff/continuation and active interruption/recovery proof are complete at their
retained candidate inputs. Direct AI-project development and a fresh equivalent-
input comparison also pass independent bounded workflow review; the comparison's
code-quality finding below remains visible. C's declared development/handoff/
active-command recovery/comparison scope is complete. The released-host restart
correction passes Provider/full/Android acceptance. New real Chat trials require
a declared scope/submission budget. D's historical route comparison is complete.
The user now prioritizes structure completion, documentation/evidence reconciliation
and an explicit development sequence before more live trials. E owns design
and feature decisions, including Linux app/host management research, and is now
complete at that design/common-idempotency-source boundary. Previous E
implementation moves to F, product qualification/delivery to G and retirement to H.
This supersedes the briefly proposed D4 label; historical A–D/D2/D3 trial labels,
source fingerprints and evidence files are unchanged. F now has its own bounded
integration acceptance; historical A–D proof is not substituted for that verdict.
G product qualification/delivery is active and unfinished, with its
applicable proof still required. Independent Core/Manager/Base/Full work remains independent.

| Phase | Work | Gate |
| --- | --- | --- |
| A | Stabilize shared Rust identity/auth, exact-call returns, native execution/history, visible failure/stop/recovery, config and candidate packaging | Bounded recoverable checkpoint accepted at acf1bdb, with exact source/artifact/proof and disposable standalone snapshot install/restart/rollback. Remaining product/lifecycle gates stay in G; no public activation |
| B | Choose native integration, expose Codex control to ChatGPT and connect both entrances | Bounded source/synthetic acceptance complete. Actual Android synthetic host: non-user bootstrap, full native tools, live public reports, repeated MCP work and same-thread incremental Chat handoff, failure/restart/stop reconciliation and original final routing |
| C | Real development, conversation renewal and comparison | Earlier bounded scope complete: initial attachment/receipt/read-only smoke, ten-cycle TMPDIR development, one actual non-user successor/receipt/same-thread continuation, active pending-call interruption/restart/recovery, direct ~/prj/ai development and fresh identical-input comparison. C2 also complete: five consecutive development turns in one saved actual ChatGPT conversation, then source-authored summary/history handoff, verified receipt and actual successor development on the same native Codex thread and original sixth CLI return. Detailed compaction policy/capacity-trigger review remains deferred. No general speed/cost winner, default change or long-duration release qualification |
| D | Select default and useful fallback | Complete at historical scope: operator/MCP plus explicit model_adapter/local_function. D2/D3 and the next entrypoint qualification refine the target direction below. Existing bare-config/installed defaults stay unchanged until G; no uncertain mid-task fallback |
| D2 | Implement and compare all four control/return combinations from Codex | Complete on9fefba99: four matched six-task development/handoff runs; all common/regression/native/CLI/saved-Chat/Stop audits pass. Timing/request/byte/resource evidence and all failed/preliminary costs preserved. ChatGPT-first and actual billing remain outside this proof |
| D3 | Unattended MCP development and result-loss qualification | Bounded acceptance complete: retained normal B/C six turns plus fresh A-only d3.uplix9nz four tasks completed by one controller, no supervisor resume/mid-run repair, sealed inputs unchanged. Both automatic recoveries and final independent native/diff/test/CLI/saved-Chat audit pass.4 app starts,4 saved Chats,14 original calls/results,0 unreturned/replayed effects. Prior connection and same-session notice verified. Arbitrary failure/long-run gates remain outside this proof |
| E | Resolve design, feature scope and acceptance; implement the settled idempotency contract | Complete: three flows, identity/ownership, reading/input/control, serial/parallel agents, app/host management, per-tool idempotency targets and F1–F9 input/evidence contracts; common source implemented, all-target static gates and source-only22e2d492 release build pass. No runtime/live acceptance inferred |
| F | Implement the E design, qualify and apply selected paths | Complete at the declared bounded F1–F9 integration scope. Common routes, reading/input/return, serial/parallel agents, host management and common idempotency are implemented; Rust target hints are applied and checked. The actual/synthetic/recovered evidence and limitations below are part of this verdict; original failed runs remain failed |
| G (active) | Finish product qualification, package and publish; qualify activation separately | Current88257d2a package/composition, exact-source CI, signed public Base89/Full90/Provider2, complete readback, actual Android public/stable consumer and non-forced promotions pass. Four installed usage combinations, same-Chat cold follow-up, physical reboot, actual86→88 update/use/rollback/use and one actual original-TUI GPT-6 dispatch after cold resume are independently accepted. The device development tasks returned14/14 calls with18/23 passing tests. Remaining gates are installed TUI downshift/re-entry, broader host-loss/approval/context and longer-duration retained-route proof; delivery does not establish them. Current physical installation remains86/previous88. Linux app removal is not a prerequisite |
| H | Close superseded experiment and code | Preserve unique work/user1m edits/licenses/proof and admitted rollback generations; remove replaced implementation policy only after accepted replacement |

Historical F call hold: released by the user at2026-10-21T13:15:32.187345+00:00 after the limit banner disappeared.
The earlier one-hour timer has a cancel marker to avoid a duplicate continuation.
That release permitted continued F work; it is not a new launch instruction.
Do not replay completed development or uncertain effects.

**Current handoff: E complete; F bounded integration complete; G active.**
Physical boot09ae7c1e-44ff-4f03-9398-e71e7b3d73ab and the preserved primary
thread01a15330-0d46-7b51-a460-5e0495030cf2 are confirmed. Actual signed public
wrapper update86->88, development on88, supported rollback88->86 and development
on86 pass independent review at TMPDIR/gdev.lvqnpcin/device-cycle-independent-review.json.
Two observed Chat inputs produced14 original calls/results,0 unreturned; original
CLI/native/saved-Chat finals and unchanged prior tests pass. Recovery was required
between tasks; billing remains unverified. Current installed generation is86,
previous88, both upstream0.163.0. Original Bridge service/state/tunnel is restored,
with actual remote16-tool/read verification and0 new model/native effects. MCP
display name is Termux Bridge; the16 definitions/annotations are unchanged.
Latest Provider88257d2a fixes managed routing, foreign execution-home filtering
and completed-lane shutdown. Its156 Rust tests, fmt/Clippy/release and39 affected
helper tests pass. New model-free qualification is accepted at
~/.cache/codex/acceptance/phase-g-post-device-20261022T081337Z/review.json:
84 composition steps,113 signed/remote-package steps,3 actual fresh-profile host
tests and synthetic native approval/question/handoff/Stop/recovery boundaries.
These have0 actual Chat requests. Exact-source public delivery now passes, including
signed Provider2, full public readback, actual18-command consumer and non-forced
promotion; see public-delivery-review.json in that evidence root. Remaining
runtime/real-Chat TUI/lifecycle gates are unfinished. Preserve original failed results and accepted
development without replay. Reboot recovery used wrlab; later primary turns
resumed under janmori101 then wrlab for quota restoration. Neither profile's settings were
changed by this audit. The saved ready-to-reboot marker is consumed, not renewed.
Quota-driven profile switches and supervisor-session resumption are excluded
from product failure/recovery costs, as clarified by the user. Recorded technical
lane-release and connector-cache fixes remain independently scoped findings.

The actual original-TUI Chat-after-cold-resume qualification used
TMPDIR/gt.z1my9k_g, actual public Core90/0502f7dd and signed Provider2/88257d2a
in an owned signed fixture. It is not activation of Full90 on the operating
device. Independent reviews preserve:
- same native thread01a162f6-4dbe-7ed1-aa4c-a37c904d1145 across native->Chat with0
  inference, one actual MCP command/result and a genuine source final, then one
  actual source summary committed as readable native replacement history;
- both actual Chat requests/finals in saved Chat6ad9da48-0808-83ec-9309-45c1d2ce5946,
  project Termux Provider: G, title Marker retention verification, last output
  UTC09:42; the user-requested Linux reveal and Android notification are confirmed;
- readable summary consumption and2 local native responses across kernel-confirmed
  exit of both owned backends and same-thread cold resume; marker retained in both
  native request inputs without ocx1:. These later native steps are not Chat output.
- R11 then sent1 real ChatGPT app dispatch from the resumed original TUI, performed
  the exact new native command once and completed the same saved native thread.
  Saved app conversation6ada19d2-9060-83ec-847f-bf6c0488874b and native rollout
  markers confirm completion. This qualifies that single cold-resume route, not
  TUI downshift/re-entry or the remaining G matrix.

Six original batch failures remain failed in the root and continuation-1..5.
Four are qualification defects: duplicate event time field, aborting a transient
re-entry owner gap, missing observer startup wait, and reusing an observer ID with
changed deadline. The two post-cold failures are the same product defect: the
managed ModelAdapter path retained a dead native owner; status fenced it before
open/submission. Its first symptom was masked as app_lane_open_uncertain, and the
next was operator_controller_fenced. Both submitted0 actual Chat requests. The
driver also waited900s despite the first new failed native turn. The root's initial0700 copy-mode
preparation failure is separately retained and fixed. None of these results proves
whole G acceptance; no uncertain native effect or completed model input is replayed.
R11's original `result.json` says failure/zero requests, but this is a verifier
false negative: candidate port34061 was omitted from `info.json`, so status and
cleanup hit original port45475 and received401. The app stream, saved conversation
and native rollout independently prove the one completed dispatch. Keep that
original result unchanged. The candidate Provider/tunnel were stopped after task
completion; the original Provider route is restored and its16-tool/read readiness
passed. The cause is fixed in the unlaunched `continuation-12-harness` copy; its
local checks cover candidate-port binding, independent local/managed dispatch
accounting, and a nonce-bound observer keeper holding PTY/input descriptors until
an idle, completed, no-pending receipt. Nine tests and static integration checks
pass; this used0 new Chat requests and did not launch that original follow-up
copy. A later separate downshift.py in that directory did launch a TUI and failed
before any model-selection or task input; its original result is preserved below.

Continuation-4 reconciles the exact retained Opening declaration with0 inference,
then verifies actual TUI Chat-profile readiness with all6 prior turns preserved.
Common wait13 and new-turn terminal-state5 local regression cases pass.
Continuation-5 freezes its unique observer ID/deadline before initialization,
prepares the actual observer and TUI in one controller and allows no Chat input
before ready-before-Chat.json plus explicit go.json. Preparation failures do not
queue another supervisor turn. That prepared task was executed once in R11: the
actual app dispatch count is1, within expected1/max2 and cohort max4 (2 previous
inputs); no Pro/GPT-5.6 was used. No request was retransmitted. Current
uncommitted Provider changes reconcile a new canonical CLI turn with the scoped
replacement native owner, retain budgets/receipts and fence old controllers. They
also guard stale completion/host observations and atomic managed admission/Stop.
The full affected Provider gate passes161 Rust,159 JavaScript and97 Python tests,
fmt/Clippy/release under phase-g-owner-round-20261022T104507Z. Debug/test symbols
and incremental output are disabled; reusable cache remains2.3GiB. Its actual
Android managed cold/Stop regression passes4 distinct TUI turns with66 concurrent
status observations in15.724s, an exact kernel-confirmed backend exit, controller
rotation, retained original history and a new turn after confirmed Stop. One
read-only delivery-unknown observation belongs to the declared cold-close window;
no mutation was retried. The broad actual Android Operator regression also passes
commands/patch/failure correction, native reviewer allow/deny, handoff, restart,
queued/concurrent Stop, questions and explicit unknown-result takeover. All use
synthetic app responses and0 actual Chat requests. Source candidate13a0956d adds
bounded local reobservation of superseded status reads; post-fix Rust/Clippy/release
pass. Preserve failed preparation/observer results in that root; none establishes
whole-G acceptance or authorizes replay. Continuation-6 prepares the one remaining
distinct live boundary on the same original thread, expected1/max2 Chat starts
(cohort2 already used; ceiling4). Saved host/service owners had disappeared while
boot ID stayed unchanged; restore records and fresh exact owners under that root
replace stale PID assumptions. The same Linux user profile and prior state are
preserved. Preparation verifies the actual remote16-tool catalog/read probe and
same original TUI thread with0 model requests. Continuation-6 launch-intent/go at
2026-10-22T11:10:55.499905+00:00 authorizes the distinct command G_TUI_OWNER_ROUND_NATIVE_OK
and final G_TUI_OWNER_ROUND_COMPLETE. The start signal was received, but the controller, Provider, relay and proxy
subsequently disappeared before the original TUI finished typing/Enter. The original
native rollout has no new turn after10:29; the managed lane stays idle with an empty
request and no pending calls. Actual new Chat requests/effects are0. There is no
automatic result or completion notice. reconciliation-after-disappearance.json
records this separate launch/lifetime failure; the termination cause is unconfirmed.
Do not report launch-intent/go as actual inference acceptance. Qualify worker
lifetime independently, then observe native-turn creation and app request acceptance
before reporting another live start. The earlier6 failed batches remain preserved.
The repeated whole-Termux exits now have confirmed Android termination evidence:
same-UID ActivityManager.getHistoricalProcessExitReasons reports seven Termux
exits on2026-10-22 at10:42:13,11:03:30,11:11:00,11:31:09,11:45:43,11:53:13
and12:05:09 UTC, all reason9 with description
`[EXCESSIVE BINDER OBJECTS] Too many Binders sent to SYSTEM`. Matching API-app
exits share the reason and occur within milliseconds. This supersedes the earlier
unknown-kill/OOM hypotheses; system-wide MemAvailable does not identify this
failure. Evidence and read-only query source/DEX are preserved under
~/.cache/codex/acceptance/g-interruption-20261022T114729Z/exit-reasons-review.json
and exit-query-source. The ordinary shell exit-info command is unavailable and
dumpsys is permission-denied; the successful query uses the existing app UID and
public ActivityManager API without privileged changes. Two isolated query setup
failures are separately recorded, including one app_process SIGABRT for writable
DEX; they are not Termux crashes or product/model requests.

Continuation-7's initial idle controller had not received begin-pipeline/launch/go
and submitted0 native/Chat inputs. Its durable recorder retained69 samples through11:53:10 UTC, phase
idle_without_linux_provider_build_or_model, before Android killed the app three
seconds later. No controller close/fault event exists. During those samples,
owned processes ranged35..45 and summed RSS about1.35..2.08GiB; these are shared
resident pages, not unique/private heap or the Binder object count. Later user
reports and app-identity changes confirm another exit even after launching via
codex termux instead of ai; the wrapper is not established as the sole cause.
Logcat shows repeated Termux showToast calls, including28 caller lines within a
roughly3-second retained interval. Local Termux source creates a fresh Toast for
every background-terminal title change; two Codex TUIs are present. This is a
concrete candidate Binder-allocation path, not proof of the exact leak. The user
did not see visible toast notifications. Session-change toast suppression alone
does not cover the separate onTitleChanged path. Existing terminals remain
preserved. Address the excessive-Binder path before another actual Chat batch;
do not blindly restore desktops or duplicate Providers. Qualification diagnostics
now supports one filtered rotating logcat child (3 files,256KiB each) alongside
the bounded5s resource sampler. A Termux-owned recorder cannot survive whole-app
termination. Preserve the original failed batches and accepted work without
retransmission; report actual native/Chat acceptance only from positive receipts.
The user authorized fixing the cause. The exact upstream0.163.0 title source
confirms a100ms spinner cadence. In the separate clean humtr/termux repository,
private branch fix/bounded-terminal-title-hints now contains candidate
3e0168d62884a86a0becf1655644dcdcc5ee1163. Background title changes use one
activity-owned, noninteractive view instead of system Toast IPC, coalescing the
latest value to at most4 renders/s, expiring after3500ms and removing callbacks/
view on activity stop/destroy. The existing session-change-toast preference also
covers this title hint; normal selection/exit Toasts and workload notifications
retain their paths. Native-source review found no separate high-rate service
title-to-notification handler. This removes the demonstrated allocation path;
the precise OEM retention mechanism and physical no-recurrence remain unproven.
Private Android build39778758013 passes all12 tests (6 existing identity,6 new
hint burst/coalescing/expiry/cleanup/focus cases), no skips, and actual APK/package/
certificate verification. Downloaded payloads were independently reviewed:
com.termux, versionCode118, version0.118.0+3e0168d.origin, certificate equal to the
installed accepted APK; candidate SHA256
faa1a8bfe3e1f73325590d9d2bff20914fa381045416bb16bf4891c1bf60644f.
APK and rollback are in /sdcard/Download/termux-binder-fix. Private source,
original APK, review, consistent native DB backups, complete original rollout
prefix, source/user-change snapshot and qualified Provider13a0956d copy are under
~/.local/state/codex/termux-binder-fix-20261022. prepare-and-open.sh refreshes the
snapshot and requests Android's APK screen; resume.sh verifies the installed APK
and unchanged original native prefix, then resumes the same primary thread under
the current janmori101 profile. It does not require a new boot or reset app data.
No process has been stopped and installation is not yet observed. The attempted
10-frame original-APK title control was rejected by the existing overlay/
background-start guard before service/terminal creation or output; preserve its
receipt in TMPDIR/g-title-control.7p62gi_5 and never count it as an executed test.
The user subsequently chose to turn off the Codex terminal-title spinner first.
APK installation is deferred, not a prerequisite for continuing G with this
workaround. At13:14 UTC, the current janmori101 config no longer contains activity/
spinner in terminal_title; original Termux/API process identities remain live,
and the latest Android kill is still12:05:09 UTC. This is a stable observed period,
not proof of every live TUI's effective setting or permanent no-recurrence. The
controlled fixture additionally uses tui.animations=false without modifying native
or user config. Preserve the candidate/rollback/snapshots. The APK-specific
resume.sh above deliberately requires the candidate hash and is not the normal
spinner-off continuation path; do not require or launch it for this choice.
If the user later chooses APK installation, separately perform model-free physical
title burst, visibility/focus/notification and fresh exit-history qualification.
G resumed on the same primary thread. Continuation-7's later preparation stopped
at a read-only old-PID check before any Linux/Provider/native/Chat start; its
preparation-reconciliation.json preserves0 effects. Android had reused old PIDs
outside current control. The replacement owner guard retains an inaccessible
controllable PID as uncertain; a signal0 permission/existence check sends no signal.
Seven focused guard cases pass. Continuation-8 prepared the same profile/state,
verified the actual remote16-tool/read surface and preserved its same-primary
callback binding. Its distinct Android TUI input created native turn
01a163c7-fc2a-7073-abf9-51537894b0e3, which failed before any Chat submission:
the Provider had restarted, so the prior MCP token was absent from its live
Broker map and `mcp_turn_unavailable` terminated the turn. The authoritative work
record remains `reconciliation_required`, with0 pending calls,0 human requests,
0 app dispatches and an idle managed lane; the final request snapshot confirms0
Chat inputs. The follow-up `codex_stop` cleanup was rejected before action because
the harness included an unsupported `controller` field. Both original outcomes
remain preserved; neither failed input is replayed.

The Provider fix reads the same-owner private journal record only to retire that
old token, preserves its receipts and pending-call guard, and never inserts it
into the restarted Broker's live capabilities. New unknown/corrupt records still
fail closed. Three MCP restart regressions pass. The full Provider gate passes
Rust, JavaScript, Python, formatting, Clippy and release build; the reusable Cargo
cache remains2.3GiB. A separate real Android TUI/synthetic-app test also passes
the combined native-backend and Provider restart boundary: four distinct native
turns, four synthetic app submissions, both owned process exits kernel-confirmed,
original history retained, controller rotated and63 status observations in
16.030s. One `native_delivery_unknown` occurred during the explicitly observed
cold-owner transition; it was read-only and no mutation was resent. Actual Chat
requests were0. Its evidence and exact input hashes are under
~/.cache/codex/acceptance/phase-g-provider-restart-20261022T133848Z.

Continuation-9's final callback is preserved at
`/data/data/com.termux/files/usr/tmp/gt.z1my9k_g/continuation-9/result.json`.
It failed before submitting any Chat request: the preparation guard hard-coded
seven native turns, while the same thread now had eight because Continuation-8
had appended its already-preserved failed turn `01a163c7-fc2a-7073-abf9-51537894b0e3`.
R9 has no `native-turn-started` or `actual-request-accepted` receipt; both final
and cleanup snapshots contain0 submissions/requests. Its actual remote16-tool
list and read-only readiness had passed. The failed task was not replayed.

The R9 finalizer also iterated every work record in the shared fixture state,
including an older `reconciliation_required` work that R9 had not created. Its
unnecessary `codex_stop` confirmed native Stop but not host Stop, leaving that old
record fenced and preserved. Before that call the authoritative state showed an
idle/not-loaded native thread,0 pending calls,0 app dispatches,0 user/tool/public
objects and no conversation ID. Provider source now treats only that complete
pristine managed-lane receipt as proof that no Chat host submission exists and
does not request host Stop; an idle label alone or any missing/nonzero field stays
conservative. The regression is
`operator::local_tests::settled_managed_lane_does_not_request_an_unneeded_host_stop`.
SPEC records the boundary. Never repeat R9's Stop or mutate its journal to claim
settlement.

The Provider gate passes from the isolated low-footprint target
`~/.cache/codex/provider-target-g-stopfix` (test debug info0, incremental0,
2 Cargo jobs): 98 library tests, all applicable integration suites, Clippy,
release build, Bun, Python and CLI/diff checks. One environment-dependent library
test and explicitly armed Android/live tests remain ignored as declared. The first
attempt on the shared target hit `ETXTBSY` executing a libc build script; no owner
was running afterward. The isolated target completed without that error. Release
candidate SHA256 is
`8467283ebad4cbf16804bbad846a206534840d4bfb47f4dd7885e38c2bade4b1`.

R9 cleanup requested graceful SIGTERM of its exact recorded Provider owner; the
15-second pidfd wait returned `provider_stop_unconfirmed`. The same recorded
process identity was subsequently absent, but the missed pidfd acknowledgment is
retained and is not relabeled confirmed. No force signal was sent. R9's candidate
route was then disconnected; the original Provider was restored under a new
verified owner and the original Termux Bridge tunnel/profile was reconnected.
Actual remote16-tool list plus read-only MCP readiness passed again, with0 Chat
requests. The same Linux desktop/app profile remains running. R9's connect proxy,
relay, state and failure evidence remain preserved.

Continuation-10 pinned the exact eight-turn idle history, bound its completion
notice to the active primary thread, and passed offline checks without creating
Provider state or a launch intent. Its first controller start stopped before
Provider/tunnel changes and before Chat input because `pipeline.py` tried to
create `desktop-owner-private.json`, already copied from the preserved R9 owner
snapshot. The result records `FileExistsError`,0 Chat requests and no setup/go
receipt; the same-primary queue accepted one failure notice, with receiver
verification pending. Preserve R10 as-is. The new execution must use a distinct
continuation root, write a separate observed-desktop-owner receipt, use a fresh
Provider state, and keep the cleanup guard that preserves the TUI/input observer
on any unaccepted Chat request. Reuse the verified original Provider/tunnel,
proxy/relay and Linux app owner identities only after rechecking each one.
The cohort ceiling remains two prior actual Chat inputs; expected1/max2 more;
GPT-6 only. Do not infer whole-G acceptance from R9 setup, R10 static checks,
Provider source tests or APK build.
Post-delivery source2cff2ef CI39754274901/39754274946 passes; canonical same-primary
result reception is recorded under TMPDIR/gci2.cumae1ub. No publication was repeated.

The following F service/process descriptions are historical pre-reboot evidence;
they are not current liveness claims. The earlier combined bundle passes
managed MCP/local, direct development and actual active server Stop. Recovery-6
locally verifies general delegation (9 tests) and same-thread follow-up (18),
actual diffs, canonical completion and original CLI finals. The original follow-up saved result now reconciles complete with37 messages and
released lane, without a new model request; the original failed batch is preserved.
Candidate40 (65c24d41) is active only on the owned F service at
/data/data/com.termux/files/usr/tmp/f-live40.e5FJz18q, preserving the same state.
Its seven source gates pass in185.631s (90 library tests pass; explicit ignored
live/native gates are not inferred passed). The separately armed native/host
proofs below close their required F scope. Candidate37's
actual Android readback of all four original finals remains exact-input evidence
with0 Chat requests/new native turns. Shared Linux was not
restarted and installed defaults did not change. Candidate30/35 proofs and failed
F9 results remain preserved. Existing pacing
proof remains bounded as below. The first agent batch's unsupported codex_answer
argument was fixed. Its recovery-1 then executed six roles:66 original native
calls/results,0 unreturned, actual scoped writer Stop/sibling continuation,
merge conflict, independent failing review and corrected integration. Independent
final review passes30 tests with original/regression files unchanged. The
corrector's final transport failed with nonmonotonic_public_message; that original
turn stays failed. Snapshot file writes were incorrectly reported as progress;
they are not evidence of ChatGPT output. The source server is now confirmed
COMPLETE and its stale local active flag is reconciled with0 extra Stop posts.
Root/corrector native Stop is confirmed; completed work was preserved.
The public interleaving, stale Stop flag, completed-host Stop reuse and bounded
observation paths are fixed. Exact original9-event replay, seven source gates,
actual Android synthetic18 and49 current helper/CI checks pass; no new model
request was made during those repairs. Recovery-2 has now ended after1369.015s
with3 app dispatches,2030 local control observations and0 intervening supervisory
model turns. Its independent review confirms84 total native calls/results,
0 unreturned, four completed turns on the original integrator thread,30 tests,
CLI19/API12 checks and saved/native/original-CLI finals. Its automatic result
remains failed: title preservation wrongly treated65 message update_time changes
as content loss; four old notLoaded native threads could not confirm tree Stop.
The source also omitted controller/text on evaluation and mistook a missing
codex_finish history item for a missing managed final. Original failures remain
preserved. Candidate30 (77df3d81) passes all seven source gates; unloaded-thread
Stop, title comparison and protocol guidance are repaired. Actual Android synthetic19 passes, and a distinct post-fix original-tree check
confirms all8 native Stops/no jobs with0 model requests. The original false Stop
result is preserved. Recovery-3 completed evaluated correction and real dependent release in617.350s
with2 app dispatches/590 local observations. Its original automatic result is
failed because all three CLI observers used the wrong execution home and stayed
at login. Independent same-backend readback now verifies all three original
native finals with0 new model requests; receipt/dependency/three-role Stop pass.
The fixture-home, observer preflight and rich-text comparison corrections pass56
helper tests. F3 additionally passes real duplicate-title and four-branch CLI/MCP
read comparisons with0 model requests; original titles/content are restored.
The first model-free F7 batch stopped on a harness error-code mismatch. Independent
audit confirms four exact observers shared one native effect/result and changed
payload was rejected as mcp_call_changed; native idle/no jobs is confirmed. Its
original false result is preserved. Continuation-1 verifies new-ID identical input
as two original effects, then stops before the loss case is admitted: its raw HTTP
helper half-closed the connection too early. Actual candidate read-only preflight
reproduces empty response with half-close and HTTP200 without it, with work records
unchanged. Continuation-2 passes all five remaining scoped checks in14.366s:
lost tool/final original-result recovery, concurrent Stop/sibling, pending index
and uncertain creation recovery, and old controller fencing. Independent audit
verifies3 original calls/results,0 unreturned and idle/no jobs on all three works.
Both completed finals are visible in passive original CLI readback; the temporary
fixture Provider definition is restored byte-for-byte. No model request/new turn
was used for audit. The late-confirmation batch stops before fault admission on
a harness assumption: Core exposes a symlink to its native socket. Its one setup
turn is complete, with0 native tools/model requests and no jobs. The corrected
proxy passes actual signed-Core alias/framing/stale-status/restoration preflight.
The first continuation stopped in its source-state inventory before starting any
service/proxy/native action. A corrected short local gate reuses the completed
setup to test delayed finish, duplicate question answers and uncertain answer/
start restart receipts; it returns inline without another queued notice. The
native-socket proxy is correctly refused by the actual SO_PEERCRED PID check,
so that injection method is retired. The replacement delays only an HTTP model
response completion event; Native ownership verification remains unchanged.
That gate passes four scoped cases in36.159s: real finish timeout/original-result
recovery, duplicate question answers, Stop during incomplete finish, and uncertain
answer/start receipts after restart. Independent canonical/CLI audit confirms3
completed turns,1 deliberately interrupted turn,1 question call/result and0
unreturned calls; scoped Stop takes0.203s. Native summary qualification confirms
two distinct compaction rounds under the same controller, old-key isolation,
uncertain-receipt blocking and old-controller fencing after restart. Its original
automatic false (requiring idle when Core correctly unloaded) is preserved;
independent recovery confirms Stop/no jobs without replay. Four virtual-round
commit regressions pass in sealed f7-settlement-34 inputs:79 library tests,
1 explicitly ignored native model fixture, fmt and all-target Clippy. These new
gates use0 actual Chat requests. F9's first batch at f9.179qboye stopped in4.022s
with0 Chat requests: wait read paged history before the first turn was persisted.
The first native turn was stopped, with0 tools/results and all four trees unchanged;
three tasks never started. Original result and independent review-1/audit.json
are retained. Candidate35 distinguishes the exact initial history rejection,
preserves cursor/explicit unavailable state and waits without replay or false
completion. All seven source gates and an actual Android pending-to-completed
history transition pass; the latter has1 real native call/result, original CLI
final and0 actual Chat requests. Helper36 additionally isolates creation keys
between distinct runs (57 Python checks). The equal four-route batch at
/data/data/com.termux/files/usr/tmp/f9c1.ina35ejy has ended after704.892s/11 model
requests/321 native-control and340 input-client observations, with0 supervisory
model turns between tasks. Independent development-audit.json accepts all four
native implementations: unchanged16-test suites and additional API/CLI checks,
actual diffs,23 original native calls/results,0 unreturned, identical1092-character
task, saved sources and original CLI finals. The original automatic false remains:
the general-origin Chat withheld its final marker, wrongly reporting that the
native final was absent. That very final is in two original MCP responses it
received; native item creation order places it before later tool reports. No
development is repeated. Candidate37's focused current-turn completed_result view
passes source and original-final readback qualification for wait/events, including
an already-covered cursor with empty incremental events. The same-Chat receipt
follow-up at /data/data/com.termux/files/usr/tmp/f9-receipt.d5mHzoCj used1 request
(2537 body bytes) and independently passes actual result receipt: one read-only
MCP wait, exact original final/turn IDs and the full original final represented
verbatim as a quoted text literal. The Chat explicitly corrects its earlier claim.
No native turns/calls or workspace changes occurred. Its automatic false remains:
the private runner selected a hidden final parent; the app stored the new user
message beneath the preceding public final and removed that hidden terminal.
Public source prefix is unchanged, but the old input receipt lacks a pre-bound
delivery cursor and remains unreconciled under its strict parent check.
Do not replay it or rewrite its journal binding. Candidate39 source now derives
a public input cursor from the already verified read, binds that exact delivery
parent before submission and retains the original source cursor. Its regression
covers backend removal of the hidden terminal, original cursor retention and
rejection of a changed public source prefix, without replay. The bounded F2
actual admission/result check at /data/data/com.termux/files/usr/tmp/f2-parent.KPCkTrJV
passes automatic and independent review:23.137s,1 app request/1617 body bytes,
0 tool/native/development calls, same saved Chat, exact input UUID/task/parent,
original cursor retained and pre-bound public delivery prefix unchanged. Fresh
MCP, Provider CLI and saved public final agree. All four original native workspaces
and transcript prefixes are unchanged. The one final notice is verified in a new
canonical turn of the same primary. Proof: review-1/independent-audit.json.
Do not replay that input. Candidate40 also re-reads this same original receipt
without inference. F7 declaration application, host management and the final
matrix audit have passed as recorded below. The Rust policy now has12 true/4
false idempotentHint values; other hints are unchanged. ChatGPT's backend
tools/list response omits idempotentHint, so internal consumption and any hint
speed benefit remain unknown. The metadata refresh itself succeeded; the overly
strict intermediate probe false is retained and reconciled without reposting.
The comparison uses the candidate
work CLI and original-TUI result observation, explicitly distinct from typing
the task into the original native TUI. Launch is recorded separately; preparation
and launch receipts do not establish successful model output.
F1–F9 is accepted at the explicit mixed-evidence integration scope below;
G release/lifecycle/device qualification remains separate.** C/C2, D/D2/D3 and
chat-entry.5wv7_5h3 retain their bounded evidence below. The direct-entry final
independent verdict is accepted; its original automatic verifier false negative
remains unchanged. E functional, ownership, app/host and recovery choices are
settled below and in SPEC; common idempotency source is implemented. F now has
explicit implementation/acceptance inputs and bounded combined acceptance for
the managed path, shared reading and coordinated agents. The earlier blanket
agent deferral was an assistant interpretation corrected by the user. G then owns
product/lifecycle delivery and H retirement.
This handoff launches no experiment and changes no installed defaults. Reuse D2 exact-input proof;
do not repeat its completed six-task runs without a changed input or specific gap.
The current working
branch is rewrite/rust-core, HEAD2d81dd74b61e0469f2fe68a63404f70faf3c958f.
The approval controls, bridge/viewer lifetime fixes, related tests and documents
are uncommitted. Inspect the actual current diff before continuing and preserve it;
do not restore files from HEAD or infer publication from a green local build.
The earlier qualified snapshot is phase-c-scoped-render-inputs.json and immutable
binary a33c3e969239ebbec12915f925ee0f4beaca98b51fc5b45b6f4ced430d85f255.
User-directed fixed annotations change src/mcp.rs and src/operator.rs relative to
that snapshot, plus desktop_mcp_stdio.mjs and related assertions. Preserve these
edits separately from the earlier approval implementation. The annotation snapshot
independently passed scripts/check.sh provider, including fmt, Rust tests/clippy/
release,115 Bun and36 Python tests. Its immutable trial binary is
eeaf90075dd092d1a78765f9b4f21b97b735b5b985a8d7e3dff1270898352af2;
phase-c-annotation-inputs.json binds that annotation snapshot. Previous candidate proof
remains separate. The first matched-input attempt below has one failed candidate
and one completed development task. The later fresh pair completes the bounded
comparison, with independent quality findings rather than an inferred winner.

### F final integration review

F's implementation and declared integration boundaries are complete. This is
not a release, activation or every-lifecycle reliability verdict. The final audit
hashes22 retained proof artifacts, checks their actual predicates and preserves
each original failed result:
~/.cache/codex/acceptance/phase-f-integration-20261021T050403Z/f-final-review/result.json.
Earlier complete-build proofs are not assigned to changed binaries; the audit
records unchanged source scopes, changed-candidate checks and separate helper
inputs. Current binary full SHA-256:
65c24d41c7796b0dcc7d16b3fa0d9048933fe91a45ae18db66d0fcda815b159f.

| Case | Accepted observable result | Evidence type / retained limitation |
| --- | --- | --- |
| F1 routes and return | Four actual CLI-MCP/CLI-local/direct-MCP/general-MCP implementations; same-work continuations and original question/final returns | Actual development plus current40 synthetic native composition. Matched timing uses the Provider work CLI and original-TUI result observation, not original-TUI task typing |
| F2 continuity/input | Actual bidirectional managed/current transfer on one integrator thread; exact destination receipt; saved-Chat follow-up with hidden-terminal-safe input binding | Actual transfers and23.137s/1-request receipt trial; draft/activity negatives use source fixtures. Old missing input bindings remain strict and preserved |
| F3 shared reading | CLI/MCP agree on duplicate-title sources, four saved branches, public order/cursors and explicit unavailable content; original titles/content restored | Actual selected accessible sources; no universal inaccessible-content claim |
| F4 serial agents | Independent review finding, real correction and30-test integration; evaluated correction releases actual dependent work | Actual native/saved/CLI evidence and post-completion CLI recovery; original failed transport/observer runs stay failed |
| F5 parallel agents | Isolated writers, actual merge conflict, writer Stop while sibling continues;8/8 tree Stops/no jobs | Actual work plus current40 native queue/dependency/tree integration; no unlimited-load claim |
| F6 management | Authentication/status without app acquisition; explicit ensure; private app boot/shutdown; credential/endpoint reuse; actual HTTP/WebSocket/RFB and connector read | Fresh owned profiles and auth/binding/unready fault fixtures; shared app/profile preserved. No existing login revoked or physical viewer observation |
| F7 idempotency | Duplicate/new-ID/lost/late/summary/restart/controller/Broker settlement cases; new40 native23/23 original returns;12 true/4 false target declarations |71.873s,0 actual Chat requests. All16 definitions otherwise identical. Backend omits the hint in its response; internal use/speed benefit unknown |
| F8 reusable inputs/history | Explicit sealed inputs outside old roots; appended/changed-prefix handling; new40 CLI/MCP66-message/3-page source equality |8.783s,0 Chat/native calls, unchanged work records;9 unavailable placeholders remain explicit |
| F9 comparison | Equal1092-character task/baseline/model/effort/approval, unchanged16 tests per route,23 original native returns and retained UTC/monotonic/request/observation costs | Candidate35, one sample per route; origin receipt corrections are separately costed. No retroactive automatic pass, universal winner or token/billing equivalence |

New proofs under TMPDIR: f7-hints.1xQIqEsX/result.json;
f6-host.QxrWHf4G/continuation-2/result.json and viewer/result.json;
f8-read.PeXdqKQ2/continuation-1/result.json; and
f-live40.e5FJz18q/refresh-reconciled.json. Host helper42 changes only the stale
owned-host test, decoding the real control envelope and requiring explicit ensure;
its fmt/Clippy/compile and actual Android run pass. Production source is identical
to candidate40. The already passed private-pipe app test was not repeated.
This final review made0 actual Chat requests. Installed/default/public state is
unchanged. G product/lifecycle/composition, signed delivery and separately
authorized activation remain next; H retirement still preserves unique work.
Final ownership/state checks at2026-10-21T23:44:28Z confirmed the F service was live, the shared app was
ready with0 active lanes and the unrelated prior Provider owner is still alive.
Loaded fixture threads are observed idle with0 jobs. Seven old work records stay
reconciliation_required rather than being guessed/replayed: independent canonical
inspection finds0 pending calls and0 unreturned original calls across those
records. They are not counted as new successes. Proofs: final-state.json and
legacy-reconciliation-review.json beside the final audit.

### G product qualification

The user authorized G after the2026-10-22 status cleanup. Retain three usage
modes: Codex managed MCP (preferred) with explicit before-start local alternative,
current-Chat direct MCP, and current-Chat general MCP into the same managed path.
Retain the complete native tool/control surface and shared saved-Chat reading,
input, agent/review/transfer and host interfaces qualified in F. No route change
or automatic fallback may replay uncertain effects. Legacy routes stay preserved
until H; selected product paths do not change installed/bare-config defaults.

Product qualification starts with real Android Core plus independent Provider
snapshots in fresh signed fixture roots: Base, Manager without enhanced TUI and
Full. The reusable driver scripts/qualify_provider_product.py binds explicit
Core/frontend/current/previous Provider and generation inputs. It checks ordinary
native configuration, generated --profile chatgpt, relocation/restart/rollback,
stable endpoint with rotating local credentials, retained catalogs and native
config/auth, optional Provider absence and Manager account registration. These
are configuration/service lifecycle checks, not model development or a signed
Provider installer. The Python driver is qualification tooling, not a production
Provider dependency. A separate actual PTY native-to-Chat-to-native check uses
owned synthetic Responses; no real Chat request or existing app profile is used.

Evidence root:
~/.cache/codex/acceptance/phase-g-product-20261022T004144Z.
The first two composition attempts (historical9fef, then prior candidate39)
stopped before registration with host_rootfs_required (4.383s/4.846s). The driver
omitted both an app rootfs and the complete native binding required for bootstrap;
this is a shared precondition, not an observed version regression. The corrected
driver explicitly binds the disposable Core/home/workspace. Attempt3 then exposed
another driver error: upstream forbids --profile on features list. MCP list is
an admitted configuration-loading command and replaces that check, without
starting development. All original failures remain preserved; no uncertain
mutation is retried. Attempt4 passed all three composition bodies but failed its
final source check because a local copy-loop variable shadowed the repository
path. Its failed verdict is preserved. After fixing that observer, the distinct
composition-5 run automatically passed all84 steps in31.733s, independently
checked in review-1.json. At that review all6 frozen helper/data hashes matched
the then-current bytes (later retention changes affect qualification tooling);
all72 noncredential stdout logs and3 fixture signatures verify. No real Chat/
native development turn was submitted, and no fixture Provider process remains.

The accepted configuration checks cover Base, Manager without TUI and Full;
each goes candidate39 ->40 ->40 restart ->39 in immutable private snapshot paths.
The profile follows the selected snapshot, the registered endpoint stays stable,
local credentials rotate and native config/auth/activation bytes and old catalogs
remain intact. Manager accounts discover the generated shared profile. Upstream
MCP list loads the actual --profile configuration without inference; this is not
an actual --profile development run or an authenticated Provider installer.
The existing F owner, unrelated prior Provider and shared Linux supervisor retain
their observed executable/start identities after qualification.

Both actual Android PTY compositions passed: enhanced TUI with Manager, and the
same TUI without Manager. Each retains one native thread across native -> Chat
route -> native, preserves approval/sandbox settings and public history, sends no
inference during native-to-Chat preparation and uses exactly one synthetic source
summary on return. Each run has4 owned synthetic Responses and0 actual Chat
submissions. Proofs: tui-full-1.json and tui-no-manager-1.json at the G evidence
root. This checks the real Provider checkpoint and terminal composition, not real
Chat judgment, physical screen interaction or whole-product release admission.

G now implements and qualifies the independent optional Provider package under
[SPEC's selection contract](SPEC.md#optional-provider-packaging-and-selection).
Core adds install/status/rollback and opaque `provider chatgpt ...` dispatch;
release-builder shares the ordered eight-file manifest codec and signs packages.
Core authenticates with its admitted update authority, verifies exact files and
private staging, probes without inference, preserves admitted rollback keys and
sequence high water, and blocks selection changes during execution. No new crate
or dependency on Provider implementation is added. Profile preparation remains
explicit; selection never rewrites an existing immutable profile binding.

Current G immutable candidates are at
~/.cache/codex/acceptance/phase-g-package-20261022T025826Z/candidate-2:
Core dabb3d9549c33ca2f5a04a924dfb27adadbd57bfc5604de3942a61ec317ab8e9;
Provider134ea670e52f6a9f5bb1f6fe7ef51d03b0dd09456f050a977fd70f2d9230483c;
release-builder536208d6ecd5b837c6a77bc6fadd6912eaaf352eae7f0151dca0475e5a9d833f.
The Provider change from F40 is its no-inference artifact-probe entrypoint. The
owned F service still runs40; no candidate was activated on the device.

package-3.json passes83 actual Android command steps in67.572s across Base/Full:
install, exact reinstall, update, rollback/high-water preservation, retained signer
keys after simulated admitted-authority rotation, direct/leased live-owner update
refusal, pinned-profile and native-default preservation, malformed optional-state
independence, changed/signed-collision/extra/symlink/mode/signature/downgrade/foreign-
signer rejection. Test generations deliberately share one Provider binary; this
checks installer selection, not arbitrary version/data migration. The authority
change is a private fixture input, not a new Core-rotation proof. Final signed
source-inputs bind124 files including embedded JS/CSS/HTML/model/summary assets;
copied source and original stdout/stderr remain in TMPDIR/g-package-nhfhu8pc.
independent-review-2.json independently verifies all package signatures/inventories,
source/log hashes, actual profile binding and admitted selection/key/digest state.
Original package-1/2 results remain accepted at their narrower source/style scopes;
package-3 supersedes them for complete source-input provenance.

composition-1.json repeats the affected composition scope on the new immutable
Core/Provider:84 steps,35.932s, Base/Manager-without-TUI/Full, actual profile loading,
endpoint/credential reuse, candidate40 -> new -> restart ->40 and Manager account
registration. Old actual PTY proofs remain scoped to their original Core/Provider;
this batch does not assign them to the changed release binaries.
Final workspace gate passes363 tests (1 explicit ignored) and Clippy; Provider
Rust passes153 (27 explicit opt-in ignored), Clippy/release build, JS152 and Python
36+55. All-target tests require their own retained opt-in actual scopes; ignored
cases are not inferred passed. Initial fmt and Clippy producer-style failures are
preserved; the final workspace gate is green. Provider's successful test segments
are retained with a separate successful final helper/docs tail: editing check.sh
while it was running broke its original tail read. Completed green tests were not
replayed for that harness fault. Seven input-schema tests and three check/scope
script tests also pass. Debug/test symbols and incremental output are disabled;
settings, logs and exact hashes are retained in this evidence root.

All this G work submits0 actual Chat requests. Automatic retention now recognizes
nested g-package fixtures as well as g-product:11 payload copies per package run
and16 for the new composition were archived with original hash/mode/restore records.
The first run's old no-op retention hook was corrected and its inactive payloads
separately archived; no proof/source/config/workspace/current Provider was deleted.
Compiler caches remain under the15GiB budget. Installed/public/default state and
the F/prior-Provider/shared-Linux process owners remain unchanged.

G still needs broader retained-route host-loss/approval/context/TUI/cold-recovery
and provisioning/release acceptance, authenticated remote optional-payload delivery,
complete public readback/disposable update and non-forced promotion. Local fixture
signatures are not a public release. Ordinary Core launch/update must not download
an app/rootfs or require ChatGPT login; third-party apps/login/user data stay outside
payloads. The next G work is the remaining lifecycle/composition boundary review
and remote delivery integration; device activation remains separate. Further real-
model trials need a declared remaining boundary and request budget; do not repeat
the completed F tasks or infer whole-G acceptance from this model-free batch.

### G full continuation in progress

**Current G state:** all four installed usage combinations and the distinct
same-Chat Termux-process cold follow-up have passed independent review. Physical
Android reboot is now observed and request-free recovery passes at
/data/data/com.termux/files/usr/tmp/gboot.12t92bu3. The saved final-2 snapshot remains
intact. The same Linux profile, CONNECT proxy, new owned relay, same Provider/state/
endpoint and owned tunnel are restored; actual remote discovery returns all16
tools and its read probe passes. Original native history and all71 saved-Chat
messages were read after boot without inference. The selected native work remains
stopped with no pending calls. The later actual device update/use/rollback/use
acceptance is recorded below; remaining product/release gates continue. The prepared return command is
`bash ~/.local/state/codex/resume-g.sh`; it checks saved artifacts before
same-profile/thread launch. Ordinary resume does not require a changed boot ID;
physical reboot is a separately observed qualification gate. The two other AI CLI
windows are user-confirmed idle, but may
be Codex launch wrappers; preserve them with the supervising session until its
final handoff instead of risking termination of the active execution chain.
The physical device is on installed generation86 with previous generation88.
Signed public delivery/promotion is accepted; this does not activate the newly
published Provider on the operating profile or change the selected device default.
The earlier resume attempt closed/restarted Termux rather than rebooting the phone. Same boot ID
with old owners gone is consistent with that action; do not label it a physical
reboot pass. The first helper wrongly made kernel reboot a prerequisite for any
resume and raised AssertionError. Its original files/error evidence are retained
privately; resume.sh now permits normal recovery while the explicit strict boot
check remains available separately. This primary thread is already resumed.
The real public update, meaningful generation88 use, supported rollback and
generation86 use after restoration are accepted below; preserve their exact
records and do not repeat them. `post-reboot-steering.json` retains the original
authorized scope.
After that Termux restart, recovery at TMPDIR/grec.ahufjs8y reuses the preserved
Linux profile, a newly verified owned relay and the exact immutable G Provider
at its previous endpoint; connector alias readback is ready/healthy. Setup uses0
model requests. The first bounded cold follow-up at
/data/data/com.termux/files/usr/tmp/grec.ahufjs8y submitted one input to the same
saved Chat and stopped with “Session terminated”; native Stop confirmed no
remaining jobs, no workspace changes, and no replay. The cause was concrete: the
Termux exit had killed the loopback HTTPS CONNECT proxy used by the still-running
tunnel. Local tunnel health/readiness stayed green while remote MCP control was
connection-refused. Restoring the exact owned proxy listener on its former port
recovered the unchanged TLS tunnel; app-cache refresh verified all16 live tools
plus a read-only call, with zero model/native effects. The failure and its original
records remain intact.

R11 at `/data/data/com.termux/files/usr/tmp/gt.z1my9k_g/continuation-11` did send
one actual ChatGPT app dispatch and completed its native Codex task in the same
saved Chat. The app stream is `Completed`, the saved Chat readback is complete,
and the isolated native rollout contains the command output marker and final
answer marker. The original `result.json` remains unchanged and says failure/zero
requests; that verdict is a harness false negative, not the Chat task outcome.
The observer and cleanup used the original Provider port45475 while the candidate
Provider listened on34061, so their authenticated requests got HTTP401. The
separate `review-reconciliation.json` records the evidence and correct dispatch
count. After the task completed, the exact candidate Provider and tunnel were
stopped, the original Provider was kept, and the original route was reconnected;
readiness then verified all16 remote tools and one read-only call without another
model request. Do not replay R11. Before another Chat input, fix the candidate
port binding and count managed/native app dispatches separately from local-source
request counters, then statically audit observer, cleanup and live-child ownership.

The model-switch review at
~/.cache/codex/acceptance/phase-g-model-switch-review-20261022T144507Z/review.json
confirms R12's separate TUI launch failed before model-selection/task input:
explicit --remote bypassed the maintained local frontend path, and the remote
bootstrap could not resolve chatgpt_desktop. No Chat or native inference occurred.
Its proposed zero-Chat budget was incorrect: the current frontend requests a
source summary for every nonempty Chat-to-native switch. R11's retained effective
history already contains24 readable items (29,860 JSON bytes), including its prior
summary and later original call/result/final; it has0 opaque items. Bytes are not
token counts. A fresh summary is not proven necessary for that case. Upstream's
own conditional compatibility/context-limit compaction must remain distinct from
this frontend's unconditional summary policy. No production transfer policy was
changed, no R11 input was replayed, and no new live proof was launched by this review.
The failed downshift script also treated its synthetic native endpoint as a real
native-model route, selected picker rows positionally, and could omit untracked
files from a claimed diff. Replace that preparation before use. The observer helper
now binds release to its actual thread and supports a PTY-only lease without fake
input pipes;10 affected local tests pass. Earlier unlaunched copies must supply
the new thread_id argument before use.

The272000 context window in this fixture comes from
scripts/fixtures/permission-model.json via qualify_permission_lifecycle.fixture;
it is deterministic test metadata, not the current account's native model limit.
All six operating default/account homes currently cache gpt-6.1-sol with a500000
context window and95 percent effective-context metadata, and have no explicit
model_context_window or auto-compaction limit override. No limit was changed by
the review. At the user's request, default/api/awrk/janmori101/szob/wrlab all now
select gpt-6.1-sol/high; only janmori101 and wrlab needed edits from gpt-6-luna.
Private backups/readback are in ~/.local/state/codex/model-defaults-20261022T143829Z.

The user then directed removal of fresh summaries from ordinary model selection.
Source1c58d1cb4732fa654a37db5ddd073acc375a6a58 implements this and updates SPEC:
the picker passes an explicit false summary request, resumes the same thread,
and leaves source-summary generation plus exact completion/failure/cancel handling
available to an explicit internal caller. This does not disable upstream's own
conditional compaction. Upstream0.163.0 also injects model-specific instructions
through its model_switch developer fragment; that is not a generated task summary
and needs no prior-model request. Existing readable history remains upstream-owned.
The Android regression now requires0 source summaries, unchanged public turns
across return, and prior Chat task/marker presence in the subsequent native input.
Local Python gate passes18+112+11 tests; pristine-source assembly, formatting and
diff checks pass. User changes outside these five committed files/hunks remain.

The exact-source native workflow39799995620 at that source SHA passes build,
50 focused tests,94 retained upstream tests and lint. The executable digest is
6b924d78d885afa68dbb818aa4f8608748f78cd668884f4e7585d4d4cba63c09.
The owned batch TMPDIR/g-switch-2n8s9z01 completed in2223.526s, including46.871s
and44.209s actual Android same-thread synthetic transitions with/without Manager.
Each uses3 synthetic responses and0 source summaries; exact original turn
preservation and prior Chat task/marker presence in the next native input pass.
Actual Chat requests and native-account inference are0. The native focused
stage spent13m27s compiling tests and3.084s executing50 tests. The original result,
commands, full native build proof and independent review are archived privately at
~/.cache/codex/acceptance/phase-g-model-switch-1c58d1c. That existing qualifier
deletes successful raw fixture PTY/rollout files; its live assertions are verified,
not independently re-read raw transcripts. No new saved Chat applies to this
synthetic batch. Canonical receipt of its final notice on primary
01a15330-0d46-7b51-a460-5e0495030cf2 is confirmed. No operating activation occurred.

The distinct0-inference check at TMPDIR/grt.v6l_b8m3 opened the local picker but
stopped before any model selection: its new temporary HOME disagreed with the
old execution home's shared-profile symlink, so the exact shared-registry guard
correctly omitted native augmentation. No inference endpoint request occurred.
All9 original turns and original rollout bytes are preserved. Its own idle TUI
and backend exits are kernel-confirmed, and its PTY keeper is released; old R11
backends were not stopped. Keep the original failed result and reconciliation.json.
Continuation-1 binds that disposable execution-home link to a private identical
profile in the current temporary HOME's registry, recording the old link for
restoration after settlement. The original source profile contents are unchanged.
Its actual original-TUI Chat-to-native selection succeeds with all9 original
turns unchanged and0 source summaries/model POSTs. One guarded native WebSocket
GET is metadata/transport preparation, not account inference. The follow-up
return picker was actually displayed; raw ANSI cursor updates split its label
and defeated the harness's literal substring check. No return selection occurred.
Its original failed result remains; reconciliation.json accepts only the native
stage and confirms idle state plus exact owned TUI/backend exits and keeper release.
The diagnostic label matcher now handles cursor/style-split labels;11 local
tests, including that actual failure pattern, pass. It is not a terminal emulator
or an execution receipt; native model/thread readback remains authoritative.
Continuation-2 reused the accepted native selection and selected the GPT-6 return
row. It then stopped on a harness caption mismatch: upstream displays Select
Reasoning Level, while the harness awaited Select Reasoning Effort. Its original
failed receipt remains. The same TUI24758 and PTY keeper25753 were preserved at
the selected None default, with9 unchanged completed turns, native provider idle,
one guarded transport GET and0 model POSTs. No return effort input had been sent.
The distinct finish-pending-effort worker stopped before input because it compared
the entire rollout digest with the pre-resume snapshot. Resume had appended one
ordinary thread_settings_applied event. Independent preflight confirmed the exact
original prefix and all9 public turns, and classified that sole added record;
the stale whole-file equality was a harness defect, not changed original history.
The separate settle-pending-choice operation attached the same TUI/PTY using
pidfd_getfd and sent only the pending Enter once. It passed in6.595s, with the same
native ID, all9 turns unchanged, chatgpt_desktop/GPT-6 selected and idle,0 source
summaries/compaction records/inference POSTs. One context-only quoted public-history
handoff was canonically injected as specified; it matches every retained public
turn/selected field and is not a new public user turn or generated model summary.
Its original native bytes remain an exact prefix. No saved Chat task was submitted
or read again. Exact TUI and two backend exits are kernel-confirmed; the keeper
exited and the original profile symlink/unchanged source profile were restored.
Raw PTY, before/after native snapshots, final rollout, all original failed receipts
and independent-review.json are archived with140 file hashes at
~/.cache/codex/acceptance/phase-g-retained-model-switch-1c58d1c. Routine owned-fixture
cleanup archived/removed6 manifest-matched inactive payloads (878,034,944 allocated
bytes); cleanup.jsonl retains restore records. Old R11 backends/state were preserved.
The completion notice is canonically received on the same primary thread in a new
turn; notification-receipt-review.json records that acknowledgment separately from
the original queue-only receipt. Delayed delivery is not a new execution intent.
This complements accepted synthetic next-input proof without repeating R11 or its
Chat task. Do not relax the routing guard or replay any earlier input.
The distinct model-free composition batch at TMPDIR/gl.e3cff5zs freezes the
qualified Core0502f7dd, frontend6b924d78 and
current stop-fix synthetic service test dfe9031a, plus its complete Python helper
inputs. It combines backend/Provider replacement, confirmed Stop/new-turn recovery,
16 further sequential turns and120s idle in a new signed fixture, with a declared
512MiB fixture-process RSS-growth ceiling. The20 synthetic native inputs are not
actual Chat requests or accepted real development repeated. Raw PTY, per-followup
times/resources, original native snapshots, status observations and whole-process
diagnostics are retained; one final-only notice returns to this primary. Do not
launch it twice. This is a bounded endurance/composition check, not hours-long
real-Chat rotation or physical device activation. Its original result remains
failed: during followup12, one public snapshot showed interrupted after the final
item but before the completed receipt was reflected. The canonical rollout has16
task_started/task_complete pairs with matching IDs, including that exact last turn,
and no turn-aborted/error record. Provider observation ends completed/idle with0
pending/human calls; all owned processes are gone. Thus12 followups were submitted,
not just the11 recorded before the observer exception. No input is replayed.
The shared settlement predicate now accepts only exact expected-turn-count plus
completed/idle, retains its bounded wait for other sampled states and never treats
a final item alone as completion. Twelve diagnostic regression tests pass,
including interrupted/failed/inProgress, wrong count and non-idle rejection.
The distinct remaining-1 continuation preserves the original raw CLI, native
rollout, app-command log and observations before restoring the same signed
fixture/profile/state/endpoint. It must observe16 prior native turns and0 new
dispatches on resume before submitting only new labels13..16, then120s idle and
the declared RSS bound. This completed and passed independent review: all20
canonical start/complete IDs match, each followup1..16 occurs once, the original16
turns are identical, and the4 new synthetic submits have distinct round IDs.
The last4 turns took2.106/2.122/1.850/2.103s and idle lasted120.745s; continuation
total134.464s. Final native state is completed/idle. Exact TUI/backend exits and
successful synthetic-service exit are verified. RSS across these fixture processes
was195548 ->47580KiB; ending swap40964KiB means this is a bounded resource sample,
not evidence of no leak or live-Chat memory behavior. Two segments with supervisor
correction do not establish a single unattended endurance run. Actual Chat and
native account inference remain0; no new saved Chat applies. All original failures,
CLI/native/app-command records, helper inputs and independent-review.json are
archived with2775 file hashes under
~/.cache/codex/acceptance/phase-g-retained-composition-1c58d1c. Its final notice is
canonically received on the same primary; profile switching did not create a new
thread. Inactive manifest payload cleanup retains restore records in cleanup.jsonl.
Do not repeat the baseline restart/Stop work or labels1..16 to erase failed receipts.
Whole G and permanent operating activation remain unfinished.

A distinct continuation at /data/data/com.termux/files/usr/tmp/grec.next-053235
reused current-reasoning work cb062e52cfb4303b076cdee2a9ea7fe84f1bc1e1ec5075e7,
native thread 01a161d7-4459-75d3-8186-ca4d371738ab, workspace
/data/data/com.termux/files/usr/tmp/gg.iprzgrte/w-direct and saved Chat
6ad98f1a-7620-83e8-be41-d187b26daf77. It added format_label and six focused
formatter tests after recording the expected missing-function failure. All13
original/new tests passed; test_original.py and test_regression.py hashes match
their pre-change values. Independent review matches the actual diff, test log,
completed native status/history, original CLI final marker and saved-Chat
completion/final marker. One follow-up input completed in100.564 native seconds;
there were no supervisor calls between tasks. The cold-continuation cohort has
used2 of its4 authorized input requests, including the non-replayed failed input.
Same-session notice delivery returned to this primary thread.

That cold follow-up proves Termux-process restart recovery. The later physical
boot changed273b044d-5536-4594-a9c4-7ae7c776a5ad to
09ae7c1e-44ff-4f03-9398-e71e7b3d73ab; strict boot/preservation verification passed
on the same wrlab primary thread. After request-free recovery, continue the
authorized actual public device update, meaningful updated-runtime use, explicit
supported rollback and prior-runtime use. The device is still on generation86;
the public advisory index currently names generation88 with the same upstream
0.163.0, subject to the real updater's authenticated admission. Both installed
generations86/85, their Core/metadata and current profile settings are archived
with file readback in /data/data/com.termux/files/usr/tmp/gdev.lvqnpcin/device-before.json;
its archive is399,183,981 bytes. The actual updater subsequently authenticated and
activated Full86->Full88; a new development task used the actual installed88
runtime, and the supported rollback restored86. Restored-runtime development is
the remaining part of that device batch; do not repeat the accepted earlier steps.

The continuation's independent review is grec.next-053235/review-1/review.json:
three native commands and two patches have all five original results; the expected
red test precedes implementation and the13-test green result. The controller's
228.341s includes setup/observation/settlement; the model's100.564s measures its
native development commands. The current profile has changed from the original
szob to wrlab; the resume command and selected rollout now follow wrlab on the
same primary thread. Both accounts' private settings are preserved. Settlement-2
retains the initial overly strict unloaded-thread assertion: the only loaded
thread was the known completed/idle thread, with native Stop and zero jobs. Its
terminal turns/idle status were reconciled before the exact native owner exited.
No unknown process, supervising Codex or launch wrapper was signalled. The next
restore must reconstruct the saved loopback proxy before connecting the tunnel,
then pass actual remote MCP catalog/read readiness before any model input.

The user selected the MCP display name **Termux Bridge**. The owned app's actual
name PATCH/readback changed only name/updated_at; all16 action definitions and
annotations are byte-equivalent to before. The connector/tunnel IDs stay bound
to the original owned reference. The trial service was settled, configured with
transport.connector_name=Termux Bridge, restarted and reconnected, then actual
remote catalog/read readiness passed again with zero model requests. New owned
connector reference and rename evidence are at gboot.12t92bu3; old reference files
and historical trial names remain preserved. SPEC/defaults/future qualification
prompts now use Termux Bridge. Annotation policy has not changed.

The first post-boot metadata read reached the app before its HTTP client was
injected and failed locally on undefined httpFetch, before a refresh mutation.
The helper now waits up to30s only for that exact pre-transport failure on its
initial read; other errors,429 and unknown mutations are never retried by this
wait. Full Provider Rust153 tests pass (27 explicit opt-in ignores), fmt/Clippy/
release pass, JS159 and Python36+56 pass; G-driver's5 tests and the affected
F/G observer39 tests pass. The initial gate omitted BUN and the later G-unit
invocation used the wrong module import path; original failures remain, completed
green segments were reused, the remaining segments ran, and the portable G test
is now included by check.sh. Exact installed-runtime ownership is explicitly
selectable in the qualifier while fixture defaults retain their previous scope.
New Provider5860d7e5ba5ad97d5eae79edd65606c31f1d50b2a1f4ec0d96c9f1d1490d1dc7
is retained under gdev.lvqnpcin/candidate, with source/helper hashes. It differs
from134ea670 at the configured MCP default name; no new public asset or operating
Provider selection is implied. That candidate is now retained separately at
gdev.lvqnpcin/candidate-before-native-routing/provider. Actual installed-device
preparation exposed two additional product conflicts: managed native routing
republished the protected shared profile, and native work discovery interpreted
seven older records for other execution homes before distinguishing their owner.
Managed routing now renders per-thread settings directly, retaining only its own
immutable catalog snapshots; explicit prepare still owns profile publication.
Discovery first filters bounded/private owner homes and retains complete record
validation for the selected home. Unknown/unsafe owner files remain errors.

The prepared device candidate is now
b3efd983e0727715ff22bc262948c0ed274507966b3af9bbbe2319ec3007506b.
Its Rust gate passes155 tests (28 opt-in ignored), fmt/Clippy/release build; the
affected F/G helper tests pass39. The qualifier also sends compact catalog bytes
matching Pool::prepare, avoiding an extra backend caused by CLI pretty output.
Actual installed generation86 native work creation/Stop, unchanged shared-user-
profile hash, exact shared preflight/work backend, original app input-client
initialization/release/disposal and local16-tool/read probes all pass with0 model
requests at gdev.lvqnpcin/preflight-native-routing-filtered. All trial owners have
confirmed exit; failed preflights and original intents remain preserved without
replaying uncertain creation. The other isolated backend from the earlier byte-
format mismatch was independently inspected with0 loaded threads and its exact
owner exited; user/supervisor backends were not signalled.

The actual public update/use/rollback/use batch is prepared at
/data/data/com.termux/files/usr/tmp/gdev.lvqnpcin. Its scope is two new MCP development
tasks (next_label after update, previous_label after rollback), preserving every
prior test; expect2 app inputs, cap4 total/2 per task,40 native calls per task,
one task at a time and15 minutes per model phase. It must use actual installed
generations, retain the supervising wrlab/primary session, restore the original
Bridge alias after settlement and deliver a final-only notice to that same thread.
No Pro/GPT-5.6 or uncertain-input replay is allowed. Consult its exclusive
launch-intent/run-started/result records before continuation; do not launch twice.
That initial detached batch has ended. Its updated-runtime task passes independent
review at gdev.lvqnpcin/updated-independent-review.json:203.848s,1 observed app input,
8 native calls/8 original results,0 unreturned,18 passing tests and all13 prior
tests' files unchanged. A real missing-function test failure preceded the source
patch; the original CLI and saved Chat final match. Public saved pages are complete;
hidden messages/private capability redactions mean content_complete is false,
so this is a verified public final, not a claim to retrieve hidden reasoning.
The complete tracked/untracked diff and unique updated source are preserved in
updated-preserved-source; exact pre-fix Provider source matching the original
binary input hashes is preserved in updated-provider-source.

The actual supported rollback to86 succeeded. Before its next task was submitted,
the local app-owner preflight found the previous completed lane still retained.
The account/owner had not changed: receipt-only Stop marked the lane stopped,
but shutdown still required a separate renderer Stop flag before releasing it.
A regression test reproduced this; shutdown now accepts the committed Completed
receipt with its matching fresh idle/error-free snapshot and release acknowledgment.
Missing receipts still preserve the server Stop requirement. Rust156 tests,
fmt/Clippy/release pass (28 opt-in ignored). New candidate is
88257d2a58fa85d10218c5df45105d109fc7d2a9af3a794374b4857a4ecea3c4.
Both original86/88 native tests stay scoped to their actual Provider inputs.

The leftover lane was reconciled against the same account, completed native work,
saved Chat and durable MCP virtual->physical round mapping, then released and
disposed with0 inference. An initial direct virtual/physical comparison rejected
before release; that original failure remains. The idle rollback Provider/native
owners have confirmed exit; no user/supervisor process was signalled. Its prepared
development task was never submitted. The continuation at gdev.lvqnpcin/recovery-1
therefore schedules only that missing restored-runtime task, on the fixed Provider,
with1 expected/max2 additional app inputs and40 native calls/15 minutes. The prior
cohort spent1 of4 allowed inputs; this continuation stays inside that ceiling.
It also verifies actual automatic app-lane disposal, restores the original Bridge
alias, and sends a final-only notice to the same primary thread. Consult its
exclusive launch/result records before any continuation; original requests and
accepted update/development/rollback must not be repeated. Existing generations
86/85 and settings remain preserved in device-before.tar.gz.

recovery-1 ended before task creation/model input: its remote readiness returned
HTTP400 mcp_provider_rejected. The new Provider88257d2a remained live with0 loaded
native threads; local authenticated16-tool/read checks passed. The tunnel connect
had returned already_running=true and reused a stdio process holding the previous
service credential. An explicit owned disconnect/fresh connect corrected that
transport boundary without restarting the Provider. A plain remote read remained
pending; its original request is preserved, not resent. One non-retrying app
metadata/cache refresh then passed actual remote16-tool discovery/read in4.819s
with0 model/native effects at recovery-1/metadata-reconnect/refresh-private.json.
Source definitions/annotations are unchanged. Local health alone was not accepted
as remote readiness. On service credential rotation, explicitly settle/reconnect
the owned stdio transport and reconcile the app connection cache before model input.

The gdev.lvqnpcin/recovery-2 continuation has ended. Independent review at
rollback-independent-review.json confirms meaningful actual installed86 use:
193.564s,1 observed Chat input,6 original calls/results,0 unreturned and23 passing
tests, with all18 preceding tests' files unchanged. A real missing-function red
test preceded implementation; original CLI/native and public saved-Chat final
G_DEVICE_ROLLBACK_COMPLETE match. Automatic shutdown now releases the completed
lane/input client and confirms both exact trial owners exited. No task was replayed.

Its original automated result remains failed because the remote read after
restoring the original Bridge alias did not settle. A single metadata/cache
refresh then verified actual16 tools/read with0 model/native effects at
recovery-2/return-readiness/refresh-private.json. The original Provider owner is
verified live. device-cycle-independent-review.json accepts the combined actual
update/use/rollback/use and return boundary, linking both independent task audits,
the original command exits and callback receipt. Entire cohort:2 Chat inputs,
14/14 original calls/results,0 unreturned; actual subscription debit unknown.
Two recovery turns intervened between tasks and final connection reconciliation
was required, so whole-cohort unattended acceptance is not claimed. Unique final
source and complete tracked/untracked diff are in rollback-preserved-source;
updated-preserved-source retains the earlier task separately. Quota-driven profile
changes/session resumptions are not counted as product failures or recovery cost;
the technical fixes above retain their own evidence. Current86/previous88
selection is confirmed; original86/85/settings remain in device-before.tar.gz.

Post-device affected qualification uses Provider88257d2a, Core02ffcfd9 and
builder61c38d51, retaining their exact input inventories. Package/composition
drivers reclaimed27 generated inactive payload copies (4,048,912,384 allocated
bytes) through verified archive records; source, workspaces and original results
remain. Two settled synthetic fixtures separately reclaimed12 payloads
(1,756,069,888 allocated bytes). The3 actual fresh-profile host tests cover
explicit acquisition, private pipe boot/shutdown, owned app loss/restart,
profile preservation and competing-owner refusal without using the logged-in
shared app or inference.

The original synthetic attempt failed at its TUI entrance because it relied on
Pool.routing implicitly publishing a profile. That assumption is obsolete: the
synthetic fixture now explicitly calls registration::prepare for its private
home, and the driver verifies loopback routing/auth before launching the TUI.
The initial attempt used an inert key and received unauthorized API errors; no
actual account/model request is claimed. Its logs remain failed. A distinct
synthetic fixture passes both entrances, real native reviewer allow/deny effects,
incremental handoff, same-Chat follow-up, native summary/question, pending-job
Stop, restart/claim, uncertain successor takeover and host failure/completion
settlement. These synthetic replies do not establish live model judgment or
arbitrary long-duration context saturation. No accepted real development was
repeated; public signing/promotion remains a separate unfinished gate.

That preceding package checkpoint is superseded for delivery by
phase-g-post-device-20261022T081337Z/public-delivery-review.json. Exact c65d3f8
source passed Source acceptance39747992023, Provider39747992008 and independent
Core39747992036; the hosted Android Provider artifact also verifies against that
source. The admitted local88257d2a binary retains its actual Android task evidence.
Eight continuity/reading/host/control modules are byte-identical to accepted F40;
the changed app shutdown guard has its current regression and actual cleanup proof.

Optional Provider signing runs39749708015/39750097551 publish two immutable same-
binary generations, chatgpt-c65d3f8900bc-s1/s2, with complete12-asset public readback
for each. This second signature enables genuine public upgrade/rollback proof;
it repeats no development or model request. Actual Termux consumer at
TMPDIR/gpub.m7wj71mf passes12 immutable-install/prepare/dispatch/upgrade/rollback
commands plus6 promoted stable update/no-op/rollback commands. Its owned Core
fixture retains its current signing key and separately admits the actual public
update authority; real package signatures are never bypassed. Original native
defaults/profile and the operating device remain unchanged. Public stable selects
Provider2 through main13a27a432f6407a0dee0f104ffdffd453f061e2c; the optional payload
is not permanently activated on this device.

The automatic Core run39748559162 passed Base/Full build, Android smoke, signing,
complete public readback and disposable consumer. Base89 promoted. Full90's CAS
then correctly rejected a stale expected main: the intervening one-file readonly
Provider workflow registration was our deployment edit. Its failed original run
is preserved. After verifying that exact intervening diff and signed component-
input source/manifest binding, a distinct non-forced promotion on latest main
accepted Full90 without rebuilding or replaying tests. Public stable readback and
canonical Git blobs match. Recovery commit is afb0f39a741293716de4f0288dfa40cc454dc65f;
Provider promotion afterward changes only its three independent pointer files.
Serialize main metadata edits after an active promotion settles.

GitHub Contents API was observed transcoding a64-byte binary signature into96
decoded bytes while reporting the original blob SHA/size. Git blob API and raw
public bytes remain exact. Provider promotion now verifies the committed tree's
canonical blobs; the regression fails before the fix and all6 publication tests
pass afterward. The Core promotion itself was not repeated for this readback
error. Signed/public manifests remain immutable. Current physical86/previous88,
profiles, credentials, conversations and the original Bridge service are retained.
G remains active at its declared runtime boundaries; this delivery review submits
0 actual Chat requests and does not qualify arbitrary context saturation or model
judgment. H and permanent activation remain separate.
The affected Python gate passes141 tests (18+112+11), workflow boundary tests5,
publication regressions6 and diff/docs checks. The cleanup helper now recognizes
the generated gpub.*/consumer fixture; after verified inactivity it archives6
manifest-matched payload copies (878,034,944 allocated bytes), independently
verifying3 existing archive objects. Results/source/signed packages remain. The
active compiler cache is2.3GiB. This retention change does not qualify a runtime
transition or replay any accepted development.

The exact-source CI freeze retains all298 source files, their hashes and a
pre-commit archive in phase-g-post-device-20261022T081337Z. source-freeze.json
binds the resulting implementation commit and immutable local88257d2a binary;
rebuilding after the synthetic-fixture-only change produces those same bytes.
Do not mistake source CI success for public signed-package/readback/disposable
consumer acceptance. Installed/Core source and optional Provider delivery keep
their applicable independent release gates. All pre-freeze changes remain
preserved in the archive and implementation history.

The user explicitly requires all of G, not stopping after a package batch.
Current continuation evidence is
~/.cache/codex/acceptance/phase-g-complete-20261022T032904Z.
Remote optional installation now reuses Core's bounded HTTPS transfer while
retaining an independent fixed asset inventory; license names flatten with `--`
for immutable GitHub Release delivery. Explicit install/update discover a detached
Ed25519-signed Provider-only stable index, binding generation/sequence/manifest
hash. Ordinary Core launch/update and Base/Full channels remain independent.
Stable/local/remote paths share immutable staging and selection.

stable-2.json passes113 actual Android commands across Base/Full in103.542s,
including real owned TLS, control authentication before payload, canonical URL/
redirect/foreign-signature/tamper/missing rejection, explicit stable install/update,
wrong-index rejection before payload and the previous local install/rollback/busy/
profile preservation gates. Original remote-1/2 failures are preserved: first
used Core's narrower release filename helper, then the fixture supplied a symlink
instead of a regular curl executable. stable-1 passed all113 command steps but
correctly failed its final input freeze because a unit-test source was edited
while it was running; it is not accepted. stable-2 fixes the frozen-input scope.
Workspace tests now pass364 (1 ignored) and Clippy, including strict stable-index
canonical/negative tests. The latest immutable Core/builder are in candidate-4;
that historical package batch used Provider134ea670 before the later device fixes.
Latest Provider88257d2a needs its own affected-path proof; old accepted package
proofs remain scoped to their own inputs.

Actual Android TUI Full and no-Manager paths pass on candidate-2 Core5ee0d3ee:
same-thread native/Chat/native, real checkpoint, four synthetic replies each,
no actual Chat requests. The initial no-Manager observation failed during the
read-only public-turn materialization window; the inspector now waits only on
that exact transient and never resends a user turn/model switch. Its failed log
and fixture are retained. New actual private-pipe app-loss/restart test passes:
explicit child Browser.close, observed disconnect, competing-owner refusal,
new explicit launch and retained fresh profile. It is not Android reboot or
active model cancellation proof. Existing shared app/profile remain untouched.

The existing F service and original26 work entries remain. Seven historical
reconciliation_required records match the prior F audit's call/return counts,
with0 unreturned calls and0 loaded native threads; no legacy task was replayed.
A lightweight status request for an unmaterialized historical thread returns
native_request_rejected; that is retained, not converted into completion. A
separate real readiness fault was stale_process_owner: package replacement had
renamed the still-running original PRoot image to proot.dpkg-tmp (deleted).
Browser PID32661/start207226 and mapped ELF1545b85b were verified; original shared
and F-relay descriptors were copied privately, then only their browser image
metadata was atomically reconciled to that same live process. Tokens/endpoints/
profiles and all process starts were unchanged. Authenticated read-only host status
then confirms ready, no active lanes and unclaimed writer; no app restart/model
request was needed. descriptor-rebind.json retains content-free identity proof.

New publication helpers prepare guarded secret-backed signing, complete12-asset
public readback, signature/inventory/index binding and non-forced Provider-only
main promotion after actual disposable Android consumer proof. Base/Full pointer
files are excluded from the edit. Six offline publication/promotion tests and
five workflow shell/privilege tests pass; YAML parses. These are implementations,
not a public signing/readback/promotion success. Exact-source CI and public stages
remain to run after live product acceptance and source review.

A distinct unsupervised installed-entrypoint batch is prepared at
TMPDIR/gg.6m9d0tc_, native signed fixture TMPDIR/gn.fzn912vz. Its actual public Core
selects the independently signed immutable Provider and forwards the complete
control surface. All three entrances plus the explicit local alternative use four
fresh release-label projects, fixed original tests, native execution and original
TUI observers. Scope is installed composition, not repeating F's ledger comparison.
Expected5–8 actual Chat starts/responses, ceiling12,900s per assignment/1800s batch,
no automatic Pro or GPT-5.6. It runs without per-turn supervisor model review;
final original CLI/native/saved-Chat/diff/tests are audited together. Preparation,
native backend readiness, real connector route switch, input-client inspection
and same-primary-thread notification binding submit0 Chat requests. The prior
owned tunnel alias now targets the new private public-command wrapper, with
positive ready/healthy readback; the F/prior Provider services and shared app remain.
No connector metadata refresh was posted because the16-tool surface is unchanged.
The G receiving-turn probe was queued at2026-10-22T03:59:32Z on the same primary
thread01a15330-0d46-7b51-a460-5e0495030cf2, queue01a161c3-9f27-7c33-ae06-e1782a4b19fd.
Queue acceptance is not receiver proof: the notified turn must run notify.py ack
and inspect launch-intent/run-started before exactly one run.py start. The absolute
wall deadline and21 sealed driver/config/info/source inputs are recorded in
launch-preflight.json. This is automatic continuation of all G, not a user pause
or a G-complete verdict. Retain original unknown deliveries without replay.

The first installed-entry batch ended at2026-10-22T04:08:03Z, not all-route
acceptance. Its managed MCP work completed in159.820s; review-1/review.json
independently confirms unchanged original tests,8 passing tests, actual diff,
original CLI/native final and that final in saved Chat. All8 native calls returned.
The local work hit assignment_app_budget_exhausted: this driver mistakenly gave
local the same4-dispatch limit although local needs another model response after
each tool return. The fourth tool was an invalid delete/add patch targeting one
file twice; its rejection returned before the next model request hit the budget.
Local source remained unchanged, all4 calls returned and native/host Stop both
confirmed with0 remaining jobs. No uncertain request was replayed. Direct/general
were never submitted. Keep this original result as failed, and the MCP proof as
accepted only at its bounded scope.

A separate remaining-case intent is prepared at TMPDIR/gg.fhei62ky. It reuses the
same installed Core/Provider/native fixture and owned connector; no service or
app restart. Three fresh projects cover local/direct/general without replaying
the accepted MCP project or the stopped local work. The new namespace fences
creation/start IDs; old works are explicitly excluded from the new audit/cleanup.
The local assignment now allows16 actual dispatches, the complete new batch20;
expected11–15 additional requests,30-minute absolute deadline and original tool
surface retained. Launch verifies sealed inputs/deadline before its exclusive
intent. Request-free connector/input readiness and same-primary-thread notice
binding pass; final notification continues G automatically. Python qualification
also completed18+112+11 tests; active Provider compiler cache is2.3GiB.

That remaining-case driver ended during input initialization at04:14:21Z with
0 development submissions/works/control calls. Read-only app inspection retains
the original input-client budget keyed only by the shared owner (12 dispatches,
old deadline), while the new plan proposed20/new deadline under the same key.
The bridge's immutable-budget guard therefore conflicts. The initialization's
specific error was overwritten by cleanup's input_client_owner_changed; preserve
that diagnostic limitation and the failed result, rather than claiming a raw
error receipt that was not retained. App input slot is absent, existing lanes
are settled and previous native work remains completed/stopped with no replay.

The corrected driver assigns a stable distinct input_client_id per batch root,
retains exclusive per-error evidence before the latest-error compatibility file,
and explicitly opens the next source after releasing direct input. Two offline
driver tests cover namespaces and all remaining-case source transitions; three
input-client tests cover title preservation and retained original errors. All
five pass. The next distinct intent is TMPDIR/gg.iprzgrte, same three fresh
projects and20-dispatch/30-minute ceiling; accepted MCP work remains excluded.
Unlike the preceding inspect-only check, its prelaunch proof initializes and
disposes the actual API input namespace with0 model requests before launch.

The gg.iprzgrte batch finished all three tasks without supervisor model calls
between them, using11 actual requests. Its final automated audit failed because
cli_final_visible did not render paired single-asterisk Markdown emphasis in the
direct final's regex; actual CLI content was retained. The corrected checker
preserves unmatched arithmetic, literal code and every cell/line;34 focused tests
pass, including negative altered-regex/arithmetic/code cases. The initial failing
negative test is retained in review-cli-tests.log; corrected proof is
review-cli-tests-2.log. No development or model request was repeated for this.
Independent review-1/review.json passes local/direct/general using original
CLI/native history, unchanged original tests, actual diffs and saved Chat.
Managed saved-final comparison also accounts for the existing presentation.rs
box projection, checked using the actual qualified Rust function; direct source
summary is compared semantically with its original work ID/marker/results, not
declared byte-identical. Source range completeness does not assert all hidden
content is readable.

| Installed route | Controller elapsed | Tests | Original calls/returns | Actual requests |
|---|---:|---:|---:|---:|
| Codex managed MCP |159.820s|8|8/8|1|
| Codex managed local |142.589s|7|7/7|8|
| Chat direct MCP |203.476s|7|5/5|1|
| Chat general to managed MCP |595.753s|10|8/8|2|

There are0 unreturned calls and28 returned native calls across accepted work.
The earlier stopped local attempt cost4 additional requests, so all G live
attempts consumed16 observed requests, not a measured subscription debit.
The failed zero-request initialization remains failed and original automated
results are not overwritten. Aggregate scope/input/evidence hashes are in
phase-g-complete-20261022T032904Z/installed-compositions-review.json. These are one
task per composition, not repeated benchmark or complete G/release acceptance.

The user selected a physical Android reboot on the current device after we prepare
job termination, preserved state and recovery instructions; the user will perform
the reboot. Preparation must account for this supervising session and existing
jobs before requesting that action. Other G work continues in the meantime.
Reboot preparation is retained privately outside TMPDIR at
HOME/.local/state/codex/g-reboot-20261022-oukenawz (pointer: g-reboot-current
in its parent). README.ko.md assigns shutdown, preservation and recovery to Codex;
the user's action is reboot after the final readiness notice. It records the
current batch, all-service/native reconciliation, scoped tunnel/service shutdown,
the shared Linux supervisor's verified graceful shutdown, final stopped-profile
and fixture archives, same-profile/same-thread resume and request-free readiness.
Preparation preserved297 working-tree source files plus binary diffs, primary
rollout prefix, private profile/auth/recovery metadata, consistent state/queue
SQLite backups and137 same-UID process identities. Artifact hashes, database
quick_check and the resume wrapper passed. This is an initial preparation copy,
not the final stopped-state archive or permission to reboot now; no signals or
Chat requests were submitted. The final batch result and ownership/jobs review
must precede shutdown and ready-to-reboot.json. Shared-host stop is deliberately
not delegated to Provider host stop (host_shared_owner).
Reboot settlement now confirms all four accepted G works fenced/stopped with
native Stop and0 owned jobs; managed host Stops confirm, while direct-current
host Stop was not requested because its source is separately released. The
earlier failed local work was already stopped. The owned connector alias is
disconnected. G/F/previous D3 Provider service exits are confirmed by their
pidfd-based stop commands. Both isolated G native servers were independently
checked for known idle threads and stopped through pinned kernel handles;
the supervising runtime was not signalled. Linux app/controller exit is likewise
kernel-confirmed after the verified owner's graceful SIGTERM; the attempted
Browser.close through F's constrained relay was refused, not an app-close receipt.
Its relay then exited with the upstream. Preserve original Stop/close/exit records
under the private reboot directory's settlement subdirectory. No model requests
or uncertain task replays were submitted during settlement. The AI launch wrappers
and supervising session remain protected; a user-approved reboot will end their
idle connection after this session's final handoff has been saved.
The private final preservation package is at the reboot directory's final/
subdirectory; its manifest binds each archive/database/rollout snapshot by digest
and restore root, with independent archived-file readback. The private resume.sh
checks preservation/installed-Core identity before opening profile szob and the
same thread01a15330-0d46-7b51-a460-5e0495030cf2. After reboot, first reconcile boot
identity, original records and surviving files/processes; then explicitly start
the preserved Linux profile, a new owned relay, the same immutable G Provider and
state/endpoint, and reconnect its alias. Never reuse an old PID/descriptor as
readiness or resubmit old work. Perform request-free readiness before a separately
declared bounded cold-continuation proof. Exact-source CI/public signing/readback/
disposable public consumer/non-forced promotion remain after lifecycle acceptance;
this reboot handoff is not whole G completion or device activation.
Device activation/default replacement remains separate and no
operating runtime or public asset has changed in this continuation yet.

### Agreed bridge structure and E work

**E is complete at its agreed design/common-idempotency-source scope.** Selected
contracts are in SPEC's E common-interface/lifecycle and idempotency sections.
Existing source and installed-app capabilities informed these choices; the source
candidate below is not combined-runtime or product acceptance.
F's selected implementation and bounded integration are complete; G
qualifies/packages/delivers the product and H retires superseded paths. The
[SPEC contract](SPEC.md#chatgpt-codex-bridge-architecture-and-work-boundaries)
distinguishes agreed goals from selected contracts and unqualified capabilities. No
new live trial follows from these decisions. The user has separately authorized
E source work; installed software and operating defaults remain unchanged.

For idempotency specifically, the user places design finalization and shared
source implementation in E, with actual qualification/application in F. The
preliminary source review below is not the settled design or a runtime change.

The optional Provider is the ChatGPT-Codex bridge: model adapter, tool adapter and
shared session/connection management. ChatGPT provides reasoning; upstream Codex
owns native execution, approvals, writers and canonical records. Core remains
independent of optional components. Existing `operator` code/config/schema names
remain compatibility labels. Logical roles do not mandate separate services.

The common managed path is native Codex work -> Linux app API -> managed work
Chat -> MCP or injected local tools -> the same native work. General ChatGPT
delegation uses this same path; direct ChatGPT work uses its current Chat for
reasoning. Current service-wide execution.mode does not provide combined routing.
Linux app removal is not the goal. Research its useful management capabilities
before designing additional wrappers, processes or per-path session logic.

#### User flows and design status

| User flow | Required outcome / agreed direction | E decision or missing product integration |
| --- | --- | --- |
| Codex development | Same native work receives ChatGPT reasoning and original tool results over repeated rounds | Unified start/attach/follow-up, MCP or explicit local choice, status and recovery contract; D2 legacy combinations are evidence, not the combined implementation |
| ChatGPT direct development | Current Chat controls native work through MCP without another primary work Chat | Reuse accepted two-turn scope; define shared interfaces and required client/account coverage |
| ChatGPT general development | Current Chat delegates into the common managed path and receives questions/results | Submit/attach/return contract, source/worker ownership and follow-up behavior |
| Shared conversation reading | Both entrances can find/select/read accessible managed, direct and unrelated Chats without inference or a native work binding | Selection, complete ranges/branches, source identity, unavailable content and common CLI/MCP-facing interface |
| Agents and orchestration | Assign work, exchange context/results and coordinate implementation/review/correction; address independent parallel work | Origin coordinator, native child/reviewer reuse, isolated parallel writes, serial integration and per-task/tree Stop implemented with bounded F proof above |
| Linux app/host management | Reuse authenticated session capabilities and simplify our code | One optional Provider host facade over existing Rust lifecycle, separate service/host/tunnel/native/viewer state, qualified at F's actual/synthetic scope; G retains broader lifecycle/physical checks |

The user postponed orchestration discussion until bridge design had been
considered, not implementation. The earlier assistant blanket deferral also
appeared in the repository-analysis prompt; that report's later-research label
is not independent evidence of a user decision. Keep the implementation goal and
resolve its concrete scope in E.

<a id="decisions-required-in-e"></a>
#### Settled E decisions

The settled decisions below are recorded in SPEC's
[selected interface/lifecycle contract](SPEC.md#e-selected-common-interfaces-and-lifecycle).
They define the implementation target without asserting existing product support.
App-version capability gaps remain explicit. The required F proof is reconciled
in the final review above; G retains product/lifecycle qualification. Update these
decisions when evidence requires it rather than creating a separate plan hierarchy.

| Subject | Settled E contract / required F implementation and proof |
| --- | --- |
| User entrances and work selection | One work service behind CLI/MCP; select by work/project identity and choose current-Chat or managed reasoning per work. Original question/progress/final return is bound once; general delegation uses the same managed path |
| Agent, task, thread and Chat identity | Stable work/native-thread/workspace; distinct role/assignment/parent/dependencies/Chat and generation. Successor receipt preserves assignment without equating an agent to one Chat |
| Coordination and review | Origin reasoning context coordinates by default; explicit additional coordinator only with its own cost. Required implementation -> independent review -> correction -> integrated acceptance; dependencies release after evaluated results |
| Parallel work and integration | Two independent assignments are the minimum F parallel scenario, with separate native threads/workspaces and declared write ownership; caller supplies finite concurrency/budget. Coordinator integrates reviewed diffs serially and handles conflicts |
| Reading and knowledge reuse | Shared CLI chats list/read and MCP codex_chats facade; account/source/branch/prefix-bound pages, stable IDs and explicit incomplete/unavailable content. Context import retains source provenance separately from instructions |
| Existing-Chat input and control transfer | Explicit selected settled same-account Chat may receive related input even if not bridge-created. Active owned work uses supported native steering or visible queuing; active unowned work is not silently adopted. Direct/managed transfer needs source coverage, reconciled effects, fencing and receipt |
| Context continuity | Saved managed Chat in an explicitly selected private project; reuse across related follow-ups. Source-authored grounded handoff only on explicit/verified boundary, not a turn counter. Assignment/successor receive instructions, selected public sources, confirmed native increments and unresolved state; detailed capacity policy stays separate |
| App/host management | Reuse Rust OwnedHost/CDP/Remote/AppHost/Pool/Broker; extend optional Provider CLI with host ensure/status/open/stop and share operations through MCP. Status cannot acquire a missing host; explicit ensure never downloads/logs in implicitly |
| State, questions and notification | Separate native/host freshness and waiting causes; unknown is not complete. Per-task Stop preserves unrelated tasks; whole-work Stop reports every child. Local event observation sends genuine questions and one acknowledged terminal notice without model supervision |
| Failure and durable records | Durable scoped intent/fingerprint/result, same-key coalescing, work-index recovery, completion resumption and virtual-round settlement intent. Reconcile existing effects without replay; reads and Stop retain fresh observations |
| Error reporting and history cost | Sanitized structured cause/capability/freshness; executable bridge with serialized data arguments. Preserve source-bound cursors while reusing immutable prefixes/paging new items; F measures long-history overhead |
| Migration and qualification | Keep legacy configs/routes/defaults and exact-input evidence. Explicit candidate/home/workspace/fixture/host/tunnel inputs; separate offline and live gates. New work-index format requires disposable migration/downgrade recovery proof before deployment; preserve unique work until H |
| Efficiency and budgets | Count all setup, coordinator/worker/reviewer/context, app/MCP/native calls, retries/failures and local/supervisor observations. Batch final audit; equal-task comparison and measured cost, without assumed allowance/billing gains |

#### Idempotency preliminary research and phased work

Preliminary read-only source research on 2026-10-21 inspected current Rust/JS
gateways, native-call and receipt paths, pinned upstream 0.163.0 source at
83086188ae0614bd882818ff1aa23f3194962012, installed app 26.1007.21434 assets and
the supported MCP ToolAnnotations definition. Preserved evidence is
~/.cache/codex/acceptance/idempotency-prestudy-20261021/analysis.json, with exact
source/asset hashes and four pinned upstream files. Original temporary source:
/data/data/com.termux/files/usr/tmp/idempotency-review.m9vjf_rn.
No implementation, annotation, test, service or model request changed in that study.

User constraint: all other annotation values are immutable, including
readOnlyHint=true, destructiveHint=false and openWorldHint=false. Review only
idempotentHint and the shared behavior it describes; do not reclassify the other
hints or require their alteration for product qualification. Current inspected
tools all declare idempotentHint=false.

Findings and limits:

- The installed app's MCP app-view path uses readOnlyHint OR idempotentHint for
  transport-closed/query retries. Its result is already true with the fixed values;
  changing idempotentHint alone does not change that branch. This does not reveal
  the unseen general ChatGPT model/backend dispatch policy.
- The reviewed upstream MCP parallel branch uses server opt-in or read_only_hint,
  not idempotentHint. Our gateway dispatch/dedup uses operation identity, payload
  and receipts rather than that annotation. The MCP definition also scopes this
  hint's significance to readOnlyHint=false; preserve the user's fixed settings.
- Existing Broker call identity coalesces/returns original native results. Reports,
  finish/handoff and session controls have different receipt/state handling. The
  study proposed read tools as true candidates, keyed execution/report/handoff as
  candidates after contract proof, and retaining false on mixed/stateful controls
  until their operation contract is settled. These are preliminary recommendations,
  not final selected values or evidence that hint changes improve throughput.

E settled logical operation/generation identity, same-key changed-payload
rejection, concurrent duplicate coalescing, durable intent/receipt and outcome
reconciliation, completion recovery and freshness/retention boundaries across all
tools, and implemented the common source logic below. Distinct intended operations may
have identical inputs; reads must remain fresh. Use relevant operation boundaries
instead of globally serializing unrelated work. A rejected duplicate alone is not
proof of non-idempotence, and an unknown effect is not permission to execute again.

F actually qualifies and applies the selected contract/hint values. Cover lost or
detached replies, simultaneous duplicates, same-key changed payload, new logical
operations with identical input, completion/post-processing failure, restart and
stale-generation reconciliation, fresh queries and unchanged other annotations.
Compare native effects, calls/waiters, retries, errors/unknown states and relevant
time/resources. Preserve old exact-input proof; run only declared necessary live
scope. Runtime deployment/default/device activation keep their separate gates.

Expected, not measured: a consistent common contract can reduce duplicate effects,
duplicate waits, avoidable retries and incomplete completion-state errors, and
their related overhead. It does not guarantee fewer network failures, shorter
model latency, lower total tokens or verified subscription debit. Merely changing
the hint is not that implementation.

Second independent ChatGPT review, retrieved and source-checked on 2026-10-21
without a new model request, identified a concrete completion-recovery gap:
the pre-E `operator.rs:527-532` returned an existing `codex_finish` receipt after checking the
native item, without the completion wait, Broker retirement and completed-phase
persist at lines 576-607. If the first call records the final item but completion
confirmation fails, the same-call retry need not settle a remaining `finishing`
state. This is a statically confirmed path, not a reproduced runtime failure or
proof of permanent irrecoverability. Existing qualification at
`scripts/qualify_provider_operator.py:172-174` checks a duplicate after successful
completion, not this failure sequence. Include safe resumption of completion
post-processing without re-emitting the final message in E's common contract and
source implementation; qualify delayed completion, lost replies and restart in F.
The second report and independent findings are preserved at
~/.cache/codex/acceptance/repository-review-second-20261021/audit.json.

E's common source now implements shared per-logical-operation in-flight
result coalescing and intent/fingerprint/result helpers in src/operation.rs. Session creation
and handoff reuse the helpers; optional keyed start/compact/claim/takeover, native
question answers and keyed native summaries have durable exact-retry handling.
Summary receipts preserve their admitted inference round and cannot answer a later
round. Legacy unkeyed input remains round-scoped; common clients supply call_id. Reads
remain fresh, Broker execution keeps its existing original-result coalescing and
Stop coalesces simultaneous observers, fences before draining admitted native
start/compact/claim/takeover/answer requests and does not queue behind report
completion. New admission and late native callbacks cannot overtake that fence. Unknown effects remain explicit
reconciliation errors, not a reason to create another call ID and execute again.

Finish first delivery and retries share native completion/job/Broker/phase
settlement, with generation/turn/controller checks. Work registration gains a v2
index with durable pending registration; first migration also discovers owned v1
work records omitted from the old index. Subsequent startup recovers a pending
registration without repeatedly scanning all transport receipts. Virtual MCP
rounds retain a durable pending settlement instead of rolling back memory alone;
exact result/catalog reconciliation commits the round without submitting to Chat
again. Reapplying an unchanged catalog tolerates already admitted next calls. Broker
validates before preparing a new lane intent, so known rejection does not poison
the lane; uncertain preparation/commit stays fenced. Receipt result settlement
preserves context. Settled registration also maintains a v1 downgrade mirror.

This is uncommitted source development, not a live fix or accepted new candidate.
The changed index requires disposable migration/restart/downgrade proof before
deployment. All current hint values, Core and installed defaults stay unchanged.
Source validation and remaining work are recorded under E status below; historical
Provider/full/Android/live successes do not qualify these changed inputs.

#### Linux app and host capability research

Research the whole management path, including reusable app APIs and session
lifecycle, before treating it as a new terminal command. Existing code is a source
of capabilities and constraints, not proof that its current packaging is optimal.
Inspect source and existing owner-bound state first without model requests;
recheck live ownership before any bounded runtime observation. No live owner is
restarted merely because a readiness check fails.

| Area | Existing implementation/evidence to inspect | E research output |
| --- | --- | --- |
| Authenticated app/API boundary | src/app_bridge.js identity, bindings, open/submit/snapshot/release and project operations; src/app.rs/cdp.rs and desktop_cdp.mjs | Capability/version/identity assumptions, one invocation and result/error contract, confirmed effects versus unknown outcomes |
| Saved Chat operations | C/C2/D2 saved source/successor readbacks, chat_entry_client.mjs, project/title helpers and repository-analysis read | Supported discovery/read/input/reuse/placement operations; gaps for arbitrary selected Chats, paging/branches and title ambiguity |
| App/host lifecycle | linux_desktop_trial.py, desktop_lifecycle.py, native_chromium_host.mjs, Rust runtime/host/driver paths | Current owner/lease/start/attach/close behavior, prerequisites and duplication; which shared mechanism replaces ad hoc setup |
| Provider/tunnel/native work | Rust runtime/operator/native controls and existing connector readiness/restoration helpers | Separate service, network, model and native-job state; reconnect/refresh/stop/reconciliation rules without resubmitting work |
| User management and viewer | open_dual_trial.py, existing reveal/view/status/stop and private viewer controls | A discoverable management interface and shared ownership path; opening a screen versus starting app/model/work; no name fixed yet |
| Configuration and dependencies | Existing Provider config, history/project settings and host/rootfs/Bun/noVNC locations | Single configuration authority, required versus optional assets, source/runtime compatibility and actionable diagnostics; signed packaging remains G |

Record verified operations, exact inputs and unknowns. Reuse existing owned login
and profiles; keep credentials and private capabilities out of diagnostic output.
Routine management should use available backend operations; physical viewer/login
interactions retain their actual scope. Browser/DOM replacement and Linux app
removal are not automatic conclusions of this research.

Source capability findings on 2026-10-21 (no live host mutation or model request):

| Capability | Evidence and selected reuse | Remaining qualification |
| --- | --- | --- |
| Account/app connection | app_bridge.js init/identity binds account/user/structure and bridge revision; Remote/CDP and desktop_cdp.mjs validate owner generations | Expired login/account switch, version change and shared-host attachment; discovery's limited DOM scope lookup is distinct from UI-driven development |
| List/search/project filtering | Installed app-initial asset uses GET /conversations (offset/limit/order), /conversations/search (cursor/query) and /gizmos/{gizmo_id}/conversations (cursor/limit) | Common facade, duplicate titles, account/access changes and stable source identity; static routes alone do not prove current endpoint availability |
| Saved Chat completeness | App uses GET /conversation/{conversation_id}, followed when needed by /conversations/{conversation_id}/messages with before/num_turns/include_has_versions; checks nonadvancing cursors | Do not assume initial mapping is full history. F covers multi-page branches, selected ranges, unsupported public content, truncation and source agreement from both entrances |
| Chat input/branch/Stop | Existing prepareCompletionStream/startCompletionStream, app lane submit and installed /conversation/new_branch and /stop_conversation clients | Arbitrary explicitly selected settled old Chat input and branch preservation are source-supported targets, not live-qualified behavior; no silent active-stream adoption |
| Projects/titles | Existing Rust/project bridge lists/reads/creates; D3/chat-entry helpers rename/place saved Chats via APIs with readback | Reuse wire/data contracts rather than trial globals. Preserve project compatibility and metadata-only mapping invariance |
| Host/service lifecycle | Rust OwnedHost owns process groups/display lease/CDP pipe; Remote accepts verified relay or pipe; runtime ensure_token refuses to restart a live-unready owner. Development desktop_lifecycle also checks PID/start/image and keyed readiness | Consolidate production policy in Rust, explicitly attach/reuse an external owner without claiming shutdown rights; verify host-less status never starts LazyRemote and scoped Stop preserves shared host |
| Screen and network | Existing view/reveal/open_dual_trial resolve current owned endpoints; connector/tunnel helpers separately restore network discovery | Opening an existing screen is not app/model/work startup. Reconnect observes original receipts, never resubmits them; viewer usability stays separate |

Installed app 26.1007.21434 was inspected directly from app.asar. SHA256:
app-initial-ed5807835d97.js =
a9f4a1a86c7486b97b750b97296d83c8c8c67074144480b697244f0c8b6ef720;
app-shared-2d77298f0100.js =
2ebc161f0135eb7c78f14505687cf3e2aa6ed4570570061860886d4aa5de8dd0.
Exact route contexts/hashes are retained in
~/.cache/codex/acceptance/phase-e-first-source-20261021/app-capabilities-static.json
(original temporary root /data/data/com.termux/files/usr/tmp/phase-e-design-pps51k09).
No current live readiness, permissions or inference/billing outcome is inferred
from this static inspection.

#### Agent and orchestration design

An agent is a logical reasoning role with assigned work/context; a Chat is a
reasoning session; a native thread owns execution/history. E selected their
mapping and lifetime without assuming one Chat equals one agent. An agent may
continue through a confirmed successor, but that does not establish arbitrary
old-Chat input or automatic takeover.

Design concrete flows such as assignment -> implementation -> independent review
-> correction -> integrated result, including questions, partial failure and Stop.
Define which independent tasks run in parallel and how their writes/results are
isolated and integrated. ChatGPT or Codex may be the coordination entrance; choose
the reasoning/controller placement and count its cost explicitly. The bridge's
deterministic transport/lifecycle is not an additional planning model. Reuse native
task/child/reviewer execution and canonical records rather than duplicating them.
The selected workflow uses the origin coordinator, implementation/review/
correction assignments, caller-bounded parallelism, isolated writes and serial
integration as recorded in SPEC. No fixed agent roster is imposed. F implements
these workflows and qualifies actual
coordinated behavior; C/C2 continuity and D3 prepared loops are building blocks,
not end-to-end multi-agent quality/cost proof.

#### Shared reading: agreed goal and bounded evidence

Shared saved-Chat reading is an F completion requirement. E determines its product
surface and source/range contract. Both entrances must use common retrieval for
accessible managed, direct and unrelated Chats, preserving message order, source
and branch, paging the complete selected public range and reporting missing or
unavailable content. Retrieval makes no model request, alters no source and does
not itself submit input, resume work or transfer control. F must compare actual
both-entrance reads with saved sources, including access and completeness cases.

On 2026-10-21 the app API read the repository-analysis Chat's public final report
(15,443 characters, completion marker, zero new model requests), using a temporary
script corrected after two local string-construction failures. Evidence:
/data/data/com.termux/files/usr/tmp/repo-review.g85k4uc2/saved-report-private.json.
This proves one saved-report read, not a reusable interface, arbitrary Chat input
or complete multi-agent exchange. Preserve it without repeating the analysis.

#### E completion and subsequent work

The selected contracts have the following concrete F acceptance. Use isolated
roots and explicitly supplied inputs; this matrix authorizes no live batch by
itself and does not request repetition of unchanged accepted trials.

| Boundary | Required observable scenario and acceptance |
| --- | --- |
| F1-routes: combined entrances | One service supports Codex managed, current-Chat direct and current-Chat general delegation by work selection. Related follow-up stays on the same native work; a real native question and final return reach the original caller. Concurrent independent work cannot cross controllers, results or workspaces |
| F2-chat-input: existing Chat/continuity | Explicit input to a selected settled old Chat preserves source/branch/draft; an active unowned Chat is reported unavailable for adoption. Grounded managed successor and direct/managed transfer fence the source, confirm receipt and continue the same native work without retransmitting uncertain calls |
| F3-reading: shared reading | Both entrances read the same selected unrelated and work-linked sources, including duplicate titles, more than one history page, alternate branches and unavailable/unsupported content. Compare ordered public IDs/content/ranges, explicit capability redactions and completeness with saved source; zero retrieval model requests and no input/control side effect |
| F4-serial-agents: serial collaboration | Implementation -> independent review with a concrete finding -> correction -> actual integrated acceptance; native child/reviewer records and saved Chat sources identify each role/result/question. Completion requires evaluated corrections, not only a coordinator summary |
| F5-parallel-agents: parallel collaboration | Two isolated writers, dependent review/integration, one deliberate conflict and task-specific Stop while the other continues. Whole-work Stop accounts for descendants/jobs; no unknown child is counted successful and no concurrent writes touch the user's tree |
| F6-management: management | Absent-host status makes zero app starts/model requests; explicit ensure/reuse proves ownership. Open/reveal does not submit work. Expired login, changed app bindings, connector refresh and live-but-unready host show actionable state without restart or replay; scoped Stop preserves shared owners |
| F7-idempotency: idempotency | Simultaneous exact duplicates; changed payload under one ID; a new ID with identical input; reply loss before/after native/Broker commit; finish confirmation timeout then retry; restart with pending creation/index/round/summary/answer; stale generation. Compare native effects and receipts, not just RPC success. Reads stay fresh and Stop/independent work remain responsive; all other hints unchanged |
| F8-history-inputs: history/input reuse | Long native/saved histories, appended pages and changed prefixes keep source coverage and completeness while measuring costs. Explicit candidate/home/workspace/fixture/tunnel inputs work outside historical roots. Source CI covers changed helpers without starting paid/live trials |
| F9-efficiency: equivalent efficiency | Equal task/input/model/effort/approval/context and declared route differences; retain UTC/monotonic start/end, per-turn/whole-run times, app starts/responses, original call/results, setup/failure/retry/context/coordinator/reviewer and monitoring costs. Actual billing remains unknown until independently observable |

**E status: complete, 2026-10-21.** No required user decision is pending.
The settled contract and source scope have been checked against every E exit item:

| E exit item | Completed output / evidence boundary |
| --- | --- |
| User flows, common roles and control | SPEC selected interfaces: three entrances, stable work/assignment/source mapping, one execution controller, original return, explicit transfer, compatible legacy defaults; F integrates the handlers |
| Reading/input/context and agents | Account/branch/prefix-complete reading and explicit old-Chat input; origin coordinator, implementation/review/correction, isolated parallel writers, serial integration and per-task/tree Stop; F implements/proves those workflows |
| Linux app/host research and selected management | Source plus installed26.1007.21434 API/hash findings, shared Rust ownership/lifecycle and per-lane transport design; web/Linux client coverage and unavailable capabilities are explicit, without claiming current live readiness |
| Common idempotency implementation | Shared in-flight result/intent/receipt logic, completion resumption, keyed session/answer/summary/handoff handling, per-work native admission/Stop fencing, recoverable v2 registration with settled v1 mirror and prepared virtual-round settlement |
| Per-tool policy and compatibility | At E exit, SPEC selected true targets for fresh reads and established keyed controls, with mixed legacy host/session/summary/Stop surfaces false; declarations were then unchanged. F subsequently qualified/applied Rust12 true/4 false as reviewed above. Other hints remain fixed. No extra execution.mode is introduced; ambiguous effects are not replayed |
| Reusable F inputs and acceptance | Versioned qualification.schema.json, explicit candidate/source/Core/frontend/fixture/config/ownership/budget/return fields, F1–F9 cases and full result/cost contract. Model-free example has real file references and all inference/native-call budgets zero; it launches no F case |
| Source review and compilation | Final all-target cargo check/Clippy (-D warnings), fmt, docs whitespace/JSON syntax and release build pass. Existing test targets compile; no test function, native development call or new Chat request ran |

The source-only AArch64 ELF64 candidate is
22e2d492ea37fc7b7bf896a5ba49794012bb90449160cc53061589a0390267df
(9,312,008 bytes). Complete source/data/helper inputs, compiler logs, candidate,
model-free F input example and gate record are retained at
~/.cache/codex/acceptance/phase-e-complete-20261021/e-completion.json.
Original preparation root: /data/data/com.termux/files/usr/tmp/e-completion-2988jz0r;
short native fixture /data/data/com.termux/files/usr/tmp/ef.d13yi7de (101-byte
prospective socket path), with no server/thread/turn created. The earlier
phase-e-first-source-20261021 record remains historical partial-source/static
proof; its 31-file list omitted embedded data/other inputs and is not the final
candidate's complete input inventory.

E completion covers design, shared source implementation and compilation. New
runtime/fault/restart/Android acceptance, application of target hint values and
combined product behavior remain F/G gates. Installed software, account/profile,
active jobs and operating defaults were not changed; no new public release or
activation occurred. F implementation and bounded qualification are now complete as reviewed above. Its implementation order and prepared
v1-input contract are specified in SPEC; do not repeat completed C/D trials just
because E changed source.

E is complete when the agreed required workflows, shared responsibilities and
identity/control model, app-management capability findings, feature/compatibility
choices and concrete F acceptance are recorded in SPEC/CURRENT, and the specifically
authorized E idempotency source implementation is complete. Resolve decisions
that affect the next implementation; any later item needs an explicit reason and
retained user goal, not a blanket future label. Do not invent fixed agent policies,
command names or process splits merely to make the design appear complete.

F then implements the selected common path, reading, management and coordinated
workflows, verifies changed paths locally first and runs only necessary declared
live scope with final batch review. Compare equal development contracts and all
costs, including failed/preliminary work, while retaining valid exact-input proof.
G completes product/lifecycle, composition, signed delivery and separate activation
boundaries; H preserves unique content before retiring superseded mechanisms.

<a id="f-implementation-and-qualification-in-progress"></a>
#### F implementation and qualification

The user authorized the whole F phase after E completion, without requiring
permission after each feature or trial turn. F1–F9 bounded integration is now
accepted in the final review above and Rust target hints are applied. The
following exact-input records retain intermediate failures and their limited
verdicts; they do not replace that final matrix. The first live F bundle
submitted one model request and then failed before native effects; its audit and
source fixes are below. G publication/default/device activation remains separate.

Current uncommitted F source adds per-lane MCP/local declarations behind one
renderer hook owner; per-work current/managed reasoning and scoped native Pools;
the candidate `work/chats/host` CLI over the existing MCP control service; source-
bound model-free saved-Chat reading with capability redaction; host observation
without acquisition; managed-worker/coordinator fencing; and assignment roles,
dependencies, bounded admission/queueing, evaluation and tree Stop. These are
implementation changes in progress, not combined-runtime proof. The candidate
also adds explicit saved-Chat input/result receipts, bounded local event waiting
and two-step completed-boundary route transfer. A source-bound input reserves
one stable user-message UUID before submission; a lost reply or restart observes
saved original message/parent/final descendants without resend. These additions
remain unaccepted until affected-path actual qualification. Complete agent
workflows, durable budgets/recovery, synthetic/actual acceptance and equivalent
efficiency evidence remain required.

Evidence root:
/data/data/com.termux/files/home/.cache/codex/acceptance/phase-f-integration-20261021T050403Z.
The initial Provider regression failed on a CDP test expecting identity reads to
initialize writer hooks; another obsolete four-request fixture was terminated
while waiting for the removed initialization request. Preserve that failed log.
The corrected CDP suite passes all six tests. Model-free app observation verifies
the existing session/host/browser owner, authenticated context and current shim
capability, without installing hooks, allocating a Chat, restarting the host or
making inference. `host-readonly-preflight.json` contains sanitized evidence.
Earlier unit/Bun checks are scoped source evidence, not the full F gate.

The second Provider regression passed on its retained source snapshot and
6528a057 release candidate. Subsequent model-free `read-retry-2` preparation
compared CLI and MCP: two returned ranges covered134 source messages, including
13 hidden/unsupported placeholders, in5.437s with zero model/native execution
calls and no native work created. The normal saved-conversation GET succeeded;
an unsupported full-conversation query caused403 and was removed. This proves
that source's read path, not all F3/F8 cases. Later DTO/cursor/input/transfer
changes require their own source-qualified evidence.

Regression3 passed its executed Rust tests, then failed on a CLI Clippy
single-pattern match. After correction, regression4 passed the full Provider
gate on164 retained inputs; release candidate96f4d312 and its immutable
`operator_native` test executable are retained together. `read-retry-3` verified
that candidate's CLI/MCP equality on134 messages/13 unavailable placeholders in
5.005s with zero model/native execution calls. This is preparation evidence,
not whole F3/F8 acceptance.

The explicit synthetic1 run failed before Core start because its immutable
source inventory lacked permission-model.json. The input consumer now rejects
that transitive omission before launch. Synthetic2 exercised actual native
commands, exact retry, independent automatic approval, same-thread handoff,
host waits, native compaction and queued-follow-up Stop; it then exposed a real
Stop-reporting defect: absent host was incorrectly called confirmed. The source
now distinguishes absence from confirmation while preserving a previously
confirmed/released Stop. Regression5 passed; synthetic3 then exposed a second
actual defect: an explicitly started follow-up reused a retired Broker token.
Its source fencing/receipt passed, but subsequent development timed out. Source
now rotates the token at that new generation. Regression6 passed on165 inputs;
candidatee5b4cb2c and its exact test executable passed synthetic4 in52.448s.
That run verified actual native same-thread successor development after receipt,
old-controller fencing, bounded local wait, two distinct native writers in
isolated workspaces, per-task Stop preserving its sibling, evaluated dependency
and four-work tree Stop, plus the retained legacy/native fault cases. It submitted
zero actual Chat requests. This is native/control integration and a bounded F7
subset, not actual Chat reasoning/independent-agent quality or whole F1–F9
acceptance. Preserve all failed runs and their disposable roots.

Regression7 is quarantined: two source files changed while its working-tree
checks ran, so its zero exit is not exact-source qualification. Regression8 failed
before building because the frozen inventory omitted embedded assets. The E
125-file snapshot and earlier partial inventories are likewise not self-contained
build inputs; retain their historical design/static evidence without widening it.
Regression9 ran all seven Provider gates inside an immutable174-file source root,
including embedded text/CSS/HTML and transitive qualification fixtures. Its source
hashes remained unchanged and candidate
df4d126504a417621e2ad4f0b7a34d47441c394bb1d5ceeaa11827b9d2a429e1
passed synthetic5 on Android in60.212s. That retains the native/control scope
above and made zero actual Chat requests.

Subsequent source preserves saved conversation IDs and successor lineage in the
lane journal, fences managed dispatch against work Stop, shares managed app
approval/reveal, releases only freshly confirmed idle/stopped owned lanes on
shutdown, and makes passive cleanup incapable of installing hooks. A focused
28-test app-bridge run passed after correcting an overbroad zero-dispatch Stop
condition: asynchronous startup failure remains uncertain. Regression10 passed all seven Provider gates from a frozen174-file source root
in235.811s, with unchanged hashes. Its candidate is
e98dbef4537faa4d932ab79d7d2982810f2533685d4ff731b32cb7935a4f16a4.
The corresponding native/control synthetic6 run also passed, including idle
shutdown and selected receipt/writer/dependency/Stop integration, with zero Chat.
Its first preflight rejected old HOME/workspace paths before execution; retain
that failed input/log as well as the corrected explicit recipe.

A private saved project, `Termux Provider: F`, and isolated native/home/workspaces
are prepared at /data/data/com.termux/files/usr/tmp/f-live.djzsl5pm. Project setup
used existing app APIs, with zero model requests and no screen automation. The
first live bundle subsequently submitted one request, as recorded below. The candidate service and
trial-tunnel route are prepared; fresh old-service checks found zero pending
calls/questions and idle/not-loaded native work. The old service remains alive.
Connector metadata/cache refresh and actual MCP16-tool/read-only readiness pass.

The existing Linux relay admits only one CDP connection. Rust observation passed,
then the separate qualification input client failed to connect. A bounded private
qualification relay now shares that one upstream socket between the Provider and
input client, without restarting/stopping the Linux app. It authenticates local
peers, correlates duplicate request IDs independently and isolates attached
sessions; its seven-assertion mock proof and actual zero-model shared-source
init/release/reopen/dispose preflight pass. No relay Browser.close/lifecycle action
is forwarded. This is declared test transport/setup cost, not a production Core
dependency or acceptance of concurrent independent app owners.

The sealed first live bundle covers matched managed MCP/local development,
current-Chat MCP development, same-Chat/same-native related follow-up via shared
chat_input/result and Chat-origin managed MCP delegation. It expects five saved
Chats, six genuine user submissions and four native works; upper limits are
60 app starts/model responses,400 native calls,2 parallel and2400s. Inputs,
candidate/config/native fixture, source/account/app binding references and the
same-session receiver are explicit v1 inputs. The local runner performs protocol
waits/original approval replies, records per-task UTC/monotonic times, captures
original CLI and performs diff/test/native/saved checks only after all tasks.
Uncertain effects are never resubmitted; failure requests scoped Stop of owned
work/source and preserves unconfirmed outcomes. It sends one final F notice.
This is a subset of the F matrix: independent review/correction, live conflict/
parallel/Stop, transfer/successor, full idempotency fault/migration/hint application,
long-history and equivalent-cost conclusions remain required. No F1–F9 case is
accepted merely by preparing or completing this first bundle; actual billing is
unknown. The F probe was physically received/acknowledged in a new turn of the same
primary thread01a15330-0d46-7b51-a460-5e0495030cf2, followed by one exclusive
launch. The final failure notice also returned to that thread and is confirmed
against its canonical rollout in final-notice-receiver-proof.json. No root model
polling or mid-run intervention occurred. The supplemental runner Python/CI
checks and private relay assertions passed separately from runtime candidate10.

**First live bundle: failed and independently audited; do not restart/replay.**
The runner ran08:19:25.185–08:19:32.208 UTC (7.022833s). Its first
managed-MCP native work started, but approve() treated host_observation:null as
an object and raised AttributeError. Completed tasks0; four-way route comparison,
follow-up and delegation did not run. Native history has one original user item,
zero native tool effects/final answers/pending calls/questions, and zero remaining
owned jobs. The exact frozen task matches both native and saved user records.
Original CLI shows interruption; actual workspace diff is empty and original tests
are unchanged. Final-only audit is independent-failure-audit.json; original
result.json, sealed driver/candidate and failure artifacts remain unchanged.

One gpt-6-thinking/extended app request and one saved Chat were observed:
6ad87592-facc-83ec-b941-d9b4dbee68f5 in Termux Provider: F. The saved public range
has9 records,4 hidden/unavailable placeholders and two later native_catalog call
attempts; it contains no successful native development. Actual billing is unknown.
The local runner's host_stop_confirmed:true was false evidence of server Stop:
the saved final is48.163507s after runner end. Existing eHt/JU cancels the local
transport and does not call stopCompletion -> /stop_conversation. Native admission
was fenced, so later source calls produced no native effects. A subsequent
read-only /conversation/{conversation_id}/stream_status returned COMPLETE;
that current terminal state does not retroactively validate the earlier Stop.

Working source now handles the null observation, retains a private exception
traceback for future failures and tests a complete synthetic managed runner cycle.
The app shim distinguishes local cancellation from server termination. For active
known conversations it uses the installed client's stopCompletion and a fresh
matching-conversation COMPLETE/FAILURE status; rejected/lost replies and missing
identity remain unconfirmed, no uncertain Stop is replayed, and release refuses
an unconfirmed stopped round. Normal completion before local cancellation and
settled zero-dispatch rejection keep their valid local proof. Public SPEC records
this distinction. Offline proof:34 Bun tests/405 assertions and18 Python tests,
plus docs/diff checks. These are source/mock corrections, not corrected live Stop
proof. A later fixed-source release candidate and its source gate are recorded
below. At that audit the trial service still used the unchanged e98dbef4
candidate. Its later verified-idle replacement is recorded below; no
hint/default/public/device activation changed.

A full corrected-source gate also exposed inherited-descriptor lock retention.
Regression11 failed an app-engine immediate-reopen check; regression12 then
exposed the same close-only lease defect in configuration update. Both frozen
failed inputs/logs remain retained. A local FD-duplication reproduction confirms
that close-only cleanup falsely retains the old flock. Journal and configuration
now share one owner-PID lease guard: only the actual ending writer explicitly
unlocks; read-only inspectors and inherited child objects cannot unlock a live
parent. Dedicated duplicate-descriptor and reader/child ownership regressions are
included. Regression13 passed all seven required Provider gates from a frozen
182-file source in282.171s, including all-target tests/Clippy, release, Bun and
Python qualification/notification/runner/CI checks; source hashes remained
unchanged. The retained candidate is
c33b064dddf93860302af996f3a9c272e60beffdc0058f84fef71cbfabb258bb,
with its exact source/input manifest and operator_native test executable in
provider-regression-13-{source,inputs.json,candidate,result.json}. This is a
source/test/release-build gate, not actual corrected Android native/app Stop/
restart/migration acceptance and not selected-hint application.
In particular, historical stopped/released records require their own provenance
and migration proof; a recorded legacy client acknowledgment alone is not fresh
server-stop evidence.

The original result notice forbade replay/new trials; its latter wording was
overbroad for the user's continuing whole-F authorization. Preserve that failed
run and never resend uncertain effects. After reconciling ended/preserved work,
continue fixes and separately sealed qualification launches under the existing
authorization. Do not treat an automatic failure notice as cancellation of F.
The user need not repeatedly request continuation. At this candidate13 checkpoint,
F remained active with the remaining cases above. Candidate13 also passed actual Android
native/control synthetic7 with zero Chat requests, including same-thread receipt
transfer, isolated writers, dependency and scoped/tree Stop. This still does not
qualify the corrected real Chat server Stop or whole F.

The completion notifier accepts an explicitly bound F label, retaining one-shot
intent, source-session/receiving-turn acknowledgment and legacy D3 behavior.
Actual F probe and final delivery were proved above. A new binding after profile
or notifier changes requires a fresh receiving-turn acknowledgment before detach.
Result notices now preserve whole-phase authorization while forbidding uncertain
call replay. The changed template, runner and input-consumer tests pass (20 tests,
zero Chat), including actual-null DTO handling and required Stop evidence.

The corrected live bundle is separately sealed at
/data/data/com.termux/files/usr/tmp/f-live.kpqtk7z6 (29 files/references), using
candidate13 and its immutable source plus separately hashed qualification helpers.
The first failed run remains unchanged. Fresh native state had no pending calls,
questions or running work; app lanes were inactive and their saved server status
was terminal before replacing only that owned test service. The shared Linux app,
original older service, credentials and saved project were preserved. Setup
corrected two helper assumptions (host-status envelope and missing explicit relay
descriptor) before any model request; failed preparation evidence is retained.
The corrected service, existing private project, shared-source lifecycle, exact
16-tool connector refresh and actual read-only MCP readiness pass with zero Chat.
Native fixture/catalog and current-profile same-thread receiver binding are
explicit. The fresh probe was physically received and acknowledged in a new turn
of the same primary thread (current wrlab profile), then this sealed bundle was
launched once as PID28077. The runner ended09:12:28.816 UTC after16.956525s
(start09:12:11.860), before any completed task. Its first codex_status immediately
after the Stop-probe native start was rejected; the old helper did not retain
that exact error response. Do not invent its lost error detail. Final independent
audit finds one original native user item, zero tool effects/changed paths,
unchanged original tests,4,508 bytes of original CLI, zero observed app requests
and no saved source Chat. Native Stop is confirmed with zero jobs; app Stop is
unconfirmed (no admitted app dispatch/active lane), not a successful server-Stop
test. The final notice physically returned in a new turn of the same primary
thread; no supervisory model observed or intervened during the run. Original
sealed inputs/result and independent-failure-audit.json remain preserved.

Model-free direct Core reproduction found an additional concrete defect:
includeTurns=true on a newly allocated persistent thread fails with
`list_turns is not supported yet`, while includeTurns=false returns idle. This
is consistent with the initial managed-status failure, whose original response
was lost; it does not prove the precise historical error. Managed status now
reads lightweight state first and requests full turns only at an otherwise
settled completion boundary. Actual synthetic qualification adds managed status
before the first native turn (zero app requests). Future F control responses are
privately captured before classification so rejected original RPCs are retained.
Frozen gate14 passed all seven Provider gates in284.808s on unchanged182-file
inputs (status correction). Before application, the same empty-history issue was
also removed from Stop of an already idle, never-started thread: it observes
lightweight state and remaining jobs without demanding a nonexistent turn list;
active threads still require actual turn interruption and terminal observation.
Frozen gate15 passed all seven gates in282.994s on unchanged182-file inputs,
including the retained rejection-response test. Its candidate is
29058149049c13e482b406a4313e555b1e28f487ce90abf93586a318d5a22669;
source/input/test executable are in provider-regression-15-{source,inputs.json,
candidate,result.json}. Actual Android synthetic8 now includes managed status
and Stop before the first native turn, alongside previous selected-control proof.
Synthetic8 failed that added case before inference: its project-free recipe
correctly rejected managed creation with managed_project_required. Preserve the
failed fixture/log; this was an incomplete test recipe, not live model failure.
Synthetic9 uses the exact candidate15/test executable with a separately hashed
corrected Python task driver: project-free recipes prove that rejection and
explicitly do not claim managed startup coverage. All retained actual Android
native/control cases pass, including same-thread receipt transfer, isolated
writers, dependent work and tree Stop, with zero app submissions. No actual
Chat server-Stop acceptance or hint application is inferred from this proof.

The new separately sealed live root is
/data/data/com.termux/files/usr/tmp/f-live.llw6kt6a, candidate15. Before launch,
its actual project-configured service creates one managed work without starting
a native turn: lightweight status returns idle/no app lane and Stop confirms no
jobs, without requesting an app Stop or inference. That concrete regression is
recorded in managed-empty-native-preflight-private.json and excluded from
development-success counts (6 total native works expected: this setup work plus
5 experiment works;6 saved Chats and7 genuine user submissions unchanged).
The first failed runs and their source/native state remain preserved.

Prior test-service Stop returned provider_stop_unconfirmed; subsequent fresh
observation found its owner absent and listener closed. The Stop was not resent
or retrospectively relabeled confirmed. That reconciliation permitted the new
state namespace; retain its shutdown timeout as unfinished F6 evidence. Shared
Linux app/older service/user state remain preserved. Preparation-helper envelope
and final-bookkeeping errors were reconciled by fresh ownership/route readback,
without replaying connection changes or inference. Actual16-tool MCP refresh,
read-only readiness and the corrected shared-source lifecycle pass.

All29 live input references verify. The already physically proved receiver is
reused only after matching current thread/profile/environment/Core/notifier
hashes; receiver-binding-reuse.json cites the original acknowledgment and final
receipt and does not claim a new probe. The batch was launched once as PID14733.
It added no supervisory monitoring between tasks; one final notice physically
returned to this primary thread. The batch ended09:48:13.915 UTC, with an actual
server Stop proved in35.335s, managed MCP development completed in198.648s and
managed local development completed in266.657s (monotonic task durations). Direct
Chat MCP development also completed with a canonical native final and a saved
public final, but its saved/title helper failed before follow-up/delegation.
Original result/sealed runner remain unchanged; exact launch-time external helper
bytes are preserved with matching hashes in review-1/launch-helper-sources.json.

Independent terminal-batch review (review-1/result-reviewed.json) verifies all
three actual diffs, full suites12/12/9, added files, unchanged original tests and
independent integer/CLI contracts. Every native final's rendered text/table cells
are present in order in the original CLI; byte-matching Markdown table delimiters
was an incorrect qualification predicate, now separately regression-tested.
Direct Chat's public final uses different wording from codex_finish but correctly
identifies the native work/thread, original-test digest,9 tests and implementation.
Managed final text is also present in its saved public source. No private reasoning
is read or exported. All native works are now idle, with zero pending calls,
questions or remaining jobs; failed-batch cleanup fenced completed work without
undoing its actual files or records. Saved graphs retain hidden/unavailable
placeholders rather than claiming complete private content.

The lost browser exception cannot be reconstructed from its generic historical
error. The title mutation is now positively reconciled by fresh reads; it was not
resent. Helper source now retains private browser exception/readback detail and
hashes all graph values canonically instead of depending on object-member order.
Two fresh reads of the original direct Chat match title/project/content hash with
zero model requests. The input client's finite budget has an explicit per-client
binding and stable admitted absolute deadline across shared-owner reopen; the
zero-request ledger/deadline was read back and reused, not reset. These are
qualification-helper changes; runtime candidate15 and all annotations remain
unchanged.22 Python tests, Bun syntax and actual model-free reopen/read proof pass.

Continuation /data/data/com.termux/files/usr/tmp/f-live.llw6kt6a/recovery-1 launched
once as PID25425 and ended after4.704s, before adoption or model dispatch, with
app_chat_input_unavailable. independent-failure-audit.json verifies zero app
requests, absent durable saved-input admission, unchanged native history and
unchanged saved cursor. General delegation never started. This is a known
pre-dispatch rejection, not an uncertain effect to replay. All199 sealed input
files and original failure evidence remain preserved.

Actual installed-app exports identify pzn as the activity predicate and ozn as
the per-conversation activity string; the former guessed aliases were wrong.
The shim now uses these actual exports and requires the capability before a
new live batch. Model-free binding-preflight-private.json reads all required
capabilities as available and zero active lanes. Mock coverage includes each
active status and missing capabilities; actual adoption still needs qualification.

The old current-reasoning work has also exceeded its original elapsed limit.
SPEC and source now support explicit settled claim with unchanged non-time caps,
exact history/context, retained start/counters and the same native work/thread.
A managed-work elapsed extension is refused until its app ledger is reconciled.
Frozen gates16/17 retained ETXTBSY failures in copied test executable launch;
the fixture retries only that pre-exec error for at most3s, without retrying any
started child or RPC. Gate18 passes all seven corrected-source gates in435.987s on unchanged182-file
inputs, candidate b8494582677cbdba82b5b65921f6bbfdd7c70fca128b41cdc81ea8bd91e40bd8.
Actual Android synthetic10 also passes both entrances, elapsed extension retaining
the original start/calls/native thread, receipt transfer, isolated writers,
dependencies and tree Stop with zero Chat requests. This synthetic fixture has
no configured project; managed pre-first-turn status retains the separate parent
live setup proof rather than claiming that skipped synthetic branch passed.

The settled owned service was stopped once with confirmed owner absence and
closed listener, then replaced with candidate18 on the SAME state/native roots.
Shared Linux app and unrelated prior service remain intact. All16 actual MCP
tools refresh/discovery/read readiness pass. Actual model-free saved-Chat
adoption/release now succeeds, preserving source node/count/title. A preparation
helper incorrectly assumed ensure had status's response envelope; fresh status
reconciled its completed effect without repeating ensure/service startup.
Opening a previously released bridge also exposed an input observer installation
order error; the helper now opens before installing its passive observer. Its
zero-request partial initialization was explicitly reconciled and preserved in
prelaunch-revision-0; real model-free reopen/read/dispose then passes.

The separately sealed continuation recovery-2 retains199 verified inputs and
the original direct Chat and
native thread01a15dd9-9520-70e3-861d-ba4aaf9438a3, then runs the previously
unstarted general delegation. Completed Stop/MCP/local/direct tasks are not
resubmitted; completed artifacts are reused only if their hashes match. Planned
new inputs are three genuine submissions/two saved Chats/one native work, bounded
by30 app dispatches,200 native calls and2400s. Prior app dispatches were14
(Stop1, managed MCP1, managed local11, direct1), continuation1 added zero; these
transport counts do not establish billed model iterations or subscription debits.
The same physically verified final-notice route is reused after current binding
checks. Preparation completes with zero model requests; launch/result status is
recorded under /data/data/com.termux/files/usr/tmp/f-live.llw6kt6a/recovery-2. At this checkpoint, whole F and remaining F1–F9 cases continued under existing authorization;
this subset alone was not whole-phase completion.

Continuation recovery-2 launched once as PID21381, then failed after16.229s
with native_request_rejected while its runner scanned the historical turn-less
preflight work during a current follow-up. Its source request was delivered:
1 app dispatch, a public acknowledgment and no new native call. Final independent
review at recovery-2/review-1/audit-reconciled.json verifies native history AND
context cursor unchanged, preserved original tests,9/9 tests and unchanged
completed sibling artifacts. General delegation never started. The original
result, uncertain Stop replies and failed chat_result receipts remain unchanged.

Fresh same-account server observation subsequently confirms COMPLETE and the
saved input lane's stopped state. The owned lane was released and its namespace
disposed without resending the input or Stop; see source-terminal-reconciliation.
The delivered input is not reclassified as unstarted. Its exact-parent readback
also exposes an app normalization boundary: a hidden final terminal was the saved
current node, but the installed app attached the user input to its public final
ancestor. Future inputs explicitly select a public final branch with a fresh
cursor, retaining hidden/sibling source nodes and the terminated input branch;
never weaken exact-parent reconciliation or decode hidden content to accept it.

The runner now observes current assignments only, preserves immutable historical
preflight proof without pretending its vanished empty thread is still loaded,
and permits completed saved proof with native notLoaded rather than demanding
idle. Current managed children receive actual connector confirmation replies in
the local runner, without a supervisor turn or interim quality audit. New helper
regressions cover scope filtering, hidden-terminal public-branch selection and
background-worker confirmation. Existing annotations remain unchanged.

An explicit claim now reloads an actually observed notLoaded native thread under
its original scope/config, with durable mutation intent and history/context
revalidation, before checking idle/jobs. Passive status/history remain passive.
Frozen gate19 passes all seven gates in545.443s with unchanged182-file inputs,
candidate d6e82b14adfe12d622d471d45ece85fe53a43874016d121194fcb33a14e3ed7f.
Synthetic11 correctly rejected a stale controller retained across restart; the
corrected driver uses an authenticated cursor-bound claim without that retired
controller, as the actual follow-up prompt already requires. Synthetic12 on the
SAME candidate passes an actually observed notLoaded -> explicit claim -> native
continuation, retaining the same thread/start/counters, then transfer, parallel
writers, dependencies and tree Stop. Source/driver hashes remain explicit; do
not relabel the failed11 run as passing or infer a new candidate from its fix.

The former CLI observer also attempted TUI resume before the native thread's
model provider config was loaded, producing a real bootstrap error. Frontend
profile overrides did not fix that server-side boundary and were removed.
An explicit same-scope native reload held open while the original TUI attaches
now renders the original canonical final (41,926 bytes, zero user input/model
requests); see recovery-2/review-1/loaded-native-cli-proof.json. The runner defers
its TUI until claim has loaded the thread and rebinds if the actual execution
owner changes or the old read-only frontend exits. It never restarts the backend
for an observer.30 affected helper/input/notice/CI tests pass after these fixes.

Recovery-3 is model-free runtime preparation only; no live batch was launched.
Before another paid input, actual saved metadata also exposed a result predicate
that waited only on the final graph leaf, although a hidden final terminal may
follow the public final. SPEC/source now reconcile the public final through only
finished hidden assistant-final terminals, preserving placeholders/exact parent
and rejecting analysis, incomplete/unsupported suffixes and a cancelled input
with commentary alone. Gate20 passes all seven gates in164.497s on unchanged182-file inputs, candidate
5ed7d40b4d408df957f02160e952a2f13cebe5fb0233f6b9e2474b514291e590. This includes
hidden-terminal positive/negative/lost-reply tests and30 helper/input/notice/CI
tests. Android synthetic13 on this exact candidate/test binary also passes the
actually observed notLoaded claim, retained budgets/cursors/native identity,
transfer, isolated writers, dependency and tree Stop, with zero Chat requests.
Recovery-4's owned test-service replacement keeps the SAME state/native roots;
old owner/listener termination is confirmed,16-tool MCP readiness passes and
model-free old-Chat adoption/release preserves its saved source. The shared
Linux host and unrelated prior Provider remain untouched. Recovery-4 seals199
inputs for a new task at an explicit public-final branch, same saved Chat/native
thread, and the still-unstarted general delegation; three new genuine inputs,
estimated3–8 app dispatches and bounded30 dispatches/200 native calls/2400s.
Actual subscription debit remains unknown. The same physical completion-notice
binding is reused after exact profile/thread/Core/environment/notifier checks.
Run status is under /data/data/com.termux/files/usr/tmp/f-live.llw6kt6a/recovery-4;
end the receiving turn after launch, with final-only audit and no supervisor
polling. Whole F1–F9 acceptance was unfinished at this recovery-4 checkpoint.
The earlier delivered input and failed receipts are not replayed or reclassified.
No prior passing development was scheduled again. The later F final review above
records the eventual bounded completion.

Recovery-4 launched once as PID18582 and ended after448.928s. Its actual final
notice67336fdb63c04b46bf80825c183f26be reached this same primary thread after the
launcher profile changed to janmori101. The retained notification environment
was unchanged; both profiles address the same physical rollout and SQLite home.
No new probe was claimed. This positive receipt supports reusing that exact route.

Independent review at recovery-4/review-1/audit-reconciled.json verifies1 app
dispatch, unchanged native history/context (13 items), unchanged workspaces and
actual preserved diff, the original final visible in its original CLI (41,861
bytes), and valid reuse of the unchanged12/12/9 project suite evidence. Claim and
its explicit elapsed extension settled successfully. Start then remained queued:
188 observations show queued/idle with no pending calls; its native-dispatch
receipt is absent and no new native turn/effect exists. The runner finally hit
app_bridge_operation_failed on a saved-result read. The original failure and
receipts remain immutable. Native Stop confirms no jobs; fresh server COMPLETE
and stopped-lane evidence permits release/dispose without input or Stop replay.
General delegation remains unstarted.

The common admission defect was comparing parent=None across every independent
root: old stopped works had also been rewritten reconciliation_required across
restart and consumed those anonymous slots forever. SPEC/source now scope
parallel limits to explicit siblings, keep unrelated root work independent, and
preserve confirmed completed/stopped phases while fencing old controllers.
Uncertain SAME-parent work still reserves capacity pending reconciliation.
Gate21 passes all seven gates in285.258s on unchanged182-file inputs, candidate
cbbb4a3ee60c4624d5a2b27902e249cde2207093e2a0b1c4bb674908e81bc6f4. Android
synthetic14 on this exact candidate/test binary passes two simultaneous native
roots each capped1, Stop isolation, completed/stopped preservation after restart,
and a third same-parent writer correctly queued until one sibling stops; its
five-work tree Stop also confirms every descendant. All reasoning is synthetic
and spends zero Chat requests; this does not replace live agent quality proof.
31 affected helper/input/notice/CI tests pass.

Recovery-5 launched once as PID4664, elapsed44.841s, and its final notice
6869dfcd79b24c789b438600e2164ff7 reached the same primary session. One app
request was delivered; three saved-result GET observations returned the generic
app_bridge_operation_failed. The batch then stopped the admitted source/native
work. Independent review at recovery-5/review-1/audit-reconciled-corrected.json
checks actual preserved diffs and every workspace file against the previous
review, unchanged native history/context cursors13/5, exactly five original works
and unchanged12/12/9 project-suite inputs. Claim/explicit extension has a settled
receipt with inference_submitted=false; no new development turn/effect exists.
The original CLI-final proof is reused only for the unchanged old final; no new
CLI result or follow-up success is claimed. General delegation never started.

Fresh same-account source reading now succeeds with58 graph nodes; server
COMPLETE, inactive lane and the original cached Stop's terminal confirmation
permit release/dispose, without resending input or Stop. Private evidence and
source reconciliation are in recovery-5/review-1. The failed GETs' original
exception/status was not captured, so their exact network/schema cause remains
unknown. The earlier three-observation fallback did not solve that defect and
must not be described as a proven connectivity repair.

SPEC/source now distinguish failed result observation from execution failure:
chat_result preserves the original input/delivery state, returns complete=false
with observation_unavailable for retrieval transport failures, and independently
observes the owned lane for approval/progress. Fresh observations remain bounded
by the original batch deadline/control budget; no input, mutation or budget is
replayed/reset. Account/permission/source/invalid-response errors remain explicit.
Host diagnostics retain static GET error codes/status only; the qualification
client passively retains the original GET exception in private local evidence.
Gate24 passes all seven gates in300.986s on unchanged182-file source inputs,
candidate8d784931e2d27f94777539220340d79d6200a0c3a6cb1c06a3df4a850af0c21f.
It covers failures in the initial GET, source-validation GET and result GET,
followed by fresh completion of the same original UUID with one submission only;
identity/permission/invalid-response failures remain explicit.144 Bun and36
Provider Python tests pass. Gate22's first-GET-only correction also passed but is
superseded; duplicate snapshot23 was canceled before tests and has no acceptance.
Android synthetic15 passes on this exact candidate/test binary, with zero Chat
submissions, including the prior admission/restart/transfer/Stop coverage.
32 current helper/input/notice/CI tests pass, including both continuation orders.

Recovery-6 replaces only the confirmed owned test service/tunnel, retaining the
same state/native roots.16-tool metadata refresh/readiness, fresh ownership,
model-free saved-Chat adoption/release and unchanged-source readback pass. After
explicit bridge initialization, its actual production MCP chat_result path is
faulted once with GET503: complete=false/observation_unavailable, unchanged original
input record, zero model/input/native-mutation replay, static public diagnostics,
and confirmed restoration. Evidence: recovery-6/read-fault-proof.json. This is a
controlled qualification; recovery-5's original HTTP/exception cause remains
unknown. The initial model-free fault-probe attempt assumed ensure acquired a
writer; the unclaimed writer was initialized by adoption before the successful
probe. No inference or failed installation was replayed.

Recovery-6 sealed202 verified references, reused the physically received
6869dfcd result's same primary rollout/Core/notifier and original bound notice
environment, and launched once as PID14741. It ran the previously unstarted
general delegation before the same-Chat/same-native follow-up. Actual app
dispatches were3 (origin/managed child/follow-up), not repeated development
submissions. It ended after1613.488s with control_observation_budget_exhausted.
The original result remains automated_gates=false; billing is unknown.

Its loop made192 unavailable saved-result observations; the retained last64 GET
exceptions all prove HTTP429 Too Many Requests on /conversation/{conversation_id}.
The first unavailable observation was12:30:14 UTC and the last12:41:44. The earlier
exceptions were overwritten by the bounded diagnostic ring; do not assert their
HTTP status. The runner continued at roughly3s intervals instead of backing off,
and also repeatedly queried completed native work, eventually spending2000 local
control observations. That is a verified pacing defect;2000 is not a model-request
count. The original cause of the first service limit and any subscription/model
quota connection remain unproved. Root6's harness has exited. Cleanup confirmed
native/source Stop with no remaining jobs, including unnecessarily fencing the
two already completed native works; their completed artifacts remain intact.

Offline review at recovery-6/review-offline/audit.json verifies the general
worker's9 tests and follow-up's18 tests, unchanged original tests, complete actual
diffs including untracked regression files, independent API/CLI contracts, native
canonical final completion,5+6 native calls with zero unreturned calls, and the
actual latest final answers in original CLI captures. The general origin's
already saved public report includes its completion marker and original native
work/thread IDs. The follow-up stays on01a15dd9-9520-70e3-861d-ba4aaf9438a3.
The CLI verifier needed to skip rendered code-fence language labels while
preserving code comments; it now verifies both actual captures. None of this
review called ChatGPT or resubmitted development. After the user lifted the hold, the original follow-up reconciled complete
with37 saved messages and released lane; its canonical native final equals the
saved final byte-for-byte. The general managed source also retains its entire
canonical final, following earlier progress text. No development was repeated.
Proofs are in recovery-6/review-resumed and pacing-7/general-managed-readback-proof.json.

SPEC/source now pace saved GETs per authenticated account across readers, without
claiming a lane: local owned stream/approval observations replace GET polling
while active; local terminal still cannot establish saved completion. GET429
waits at least60s and backs off60/120/240/300s, honoring available Retry-After.
Concurrent reads are bounded; cached cooldown callers perform no GET. Other
transient GETs also back off; success resets failure counts, and another account
has independent state. Native Stop remains responsive during a held GET.
The runner honors retry timing while observing local approvals/events, reuses an
already fetched native status, and excludes completed works with original CLI
observers. Failure cleanup preserves a completed native work instead of issuing
another Stop. All tool annotations, including idempotentHint=false, are unchanged.

Gate25 passes all seven gates in225.987s on unchanged183-file source inputs,
candidate216c30de380494e8c4e3fb5c9fa3823ecf0ad225f0ba259fc35393ceb2a2e070.
149 Bun tests include fake-clock cooldown/cross-reader/disposal/account isolation,
Retry-After, counter reset and in-flight Stop cases. Rust tests prove an active
owned stream performs zero saved GETs and cannot falsely complete, then reconciles
the original saved UUID after settlement.36 Provider Python tests pass;35 helper
checks in the sealed snapshot pass, and36 current helper checks separately pass
with completed-work cleanup preservation. The current helper differs from the
snapshot only by that qualified cleanup guard/test. Candidate25 is now applied only to the owned F service at
/data/data/com.termux/files/usr/tmp/f-live.llw6kt6a/pacing-7, using the same native
state and fixture. Exact-binary synthetic Android16-tool qualification passes.
Actual Android MCP pacing qualification injects one narrowly scoped GET429 with
Retry-After61, then confirms repeated/cross-reader calls perform zero extra GETs,
leave the original input unchanged and expose the shared cooldown. After real-clock
expiry a fresh real saved read reconciles that same original input complete and
resets cooldown. Both phases use zero model requests. These are bounded transport
proofs, not a guarantee against service rate limits or whole-F acceptance.
No default/runtime/device/public release was activated.

The user released the call hold at2026-10-21T13:15:32.187345+00:00. Its earlier
one-hour timer has a cancellation marker; no duplicate continuation is needed.
During the hold only local evidence, code and offline checks ran. Persisted hold:
~/.cache/codex/acceptance/phase-f-integration-20261021T050403Z/chat-call-hold-current.json.

The original agent batch /data/data/com.termux/files/usr/tmp/f-agents.mzab90oj
failed after90.311s and one app dispatch. Its only native work reached the real
fixture question; codex_answer was rejected before effects because the harness
added unsupported call_id. There were no development calls; every disposable Git
tree remains unchanged. Native Stop is confirmed with no remaining jobs; the
source terminal and the actual same-primary notice are confirmed. Its saved Chat
is6ad8c772-d4a8-83ee-852a-d391d6f161f7 (complete14-message public range, explicit
unavailable/private content). Original failure is retained in result.json;
review-1/audit.json owns the independent verdict. An offered cancelled call stays
visible in stopped history; it is neither an unanswered live job nor completed work.

The helper now omits unsupported answer arguments and checks every fixture control
against the exact candidate tools/list declaration before dispatch. Corrected payload
acceptance and rejection of the original extra field are proven locally against the
actual16-tool schema, with no native/model submission.41 helper/CI checks pass.
Recovery-1 created six of the planned seven roles before repair interrupted
its owned observation loop; canary/final managed return were not reached. Its
original result remains failed, with the explicit local interrupt intent retained.
review-2/native-audit.json and integrated-tests.log independently verify66 actual
native calls/results,0 unreturned, preserved original tests, the genuine merge
conflict/review failures and30 green integrated tests. Saved source/reviewer/
corrector readbacks cover25/30/26 messages with explicit unavailable/private
content. The source has no final report; the corrector has a saved public final
but its native stream failed before delivery, which is not accepted as completion.
The integrator's two genuine completed turns and original CLI final are retained.

Root cause: the app revisited an earlier public message after a later progress
item. Native Responses forbids reopening a completed item. The app projection now
emits append-only continuation items and preserves logical prefix/phase checks;
phase-only final promotion of a closed message publishes a distinct full final.
Exact original9-event private replay now projects without dropping/reordering text.
A second verified defect left streamActive=true after server completion. Stop now
clears that flag only on authoritative terminal proof and fences late owned
starts before changing active state. Actual original-source reconciliation passes
with0 additional Stop POSTs/model requests; the original observer is restored.
Cached committed Completed/no-pending host responses now survive context replacement
for Stop; an idle label alone does not qualify. Native Stop still checks jobs.
The observer also detects stopped sources and bounds absent activity separately;
snapshot writes/stream flags do not reset it. Absolute wall deadlines remain
applicable across suspension; wall and monotonic elapsed times are recorded
separately. The old immutable-input guard hashed129,439,474 bytes/210 files per
control check. It now pins the manifest, hashes before work/final audit, and
rehashes on file identity/size/mtime/ctime change. A model-free local benchmark
measured0.343193s per old check (3 samples) versus0.012504s cached checks (20),
27.45x for this guard only; both full checks pass. Evidence is recovery-2/
observation-cost.json. All tool annotations remain unchanged, including idempotentHint=false.

Gate28 passes all seven gates in272.576s on immutable inputs, candidate
f4cb84db5ec94891f66153651a2720e2e082a026d11c80f0b256279ae369ef69.
The actual original stream replay is pinned by SHA in its result.49 current
helper/CI tests and helper syntax build pass; supplemental helper changes after
the frozen build are sealed with the remaining-work input. Actual Android
synthetic18 validates the exact candidate/Core/frontend/native controls without
Chat requests. The owned service/tunnel is settled-9; shared Linux was not
restarted, native state is retained, and no default/device/public activation
occurred. The earlier candidate25/26/27 failures and proof retain their scopes.

Recovery-2 reused the original root/integrator threads and preserved all six
original canonical prefixes. It added18 native calls to the original66, all84
returned. Independent review at f-agents.mzab90oj/recovery-2/review-1 verifies
actual diffs, both unchanged30-test trees, verifier CLI19/API12 and integrator
CLI13/API9 results. The same integrator has four genuinely completed turns.
Saved source spans145 messages/two pages; verifier/integrator cover28/24 messages.
Their public native finals remain in both original CLI and saved Chat. Public
range enumeration is complete, while unsupported/private content remains explicit;
content_complete is false. The original corrector's failed stream remains failed. A separate zero-model
CLI/MCP read comparison agrees on every ordered public ID/content/cursor for the
145-message source (four pages per facade),28-message managed verifier and134-
message unrelated second repository-review Chat (four pages). Native work journals
remain byte-identical. Unavailable items are explicit7/6/13, so full public ranges
do not imply content_complete. This reuses the unchanged candidate28 reading path;
duplicate-title/alternate-branch and broader F3/F8 qualification remain distinct.
Proof: recovery-2/review-1/read-parity/result.json.

The original automatic result is false after1369.015s with3 app dispatches and
2030 local control observations,0 supervisory model turns between roles; billing
is unknown. Its final title PATCH succeeded. Exactly65 message update_time fields
changed; text, hidden metadata, IDs, branches and graph edges were identical.
The helper now excludes only those volatile fields from content comparison,
retains raw title readback evidence and has positive/negative regression tests.
Its prior events.jsonl was absent and turns.md empty after the title error;
review-1 retains canonical timings instead of inventing historical events. The
helper now records batch start and preserves task elapsed time on observation
failure. Cleanup does not resend work already at stop_requested.

Source evaluation requests omitted controller and findings text and were rejected
before effects. Managed/native completed status and actual final establish managed
completion; an explicit codex_finish item is not part of that path. Tool guidance
now states the exact evaluation fields and history pagination rule. The source's
claim that the managed finals were missing is superseded by independent evidence,
not by rerunning those tasks. Tree Stop fenced all eight roles and confirmed the
active canary/no jobs plus three others. Its four older native threads were
notLoaded; thread/backgroundTerminals/list rejected before job observation. Stop
now verifies scope, loads that exact thread with the declared routing and observes
its jobs without a new turn/inference. Original confirmed=false is retained.

Candidate30 source gates pass7/7 in297.224s, immutable binary
77df3d81ee457ec4bdec2fb3d4003bae82cfb561be0ca1434e55e5e0c68db5ea.
152 Bun tests include timestamp-only title equality and preservation failures for
text/metadata/ID/branch changes.51 latest helper/CI tests pass separately. Actual Android
synthetic19 passes with0 Chat submissions, including unloaded completed-thread
Stop. Candidate30 is applied only to unloaded-10, with the same native state and
shared Linux app retained. A distinct scoped post-fix original-tree check
confirms8/8 native Stops, applicable host confirmations and0 jobs with0 model
requests; its claim/Stop intents are independent of the original failed request.
The original confirmed=false result remains preserved. A distinct
recovery-3 input checks only the missing evaluated correction receipt, actual
dependent release and new three-role Stop; no original implementation or accepted
tests are repeated. Expected2–3 app starts, cap5; wall1800s/inactivity420s, only
end-of-batch review/notice. It launched once as PID17953 at
/data/data/com.termux/files/usr/tmp/f-agents.mzab90oj/recovery-3; launch intent and
process receipt are retained.212 unique sealed file inputs and all8 prior native
prefixes were verified before launch. All16 tool annotations match the preceding
fixture exactly, including idempotentHint=false. It ended after617.350s with2
app dispatches/590 control observations and0 supervisor turns between roles.
Original result=false at receipt_cli_final_missing. All three actual records had
separate explicit execution homes, while the observer inherited the common home;
their native default provider also still required OpenAI login. No credentials
were copied. At confirmed stopped/no-job boundaries, only those disposable homes
received their existing generated no-OpenAI-auth Provider config. The ineffective
common-home preparation was restored byte-for-byte. Observer CODEX_HOME now binds
the actual record; account/read checks the real login requirement before capture.
A new fixture preparation helper refuses existing nonmatching config files.

Independent review at recovery-3/review-1/audit.json accepts the scoped evaluated
correction receipt and real dependency release. Nine native commands/results are
present; one root command had a literal-newline quoting SyntaxError and subsequent
native commands fixed it. No original code/tests or historical30/19/12 probes
were rerun. The verifier's exact accepted cursor releases the actual dependent,
all three canonical finals and confirmed Stop/no jobs remain recorded. Separate
same-backend passive CLI readback displays all original finals with0 model
requests/new turns; the original failed live captures/result are not overwritten.
Rich-text comparison also handles repeated opening layout lines and partially
rendered bold/inline-code markers while preserving fenced code exactly.56 helper/
CI checks pass on immutable observer-helper-32 inputs;101 unchanged Provider build
inputs reuse exact candidate30 source/native qualification. This is post-completion
CLI recovery proof, not a successful original live observer.

Saved source/verifier reads enumerate71/14 messages completely, with unsupported/
private content explicitly unavailable. The saved verifier retains earlier progress
and box markup; the actual unchanged Rust presentation projection contains its
entire canonical final (raw-byte equality is not claimed). Original app/native/
CLI/history/artifact evidence is retained under recovery-3/review-1.

The subsequent zero-model F3 check temporarily gave two owned completed F Chats the
same title, compares both CLI/MCP list/read by distinct IDs, then restores their
original titles and complete graph/content (only message update_time is volatile).
It also reads all four actual saved branches of the existing qualified follow-up
Chat through both facades:38/32/66/45 public-range items in2/2/3/2 pages each,
explicit12/12/23/13 capability redactions and4/4/9/5 unavailable items. Ordered
IDs/content/cursors agree; a changed prefix is rejected by both facades, and native
work records are unchanged. Proof: f-agents.mzab90oj/f3-reading/{comparison,
restoration-proof,branches-result}.json. All these checks use0 model requests.

The original model-free F7 batch at
/data/data/com.termux/files/usr/tmp/f-faults.20261021 ended after6.602s with its
original automatic result false. Four simultaneous exact native requests shared
one canonical call/result and one file effect. The next changed-payload request
was correctly rejected by the Broker as mcp_call_changed; the harness incorrectly
expected the report/session layer's operator_call_changed. No BAD effect occurred.
The owned synthetic service stopped, its native turn was interrupted and the
independent native observation confirms idle/no background jobs. Passive loading
of the unloaded thread appended only two thread_settings_applied records, with
no new native turn/model request; the complete original prefix remains retained.
Proof: original audit.json, canonical rollout,13 original RPC receipts and terminal
observations. The original failed result and all receipts remain unchanged.

Continuation-1 at
/data/data/com.termux/files/usr/tmp/f-faults.20261021/continuation-1 ended after7.463s
with its original automatic result false and0 model requests. Two distinct logical
IDs with identical input produced two canonical calls/results and exactly `xx`.
Its raw HTTP loss helper then used shutdown(SHUT_WR) before receiving a response;
the server closed without admitting that request. Independent native/Broker/file
audit confirms zero dropped-call entries, zero dropped-call native effects, no
pending calls and terminal idle/no jobs. A passive thread load appended one
settings-only record, preserving the complete original prefix. The failed result,
original receipts and audit.json remain retained.

A read-only preflight against the actual owned candidate30 service separately
reproduces the transport issue: half-close receives0 bytes, while the same MCP
list without half-close receives HTTP200/6257 bytes. Native work records remain
unchanged; both requests make0 model/native execution calls. Proof:
continuation-1/transport-proof.json and both original private HTTP responses.
This diagnoses a qualification helper error, not a failed admitted native tool.

Continuation-2 at
/data/data/com.termux/files/usr/tmp/f-faults.20261021/continuation-2 passes its five
scoped checks in14.366s, with41 local RPC observations/4 execution requests
(including original-result recovery),3 actual native calls/results,0 unreturned,
0 model requests and0 supervising model calls between cases. The lost effect
occurs exactly once; the lost final is reused, not re-emitted. Four concurrent
Stop observers confirm no jobs while the independent sibling writes its own file;
the measured Stop/sibling interval is0.483s. Restart repairs the pending index,
returns native_session_creation_unknown without a new work, and rejects the
old controller. The original native final counts and all effects match raw
rollouts. Owned service exit and idle/no jobs on all three backends are independently
confirmed. No live saved Chat exists for this explicitly synthetic reasoning case.

Final independent proof: continuation-2/review-1/final-audit.json. Passive readback
shows both actual completed finals in the original CLI with0 new model requests/
native turns. The first two capture variants failed because the new synthetic
home lacked the generated Provider definition; invocation-only overrides did not
supply it to remote thread/resume. Those captures and partial audit remain saved.
Only the owned completed fixture temporarily received that definition; its
original config is backed up and restored byte-for-byte. No real user/host config
changed. Canonical original prefixes remain unchanged; passive loading appends
only settings observations. The original automatic successful result is unchanged.
This bounded success does not settle all F7 or whole F.

The late-confirmation attempt at
/data/data/com.termux/files/usr/tmp/f-faults.20261021/late-confirmation ended after
6.164s before installing its proxy or starting any fault case. The harness assumed
the Core catalog's `s` entry was the physical socket; it is an owned symlink to
Core's shorter socket path. The one setup native turn completed with its original
final,0 native tools/results,0 actual Chat submissions and no pending work. The
owned service exited successfully. Independent audit.json confirms native idle/
no jobs, preserved original prefix, unchanged alias/target and no proxy installed.
The failed result remains immutable.

The intermediate proxy resolved the verified native alias and temporarily interposed
only the underlying owned socket, preserving the alias bytes/inode and restoring
the exact original target inode. Before another batch launch, its actual signed-
Core read-only preflight observes real completed -> injected stale -> real completed
through that same alias, then restores it. No original rollout byte, native turn,
execution result, user/host configuration or authentication changes. Proof:
late-confirmation/continuation-1/proxy-preflight.json and retained original/injected
response/restoration records. This validated raw WebSocket transport only, not
the Provider's PID ownership check; its rejection and replacement are below.
No delayed-finish outcome is claimed from that preflight.

The first continuation at late-confirmation/continuation-1 ended after1.488s
before any Provider start, proxy installation, native request or new turn. Its
inventory loop treated the nested catalogs directory as a regular file. Independent
audit.json confirms all8 original state files, the canonical prefix and both
alias/target inodes unchanged; native state is notLoaded with the preceding
confirmed idle/no-jobs boundary retained. The original failure and receipts stay
immutable. This is a qualification preparation defect, not a product execution
failure or delayed-finish result.

The recursive-inventory correction at late-confirmation/continuation-2 executes
its exact source preflight, then the actual Provider refuses the native-socket
proxy with native_socket_owner_changed in1.618s. The signed native client verifies
SO_PEERCRED against the bound runtime PID; a same-user raw WebSocket client had
not exercised this ownership gate. The gate is correct and remains unchanged.
Independent audit.json confirms0 native tools/new turns/model requests, the
original canonical prefix and restored socket/alias inodes. Only the service's
controller capability rotated at restart; all other original state fields agree
with the retained snapshot. No uncertain native mutation was replayed. Native
socket interposition is retired as a test method.

The replacement local gate at late-confirmation/http-completion delays only the
selected final response.completed HTTP SSE event between the actual native model
client and this owned synthetic Provider. Real native RPC sockets, PID checks and
execution remain intact. The existing test-only synthetic service gains an
explicit CODEX_PROVIDER_OPERATOR_BASE_URL override restricted to a canonical
127.0.0.1 port/v1; production code/default behavior and all annotations are
unchanged. Its two focused Rust checks, fmt and Clippy pass on immutable
f7-http-fault-helper-33 inputs; the exact test executable is608abae6. Current
production source matches candidate30; existing unchanged production qualification
is retained. The HTTP framing/withhold/release preflight passes with0 model calls.

This gate reuses the completed setup in the same private fixture. It keeps209
sealed inputs, original state/bridge/ready snapshots and native prefix proof.
A settled current-reasoning claim extends only the fixture elapsed cap to1800s
with original start, parallel/app/native caps and consumption retained; the batch
has a120s deadline and12 native-tool-request cap. Three new scoped user turns
follow across two native works,0 new setup turns and0 actual Chat requests. The
cases are four finish observers timing out while native completion is delayed,
then same-ID original-final recovery; independent native question/four identical
answers and changed-payload rejection; responsive Stop during another finish wait;
and restart with uncertain answer/start receipts without a second answer/turn.
The Stop case deliberately interrupts a genuinely incomplete native turn: a
recorded final item is not counted as completed work. It returns one inline local
result, with end-only review and0 supervisory model calls between cases. Live Chat
batches retain their detached final-notice contract.

The HTTP-completion gate passes all four scoped cases in36.159s with43 local RPC
observations and0 actual Chat requests. Four finish observers share the same
native_completion_unconfirmed result at15.082s; after release, the same ID returns
the original completed final without a new final item. Four identical answers
produce one original native question result; changed payload is rejected. Scoped
Stop takes0.203s during a second incomplete turn, which remains interrupted even
though its final item had been recorded. Restarted uncertain answer/start receipts
stay fenced without resending. Independent review-1/audit.json verifies raw native
records, all four markers in the original CLI, preserved original prefixes,
3 completed turns/1 interrupted turn,1 question call/result,0 unreturned calls and
idle/no jobs. The audit uses0 new model/native turns; no live saved Chat exists in
this explicitly synthetic reasoning case.

The separate summary-rounds scope confirms two native compacted records in two
distinct rounds under the same controller. Four identical first-round and four
identical second-round summaries each settle once; a cached old key does not
answer the next round, changed payload is rejected, and an uncertain original
receipt stays blocked before and during a third pending round. The original
40.732s automatic false is retained: its harness required idle after Provider
exit, while Core correctly unloaded the native thread. Distinct review-1 recovery
fences both old controllers and confirms native Stop/no jobs with0 new model
requests, compactions or development calls. Final independent verdict:
summary-rounds/review-1/final-audit.json. The original unknown receipt and every
prior failure remain preserved.

Four test-only virtual-round settlement regressions cover validation before
intent, Broker write failure after preparation, lost lane acknowledgment after
Broker commit and a new Broker generation encountering the prepared old round.
They retain original results without replay/reoffer. Immutable acceptance inputs
are f7-settlement-34 under the F cache:79 library tests pass,1 explicit native
model fixture is ignored, fmt and all-target Clippy pass in65.719s,0 Chat requests.
Production settlement code is byte-identical to candidate30; these additions do
not claim a rebuilt or deployed production binary. At this candidate34 checkpoint,
F7 hint application and other unmatched F scopes were unfinished; tool annotations
were unchanged. Candidate40's later hint qualification/application is reviewed above.

F9's first matched batch at /data/data/com.termux/files/usr/tmp/f9.179qboye
has239 sealed inputs and the preserved candidate30 source. Four identical
disposable projects each have8 original and8 supplied regression tests; the
16-test baseline actually fails before implementation. Four isolated native homes
and idle work handles were prepared without a user turn or Chat request. The two
Codex-side routes submit through the real candidate Provider `work start` CLI;
the Chat routes supply current reasoning or delegate to the same managed MCP
path. All routes receive the same development task, model gpt-6-thinking/extended
(native gpt-6/high), approval policy and sandbox. Route-specific wrapper/system/
tool context is unavoidable and measured separately, not claimed byte-identical.

An earlier model-free original-TUI preflight cannot resume either empty thread:
upstream has not persisted its first turn yet. Both original error captures are
retained. This does not require a model/setup turn: use the selected candidate
work CLI, then attach the original TUI after the genuine first turn is persisted.
Actual native TUI task entry remains distinct proof; this F9 sample must not be
reported as that entrance. Subsequent real work-CLI status/read preflight passes
for both routes, with all four works idle,0 pending calls and0 Chat requests.

The serial batch has four development tasks/five saved reasoning-or-origin Chats,
an estimated8–14 model requests and hard cap20. Each work has48 native calls,
12 app dispatches and600s; the whole batch has2700s/5000 control observations,
with10s between cases. Actual request counts/bytes/model/effort, UTC/monotonic
times, CLI submission/results, native calls/returns, baseline failure/correction,
unchanged tests, diffs and saved public finals are retained for one terminal
audit. There is0 supervisory model work between cases, no uncertain replay or
automatic route fallback, and one final same-primary notification. One sample
per route is bounded comparison, not a statistical winner or billing proof.
The existing same-primary notification binding is reused without another probe;
launch-intent/run-started files distinguish launch from the prepared state.
The original batch launched once (launcher PID21167), then stopped in4.022s before
any actual Chat request. The first native start was accepted, but its immediate
wait received native_request_rejected before upstream persisted the first turn.
Failure cleanup confirmed that native Stop/no jobs. Independent review-1/audit.json
confirms0 native tools/results, all four project trees unchanged and the other
three turns unstarted. There is no saved model response/development final to
accept; only the earlier empty-thread TUI error captures exist. No original
mutation was replayed. A read-only raw native probe retains the exact upstream
-32601 / thread/items/list is not supported yet response on the unstarted thread.

Candidate35's scoped correction retains the pending method with its native reply,
classifies only that exact paged-items rejection as native_history_not_ready and
keeps other failures rejected. events explicitly reports history unavailable,
retains the supplied cursor and does not certify its range; wait remains pending
until actual history is observable, even if work status says completed. Questions
and Stop stay observable. The selected contract is in SPEC. Five new Rust checks
cover classification and completion/attention semantics. All seven frozen source
gates pass in291.411s:84 library checks (1 explicit native fixture ignored), full
Rust tests/Clippy/release,152 Bun checks and Provider/helper Python suites.
Exact candidate: e1ffb0fd03a05ff94e4eaf3b373d45569db5fee658ec94f21267857a0d00b579.

Actual signed Android proof is provider-regression-35/native-startup-2/
independent-audit.json: empty managed events are explicitly unavailable, wait is
pending with unchanged cursor, malformed cursor is rejected, then a genuine
current-reasoning turn executes1 native command with1 original result. Its final
is delivered through observable history and the original CLI; two scoped Stops
confirm no jobs. Actual Chat requests are0. Its original automatic false is
retained: the final harness observation assumed a bridge log existed before any
bridge operation. Shutdown subsequently records only3 dispose operations and
0 submissions; independent canonical/file/CLI/RPC audit accepts the scoped proof.
The earlier missing-fixture-marker preparation failure is also preserved, before
any native request. Neither helper failure requires repeating accepted effects.

The owned runtime replacement uses fresh process/hash/host/account/tunnel checks.
The old zero-turn preflight thread's status rejection is reconciled against its
original Stop/no-jobs proof, no canonical turn and absence from every verified
fixture backend's loaded list. Other prior reconciliation states remain explicit;
they are not marked completed or retransmitted. Shared Linux and unrelated
Provider remain untouched. Tool declarations are unchanged, so no redundant
connector metadata refresh is needed. Runtime receipt:
/data/data/com.termux/files/usr/tmp/f-live35._pzrw09a/continuation-1/runtime-ready.json.

The next distinct four-route input is /data/data/com.termux/files/usr/tmp/f9c1.ina35ejy.
A ready host may be unclaimed before the first owned lane; require no active lane
and verify ownership at init rather than require an already claimed idle hook.
The previous global create IDs correctly reject changed workspace payloads as
operator_call_changed before effects. New create IDs include this distinct run;
shared Batch.created now namespaces IDs by canonical owned root, stable within
that run. Helper36's57 Python checks pass with production source unchanged from
candidate35. The original known rejection/receipts remain preserved. The new
fixture seals227 inputs and has four separately prepared idle works, identical baseline/task and the
same declared budget/measurement/terminal-notice contract above. F9 acceptance
at this launch still required the terminal original CLI/native/saved-source/diff/test audit;
tool annotations were unchanged at that point. This distinct batch launched once
(launcher PID17914) and ended after704.892s. Its automatic result remains false:
source_finished_without_expected_marker on the general-origin Chat. All four
native works were already completed and preserved; cleanup sent no native Stop
to those completed works. No uncertain original mutation is retransmitted.

Independent terminal audit is f9c1.ina35ejy/review-1/development-audit.json.
All four exact native task strings match the supplied task; upstream records
them as user response items, not user_message events. Each native worker shows
the real16-test baseline failure before its patch and a passing final suite.
Independent unchanged-suite/API/CLI/diff checks also pass; original CLI/canonical
finals and complete saved-source reads are retained. Both protected test files
remain byte-identical. Calls/results are5/5,6/6,6/6,6/6 respectively (23 total,
0 unreturned). There were0 supervisory model turns between the four tasks.

| Route (candidate35, one sample each) | Entrance-to-result seconds | Native-turn seconds | App requests | Request body bytes | Native development / origin report |
| --- | ---: | ---: | ---: | ---: | --- |
| Provider work CLI, managed MCP | 93.043 | 92.567 | 1 | 10,275 | accepted / completed |
| Provider work CLI, managed local | 130.051 | 129.580 | 7 | 53,784 | accepted / completed |
| Current ChatGPT, direct MCP | 170.657 | 141.255 | 1 | 3,674 | accepted / completed |
| Current ChatGPT, general MCP delegation | 277.656 | 202.896 | 2 | 13,821 | accepted / marker withheld incorrectly |

Total app request bodies are81,554 bytes. These transport body sizes exclude
server-side retained context and internal MCP traffic; they are not token or
billing measurements. Actual allowance debit and per-route CPU/RSS were not
observed. One serial sample does not establish a universal latency ranking.
The local path has7 app dispatches versus1 for managed MCP in this actual task;
general delegation has a separate origin and managed-worker request. Native work
and coordinator completion are reported separately, without changing the original
failed result into a passing one.

The general-origin saved final says the14-item native history lacks the final.
Original source tool-return evidence proves otherwise: two received MCP responses
(28,109/33,519 characters including their wrapper) contain the original622-character
native final, phase final_answer, ID77b1c0cc-7650-46b8-89d1-9f8202c4302d. It is
the third native item, preceding later reports in native creation order; the
after-item14 read correctly returns no additional events. Proof:
review-1/original-source-tool-return-summary.json, native-history-comparison.json,
native-turn-timings.json and the original saved source. This is an origin-model
inspection error, not lost native output or a failed implementation.

The bounded correction adds completed_result to events/wait, selecting only the
current native turn's original public final items from that same native history.
It remains present after a cursor covering the final, without creating another
result or changing history order. Missing current-turn finals keep delivery
pending; stopped/interrupted work cannot acquire a completion view. SPEC and the
tool description explain this focused receipt and incremental cursor use.
Candidate37's seven source gates pass in275.392s with unchanged sealed inputs.
Its refreshed connector and actual Android MCP/Provider CLI readback return all
four original finals identically, including an empty incremental events list
after the full cursor. Native prefixes remain unchanged with0 new calls/turns or
Chat requests. Full versus incremental tool payloads are27689→2342 bytes for CLI
MCP,27557→3290 for CLI local,27506→3064 for direct MCP and27685→2383 for general
MCP; these are response bytes, not tokens or billing. Proof is
/data/data/com.termux/files/usr/tmp/f-live37.8l66sele/original-final-readback/proof-private.json.
All annotations remain unchanged. The receipt-only follow-up above confirms
actual reading and model correction in the same general-origin Chat, independently
of its failed automatic saved-input reconciliation. Audit:
/data/data/com.termux/files/usr/tmp/f9-receipt.d5mHzoCj/review-1/independent-audit.json.
Its1 app request and29.830s automatic-run duration are separate correction costs;
that duration ends at the observer failure, not an independently measured full
model latency. The observed hidden-parent normalization is addressed by binding
the delivery cursor before input; old missing bindings are never guessed or edited.
The incomplete first correction at provider-regression-38 failed compilation and
was not deployed. Candidate39's complete immutable source gates pass; the owned
runtime replacement retains the shared Linux app and all native work. A private
preparation path-selection error first relocated candidate37 without model calls;
the separately recorded continuation selected candidate39 by its verified full
hash. No connector refresh is needed because its tool definitions are unchanged.
Account for that follow-up and its candidate separately from this original
four-route performance sample.

The parent bundle's cancelled native work and three development routes retain
the concrete results above. Same-thread follow-up and Chat-origin delegation
have now executed as described above; their remaining readbacks do not require
repeating development. The local runner handles protocol observations and approval
replies; quality audit is once at batch end, with one final notice and no
supervisory model turns between tasks.

`scripts/provider_qualification.py` now validates/seals explicit v1 inputs without
launching work; the synthetic operator driver accepts `--inputs` with explicit
generation/test-executable roots and a one-time launch intent. Historical flags
remain supported. Source CI/check scope now includes the F input and notification
helpers, without enabling paid/live CI. The E offline example remains an immutable
example: verifying its inputs does not launch or pass an F case.

#### Second repository review: accepted findings and disposition

The completed second report in saved Chat
6ad7f88b-6548-83e8-827b-a3789ff33d92 (assistant message
6644ef5f-c4c6-4427-a231-702716d144f1) was retrieved without new inference and
independently compared with current source. Evidence and the original report:
~/.cache/codex/acceptance/repository-review-second-20261021/audit.json.
Before the review note, HEAD was unchanged, with 24 modified tracked files,
19 untracked files and a 3,738-addition/315-deletion tracked diff. Five reviewed
gateway/native source hashes matched the preceding idempotency study. There is
no complete byte snapshot of the first review's dirty tree; counts/mtime alone
cannot establish all intervening source changes or product improvement.

| Finding | Accepted scope | E disposition / F evidence required |
| --- | --- | --- |
| Finish retry skips completion post-processing | Confirmed conditional path, not a reproduced outage; the successful-first-call retry check does not cover it | E: resume settlement from original receipt without emitting again. F: delayed completion, lost replies, restart and controller fencing |
| Work record/index are separate commits | Possible undiscovered record after interrupted registration; no observed loss | E: settle discovery/commit authority and implement common recovery. F: interrupted creation/index writes and restart |
| Virtual-round intent precedes Broker settlement; error restores only memory | Conditional durable/in-memory discrepancy, not observed replay | E: durable reconciliation state and shared settlement contract. F: failures before/after Broker commit and lost wakeups |
| Previous Chat settlement has no overall deadline | Intentional wait with Stop checks and per-observation timeout; not proven deadlock | E: fresh cause/state, cancellable observation and explicit reconciliation. F: delayed/stale/unavailable host without submitting twice |
| Native history is repeatedly reconstructed | Potential long-history overhead; no measured regression | E: source-bound incremental/paged retrieval design. F: long histories, changed prefixes and completeness/resource comparison |
| Qualification uses historical roots and limited CI helper scope | Reuse/coverage gap; old exact-input evidence remains valid | E: explicit candidate/home/fixture/tunnel inputs and offline/live separation. F: reusable exact-input manifests, meaningful local gates and declared live batch |

The report correctly withdraws its earlier recommendation to change the user's
fixed non-idempotency annotations. Its E/F/G/H order is current; device activation
is separate from G publication despite their abbreviated grouping in its table.
Shared reading, combined routing, coordinated agents and common management remain
implementation goals, not newly accepted features from document edits. Preserve
native approvals and actual read/write capability. These source findings establish
neither data loss nor replay nor measured speed/token/billing improvements.

Selective TS or JS/JSDoc/checkJs remains optional follow-up, not an E/F/G condition.
Detailed compaction necessity/capacity/trigger policy, noVNC visual/touch/IME polish,
Core update-display/prompt work and broad project-setup UX retain their recorded
priorities. Project operations needed by selected workflows still belong in E
research and F implementation. Alternate app-free/authentication candidates remain
capability/efficiency research; identity does not establish subscription debit.


**Evidence map; these scopes are not interchangeable:**

| Evidence | Established | Does not establish |
| --- | --- | --- |
| A/B and source/full/Android checks at recorded inputs | Recoverable baseline, optional-component boundaries, native control and synthetic transport/return/recovery | Real Chat behavior or signed operating composition from synthetic proof alone |
| C/C2, including eeaf9007 ten-cycle and 2edc2215 saved-Chat successor proof | Real commands/patches/tests, related follow-ups, bounded approval/recovery, five-turn continuity and actual successor on the same native thread | Arbitrary old-Chat reattachment, measured context capacity or every current combined route |
| D and D2 exact 9fefba99 four-route comparison | Matched Codex-entry development/handoff at legacy mode/transport combinations; timings, quality findings, failed costs retained | General performance/billing optimum or combined ChatGPT delegation acceptance |
| D3 normal B/C and final d3.uplix9nz A run | Bounded unattended development, receipt reconciliation/fault recovery and terminal notice | Every failure, long duration or agent orchestration |
| chat-entry.11uglt0w | Preserved negative: 116-byte fixture path rejected before native creation, no development effects | Development success; error is not evidence of an MCP timeout |
| chat-entry.5iupe0uk/recovery-1 | Two-turn development/checks/native results after controller repair; preserved actual wrong-backend CLI visibility failure | Uninterrupted development or complete CLI visibility acceptance |
| chat-entry.5wv7_5h3 final-review | Two same-Chat/native-thread turns without mid-run intervention, 13 tests, 17 native call/result pairs, full final answers in exact-socket CLI, restoration/notice | Original automated verdict being green: its terminal-line false negative is preserved; no matched new-entrance comparison/actual allowance proof |
| Proposed common bridge and user flow above | Agreed target/composition/sequence | Implementation, migrations, general-delegation return, cross-entrance takeover or product release acceptance |

Detailed immutable inputs, failures, public/source/native observations and final
review paths remain in the retained sections below. Do not erase negatives, infer
broader scope from a Chat title, or repeat green tests unless their inputs changed.

### Next entrypoint qualification

The accepted trials and earlier research below inform E design and F implementation,
not an instruction to launch another trial now. The agreed bridge structure above
supersedes the earlier Linux-app-removal-first order. Model-adapter/MCP remains
the managed Codex target with explicit local choice; direct ChatGPT uses MCP and
general ChatGPT reuses the managed path. None is an accepted combined operating
default or a proven general speed/cost optimum. Legacy operator/local is retained
but excluded from the immediate preferred comparison due to its recorded errors/
overhead, not because local transport is categorically incapable.

Use the installed Termux Codex wrapper/native app-server as the execution and
recording endpoint; do not confuse it with the trial Linux ChatGPT app. Model
mapping and native tool/control adaptation both need session management. The
legacy `operator` label refers to the latter implementation. One bridge may expose both roles
with shared identity/binding/recovery. Current execution.mode is service-wide;
the combined entrypoint-dependent interface is not implemented/qualified merely
because both roles exist separately.

**Audited failure: chat-entry.11uglt0w.** The unchanged D2-qualified 9fefba99
Provider received one genuine ChatGPT source request (gpt-6, extended), but native
preparation failed before creating any thread/work. My qualification fixture's
long HOME produced a 116-byte Unix socket path; Core rejects paths over 107 bytes
before backend spawn. Retained catalog owner/models, absent socket/PID/server/
native rollout and empty work list corroborate that boundary. Four actual MCP
calls/results were create/native_core_prepare_failed, list/empty, same-ID create/
native_session_creation_unknown, list/empty. Keep the creation intent; do not
resubmit. This is a fixture preparation failure, not an observed MCP timeout.
Controller elapsed 48.771s; completed 0/2 development turns, commands/patches/tests 0.
Actual workspace Git status/diff are empty and both original file hashes match.
No native thread existed for the original CLI or canonical development export;
these acceptance gates remain untested. Saved non-temporary Chat is preserved in
Termux Provider: Chat entry as `진입점-직결mcp:1턴실패·native준비`; title readback
preserved the mapping hash. Final notice receipt was verified in the original
Codex thread's canonical user record. The settled source bridge was disposed;
the original Provider binary/state/home/workspace/port and healthy tunnel were
restored. Prior work remains notLoaded/released_after_stop, pending/human 0;
desktop/login were not restarted. Latest connection info is under the trial's
prior-service/info.json, not the superseded Provider PID/profile snapshot.

Failure result/frozen inputs and independent final-review.json/.md are retained
under /data/data/com.termux/files/usr/tmp/chat-entry.11uglt0w and durably copied to
~/.cache/codex/acceptance/chat-entry.11uglt0w. scripts/qualify_provider_chat_entry.py
now allocates a separately owned short TMPDIR native fixture (101-byte socket
with the current TMPDIR), checks the path budget, and requires actual native
catalog preparation before releasing the prior connection. Its latent final
model check now matches the requested/saved gpt-6 and extended effort instead of
requiring gpt-6-thinking. Local path/model/syntax checks passed; the sealed failed
run was not edited. No new fixture/backend/Chat was started for the correction:
the new preparation gate and two-turn development still require actual proof.
The execution Provider had no --descriptor or host.rootfs; the separate source
input client used the existing authenticated Linux ChatGPT backend. This failure
does not establish app-free primary development or actual allowance attribution.
Audit/restoration submitted 0 additional model requests. No trial was restarted.

**Audited partial result: chat-entry.5iupe0uk/recovery-1.** The authorized corrected
trial passed actual native catalog preparation with a 101-byte socket and the
14-tool live MCP probe before its first model request. Desktop controller and
separate app host were reverified by PID/start/image; the prior-profile check now
uses the frozen owned snapshot. Two genuine source turns completed in one saved
Chat and native thread 01a15a1b-a5a5-7b13-ae2c-b077e81a1008. The first controller
failed treating a list-valued journal payload as an object; its result/notice
remain preserved at the parent root. The reader was corrected, and recovery-1
continued that existing Chat/thread without resubmitting the request or effects.
The following turn was submitted automatically. This intervention prevents a
claim of uninterrupted unattended acceptance for this attempt.

Final independent review verified ledger.py implementation, both added regression
files, unchanged original and turn1 tests, 19 passing full-suite tests and separate
API/CLI input checks. All four current files match the checked final snapshot, so
green tests were reused. audit-full.diff includes the added untracked test files;
actual.diff alone does not. Native canonical history contains both completed turns
and both final answers. Original native calls/results are 15/15, unreturned and
duplicate IDs 0: command attempts 7 (6 executed, 1 rejected before execution),
patches 4, clock calls 4. The rejected rm -f-containing verification command and
the model's separate safe replacement remain recorded. Source MCP calls 34
(17/turn); actual source model requests 2, gpt-6/extended. Admission-to-completion
observations were 148.727s and 145.798s; source clock intervals were 96s and 84s.
Controller CPU across both runs was 1.405416s, peak RSS max 35884KiB; whole-pipeline
resources, supervisory recovery token cost and actual allowance attribution were
not measured. Do not compare these different tasks directly against D2 timings.

Overall acceptance remains false: original result.json failed at `native CLI final
markers missing` after the batch tests and canonical export. The original CLI
showed first-turn progress, then `This conversation is read-only or unavailable;
no operation was sent` and `This conversation is open in another app`. Turn1's
marker occurs only in the displayed user task; turn2's is absent, although both
native final answers exist. Preserve this actual visibility failure. Do not
weaken the marker check or relabel it a development/model/MCP execution failure.
The complete CLI observer/owner-binding cause is not yet diagnosed; repair the
observer connection without taking execution ownership before repeating that gate.

The non-temporary Chat is retained in Termux Provider: Chat entry as
`진입점-직결mcp:1~2완료·CLI표시실패`, with mapping hash unchanged on rename.
Both final notices were independently reconciled to their actual original-thread
user receipts. Source bridge, owned CLI and trial native servers were stopped
after completed/idle state, no pending/human calls and no jobs were verified.
The original Provider binary/state/home/workspace/port and healthy tunnel were
restored; latest info is /data/data/com.termux/files/usr/tmp/chat-entry.5iupe0uk/
prior-service/info.json. Desktop/login and unique work were preserved. Native
history remains locally readable after source/native shutdown; offline TUI resume
is not established by that file readback. No new Chat or trial was started during
audit. Immutable failures, final-review.json/.md, full diff, tested workspace,
native receipts/rollout, original CLI and saved Chat are retained under
/data/data/com.termux/files/usr/tmp/chat-entry.5iupe0uk/recovery-1 and durably copied
to ~/.cache/codex/acceptance/chat-entry.5iupe0uk/recovery-1. Provider execution had
no descriptor/rootfs/app driver, while the separate source input client still used
the Linux ChatGPT app. Primary direct development is evidenced; live CLI display,
uninterrupted controller acceptance, matched route comparison and allowance
attribution remain unfinished. Do not restart either trial from a delayed notice.

**Accepted bounded contract: chat-entry.5wv7_5h3.** Final independent review
accepted both genuine development turns, same saved Chat/native primary thread,
original tests/regressions preserved, full 13-test suite and independent API/CLI
checks, original native receipts/history, both complete final answers displayed
in the original CLI, shutdown/preservation/restoration and same-session notice.
The unchanged D2-qualified 9fefba99 Provider is the candidate; no product binary,
Core or frontend was changed. The qualification CLI now attaches with --remote to
the current work's exact execution socket, validating PID/start/image; generic
resume previously selected another backend. Discovery excludes archived
controllers and waits for a durable native turn. Added tests are captured in the
full diff. Zero-model copied-history checks remain separately scoped; the new
live two-turn evidence establishes the actual path.

One controller ran from first task through automatic follow-up and completion
without supervisory-model/quality intervention or repair between tasks. Source
requests 2 (gpt-6/extended), source MCP calls/results 36/36, native calls/results 17/17
(exec 9, patch 4, clock 4), unreturned/duplicate 0. One source execute was rejected
with operator_controller_fenced and corrected within the same model request;
the denied call and recovery are retained, with no uncertain effects replayed.
Turn 1 reached 6 passing tests, turn 2 reached 13; original and turn 1 test bytes match
their baselines/native add records. Current files match the checked snapshot,
so green tests were reused. Monotonic admission-to-completion was 104.514s and
105.208s; UTC wall observations were 110.238s and 146.037s. The clock discrepancy
is retained without attributing suspend or clock adjustment; model-reported real
native work intervals were 63s and 96s. Controller CPU 1.685268s, peak RSS 30856KiB;
whole-pipeline resources, supervisory tokens and actual allowance remain unmeasured.

Original result.json still reports automated_gates=false / native CLI final marker
line missing. This was a terminal-stream verifier false negative, not another
visibility failure: cursor controls placed the prompt after the marker on the raw
captured line. Both complete canonical final answers, including markers, occur
in the unmodified original CLI after terminal controls/wrapping whitespace are
normalized. The verifier now requires each *entire* canonical final answer rather
than a raw-line anchor/short prefix. It rejects both the old actual ownership
failure and a missing-marker regression. It was repaired after task completion;
the original failed automatic verdict is not relabeled green. Original sealed
run.py, result and raw CLI remain intact. Final-review.json explicitly separates
accepted real-task evidence from that preserved automatic false negative.

Evidence root /data/data/com.termux/files/usr/tmp/chat-entry.5wv7_5h3 and durable
~/.cache/codex/acceptance/chat-entry.5wv7_5h3 contain final-review.json/.md,
full diff/checked workspace, native rollout/receipt pairs, source calls/results,
original-cli-binding/completion proof and verifier regression evidence. Native
thread is 01a15b14-4fb0-7052-acbe-e0646624f9c3. Saved Chat remains in
Termux Provider: Chat entry as `진입점-직결mcp:1~2완료·CLI검수통과`, with mapping
hash unchanged by rename. Both source turns were recorded and displayed through
the same native thread; native history remains readable after source/native
shutdown. This does not separately prove offline TUI resume.

Pending/human calls and remaining jobs were 0 before shutdown. Owned source bridge,
CLI and native servers were stopped; previous binary/state/home/workspace/port and
healthy tunnel were independently reverified. Current connection info is under
the evidence root's prior-service/info.json; desktop/login/unique work were
preserved. Final notice was verified as an actual user item in the bound original
Codex thread. The audit sent 0 model requests and started 0 trials. Execution
Provider had no descriptor/rootfs/app driver; source input still used the Linux
app backend. Actual allowance attribution, source-client removal, matched route
comparison and release/device activation remain outside this bounded acceptance.

1. The bounded direct primary gate above is accepted: ordinary ChatGPT ->
   authenticated MCP -> native Codex actual development,
   follow-up, reports/completion, CLI visibility and same-thread durable history.
   The execution Provider had no Linux-app connection in the disposable setup;
   preserve the user's live host/login/jobs. Existing operator code already uses
   native app-server, so this is an app-dependency gate unless a genuinely distinct
   integration is identified. Native work records are not a full mirror of every
   external Chat message; the submitted task, native reports/execution/finals are
   independently recorded above. Do not repeat that passing development gate
   solely to repair its preserved terminal-verifier false negative.
2. E design/functional decisions are complete. After F shared implementation, qualify the missing general
   ChatGPT -> common managed path bootstrap/question/result return, then compare
   retained routes under the same real-development contract. Preserve write/
   development capability and the no-per-turn-supervisor/final-review requirement.
3. App-free ordinary-ChatGPT connection is optional deferred research, not a gate
   before E design, F implementation or G delivery. Revisit only for a concrete capability/efficiency benefit; preserve
   the useful Linux app session path. Do not replace it with browser/DOM merely to
   remove an app or substitute Work/Codex allowance/API billing for the user target.

| ChatGPT-entry case | Boundary to qualify |
| --- | --- |
| Delegate through adapter/MCP | MCP entry/bootstrap -> native Codex -> existing app-backed adapter/MCP; source and delegated Chats are distinct contexts and their costs both count |
| Delegate through adapter/local_function | Same MCP entry/bootstrap, with local functions only for the inner adapter return; this is not a connector-free ChatGPT entrance |
| Direct MCP native-work control | Existing ChatGPT supplies intelligence; native Codex executes/approves/records without starting a second primary Chat/model |
| Linux-app-absent direct condition | Prove the direct primary path cannot acquire/use an app; count separately only if implementation and execution path genuinely differ from the previous row |

Record exact source/runtime/model/effort, baseline/prompts, entrypoint and every
participating Chat. Measure request admission -> first effect -> native completion
-> Chat settlement separately, plus MCP/control/app/model requests, bytes, full
process CPU/RSS, startup/manual steps, interruptions and all failed-attempt costs.
Compare actual diffs, unchanged/regression tests, original call/result IDs, CLI,
canonical history and saved Chats. Preserve the same primary thread across follow-up/
handoff, and verify local record readback after remote disconnection. Reuse sealed
D3 controller/terminal notice and batch final review: no supervisory model polling,
per-turn quality intervention, uncertain replay or automatic mid-task fallback.
Before any new real Chat run, report its concrete gap, task/start/response budget
and anticipated use; old green inputs do not justify repeating D2/D3.

**Authentication and allowance boundaries.** The user has provisioned the Linux
app, authenticated browser and administrative access; reuse verified ownership and
existing authorization to investigate real connection capabilities. Do not print
credentials or infer permission from a saved URL/PID. SIWC identity can identify
the same user for each MCP client; the service must separately grant MCP access
and bind that user's work. This neither automatically authorizes the opposite
product's model API nor selects an allowance. Identity-only SIWC provides no
documented ordinary-Chat conversation/session-renewal capability; combining it
with an existing authenticated Chat session is a research candidate, not a proof.
The workspace Access tokens page at https://chatgpt.com/admin/access-tokens issues
credentials supported by Codex CLI/app-server; installed0.163.0 exposes
login --with-access-token. Those model-request credentials are distinct from MCP/
app-server transport authorization and are not demonstrated general-Chat credentials.

The target must preserve ordinary ChatGPT intelligence/context rather than solve
connection by consuming Codex/Work allowance or separate API billing. Work/MCP
Events and SIWC plan usage therefore are outside the proposed solution set.
Identity-only SIWC is not excluded. Establish actual token audience/scope, session
maintenance, endpoint acceptance and allowance source independently. App starts,
MCP calls and token estimates do not prove quota/billing; unavailable debit data
stays explicitly unverified. Include independent reviewer inference in accounting.
Same-account authentication does not alone prove subscription/model entitlement.

Official references checked2026-10-20: [SIWC identity and connector scopes](https://developers.openai.com/siwc/quickstart),
[separate connector and SIWC transactions](https://developers.openai.com/siwc/chatgpt-plugin),
[workspace tokens and native/transport separation](https://learn.chatgpt.com/docs/enterprise/access-tokens),
[plan usage consumes Codex/Work allowance](https://learn.chatgpt.com/docs/sign-in-with-chatgpt),
[MCP Events requires supported Work contexts](https://developers.openai.com/plugins/build/mcp-events).
These are documentation/code findings, not new live Chat/credential qualification.

### Retained D comparison

**D — bounded route comparison and delivery selection: complete (historical scope).** This is not an
exhaustive comparison of every control/transport/entry combination. D selected operator/MCP as
the product delivery target for fully bound MCP-capable work, with
model_adapter/local_function as an explicit before-start fallback for development
without a Chat-side MCP connector or deliberate native-model-adapter use. Existing
bare-config defaults remain model_adapter/local_function because operator needs
explicit Core/home/workspace bindings; selected-default installation/provisioning
and operating replacement belong to G. The next entrypoint qualification supersedes
this historical route direction; model-adapter/MCP is the Codex-entry target.
Preserve current source/config support and proof
until G/H reviews its distinct coverage, shared code, unique work, provenance and
migration; do not remove it incidentally.
Installed JS/Bun/Python remains the protected migration reference, not an automatic
fallback. No uncertain-dispatch retry, account switch or mid-task route fallback.

| D criterion | model_adapter/local_function | operator/MCP |
| --- | --- | --- |
| Actual two-turn development | One saved Chat/native thread; baseline failures, real patches, added regressions, preserved original/prior tests, native CLI/canonical finals | Same input contract and prompts, one saved Chat/native thread, same actual development/result checks |
| Original native execution calls/results |20/20; zero unreturned |20/20; zero unreturned |
| Observed app dispatches |16+7=23; includes a rejected gateway request, not an extra native effect |1+1=2; native tool-result/control work continues within each app request |
| Full generated project tests / independent common assertions |15→22 tests /18→35 assertions; exact5000-digit API/CLI pass |16→24 tests /18→35 assertions; exact5000-digit API/CLI pass |
| Additional valid131073-digit CSV integer |API ValueError/CLI exit2: inherited CSV131072-character field limit violates this generated task's arbitrary-integer contract |Exact API total and CLI output/exit0; parser/helper ASTs show this finding persists through both turns |
| Continuity/control evidence |Earlier native/AI development and model-adapter context/recovery proof; D same-Chat follow-up confirmed |C actual native work/recovery plus C2 five real user turns/source-authored saved-successor handoff and original-return continuation |
| Required dependencies |Same optional Rust Provider, owned app/CDP and upstream native runtime; no Chat-side MCP service/tunnel for this route, actual /mcp404 verified |Same shared foundations plus authenticated Chat-side MCP connector/reachable service; tested dedicated tunnel adds a process/network dependency |
| Maintenance consequence |Retains useful connector-free coverage at the cost of a second adapter policy |Primary development path for full native control and separate Chat renewal; keep shared native execution/history/config/host code and retire redundant transport-specific policy only under H |

**Connection combinations and evidence boundaries at D2.** Execution
control, return transport and entrypoint are separate axes. All four use actual
ChatGPT GPT-6 High intelligence and upstream native execution/history. “No MCP”
means no Chat-side connector/return transport, not offline or restricted native tools.

| Control + return | Relation to the user's proposal | D2 Codex-entry evidence | Unverified capability / possible value |
| --- | --- | --- | --- |
| model_adapter + local_function | Native model-loop adapter; no generic Operator Host management surface | Five same-saved-Chat development turns, source-authored native compaction, distinct saved successor and original sixth result; all common/regression/Stop audits pass | Connector-free development. Generic ordinary ChatGPT-web-first control not provided. Results continue through additional app requests |
| model_adapter + MCP | Previously discussed third way: native model loop with original MCP result return | Same full six-task contract and independent audits on the exact common candidate | Already avoids per-result app resubmissions. Its native-loop role should be reviewed before G/H retirement; generic Chat-first session management not provided |
| operator + MCP | User-proposed generic native-session workflow through the shared Operator Host; both intended entrances have B synthetic evidence | Same full six-task contract, source-authored codex_handoff, saved receipt/successor and original return; C/C2 recovery remains scoped to earlier inputs | Ordinary human ChatGPT-web-first development/handoff/recovery lacks equivalent real acceptance. Requires reachable MCP connector/service; broader lifecycle remains G |
| operator + local_function | Same generic Operator Host with connector-free app gateway; now implemented | Same full six-task contract and independent audits, source handoff/receipt/Stop, no Chat-side connector/tunnel | Local continuation submits another app request per control result. Ordinary ChatGPT-web-first local discovery remains deferred. Review request overhead and retained failure history before selecting connector-free delivery fallback |

D's two-route table above is historical. D2 below supersedes its previous missing
all-combination comparison. The table's Chat-first gaps describe the D2 boundary;
later direct-entry acceptance, completed E design and bounded F integration are recorded under
[the agreed bridge structure](#agreed-bridge-structure-and-e-work).
App starts, saved Chats, user turns, model tool-call
messages and charged allowance remain different measures. Fewer app submissions
are a property of the MCP return path in both tested control modes; they are not
an operator-only advantage. D2 establishes equal bounded functional acceptance,
not a general intelligence, latency, billing or long-duration stability ranking.

At D, the choice follows native work/context continuity and fewer observed app submissions,
with the additional MCP dependency covered by the deliberate local-function option.
This comparison covers only model_adapter/local_function versus operator/MCP.
The existing model_adapter/MCP return path also settles results on the original MCP
call without another app submission (src/mcp_bridge.rs); fewer app submissions are
not an operator-only property. D has no direct comparison against that third
combination. Preserve its distinct native-loop coverage and inspect shared code
before any G/H retirement decision; do not infer redundancy from23 versus2.
It does not assert that model compute, charged allowance, total latency or total
maintenance cost decreases by the dispatch ratio. Actual model-quality causality
is not established by these two generated implementations. The observed CSV-limit
failure is retained without a supervisor source repair. First-task native user→final
latency is531.087s local/407.921s operator: one valid paired sample, not a general
speed winner. Second-task observations172.954s/183.526s are retained but excluded
from paired latency conclusions because of the supervisor overlap below.

Independent decision proof is phase-d-route-decision-review.json; owned fixtures
are /data/data/com.termux/files/usr/tmp/d.n3y8udaf. Both routes use immutable
2edc2215 and byte-identical baseline files/prompts, independently matched native
runtime hashes and actual saved-backend gpt-6-thinking/extended (GPT-6 High).
Local native thread01a1555a-6904-77f3-b898-f28e990561e5 uses saved Chat
6ad65cd2-1978-83ee-96ab-b245c38e0d52. Operator native thread
01a15569-cf0a-75c0-8a28-33e3eb7a92ac uses saved Chat
6ad660bb-91a8-83ec-8ab3-9721178da569. Each has two genuine user submissions in the
same saved Chat. All40 accepted original native execution calls/results settle;
app dispatches25, consent replies0. Saved Chats remain readable after local-cache
release. Actual allowance debit/title-model requests remain unmeasured; no screen
input, Pro or GPT-5.6 sample was used.

Supervision limits are part of this proof. A status-envelope observation error
and erroneous relaunch submitted the first prompt to unintended native thread
01a15563-4ca0-7773-9f6d-cbacd92b1fc6 /saved Chat
6ad65f10-01e8-83e8-8af4-3f929ad813cb. Its three original read/check commands all
returned and made no product patch; native interrupt and app Stop were confirmed.
One stopped gateway proposal was never dispatched natively and remains in the
preserved stopped journal; do not replay it. Its four app dispatches are excluded
from the accepted pair and included in actual total29. Initial24-dispatch budget
was explicitly revised to32. The normal second local turn still completed with
its original native final despite the observation-limit error; no recovery Chat
request was needed. Initial local CLI final was checked live, but its full raw
screen was superseded during resume; canonical records and the later CLI view
remain. Local cleanup reply timed out after disposal; saved Chats were re-read
independently without repeating the mutation. The operator keeps its same CLI,
passes positive native-idle/app-settlement checks, and both native/host Stops are
confirmed. At D acceptance, both trial Providers, the dedicated tunnel and both
CLIs were stopped; the Linux desktop host/profile, saved Chats, native homes/projects/journals and all
earlier proof are retained. One-time submission markers now fence sampler reruns.

**Current owned connection readiness (after accepted unattended A qualification).**
Current restored connection info/environment:
/data/data/com.termux/files/usr/tmp/d3.uplix9nz/prior-service. This retains the
previous d3.ivyzs628 prior-service destination and the old d3.tvxubwxa
normal_operator binary/state/home/workspace. The dedicated codex-provider-rust-trial
alias is restored under d3.uplix9nz/prior-service/profiles. Official cache Refresh
and real backend MCP readiness passed on restoration. Independent readback after
the final notice confirms exact live Provider kernel image/PID, healthy/ready owned
tunnel and previous native work notLoaded/released-after-stop/no pending/human
requests. See connection-restored.json and final-restored-readback-private.json.
No qualification controller is running; PIE408 has exited. Both trial Providers,
private native servers and app observers ended/released; all saved Chats, projects,
effects and earlier evidence remain. Before further action recheck ownership/
current kernel state, not historical PIDs/ports. No new Chat or trial was started
by the final audit. The following restoration paragraph is historical evidence.

**Prior owned connection (after first D3 failed-start review).** The original D trial binary,
state, disposable CSV-ledger workspace and home were restored through the same
codex-provider-rust-trial tunnel and exact owned Termux Codex Provider connector.
Current info root is /data/data/com.termux/files/usr/tmp/d3.mju0k2wj/prior-service;
inspect its info.json, connection-restored-after-review.json and current kernel/
tunnel state. The former d2/restored-connection and dr.43cajaz7 PIDs are historical,
not readiness proof. Original file credential reference is reused; it is not an
Admin API key. Local tools/list exposes14 actions and read-only codex_sessions/list
succeeds. D3's candidate app Refresh returned HTTP424; it was not retried during
restoration. The earlier D2 backend14-action Refresh readback remains historical.
The D3 restoration made zero model requests
and no new Chat, task, desktop restart or installed/default change. Linux host,
profile, saved Chats, projects and all stopped D2 work remain preserved. The user's
earlier web Refresh no longer displayed a failure, but its button HTTP response/explicit
success toast were not captured; keep that observation separate from successful
backend Refresh and14-action readback.

**D2 — equal Codex-entry comparison: complete.** All four combinations are now
implemented and pass the predeclared development/handoff gates. Additional
numeric edge probes below retain generated-code defects; passing these gates does
not imply defect-free or identical generated-code quality.
The shared local Operator Host supplies the same controls as MCP using the app's
kwargs/params gateway ABI, unique per-driver function names, correlated original
returns, bounded pre-effect JSON correction and Stop/controller fences. The user
proposal is the generic Operator Host capability; it is not tied to MCP. Both local
routes use the selected ChatGPT model's intelligence.

All matched runs bind immutable candidate
9fefba99a270b4d899162bb14f9e16c51e228b8fef63d2b94f8a9824f1cefa5c,
identical signed native Core/runtime/frontend, account/app, baseline source and six
user prompts, actual saved gpt-6-thinking/extended (GPT-6 High). Serial accepted
order is native local adapter, operator/MCP, native MCP adapter, local operator.
Each completes five real development follow-ups in one saved non-temporary Chat
and a sixth feature through a source-authored summary in a distinct saved successor,
on the same primary native thread/original return. Source summaries exactly match
saved source readback and canonical compaction/handoff; receiver acknowledgment or
receipt precedes successor effects. Native adapters compact after turn5; operators
renew during original turn6. Compare those handoff mechanisms separately.

| Route | Full project tests by turn | Original native calls/results | First five native task seconds | Sixth native task seconds | Observed app requests | Saved model tool-call messages |
| --- | --- | --- | --- | --- | --- | --- |
| model_adapter/local_function |7→14→20→26→33→40|36/36|662.600|123.961|43|36|
| operator/MCP |9→18→24→33→42→50|50/50|710.776|275.082|7|98|
| model_adapter/MCP |9→19→25→33→43→49|48/48|559.587|191.980|7|56|
| operator/local_function |9→19→25→33→43→52|39/39|1045.770|256.115|98|91|

Every route passes all seven final common functional groups and six unchanged
regression-module previous/current failure/success audits. Original requirements
and prior tests remain byte-identical. Actual diffs, full suites, original CLI and
canonical finals are independently checked. Every original native call/result is
one-to-one with zero missing returns; final native idle/jobs0, applicable native/
host Stop, Provider/tunnel/private native server exits are confirmed. Two saved
Chats per route remain readable after cache release. Generated test counts are
not a quality score. Additional identical post-run numeric probes find real
generated-code differences: both local outputs emit nonstandard JSON Infinity
(exit0) for 1e308+1e308, while both MCP outputs reject it with exit2. Adding 10^400
and1.0 produces exit1/traceback in both local results and operator/MCP; native MCP
adapter rejects with exit2/no traceback. These are retained quality defects beyond
the seven predeclared common groups, not supervisor-repaired away. Native MCP's
result handles both boundaries better in this sample; this is not a general model
intelligence or transport-quality causality claim. Evidence: additional-quality-probes.json.

Canonical task_started→task_complete duration is the common timing metric. Setup,
external review/waits, manual compaction and teardown have separate UTC/monotonic
records. Adapter observed compaction: local25.105s, MCP28.336s. Observer query wall
and sampled Provider CPU/RSS are recorded separately; observer work overlaps task
time and must not be simply subtracted. App body bytes omit MCP tool HTTP traffic;
request, native call/result bytes and saved model controls are distinct proxies.
Neither actual money, billed tokens nor allowance debit was observed. Native
adapter byte/4 token estimates and operator transport0 are not comparable billing.
One generated run per route with serial/uncontrolled service load supports bounded
observations, not general speed/model-intelligence/stability causation.

**D2 control/transport and actual app-output follow-up.** The local adapter's43
requests are7 starts (including native summary/successor) plus36 execution-result
continuations. Local operator's98 are7 starts plus39 execution results and52
control results: attach14, history7, tools6, report17, finish6, handoff/status2.
The55-request increase is3 more actual executions plus52 explicit controls, not
an observed retry storm. Both MCP modes instead continue these tool/control
round-trips within their seven app requests; saved model tool-call messages still
differ56 vs98. Equal app-request counts do not mean equal inference/token/billing.
The model-adapter exposes native model/tool responses; Operator Host additionally
exposes session/history/receipt/report/finish/handoff controls to that same model.
Actual execution/reviewer/history remains native in both; Host is not another AI.

At the user's request, opened all12 saved D2 Chats in the existing owned Linux app,
verified exact selected conversation IDs, scrolled lazy content and reconciled55
saved public final answers by substantive text/table cells. All matched routes'
source1–5/handoff and successor6 public answers render; Korean/tables/final markers
are visible. Preliminary14d source/successor and both retained failed sources also
open. The843 ABI attempt has no accepted saved Chat to open. All original saved
public answers remain represented; UI inspection submitted zero model requests.
The a78 source displays a fourth-turn incomplete notice. First9fef failed source
retains the fifth request/partial progress and explicitly renders **Unusual activity
has been detected from your device. Try again later.** with a Retry button. Retry
was not clicked. This adds a concrete app error to the earlier generic stream
failure; its root cause/rate/context/allowance causality remains unestablished.
Do not claim every attempted test completed or rendered without error. The four
accepted matches render successfully; negative stream output remains failed.
Owner-only screenshots, text reconciliation, request decomposition and scripts are
in acceptance/development-audit-1t4txdwt/phase-d2-ui-followup; its report.md and
review.json supplement the immutable original D2 comparison evidence. The passive
inspection hook is restored, and the successful local-operator successor is left
open; desktop, saved chats and the original restored trial connection are retained.

The user requested combination-based saved Chat titles. All12 titles now use
D2-어댑터mcp, D2-어댑터local, D2-오퍼레이터mcp or D2-오퍼레이터local,
followed by :1~5 for the original Chat or :인계후6 for its successor. Preliminary
MCP Chats add ·예비14d. Failed local-operator Chats are
D2-오퍼레이터local:4턴실패·JSON·a78 and
D2-오퍼레이터local:5턴실패·비정상활동·9fef.
Server titles and actual app headers are independently verified; conversation
mappings are unchanged and no model requests were submitted. Current title/ID
mapping and preserved prior titles/proof: acceptance/development-audit-1t4txdwt/
phase-d2-chat-titles/{titles.md,results.json,combination-titles}. The successful
D2-오퍼레이터local:인계후6 remains open.

Detailed adapter/MCP vs operator/MCP turn review is in acceptance/
development-audit-1t4txdwt/phase-d2-mcp-detail/{report.md,turn-detail.json,
turn-detail.csv}. First-five native duration559.587s vs710.776s; both29 commands
and15 patches, with write_stdin4 vs6 accounting for original48 vs50 calls. Saved
model tool messages56 vs98 include different catalog/control surfaces. The adapter's
six native tasks plus separate observed compaction total779.903s; operator's six
tasks including in-turn renewal total985.858s. Source-authored handoff orchestration
and final app settlement are separate measurement boundaries. Retained evidence
also identifies adapter test-fixture JSON correction in turn5 and operator CSV
text/raw newline-test correction in turn4; neither changed prior tests. No new
inference/development trial was performed for this detailed comparison.

Both MCP modes use7 app requests for six tasks plus handoff. This confirms that
per-result request savings are not operator-exclusive. Local operator now supplies
the generic controls without a Chat-side connector/tunnel, but its additional
control results incur app requests. D's operator/MCP delivery direction remains
reasonable for generic control; native MCP adapter's equal functional acceptance
and request savings support the subsequent Codex-entry target above. Retain
adapter/local as the explicit connector-free choice, subject to applicable
G lifecycle proof; do not infer that implementing local operator automatically
retires native local adapter. Installed/bare-config defaults stay unchanged.

Private root: /data/data/com.termux/files/usr/tmp/d2.jucx6dti.
Durable comparison: acceptance/development-audit-1t4txdwt/phase-d2-comparison-review.json
and phase-d2-evidence/{report.md,comparison.json,turn-timings.csv,matched-audit.json,
evidence-manifest.json}. Candidate/source hashes, prompts/baselines, phase/task
boundaries, independent reviews, generated snapshots/diffs and gate logs are retained.
Private raw records remain owner-only; no credential values are published.

Retained preliminary/negative attempts count toward actual use, not matched ranking:
14d native MCP adapter completed6 tasks with7 requests;843 local-operator ABI failure
used1 request with zero native effects/no accepted saved Chat; a78 local operator
accepted3 turns then failed4th on nested JSON,59 requests/all native calls returned,
native/host Stop confirmed. The first9fef local operator accepted4 turns then failed
5th with generic app_stream_failed after72 requests: its29 tests pass but original
native final is absent, so turn5 is not accepted. Cause is unestablished. All33
native calls returned, native Stop/no jobs are confirmed; remote Stop is unconfirmed.
Its fenced inactive dispatcher was narrowly released after service exit and saved
source readback, without deleting the Chat store or replaying uncertain work.
The843 remote Stop is also unconfirmed. Preserve both negative states separately
from the four fully accepted runs. Model JSON correction and stream failure are
distinct; the latter is not proven to be quota/context exhaustion.

Actual total:40 genuine user tasks,294 observed app submissions (declared maximum400),
12 saved Chats collected including failed sources. Supervision interruptions were
reconciled against canonical completion/idle/original
returns; already submitted tasks were not replayed. A teardown-resume harness lost
its in-memory thread ID; restoring the recorded same ID completed cleanup with
zero new model requests. The final fixture's initial folder-trust omission was
corrected before any user/model submission, preserving the startup capture. No
screen computer-use, Pro, GPT-5.6, public deployment, default/device activation or
desktop restart occurred. Real context saturation/compaction-policy review remains
deferred; assuming full context proves this handoff boundary, not a capacity trigger.

**Current D3 — restarted from the beginning at the user's explicit request.**
The user first requested resume, then explicitly replaced that with “처음부터 해 d3”.
Current root: /data/data/com.termux/files/usr/tmp/d3.mqvjkjbb; cache:
acceptance/d3.mqvjkjbb. Initial detached PID11421 ran frozen run.py from rewrite/rust-core
HEAD2d81dd74b61e0469f2fe68a63404f70faf3c958f plus preserved uncommitted changes.
Four fresh identical baseline projects/native homes run normal adapter3 tasks,
normal operator3, fault operator2, fault adapter2. Accepted9fefba99 candidate and
exact product inputs are retained. Expected10–14 app dispatches, cap24,10 genuine
work tasks; actual billing remains unobserved. The project is the existing private
Termux Provider: D3, not a newly created duplicate. Each case keeps its own primary
thread/saved Chat, with only positively reconciled failure successors admitted.

The repaired Refresh includes official app cache invalidation and one actual
backend MCP readiness sequence; repeated successes are not required. Fifteen
local zero-model setup groups pass; fresh preflight verifies kernel host/service/
tunnel ownership, prior native notLoaded/released-after-stop and no pending calls.
The prepared native/host Stop proof is explicitly bound to the actual previous
connection, not an old unrelated PID. Original connection restoration also runs
this readiness check after the last case. Exact notifier/same-supervisor receiving
proof is reused; no additional probe Chat/model request. End this supervisor turn
after first-task submission verification. No per-turn supervisory quality review;
all quality checks are batched after all work ends, followed by terminal notice
and independent final audit. Do not supervise or restart an active run.

The previous d3.tvxubwxa run is superseded and fully preserved. Its paused owned
controller PIE677 was ended via identity-checked pidfd only after13 original
native calls/results, idle and0 jobs were reconciled. Its owned CLI and native
servers were stopped; saved Chats, modified projects, canonical records and costs
remain. The old idle app observer was disposed without removing conversations.
The live previous candidate Provider/tunnel was retained until the fresh run's
owned handover; it is the exact restoration destination in the new prior-service.
Do not resume any old controller or treat a delayed old notice as a new failure.
See d3.tvxubwxa/ended-for-user-requested-fresh-D3.json and
superseded-by-user-requested-fresh-run.json. Earlier incident diagnosis below is
historical evidence, not the current run's status.

**Earlier D3 terminal result (preserved) — normal development passed; unattended recovery failed.**
All10 genuine tasks in the fresh d3.mqvjkjbb run are terminal. Original initial
and recovery-1 result/failure notices are preserved; do not relaunch or resend
uncertain/failed requests. Authoritative final review:
~/.cache/codex/acceptance/d3.mqvjkjbb/final-review.json, with source evidence and
independent-review.py under the trial root. Both normal routes completed3
successive development tasks automatically in one saved Chat and native thread.
Their exact snapshots pass12/22/25 independent CLI cases and2 external Rust API
tests per stage, original/accumulated tests and previous-product red checks.
Adapter accumulated tests:10/19/29; Operator:7/14/20. The8-second probe ran once
per normal route; its original write_stdin session returned the result. Frozen
inputs/previous tests, actual diff, canonical finals, original CLI timing footer,
saved Chat finals/project/title/content preservation all reconcile.

Fault Operator A1 executed its effect once and withheld the exact actual result.
Its truthful source Chat ended without codex_finish; native reported
operator_app_round_incomplete/systemError while host stream_settled. Supervisor
repaired the controller's too-narrow expected-cut recognition and reconnected only
the owned tunnel. Original A1 was never replayed. Genuine A2 then recovered the
original result on the same primary thread, wrote recovery-note.txt and passed
checks in a successor saved Chat. This is assisted recovery, not an unattended
success. The original failure, result-before-controlled-cut-resume.json and all
costs remain. The two fault routes did not receive equal reconnection treatment;
this run cannot establish an intrinsic recovery winner.

Fault Adapter A1 also executed exactly once, with matching durable effect, original
canonical return and withheld native_tool receipt. Its source Chat could not read
the terminated MCP connection. Genuine A2 in the same Chat/thread again received
Session terminated and truthfully reported failure; no recovery note or probe
re-execution. The controller additionally sampled active immediately after the
canonical task_complete, then interrupt returned no active turn. Later readback
confirms idle/0 jobs/idle app/no pending calls. terminal-reconciliation.json and
complete-2.json capture this failed terminal outcome without erasing failed-2.
This state publication race explains the controller stop reason, not the actual
MCP recovery failure.

Batch quality audit ran after all submitted tasks ended. Its first aggregate
misclassified clean git diff --no-index --check exit1 as failure. Actual external
and regression gates were already green; independent final review reuses their
exact snapshot hashes and corrects only the diff interpretation. Original
batch-quality-review.json is retained. Harness now checks both exit code and
absence of whitespace/error diagnostics, supports qualified Adapter id/name wire
receipts as well as Operator original call_id payloads, and admits a bounded Idle
publication wait only after that exact turn's positive terminal event.19 local
setup groups pass with0 model requests, including negative guards. Frozen live
run.py/run-revision2.py and candidate inputs are not rewritten; no new Chat test.

Fresh run totals:10 /f/conversation starts,5 saved non-temporary Chats,71 unique
original native calls and71 returns,0 unreturned/duplicate effects/replayed
requests. Both normal routes used3 dispatches each; both fault routes2 each.
Combined detached controllers used35.211 CPU seconds, peak40944KiB RSS,
127093 kernel wakeups and25 Operator boundary status calls/2.191 seconds.
These counters exclude supervisor repair/final-review token cost; actual billing
and allowance debit remain unknown. Final D3 acceptance is false: normal sequential
development is accepted, fully unattended result-loss recovery is unfinished.
The then-next work was the terminated MCP connection recovery lifecycle and
controller boundaries; the accepted fresh unattended qualification below now
completes that bounded requirement. Preserve this negative record and normal
green proof; historical notices never justify replay or a duplicate run.

**Unattended loop qualification — first turn through final notice.**
User clarified that a paid-task stop followed by supervisor code repair/resume
is not the requested loop. d3.ivyzs628 finished all4 A tasks and its automated
checks passed, including both real A2 recoveries; its initial guard fix/resume
remain explicit intervention. assisted-terminal-outcome.json preserves this
negative unattended result. Earlier B/C green proof and all A failures remain;
no B/C replay or rewriting an assisted run as unattended success.

Fresh A-only root d3.uplix9nz starts with two new identical disposable projects/
homes and the unchanged9fefba99 candidate/26 D2 product inputs. It runs Operator
A1→automatic recovery→A2, then Adapter A1→automatic recovery→A2:4 genuine tasks,
expected4 app starts/cap6 and4 saved Chats in Termux Provider: D3. Titles include
combination/automatic reconnect/무감독시험. No Pro/GPT-5.6, screen input, separate
executor or speculative planner. Preparation/probes start0 models. The existing
same-supervisor/Core/notifier receipt is reused without another probe Chat.

The single frozen controller now handles the tested recoverable boundaries:
- Exact durable effect, original canonical call/result and withheld wire hash
  reconcile before reconnect. Both still-running and exited owned tunnel states
  must match the original admitted profile/destination/command registration.
- It issues at most one Stop when needed and one Connect. A lost reply is resolved
  by read-only observation of the exact owner/new registration and readiness;
  an unknown effectful request is never repeated. Readiness lag has bounded local
  re-observation. Changed owner/destination/command stays a terminal failure.
- Operator explicitly fences its old driver with positive native/host Stop and
  uses existing recovered native receipts in a successor Chat. Adapter uses the
  existing checkpoint serializer plus one inject_items public-history handoff.
  Both preserve the source Chat and original primary thread, never replay A1.
- Only that exact native turn's task_complete/turn_aborted allows progression;
  intermediate error events and old-turn completions do not count as completion.
  Evidence capture is cheap; quality checks happen once after all work ends.
- Final checks include original calls/returns and exact genuine-turn counts,
  affirmative completion/timing, single effect/recovery note, snapshots/frozen
  inputs, saved Chat/project/title, positive Stop, cleanup and prior restoration.
  Mid-run controller/product/prompt/config changes fail the sealed-input check.
- Success explicitly reports unattended_control_accepted only for a fresh run
  with green gates; supervisor_resume_used and sealed_inputs_unchanged are recorded.
  Final semantic review is one terminal independent audit, never per-turn control.
  Terminal failure preserves evidence and sends one final notice without replay.

28 zero-model local groups pass on the exact sealed controller. They exercise the
real full controller/proxy/one-time effect/stdio/checkpoint/recovery/A2/restoration/
batch path for both integrations, with external native/tunnel/Chat observations
explicitly doubled; they do not substitute for actual Chat acceptance. Negative
coverage includes changed owner, live/dead owner, lost Stop/Connect reply, unknown
reply without receipt, readiness lag, intermediate/old-turn events, denied success
text and mid-run input modification. New registration checks also accept both
retained actual reconnect snapshots from d3.ivyzs628. **Actual unattended A proof: accepted.** Detached PIE408 ran the sealed run.py
from2026-10-20T10:01:16.261158Z to10:21:47.987171Z (1231.726 seconds). One
controller submitted Operator A1/A2 then Adapter A1/A2, with no run_resumed,
case_resume, preserved_stop, mid-run repair or supervisor instruction. A1's
intentional result-delivery failure is preserved, not relabeled source Chat
success. Each automatic recovery reconciled the exact original canonical/wire
result and single effect, fenced/retired the old Chat as appropriate and issued
one owned tunnel Connect. Recovery after A1 capture took3.022 seconds Operator
and2.599 seconds Adapter. The same primary native thread continued in each case;
source and successor saved Chats remain readable.

A2 actually read its original execution/history, verified fault.effects equals
exactly fault LF, created recovery-note.txt through native apply_patch, checked
its actual diff/content and returned an affirmative completion/timing footer.
Operator original2 tests pass with explicit effect/note byte checks; Adapter adds
one regression, sees its missing-note failure then passes3 tests. Existing product
logic and original tests remain unchanged. No extra regression-count requirement
or quality advantage is inferred. Snapshot/final workspace hashes, protected
inputs, original native call/return IDs/output, original CLI footer, saved Chat
final/model/effort/title/project/content and positive Stop/0 jobs/observer release
all pass the independent final audit. No repeated green test or Chat start was
needed for final review. Sealed inputs also pass after execution; candidate remains
9fefba99 and all26 D2 product source inputs match.

Observed totals:4 app dispatches,4 saved non-temporary Chats,14 unique original
native calls and14 returns,0 unreturned/replayed effects. Operator6 calls/results,
Adapter8. Each probe ran once; each route reconnected once. Native-final observed
times A1/A2: Operator424.917/142.104 seconds; Adapter387.106/159.331 seconds. These
include the deliberate delivery-loss wait and do not rank general model speed.
Controller CPU17.012485 seconds, peak41128KiB RSS,47881 wakeups,5 Operator boundary
status queries/0.579439477 seconds. No supervisory model work between tasks;
initial setup/start confirmation and one final audit remain outside that counter.
Actual model tokens/allowance debit remain unknown.

Full result and reproducible final audit:
~/.cache/codex/acceptance/d3.uplix9nz/final-review.json and final-review.md;
trial-root final-independent-review.py. result.json retains its original automated
outcome; final-review.json is independent semantic acceptance. Terminal notice
b24442f028e14e8c81d3117b23026bdb arrived on the original bound supervisor session.
The earlier d3.ivyzs628 assisted result and all prior failures/costs remain separate.
No further A retry is required for this accepted input/scope. Current F integration
precedes G delivery; do not repeat green B/C or A for a fresh status check.

This proves the requested bounded unattended loop for one declared result-loss
EOF per MCP integration. Arbitrary network/device/process loss, real lost
Stop/Connect responses and indefinite autonomous recovery are not established by
this Chat run. Lost local replies/ownership guards have local fixture proof.
Saved output was inspected through backend data, not desktop screen rendering.

**Connection repair and bounded proof.** scripts/d3_refresh.mjs previously did
POST refresh_actions and metadata GET only. The installed app's actual Refresh
onSuccess also invalidates plugin/app/system-hint/mounted-capability queries;
the helper now performs that same cache refresh. It additionally requires one
real backend MCP tools/list probe and, for Operator, one actual read-only
codex_sessions list responses with a validated work array. These calls use
/ecosystem/call_mcp without starting a Chat/model/native action. Failure preserves
private diagnostics and prevents a model start; a metadata match alone is no
longer readiness. Exact helper passes on the owned14-tool connection. Separately,
20 read-only calls all passed,1.112–2.656 seconds each,0 model requests. Evidence:
acceptance/d3.tvxubwxa/connector-diagnosis/review.json and repeated-read-probe.json.
This repairs a verified setup gap and proves current bounded connectivity; the
original task1 TimeoutError is still only reported by the saved Chat, without a
retained failed MCP response/request trace. Do not claim its precise cause or a
general reliability cure from these passes. No credential/role/default/device
change or desktop restart was made.

The current candidate Provider/tunnel is idle and available for connection probes;
inspect normal_operator/info.json and kernel state rather than a saved PID. All13
original native calls/results from operator B2 reconcile, its native completion
at2026-10-20T06:45:50.985Z preceded the diagnostic Stop, and native is idle with0
jobs. Thus B2 was not aborted by that Stop. Its saved Chat and the failed source
Chat were renamed with the requested D3/combination/failure labels through backend
APIs, preserving conversation content. Development quality is not reviewed yet.
The old controller remains held; do not blindly SIGCONT it (its frozen helper and
boundary assumptions predate this repair). Original run/refresh sources remain
unchanged. refresh-with-readiness.mjs and connection-readiness-repair.json pin the
new helper; a freshly qualified controller selects it for subsequent cases.
Resume must preserve completed task2/current effects and submit no failed task1
again. At this intermediate D3 checkpoint, further work was pending connection
repair. The later completed D3 verdict in the phase table supersedes that backlog;
the original failure and its evidence remain preserved.

**D3 — same-thread continuation; quality review after all work.**
The active root remains /data/data/com.termux/files/usr/tmp/d3.tvxubwxa.
Current detached continuation PID31732 runs run-revision3.py, recovery-2.
Adapter tasks1–3 finished without intermediate quality review. Operator task1
ended with operator_app_round_incomplete after actual Chat tool discovery returned
TimeoutError, before any native call or workspace modification. The original
CLI/canonical error, failed saved Chat, unchanged baseline and native/host Stop
with0 jobs are preserved. This is an actual failed task, not a passed result.
After preserving it as complete-1.json with explicit failed outcome, the verified
stopped owned Provider was restarted on the same state/home/workspace and Refresh
succeeded. The failed request is **not replayed**: continuation starts the next
frozen genuine request, task2, on native thread01a15801-97c5-7522-9532-a56c945ecb11.
Any successor Chat and the failed first dispatch remain visible in final cost and
continuity assessment. recovery-2 archives previous service/evidence; its pinned
inputs and exclusive launch markers prevent duplicate launches. Fourteen local
zero-model setup groups pass, including explicit terminal-error handling for the
declared result-cut case without admitting ordinary errors as success. Final
quality review still waits until all remaining work ends. Completion notice uses
the unchanged notifier and recovery-2 destination to this same supervisor thread.

Earlier continuation PID30588 ran run-revision2.py; original PID31817 exited.
Adapter task1 completed on native thread01a157e9-9f90-7693-a70a-3f7250aae5fb
at2026-10-20T06:08:44.731Z, native duration231.752 seconds,10 original native
calls and10 results. The external checker incorrectly assumed nonpositive transfer
parsing must succeed; the model correctly rejected it earlier. Corrected checker
proof passes on the unchanged model project. This was a validator defect, not
an established model development failure. Original result/logs/checker and unique
work are preserved; acceptance/d3.tvxubwxa/before-validator-repair archives them.
validator-repair.json and recovery-1/inputs.json pin the repair and resume inputs.

The latest user explicitly requires one quality review after **all work ends**.
The controller now only reconciles canonical completion/original results/native
idle/jobs/host state and captures snapshots between tasks. External contracts,
regression red/green, whitespace and saved-Chat quality checks run as one batch
only after all four cases finish; each quality failure is recorded without
preventing review of the other completed tasks. Model-side development tests
remain part of its requested work. Thirteen zero-model setup groups pass,
including proof that the real controller sends the next task without intermediate
quality review and an early-parser-rejection positive control.

After positive idle/no-job/10-of10-result reconciliation, only the orphaned owned
CLI client PID2536 was stopped using pidfd and reattached to that **same native
thread**; the native server, Provider and saved Chat were preserved. No task1
replay. Task2 was submitted at2026-10-20T06:22:30.643162Z. That continuation completed adapter tasks2–3, then stopped on the operator
connection error described above. The current recovery-2 notice supersedes its
already-delivered recovery-1 notice. Do not supervise subsequent turns. End-to-end
final comparison is pending.

The earlier Refresh recovery succeeded on attempt2 after a definite424 MCP
connection timeout. scripts/d3_refresh.mjs retries only definite424 metadata
failures up to3 times and independently reads back exact2/14 actions. Unknown
outcomes, auth changes and native effects are not retried. Candidate/Core/runtime,
starting projects, prompts/order and10-task/24-dispatch envelope are unchanged.
The user's continuing authorization is not revoked by the notifier's generic
no-blind-restart reminder; reconcile state and fix known harness/setup failures
within scope without replaying uncertain effects.

**D3 first-start evidence (preserved).**
The user explicitly authorized completion-notice implementation/demonstration,
followed by the actual D3 start. Same-thread canonical notice receipt on a new
native turn was verified at2026-10-20T05:52:55.291053Z. The detached D3 runner
was then started once (PIE014) and exited after26.027s. The candidate Provider and
tunnel started, but app connector Refresh returned HTTP424 at05:53:24Z, before the
native CLI, primary thread or first task submission. Do not start it again.
Private prepared root is
/data/data/com.termux/files/usr/tmp/d3.mju0k2wj; durable setup evidence and report
are /data/data/com.termux/files/home/.cache/codex/acceptance/d3.mju0k2wj.
The Provider remains the D2-qualified9fefba99 binary, with equal exact Core,
runtime and frontend hashes for all four disposable fixtures. No live service,
installed/default path or public asset was replaced during preparation.

Independent failed-start audit: app observer readback has0 dispatches/0 saved
Chats, all four projects exactly match the frozen baseline including file sets,
and no submissions/native thread/effect files exist. Original CLI transcript and
turn output do not exist because the CLI was never launched; there are no model
answers or development diffs to review. The planned A/B/C comparison remains
unperformed. Native stdio initialize/tools/list succeeds both directly and through
the same owned fault proxy (2 adapter actions); this does not establish why the
remote Refresh returned424. No Refresh/task retry or new trial was performed.
The local setup proof did not include a successful actual D3 connector Refresh;
that readiness gap remains explicit.

The actual failure notice was enqueued at05:53:26.809244Z and appeared as the exact
canonical user input in this same supervisor thread on receiving turn
01a157df-7176-7be0-88ba-d9441d8aa276. notification-result-ack.json and durable
notification-result-live-review.json independently confirm receipt and that the
result file preceded the notice. Both probe wakeup and actual failure wakeup work;
no per-turn supervisor model polling occurred in the runner. Recorded runner-only
CPU0.152454s and peak RSS40,272KiB exclude children/Provider/Bun and Codex review
token costs. Actual Codex token/allowance debit was not independently totaled.

After verifying0 app lanes/requests, no native work and exact current ownership,
the reviewer stopped the candidate Provider/tunnel once, disposed the empty app
bridge and restored the original binary/state/home/workspace connection. Fresh
local14-tool/session-list and tunnel ready/healthy checks pass; Linux desktop was
not restarted. All files/projects/profiles/chats remain preserved. Restoration is
not a D3 retry; connection-restored-after-review.json records the exact outcome.

| D3 item | Equal task / acceptance | Planned cases |
| --- | --- | --- |
| A | Cut owned MCP stdio only after the exact probe's durable one-time effect and original native/MCP result exist, before forwarding that MCP response; reconcile canonical calls/results, then a genuine follow-up writes a recovery note without rerunning the effect | Operator/MCP, then adapter/MCP; 2 declared genuine turns each. A missing response is not success or rollback. A2 requires affirmative reconciliation; unknown state stops rather than replaying A1 |
| B | Dependency-free multi-file Rust ledger: transfer and whole-batch rollback, then strict persistent state with atomic replacement and preservation on error | Adapter/MCP, operator/MCP; 2 sequential genuine development turns each |
| C | Dry-run without state writes plus one8-second native process, short initial yield and write_stdin collecting its original session result exactly once | Third turn of each B conversation; preserve original and accumulated regression modules |

Execution order is B+C adapter, B+C operator, A operator, A adapter. Each case has
an independent identical starting project and native home; within a case all user
turns use the same native thread and saved Chat, except a positively reconciled
failure may require a successor Chat on that same native thread. A negative A
result is useful evidence, not automatically a passed recovery. Unknown native
effects, outstanding results or unconfirmed host state stop further unsafe
submission. Quality/contract findings are collected only after all work ends. The controller requests a bounded owned Stop and records whether it
was actually confirmed; it preserves work/evidence and never retries an uncertain
submission or chooses another route as fallback.

One detached local process submits the frozen next prompt only after the prior
canonical completion and idle/job/result control checks pass. Quality acceptance
is deferred until all cases finish and does not gate the next development turn.
Linux inotify watches appended canonical records; no LLM supervisor, go/finish
file gate, steady RPC/CDP polling or repeated full-rollout read is required.
Operator host settlement uses at most6 boundary status queries, with bounded
delays; adapter checks its persisted lane and positive-terminal source contract.
This removes supervisory model calls between turns, not local CPU/checking cost.
Local CPU/peak RSS, kernel wakeups, boundary query count/time, UTC/monotonic turn
start/final-observed/capture times and final batch-review times, native original calls/results, diffs/snapshots,
external checks and actual app dispatch metadata are retained. Actual billing
and allowance debit remain unobserved. The automated result still requires one
final semantic/native-CLI/saved-Chat audit; it never declares product acceptance.

The declared envelope is10 genuine user tasks, expected10–14 observed conversation
dispatches, hard cap24 observed /f/conversation starts and4–6 saved Chats. This
does not count or establish invisible model inference/billing. A turn has a
1200-second deadline; setup and Stop have separate bounded local timeouts. No
Pro or GPT-5.6. Setup consumed0 requests; task1 consumed1 actual app dispatch
before the same-thread continuation from task2.

The private owned ChatGPT project was actually created and renamed to
**Termux Provider: D3**, ID g-p-6ad6fb100b708191abedab29f23bc660, through authenticated
app backend project APIs with0 model requests and no screen/keyboard/mouse input.
The initial rename applied but strict empty-value readback failed; a subsequent
GET-only reconciliation confirmed the same ID/new name without repeating PATCH.
Existing authenticated app execution context is required; visible screen
inspection is not. Provider project config pins this ID, disallows create-on-miss
and verifies owned private destination. D3 conversation titles will be
D3-어댑터mcp:개발1~3 / D3-오퍼레이터mcp:개발1~3 and corresponding
:결과단절·복구 titles, adding a successor suffix if needed. Titles are patched
through the backend and read back with unchanged conversation-content hashes
after the owned service releases its CDP connection. The adapter case has begun
its saved Chat; title/output readback remains part of the final batch review.

Completion uses scripts/d3_notify.py, frozen as notify.py in the prepared root.
Its destination is the explicitly verified existing supervisor thread
01a15330-0d46-7b51-a460-5e0495030cf2 and its original Core/CODEX_HOME/state namespace,
independent of disposable trial homes. `codex queue` enqueues once without
changing model/effort and dispatches on the existing loaded thread's idle event.
An exclusive durable intent prevents blind retries after timeout/unknown result.
The final result file is written before sending one success or failure notice;
the controller does not ask ChatGPT for another model response to compose it.
Queue acceptance is not receiving proof: a probe must appear as the exact
canonical user input on a different native turn of this same thread. `notify.py
ack ROOT` records that evidence; D3 start requires this ack and the exact notifier
hash. Five local notification tests pass, including duplicate/timeout/foreign
session rejection and separate enqueue/receipt semantics. The actual probe was
accepted at2026-10-20T05:52:18.881345Z, queue item
01a157de-25ae-7701-b509-c689374f014c. Same-session receipt was verified after the
source turn completed. The receiving turn
01a157de-a025-7bd2-8273-88c0f9576cb9 differs from source
01a157d8-d595-7522-9ff1-974e93c7125e; notification-probe-ack.json confirms the
same thread and exact frozen notifier. Durable proof is
acceptance/d3.mju0k2wj/notification-live-review.json. D3 was subsequently launched
once; its terminal notice returns here for final review. Supervisor polling is
not required while it runs.

The probe's receiving turn ran the frozen notify.py ack command and then the
already-authorized frozen run.py start command once. Do not repeat ack, request
authorization again or restart the existing launch. After launch verification
the supervisor response ends; no model polling while D3 executes. Its terminal
result notification requests the final
independent audit and report. If the supervisor thread/app is closed or the runner
is forcibly killed, immediate delivery is not guaranteed; preserve queue/result/
intent evidence rather than silently resending or launching another thread.

Each requested final answer ends with `소요시간: 약 N초 (최종 시계 확인까지)`,
computed from the supplied native monotonic start and a real final clock read.
Operator must include it both in codex_finish's native summary and visible Chat
final. The controller separately appends the completion-observed exact elapsed
time below each turn in turns.md, with failed/unknown elapsed records too; these
two measurements have different end points and are not interchangeable.

Repository setup tools: scripts/qualify_provider_d3.py (prepare/seal/preflight/
start/status), scripts/d3_contract.py, scripts/d3_fault_proxy.py,
scripts/d3_app_setup.mjs and scripts/qualify_provider_d3_setup.py; model starting
project is scripts/fixtures/d3. Fourteen local zero-model setup groups pass,
including65 CLI contract cases; setup-validation/review.json identifies the
exact attempt and source hashes. The durable report is
/data/data/com.termux/files/home/.cache/codex/acceptance/d3.mju0k2wj/report.md.
The setup proof covers external
contracts against both missing-feature and independent positive implementations,
new regression red/green checks, incremental partial/nested canonical records,
real three-turn PTY/inotify/controller progress with explicit local peers,
original-session positive/negative checks, exact-result stdio EOF/no replay,
Termux pidfd Stop and boundary reconciliation rules. It qualifies setup mechanics,
not live ChatGPT behavior. The original owned desktop/Provider/tunnel readiness
was independently rechecked read-only; the candidate currently owns the connection. A fresh start repeats
fresh ownership/idle checks, releases that connection once, and restores the
original binary/state/home/workspace only after all cases settle and close. A
failed/unknown trial leaves explicit reconciliation evidence, never blind restore.

At the D3 boundary, ChatGPT general delegation into the managed path was
unaccepted. It subsequently received bounded F acceptance; direct ChatGPT
two-turn development retains its separate accepted proof above. Real context
saturation/compaction need/policy, wider
approval/question/reviewer boundaries, repeated performance ranking and
deployment/default/device activation remain unfinished. Reuse D2's existing proof;
do not repeat its six-task runs just to prepare D3.

**G is active after completed bounded F integration.** The
[agreed bridge structure and E work](#agreed-bridge-structure-and-e-work) is settled.
F implemented the common managed path/three usage modes, coordinated agents and
matched efficiency evidence. Linux-app removal remains optional research. Use
that evidence to review retained/shared product compositions in G. Apply each
retained path’s applicable acceptance matrix; preserve green exact-input proof and close
remaining host-loss/approval/checkpoint/handoff interruption and applicable context/
TUI/lifecycle/composition gates. Complete signed optional packaging/provisioning,
installer/update/rollback, public readback/disposable update and non-forced promotion
before operating replacement. Device activation remains separate. Provider
source changes remain uncommitted; effective installed/bare-config defaults, public
assets and device activation are unchanged. Do not start new actual Chat requests merely to repeat C2 or improve D's
sample count; any additional G experiment needs its own concrete scope/budget.

**C2 — sequential development and saved-Chat handoff: accepted.** The improved
criteria require five genuine native Codex development user turns in one saved
Chat, with each follow-up queued only after independent review of the previous
actual diff/tests. A sixth native request assumes the context is full; the source
model writes a summary grounded in confirmed native history/context and invokes
codex_handoff with renew=true. A distinct saved successor must acknowledge the
current receipt before actual development and return through the original sixth
native turn/CLI on the same primary work/thread. Both saved Chats must remain
readable after handoff and local-cache release. Use the Rust Provider's app backend
with isTemporaryChat=false, without screen input or the older DesktopLocalTransport.
One prewritten five-part request, prepared summary alone or new primary thread
does not meet these criteria.

The accepted run is /data/data/com.termux/files/usr/tmp/c2.f6g1r8va/attempt-3;
independent proof is phase-c2-continuity-review.json. The preserved project is
/data/data/com.termux/files/usr/tmp/pm.ycwegh4e/w/multiturn-continuity,
CODEX_HOME /data/data/com.termux/files/usr/tmp/pm.ycwegh4e/h/.codex,
workfe789b4578d2a2b7b01b145d4803f7fc84640417103b9142 and primary native thread
01a1544f-af9d-77e0-967e-e61af545f415. The earlier completed parser was retained.
Five real source user turns added aggregation, stdin/file CLI, minimum-count
filtering, JSON/CSV output and atomic output-file replacement; independently
reviewed full project tests progressed11→17→23→29→36. The successor added sorting
and passed43 tests plus all seven independent contract groups. Original requirements
and earlier tests remained unchanged. Actual code/diffs, all original calls/results,
CLI reports/finals and canonical history were independently reviewed.

Saved source Chat6ad6210c-7408-83ee-93b5-8db246a8280a has five completed development
turns plus the sixth handoff request. Saved successor
6ad628b3-949c-83ec-9aa3-ef9392923382 starts once with system-only bootstrap and zero
user submissions. Both actual backend records are non-temporary, report
gpt-6-thinking/extended (GPT-6 High), and remained readable after Provider shutdown,
observer removal and local Bridge/cache disposal. The source-authored canonical
summary and history/context cursor hashes match independently reconstructed inputs.
The successor's exact receipt hash, current-controller journal acceptance and
correlated original MCP reply establish receipt before execution. The saved ChatGPT
backend omits tool-result bodies (can_save=false); those bodies are not claimed
as saved-Chat evidence. All original execution results are retained in authoritative
native history. Source and successor finish on the same primary work/thread, with
handoff and successor coding in native turn01a1548b-7a7e-79f3-9905-992b08dde6f8.

Measured acceptance totals:7 app dispatches (six source plus one successor), zero
consent replies,18 commands/12 patches and all31 original execution-call returns;
zero unreturned calls/jobs. Two earlier failed/partial saved-Chat attempts add two
actual app dispatches, preserved separately: total9 measured dispatches/four saved
Chats across these attempts. Title-model requests and actual allowance debit are
unmeasured. Native and host Stops are confirmed; the exact owned trial Provider
and dedicated tunnel were stopped. The Linux ChatGPT desktop host/profile, saved
Chats, native home/thread, project and older proofs are preserved. No public,
default-path or device activation was performed. Reuse requires fresh owner/history/
pending-call reconciliation and original-MCP readback after any tunnel reconnect;
do not replay queued requests or start a competing CLI/controller.

The post-C audit is phase-c2-post-c-audit.json. A second CDP client's HTTP409 was
normal exclusive Provider ownership, not a requirement for user action. The empty
Temporary-chat UI detour was unnecessary; no prompt was submitted there. UI Instant
and native-home defaults do not establish the actual overridden backend model;
the user's supervising-model complaint was not trial-model mismatch evidence.
The earlier ten-cycle single request was not five follow-up user turns. The
previous/current operator-build comparison also does not replace D's local-function/
operator route comparison. The requested AGENTS principles and earlier C proof
remain preserved. Mistaken Chat-origin workca87baac83d8958defda5c7fe1872785462f8c129bfa6901,
thread01a15427-9abe-7553-a10d-eaeaa5ac847f, had no native calls/app dispatches; its
native Stop is confirmed with no pending calls/jobs and its journal is preserved.

The first actual C2 attempt failed before effects because Provider restart rotated
credentials while tunnel stdio retained its old bearer. The second saved source
completed the parser (seven independent tests/five original native returns), but
its follow-up failed before app dispatch. Both negative/partial Chats and their
actual gpt-6-thinking/extended metadata are retained, excluded from acceptance.
The historical failure's precise app error was not captured; do not infer it from
the later regression. Inspection found that a terminal done event alone could
prematurely admit same-Chat reuse while app submission/stream work remained active.
The correction requires positive app settlement, keeps Stop responsive while
waiting and records safe startup error codes. It removes an arbitrary overall
settlement timeout while bounding each observation and rechecking Stop fencing.

Current immutable correction binary is
/data/data/com.termux/files/usr/tmp/c2.f6g1r8va/candidate/codex-chatgpt-provider,
SHA2562edc2215f4ea66249d36d66752a46eda663f6976852d07cdbf66a3edf13dfd34.
phase-c2-correction-inputs.json binds exact sources and green Provider101 Rust/
116 Bun/36 Python, full358 Rust/135 Python, fmt/clippy/release and actual Android
operator gates. Android explicitly proves delayed settlement preserves the same
Chat and Stop fences a waiting follow-up without late dispatch, alongside both
entrances and retained tool/handoff/recovery gates. C2 now independently verifies
the actual app path. This is an assumed-full handoff; actual capacity detection,
threshold selection, repeated long-duration rotation and the necessity/design of
native compaction remain deferred. D's completed route decision above uses these
results; it does not imply publication or activation.

For the ten-cycle trial, the owned Termux Codex Provider tunnel targeted that fixed
binary and its disposable state. The app's actual Refresh tools API succeeded; independent app
readback matches all14 advertised names and the fixed readOnly=true/other=false
hints (phase-c-annotation-refresh.json). A fresh GPT-6 High task then completed
ten sequential source-read/native-patch/individual-test cycles in TMPDIR, created
API documentation, passed the unchanged full ten-test suite and diff check, and
finished on the same primary native thread. Independent inspection verified25
commands with original outputs,11 patches, all36 native execution results settled
on their original MCP calls,
rendered CLI commands/output/diffs/final and canonical history. One actual app
dispatch/one user object/zero observed permission replies; no approval action was
attempted. Native and host Stops confirmed with no jobs. Evidence:
phase-c-annotation-rw10-review.json; project /data/data/com.termux/files/usr/tmp/pc.7I90nl/w.
The primary native thread is 01a152f7-4bfe-7651-951a-72a6ee490d3c, with CODEX_HOME
/data/data/com.termux/files/usr/tmp/pc.7I90nl/h/.codex. Its matching session_meta is
retained in sessions/2026/10/19/rollout-2026-10-19T07-01-41-01a152f7-4bfe-7651-951a-72a6ee490d3c.jsonl
under that home. Use this home when locating/resuming the trial thread; a default
profile's session lookup is not equivalent. Verify temporary paths still exist
and preserve unique project/history/proof before cleanup or relocation.
This is bounded temporary-project development proof, not direct AI-project,
renewal/recovery, comparison, signed release or ordinary-CLI activation proof.

The subsequent declared matched-input attempt is
phase-c-matched-development-review.json. Separate identical TMPDIR JSONL projects,
requirements and task bytes used GPT-6 High with a33c3e96 and eeaf9007. The previous
candidate failed with app_stream_failed after its initial dispatch and one delegated
conversation approval reply, before any native execution; project/CLI/canonical
inspection confirms zero commands/patches, zero pending native calls and native
Stop, with host Stop unconfirmed. Its Chat, journal and failed threads are preserved;
the failed local observer was detached after native fencing and service/tunnel exit,
without resubmission. This does not attribute the failure to tool annotations.
The current candidate completed baseline failure→source/README patch→unchanged full
eight-test success and diff review: two commands, one two-file patch and all three
original MCP results; independent tests, CLI diffs/final and canonical calls/returns/
final pass. Native/host Stops confirmed, no owned jobs. One initial dispatch and no
approval reply; the pair totals two initial dispatches plus one approval reply.
Actual allowance debits and independent title-model starts remain unmeasured.
That failed attempt alone establishes neither a quality/latency winner nor C/D
completion; the later fresh pair is recorded separately below.

Bounded C handoff proof is phase-c-same-thread-chat-handoff-review.json, at the
retained eeaf9007 candidate. After zero-Chat latest-history/owner reconciliation,
the same comparison work/thread continued actual development: nine new regressions
failed before implementation, then all17 original/new tests passed. Chat supplied
a history/context-cursor-bound summary; actual system-only successor startup used
zero user/tool bootstrap messages and acknowledged controller generation3→4.
Six source and three successor native calls all returned on their original MCP
calls. The successor read current files, performed20 independent CLI checks and
finished on the original native turn/CLI binding. Supervisor tests independently
pass17 and reproduce the new nine-test failure on saved pre-change source; actual
diff, source cursor hashes, controller receipt, CLI and canonical calls/returns/
final are verified. Two app starts/zero approval replies; both Stops/no jobs.
The saved backend-list lookup of the successor is inconclusive; actual live app
dispatch/user counts, accepted controller transition and subsequent MCP execution
establish the bounded handoff. This is one renewal, not long-duration/repeated
rotation, physical UI or charged-allowance proof. The supervisor fixture is
/data/data/com.termux/files/usr/tmp/cc.4bpwfnsa.

The next zero-Chat restart preparation exposed an unqualified lifecycle boundary:
shutdown released an already stopped app lane but retained its driver reference.
After service restart/explicit native-history claim, native Stop/no jobs confirmed
while host Stop could not address the absent lane. No task or Chat was submitted;
phase-c-released-host-restart-negative.json preserves the failure and trial state
under /data/data/com.termux/files/usr/tmp/cr.cbc3kuhr. The source correction now
checkpoints a released/stop-confirmed boundary only after both host Stop and release
acknowledgment. A recorded retirement can confirm that prior Stop without another
app action; missing/unrecorded lanes remain unknown. Provider acceptance passes101
Rust/115 Bun/36 Python plus fmt/clippy/release; full passes358 Rust/135 Python.
Explicitly armed Android operator acceptance includes missing/released lane
semantics, no-app restart/claim/recorded Stop and a new genuine user turn on the
same native thread. Exact inputs and new binary
18e5fba247a7bb9575a9cfc8c31648315771c21b97428123953cd778e16ad732 are bound by
phase-c-release-checkpoint-inputs.json. The old journals have not been migrated
or edited to manufacture a retirement receipt; earlier Chat proof retains its
eeaf9007 scope.

Use the retained candidates/proof in
~/.cache/codex/acceptance/development-audit-1t4txdwt. On fresh inspection the earlier
host/Provider owners were stale; the same desktop profile was reopened without
copying or Chat submission. The preserved comparison/handoff state is
/data/data/com.termux/files/usr/tmp/pm._wo_0ia1/provider-state, with native home
/data/data/com.termux/files/usr/tmp/pm._wo_0ia1/h/.codex and completed thread
01a15341-c177-76c3-9dbc-3849d3ec6df0. That same thread completed the later handoff.
The old trial service/tunnel were restarted without Chat after readback shutdown;
recovery preparation failed host Stop as described above. They were then stopped;
only verified stopped app lanes were released. Preserve the journal/history; this
old state is not ready for another task submission.
This is a separate comparison thread; preserve the ten-cycle thread
and its exact home above. Controller scripts/fixture locations are retained under
/data/data/com.termux/files/usr/tmp/pc-matched-kwd6py8p. Keep native socket paths
within107 bytes; the short fixtures use100. Verify current kernel ownership before
reuse. Saved URLs, ports, PIDs
and relay descriptors are historical references, not fresh readiness. Verify
pending calls and native history before recreating an owned host/tunnel. Finish
zero-Chat readiness first. Explain the actual task, summary/successor starts,
possible consent replies and bounded budget before any real model submission.

Actual active-interruption/recovery passes independent review in
phase-c-active-interruption-review.json, using immutable18e5fba2 under
/data/data/com.termux/files/usr/tmp/ct.0hjbe6su. Native home is h/.codex there;
primary thread01a15381-787c-7192-836d-1fc3955aadf7 is preserved. Stop interrupted
one pending native worker command; its kernel owner exited, progress stayed
unchanged and completion remained absent. Stop left one pending receipt, which
the explicit latest-history claim reconciled as failed/exit-1 after Provider
restart. The acknowledged released-stop checkpoint confirmed the prior host Stop
without another app action. A genuine resume continued the same work/thread;
two commands independently checked the stopped worker/stable output and recorded
recovery, without rerunning the original command. All original calls/returns,
CLI interruption/resume/final and canonical history are verified. Final pending
calls/jobs are zero and both Stops confirm. One initial plus one resume Chat,
zero approval replies; charged allowance remains unknown. The settled service/
tunnel were subsequently stopped for the AI trial; state and project are retained.
This bounded active-command boundary does not qualify every host-loss/approval/
checkpoint/handoff interruption or reboot gate retained in G.

The operator's direct ~/prj/ai development passes independent review in
phase-c-direct-ai-operator-review.json. AI main43abdc8 was clean before this trial;
the model added tests/test_session_option_values.py, observed its real regression
failure, then minimally corrected lib/ai_resource.py so a missing option value
cannot consume the next recognized option. Existing tests/verifier/AGENTS hashes
are unchanged. Supervisor execution reproduces failure against saved baseline
source in memory and passes the new/prior filter tests on current source. The
first full verifier failed an existing tmux focus test; focused rerun and the
second unchanged full verifier pass50 checks with zero warnings/failures. Eleven
native commands/two patches and all15 original MCP results, including command
polls, are settled; real diff, CLI and canonical calls/returns/final are verified.
One GPT-6 High initial Chat, zero approval replies; both Stops, no pending calls/
jobs. AI edits remain uncommitted and uninstalled. The fixture is
/data/data/com.termux/files/usr/tmp/ad.jepqk_r8, native home h/.codex and thread
01a15390-08bc-7f20-9d32-bc7b9663c1c0. The same immutable18e5fba2 candidate was used;
charged allowance remains unknown.

The fresh identical-input comparison is independently reviewed in
phase-c-fresh-comparison-review.json. Locations/inputs/task hashes are retained in
/data/data/com.termux/files/usr/tmp/cm.lxjte2dw. Previous a33c3e96 used
/data/data/com.termux/files/usr/tmp/pm.9938awg4; current18e5fba2 used
/data/data/com.termux/files/usr/tmp/pm.ycwegh4e. Both actual GPT-6 High runs observe
the baseline failure, patch source/README and pass the unchanged eight tests plus
12 independent CLI cases. Each has two native commands/one two-file patch/all
three original results, verified diff/CLI/canonical final and both Stops/no jobs.
The review's additional valid400-digit integer exposes an OverflowError/traceback
in the previous output; the current output preserves the integer and succeeds.
The previous sample therefore does not fully satisfy its finite-integer contract,
despite passing the supplied tests. This finding is preserved, not supervisor-
patched or hidden by a passing test count. It is one observed code-quality
difference, not a general model-quality claim. Measured task times are106.264s
previous/113.651s current; one pair establishes no general latency/cost winner.
Two initial Chats/one delegated approval reply total; charged allowance and title-
model starts are unmeasured. The earlier failed comparison remains preserved.

At the earlier C comparison handoff, the owned service/dedicated tunnel targeted
the immutable18e5fba2 fixture above; phase-c-final-readiness.json records that
historical zero-model-request ownership/readback. Its native home h/.codex and
thread01a1539b-475f-7103-95c8-00a762fe5efe remain preserved. This is not current
readiness: the later accepted C2 used2edc2215 and explicitly stopped its owned
trial service/dedicated tunnel after native/host Stop confirmation. The desktop
host/profile and all older projects/journals/chats are retained. Before future
reuse, inspect current kernel ownership, latest native history, pending calls and
observer/writer state; reconcile ownership before resuming a CLI.

The user-scoped comparative development experiment uses separate identical
projects under TMPDIR, with actual file edits, commands, correction and tests.
Do not silently replace it with a read-only command or synthetic response. This
bounded experiment is distinct from the direct ~/prj/ai proof accepted above,
which respects that project's AGENTS and user changes.
ChatGPT performs the development through MCP; the
supervisor checks the actual diff, failures/corrections/tests, native CLI and
canonical history independently. After sufficient MCP work, prove a grounded
summary plus confirmed incremental history reaches a real successor Chat and its
receipt permits continued work on the same primary Codex thread/original return.
Also prove interruption of active work and reconciled resume using latest history
and unresolved calls; completion-time Stops alone do not establish that boundary.
Keep existing successful smoke evidence instead of repeating it as the next task.
Unresolved non-user admission or delivery is recorded without replay or automatic
path switching. C/C2 and D retain their recorded scope and E design/source is
complete. F bounded integration is also complete; G's remaining acceptance matrix
is next. Linux app/host management research was resolved in E, with selected
implementation qualified at the bounded F scope. Core
display/update follow-ups, viewer polish and project setup UX retain lower priority; G delivery/activation
and H retirement retain their separate gates.

Completed implementation is not a new backlog: Core independence/config isolation,
validated Provider config/catalog, one-line /switch rows, source same-provider model
selection, native execution/context fixtures, configured-project operations and
bounded AI/MCP live proofs are retained below. Full publication and operating
Provider migration remain separate. Detailed open qualification belongs only to
the acceptance matrix; the proof index records completed evidence at its real scope.

Phase A's shared correction closes a false service-stop acknowledgment: after
SIGTERM, losing the recorded executable path no longer counts as process exit.
Only the pinned kernel handle confirms shutdown; a surviving owner returns bounded
`provider_stop_unconfirmed` and preserves its journal. The actual Android regression
failed on the prior implementation and passes on the repaired source and downloaded
NDK executable. This is service shutdown, not confirmation that native tool jobs or
Chat work stopped. Existing exact-call/history/recovery proof retains its old scope.

The standalone candidate now has one create-new package/readback helper, including
source-bound lock/notices/licenses and a complete seven-file inventory. Hosted
qualification39173039235 passes at exact sourceacf1bdb; executable SHA-256 is
bd6e6068364791f02fad5bc53637e7dfc50b0609d87848403e17da4e5211a32c. The exact
artifact and proof are retained privately beyond the30-day hosted retention.
Actual Android c463→acf1bdb→restart→c463 snapshot transitions preserve the owner,
endpoint, disposable auth/config/app data and rollback profile while rotating local
credentials and rejecting a second writer. A saved-port collision preserves the
record. This qualifies bounded standalone candidate recovery, not a signed product
installer, real host-loss/reboot, physical UI or long-running task recovery.

Local `full` acceptance passes358 Rust and135 Python checks; Provider passes98 Rust,
108 Bun and35 Python checks, fmt/clippy/release. Source contract39173039249 and both
hosted Provider jobs pass. Unarmed live/physical gates remain in the matrix below.
Added qualification uses disposable roots and a synthetic host. The candidate
lifecycle trial made zero host requests/Chat submissions; there was no public
signing or operating activation. The later user decision authorizes B independently
of this baseline; C–G retain their separate trial/default/delivery gates.

This design correction does not invalidate Phase A's exact-source service-stop,
identity, correlation or candidate-recovery proof. Preserve acf1bdb as the recovery
baseline instead of folding the unfinished B draft into it. Those passing gates
are not evidence for B's same-thread incremental handoff or changed dependency
closure. Requalify affected inputs when B changes them. Do not expand old adapter
context/rotation policy simply to finish A; keep shared correctness and defer
path-specific hardening to D's retained scope. No Phase A implementation change is
required by this documentation correction.

The user's required AI trial was run directly in ~/prj/ai, not a copied project or
worktree, after the Rust no-MCP execution/context/project path was established.
The model performed the actual source work. Future trials must likewise inspect
AGENTS/Git and preserve uncommitted/untracked content before mutation; Git history
alone does not preserve it. Supervisor review and applicable tests remain mandatory.
Do not substitute the old adapter or a supervisor-made patch for model development.

Complete independent source work while hosted checks run; admit releases in their
dependency order. Do not stop after each individual file change or invent a new
planner/checkpoint hierarchy. Existing user permission and verified remote ownership
remain valid for authorized baseline and Phase B work. They do not authorize
undeclared real Chat trials, a new default or operating activation.

### Deferred Core follow-up

User-requested, low-priority work to bundle with the next substantive Core change;
do not implement it now or make it a prerequisite for the Provider work order.
The85→86 update succeeded, but the user observed `unknown` in the right-hand
release date/other details. Core bytes were unchanged across those releases.

- Improve update information so upstream Codex, Core, Manager and applicable TUI
  each have identifiable installed/target versions or source revisions and release
  dates. Make unchanged or absent optional components and unavailable metadata
  understandable. Diagnose the missing display data; use authenticated metadata
  and its actual date meaning, never infer a release date from installation/file
  time or imply every component changed because the Full sequence increased.
  Choose a concise, consistent presentation for explicit update and startup prompt
  when implementing; exact layout and metadata mechanics remain design choices.
- On every eligible all-TTY, zero-argument bare `codex` launch with a verified
  available update, show the useful current→target component information and ask
  whether to update. A prior decline/timeout must not hide that same update on the
  next eligible launch. Missing optional component details must not hide a verified
  update or prevent its prompt. Keep discovery/display acquisition bounded,
  continue normal launch on decline, timeout or failed discovery, and retain
  rollback/trust guards.
- Preserve all upstream commands/options and raw execution behavior. Any arguments
  or any non-terminal standard stream bypass startup discovery, prompts and
  advisory writes entirely; preserve stdin, argv, streams, exit status and signals.
  No optional Manager/Provider dependency or unattended activation is introduced.

Before implementation, amend SPEC's current display-acquisition and six-hour
generation-snooze rules. This records future work, not changed shipped/default
behavior. Verification belongs to the eventual affected Core change.

<a id="deferred-linux-chatgpt-development-host-management"></a>

### Linux ChatGPT app management and deferred viewer work

App/host management research and design are complete in E. The findings cover
authenticated APIs, Chat/project creation/read/input/reuse, ownership/lifecycle,
Provider/tunnel connections and status/recovery/helper surfaces. F reuses the
selected Rust mechanisms and common management facade.
This is broader than designing a terminal opener. Selected management behavior
is implemented in F after the design decisions. noVNC visual/touch/IME polish
retains its separate lower priority. Neither research nor documentation activates
the operating Provider or restarts existing work.
Use “Linux ChatGPT development host” as a descriptive name for the existing Linux
app/Provider/viewer development environment. E now selects an optional Provider
host lifecycle facade over the existing Rust mechanisms; it is a target interface,
not an installed new command. The old blanket deferral/C-order wording no longer
defers management research.
The [E capability findings](#linux-app-and-host-capability-research) contain the required
research output. The management behaviors below inform that interface without
requiring a new process hierarchy.

There is already a terminal equivalent of the Linux notification's open action,
run from the repository root:

```sh
python -B providers/chatgpt/open_dual_trial.py linux
```

It opens the current owned running Linux session's viewer through Android's
browser intent, resolving the current private viewer capability at invocation.
It validates the recorded owner and does not launch the Linux app or submit a Chat
task. An accepted intent is not proof that the physical screen became visible.
This helper and linux_desktop_trial.py prepare/serve/start are development paths,
not a unified installed management command. Keep the notification and explicit
terminal action on the same opening path.

- Provide one discoverable entry point for explicit start/reuse, status, view,
  reveal-existing-Chat and stop. Reuse the current host/Provider controls and owned
  session discovery instead of adding an executor, history database or planner.
  Separate opening a viewer, starting a service, submitting model work and stopping
  execution. Help/status should explain prerequisites and missing/expired state.
- Make rootfs/Bun/noVNC locations and host lifetime manageable without long ad hoc
  invocations. Choose clear foreground/background and bounded-lifetime behavior;
  report existing owners and reconnect without restarting live work. Preserve
  credentials, profiles and conversations. Status should distinguish app host,
  Provider/tunnel, viewer and native/model work, including unobservable state and
  requested versus confirmed Stop. Silence or a disconnected viewer is not idle.
- Improve noVNC's mobile layout, visual hierarchy, sizing/orientation and toolbar:
  useful screen space, legible controls, large touch targets, keyboard/IME viewport,
  scrolling versus dragging/selection, Esc/arrows/Ctrl shortcuts and fullscreen.
  Keep clipboard transfer explicit. Validate touch, Korean IME and browser behavior
  on actual Android; HTTP or mouse automation alone does not qualify usability.
- Recover a lost viewer/relay connection against the current owned session and
  intended Chat without resending development work. Reuse the existing host bridge
  and its connection ownership; avoid competing clients. Keep private viewer/CDP
  capabilities out of command output and shared diagnostics; offer concise status
  and sanitized failure details rather than raw conversation/credential dumps.
- Consolidate proven helpers when making this interface, with clear dependency/
  asset versions and installation diagnostics. Optional viewer assets must stay
  optional for inference and Core launch. Provisioning/signed distribution belongs
  to G, with no implicit download/login. Preserve unique trial history and work
  before any cleanup; generated trial data stays outside prj.

Record selected public management behavior in SPEC during E before F implementation.
The viewer usability proposals retain their lower priority. This section does not
change the currently shipped command surface.

## Context and easy configuration

The Rust authority is $XDG_CONFIG_HOME/codex/providers/chatgpt.toml, default
~/.config/codex/providers/chatgpt.toml. It owns settings and catalog.models.
Initial routes are default-models.toml data, not hardcoded Rust model families.
GPT-5.6 is excluded; retain GPT-6, manual-only Pro and hidden Instant alias.
Listing/editing a model is not entitlement or execution proof.

The candidate CLI supports models, catalog, config show/effective/validate/set/
unset/edit and prepare --home PATH. Related fields commit atomically; cooperating
writers serialize, stale editors refuse overwrite, invalid/no-op edits preserve
original content. Effective output reports file/default/per-model origins and
runtime_application=not_queried. The installed JS adapter does not consume this file.

Provider reads host routes/history config at request admission. Catalog snapshots
and generated upstream startup settings remain immutable: prepare again for the
next startup rather than editing a bound server. Per-model context overrides file
settings; explicit native invocation overrides win. /model is primary; generated
upstream --profile chatgpt is the fallback. Manager is not required. No bearer is
stored in the profile. `token`/prepare may lazily start the same Rust executable
against a provisioned rootfs; concurrent startup serializes. `status` is read-only.
A failed readiness check never authorizes restarting an existing live owner.

The user1,000,000-token setting is preserved policy, not measured Chat capacity or
a qualified ceiling. Do not impose an arbitrary32k minimum. Native token triggers,
model/hash compaction, HTTP byte bounds and true host capacity differ. An unset
explicit threshold delegates to upstream, not an all-compaction disable switch.

The following describes the existing model adapter, not B's Chat-renewal policy.
Cold admission projects/imports public native history; a warm lane sends needed
deltas/results. Neither step necessarily asks a model to summarize. Native Codex
requests inference-context compaction on its own lifecycle triggers, asks its
configured model endpoint for a summary and commits the accepted replacement to
active inference history while retaining the original durable rollout. The Chat
Provider supplies that summary through ChatGPT and retires/rebuilds its old lane
only after native acknowledgment. This couples native compaction to Chat renewal;
it is not an independent measurement of actual Chat remaining context. B replaces
that coupling with the [same-thread handoff policy](#same-thread-continuity-and-chat-context-handoff).

Responses Lite declarations are transport metadata, not canonical conversation
items; compaction may remove/reinsert them without changing the accepted prefix.
Compaction is recognized from actual request metadata/explicit V2 triggers, never
user prose. Summary-only policy and replies stay transient until upstream commits
the exact offered checkpoint. Retire a settled lane once after acknowledgment;
identical summary text in an unchanged old prefix is not acknowledgment. Reassert
normal native policy after control operations. V2 ocx1 encodes public readable
summary, not private reasoning and not decrypted foreign OpenAI context.

Source `checkpoint` serializes a complete idle public native snapshot as readable
external user context, with no credentials, app startup or inference. Upstream
persists it with its existing inject API, preserving original items/thread ID and
the old provider's ability to read them after an entry failure. Provider admission
accepts only an explicit handoff in the same thread or native-declared fork/parent
lineage before foreign opaque input can be
projected from public history; current policy/context settings and later input stay.
Unknown public item types refuse; reasoning/internal metadata are excluded.
The catalog no longer advertises a fabricated opaque compaction compatibility hash;
token/window compaction remains active. Source TUI waits asynchronously for the
source Chat Provider's readable summary before reverse entry, retaining the source
on failure/stop/event loss. These source changes are not an activated composition.
The real summary trial used two app model starts in33.9s, observed four user-role
objects, no tool return/Pro/retry and unknown Chat allowance debits. Native provider
responses in that trial were local fixtures; actual Chat inference and both native
process re-entries were real. It preserved the exact continuation marker, three
explicit native user messages, original public history and foreign opaque input.
One separate compaction turn is not another explicit native user request. The
current native user/context journal remains upstream's authority.

Public output includes initial/intermediate/final reports and actual native tools.
The app's final channel alone is not a final answer: finished_partial_completion
with end_turn=true can precede a tool. Stream once and promote an open item only
when successful end-of-turn is established. response.failed preserves native error
reasons; static last_error records contain no request/credential text. Known MemoryCite tags and box containers now have a stream-safe Markdown
projection, preserving useful text, optional titles, unknown markup and literal
code. Every UTF-8 prefix and native warm/cache/tool-result paths pass without
extra app submissions. Raw app/native tool data is not rewritten. Nested JSON
argument overhead and actual app sampling of markup remain separate work.

## ChatGPT projects and history

Retain history by default. Project mode uses history.mode, history.project_name,
optional exact history.project_id and history.create_if_missing (true by default).
This is a remote ChatGPT project, not CWD, Manager profile or API project.
The bridge uses the installed app's authenticated list/read/create operations;
no account bearer is exported. Current admitted destinations are owned, private, unshared
projects; their existing memory scope is preserved. Shared destinations refuse.

Bindings include account/user/workspace and exact project ID. Reuse an unambiguous
name, or select its configured ID. Duplicate/deleted/inaccessible destinations do
not create a replacement or silently fall back to root chats. Serialize creation;
record intent before dispatch. Reconcile uncertain delivery before another create;
a definite rejection is separate. Use app defaults rather than nullable memory/
sharing options. Destination changes affect new lanes; model/effort changes reuse
settled conversations. Do not move/delete existing chats or create one per tool.

Actual metadata create/reuse/readback spent zero Chat submissions. The native
project trial verified the declared ID before streams and read back persisted
conversation membership after inference. Project instructions/files/memory still
require priority and sibling/guardian isolation proof; do not change personalization,
sharing, native reviewer or MCP permissions silently. Grouping reduces clutter,
not stored history or established allowance units. Temporary/archive/delete modes
are not admitted and must never mass-delete history.

Deferred user-facing setup/management: let the user discover/select a ChatGPT
project, create one when explicitly chosen, inspect/change the configured
destination and verify actual conversation membership. Reuse the existing
history configuration and authenticated project operations; do not add another
settings store. Broad project-setup UI retains its lower priority; do not defer
project operations needed by chosen agent/session workflows with it. E selects
explicit private-project binding and saved Chat reuse through the capability map;
F implemented and boundedly qualified that behavior. The old C-order wording is not an unfinished C gate. This does not start setup-
UX implementation or activation. Provider must
remain usable without Manager/TUI; any optional management surface uses the same
configuration. Account/workspace and exact project binding, safe-boundary changes,
successor placement and isolated reviewers retain the contracts above. Existing
conversations, instructions/files/memory and access settings stay preserved.

The current C trials deliberately use the default retained, non-project mode.
They do not qualify a user's project setup or placement. Earlier explicit project
metadata/inference proof retains only its recorded scope.

## Confirmed operating Chat blocker

Installed Full60 long native→Chat transitions failed400 unsupported_desktop_route
because ModelDownshift compaction sent the old gpt-6.1-sol model to the Chat
endpoint. Two requests were rejected before any Chat submission. The retained
conversation also has an unreadable upstream opaque checkpoint. An alias rename
or the1m setting does not repair either boundary. Keep the conversation.

Source Rust manual compaction and fresh native Chat now have separate real proofs;
actual Android TUI transfer now passes against an unsigned candidate in a signed
disposable composition, but final hosted artifact admission is unfinished. Obtain an upstream-owned readable
handoff at a settled boundary before routing it to another Provider; never decode,
drop or relabel opaque state or replay tools. Evidence: chat-full60-operating-
diagnosis.json and chat-full60-cross-provider-admission.json in the acceptance root.
Fresh startup is not blanket proof of all long conversation transitions.

## Verification and publication

Private acceptance root: ~/.cache/codex/acceptance/development-audit-1t4txdwt.
Logs/state there may contain private trial transcripts/account bindings; do not
publish them wholesale. Repository records contain only safe scope/results.

The index below retains useful latest/negative proof, not a chronological backlog.
Older source-run counts and superseded candidates stay in Git and the private
acceptance root. A synthetic host exercises native product code but does not
qualify actual Chat judgment, physical UI or charged Chat message allowance.

| Proof | Scope |
| --- | --- |
| ~/.cache/codex/acceptance/phase-e-complete-20261021/e-completion.json | E functional/ownership/reading/agent/host/idempotency contracts complete; common source implemented, all-target type/Clippy checks, fmt/docs/JSON syntax and Termux release build. Source-only22e2d492; complete source/data/helper inventory and real-reference zero-inference F input example. No test function, native development call, new Chat, runtime deployment or F/G acceptance |
| core-settings-android.json |57 signed Android steps: actual config RPC/FD34 normal→Core-only→normal with live sibling, optional malformed/hung settings and TUI without Manager; no Chat |
| tui-admission-39129327503.log / provider-tui-accepted-no-manager.json / provider-tui-no-manager-current-core.json | Exact TUI source15da596, 50 focused+94 retained upstream tests/lint and binary8d84f1dd; actual Android same-thread native↔Chat /model, permissions, real checkpoint encoding and four synthetic Responses with one source summary. Current Core e83083f6 built explicitly, Manager absent; no Chat or operating activation |
| provider-native-question.log / provider-context-seven-android.log / provider-bidirectional-handoff-native.log | Actual native manual/automatic/interrupted compact, public handoff, API re-entry, fork/nested child warm/cold resume and question→actual answer on original call. Foreign opaque items preserved; synthetic Chat replies, no inference at injection |
| provider-handoff-live-native.log / provider-handoff-live-state | Real GPT-6 response and readable summary across native API provider re-entry, two app starts/four user objects; source responses synthetic. Not physical real-Chat TUI transition proof |
| provider-native-mcp-discovery-fixed.log / provider-native-correction-regressions.log | Actual native deferred search and exact canonical tool names, MCP effect, rejected-gateway correction, commands/patch/tests/final. No partial rejected-batch effects/user resubmission; synthetic host, no Chat |
| provider-native-reviewer-composed.log / provider-reviewer-project-isolation.log | Independent native guardian lanes, schema-valid allow/deny and actual allowed/prevented command effects; reviewers outside primary project with supplied context only. Synthetic replies, not real Chat reviewer judgment |
| provider-explicit-stop-android.log / provider-viewer-owned-android-final.log / provider-auth-bootstrap-android.log | PID-bound service shutdown/refusal of forged identity, same-endpoint restart, concurrent startup coalescing, owned fresh-profile HTTP/WebSocket/private RFB and capability revocation. No existing auth copy or Chat; physical fresh-profile viewer remains separate |
| provider-terminal-identity-native.log / provider-native-failure-reason-final.log | Account change before publishing tools/final refuses; restored identity reconciles original request without resubmit. Exact Provider failure reaches native UI. Synthetic host/no Chat |
| provider-sse-idle-regression-before-corrected.log / provider-sse-idle-android-fixed.log | Comment-only heartbeat fails actual native2.5s idle deadline; content-free data events pass four six-second waits/three effects without extra model/transcript items. Synthetic host/no Chat |
| provider-mcp-live-qualified.json / provider-mcp-real-native-v11-result.json | R11 at c463c69: actual signed Chat app→dedicated Rust tunnel→Android two exact commands/effect+verification→original MCP results→exact native final and public reports. One user object, two app dispatches including actual delegated Allow reply, two settled logical calls, no result resubmission/pending/replay; Chat debits unknown |
| provider-c463-android-smoke.json / provider-qualified-settled-release.log | Hosted39163366831 exact source/Android jobs; API24/AArch64 inventory/lock/source matches and actual device models/catalog/config validation makes no state writes/Chat starts. Unsigned/unactivated. Trial service/tunnel/proxy and settled hooks stopped/released, chats/auth/Tdev preserved |
| phase-a-baseline-proof.json / phase-a-candidate-lifecycle.json / phase-a-core-full.log / phase-a-provider-full.log | Sourceacf1bdb, hosted39173039235 and source contract39173039249; complete seven-file source-bound readback, actual Android standalone snapshot install/replacement/restart/rollback, exclusive owner/endpoint/credential and preservation checks, downloaded-binary unconfirmed-stop regression. Full358 Rust/135 Python; Provider98 Rust/108 Bun/35 Python; fmt/clippy/release pass. Zero Chat/host requests; unsigned/unactivated |
| phase-b-operator-android.json / phase-b-operator-android.log | Actual Android disposable Core/TUI plus synthetic host: both entrances, full native catalog, command/patch/failure/correction/test, exact retry and detached HTTP observer, independent child/reviewer allow/deny, same-thread manual/automatic Chat handoff, late committed input on original MCP return, initial/intermediate/final live CLI and native history, same Chat across later user turn and native compaction, owned-job Stop, same-endpoint Provider restart/current-history claim, original question answer, failed successor fencing/explicit takeover, concurrent host start/Stop serialization with no late dispatch after confirmation. Zero real Chat submissions; fixture ob-yb79zqmn. Does not qualify actual app bootstrap/renewal, physical Chat UI, long-duration behavior or Core-process replacement |
| phase-b-operator-inputs.json / phase-b-provider-full.log / phase-b-core-full.log | Exact code/test/release-binary digests retained. Provider101 Rust/108 Bun/36 Python and full358 Rust/135 Python pass; fmt/clippy/release pass. The explicitly armed synthetic operator gate is additional Android proof, not inferred from ignored tests. Core, Manager, TUI and installed optional runtime/state are unchanged |
| phase-b-shared-android.log |14 explicitly armed Android cases on the current qualified native runtime: original MCP command/patch/test returns; deferred MCP discovery/error correction; independent reviewer allow/deny; silent-host wait; native manual/automatic/interrupted compaction, public handoff, bidirectional re-entry, fork and original question answer. All synthetic/zero Chat. Required after shared Broker dispatch changes; separate from operator-specific proof |
| phase-b-candidate-readback.json / phase-b-candidate-android.json | Exact source f0cf4ad1be96463964e271752859084b548f2d51; source acceptance39185572035, independent Core build/smoke39185610111 and Provider source/Android39185572029 pass. Complete API24/AArch64 seven-payload-file inventory, lock/licenses/source readback; downloaded binary on actual Android passes models/catalog/config validation, no-desktop/no-Core operator startup, authenticated MCP stdio tool/session discovery and kernel-confirmed service stop in a disposable root. Zero Chat; unsigned, no Provider installation or operating activation |
| phase-c-provider-final-gate.log / phase-c-operator-corrected.log / phase-c-operator-android.json / phase-c-correction-inputs.json | Bounded correction: Provider101 Rust/111 Bun/36 Python, fmt/clippy/release; explicitly armed Android full operator fixture additionally covers first genuine user input, partial final remaining pending, host permission/authentication waits in CLI/history/status, owned reveal without inference and rejection of another account, failed native systemError Stop/current-history claim, and explicit Chat final without native finish. Synthetic/zero Chat; exact code/binary digests retained. Unchanged Core/Manager/TUI retain B's full358 Rust/135 Python scope |
| phase-c-shared-accepted.log / phase-c-connector-refresh.json / phase-c-safety-negative.json |14 shared Android cases cover the changed app bridge with zero Chat. Only the owned Rust trial connector was refreshed/read back with all14 actions; process-only DNS/CONNECT and actual tunnel child were verified. The earlier three real primary dispatches reached wrong-name discovery, app consent or safety refusal; zero native effects/host replies. Latest actual refusal reached CLI/canonical history and both stops confirmed. Non-user renewal, actual operator development and charged allowance remain unqualified |
| phase-c-candidate-readback.json / phase-c-candidate-android.json | Exact source9d5a3c6897e1de52865fe1a773dd94c41182aace; source acceptance39191287088 and Provider source/Android39191287095 pass. Complete seven-payload-file inventory/source/lock/licenses/API24 AArch64 readback; downloaded binary4bce1853 on actual Android passes models/catalog/config, no-desktop/no-Core operator startup, authenticated stdio discovery of all14 tools/empty sessions and kernel-confirmed service stop. Zero Chat, unsigned/uninstalled/unactivated; this does not close C's real attachment/renewal/development gate |
| phase-c-fast-permission.json / phase-c-fast-permission-review.json | User-confirmed Android viewer and explicitly delegated actual Allow once. Fresh real GPT-6 High: attachment/receipt, initial/intermediate CLI/canonical reports, one read-only git status in real ~/prj/ai, original MCP return and canonical final. Two app dispatches/one user/one host reply/zero result resubmissions; both Stops confirmed, no jobs. Read-only permission smoke only, not long development/renewal or charged allowance proof |
| phase-c-background-final-gate.log / phase-c-background-final-operator-android.json / phase-c-background-approval-review.json | Provider101 Rust/112 Bun/36 Python, fmt/clippy/release; Android operator synthetic regression covers authenticated approval, account/stop/recovery fences and zero Chat. Actual background app action sent one real allow/remember=false reply with no reveal/viewer requests (two dispatches/one user/one host reply). Later model ended without accepting receipt or native effects; correctly incomplete, both Stops confirmed. Approval mechanism qualified; continuous execution remains open. No installation/default change |
| phase-c-scoped-render-final-gate.log / phase-c-scoped-render-inputs.json / phase-c-scoped-ack-operator-android.json / phase-c-scoped-lazy-viewer-android.log | Latest Provider101 Rust/115 Bun/36 Python, fmt/clippy/release pass. Actual Android synthetic proof covers delayed real-reply observation, known-capability UI fallback, uncertain approval without retry, account fences and the existing two-entry native tool/report/stop/recovery flow; empty-profile lazy-viewer capability/HTTP proof spends zero Chat. The later cold-render readiness change additionally has the exact-source real UI proof below; unchanged Core/Manager/TUI retain B's full gate scope |
| phase-c-scoped-main.json / phase-c-scoped-main-review.json / phase-c-scoped-ui.json / phase-c-scoped-ui-review.json | Separately fingerprinted actual GPT-6 High trials pass background app-action and actual-button conversation approval through receipt acceptance, one read-only AI command, original MCP return and CLI/canonical reports/final on the original native thread. Each has two app dispatches/one user/one approval reply/zero result resubmissions, both Stops/no jobs. No once after conversation approval; latest UI candidate a33c3e96. Bounded approval smoke, not long development/renewal, charged allowance or device activation |
| phase-c-annotation-inputs.json / phase-c-annotation-refresh.json / phase-c-annotation-rw10-review.json | Current annotation binary eeaf9007; current Provider check passes, owned tunnel target verified and actual app tool refresh/readback matches all14 tools. Actual GPT-6 High TMPDIR development: ten sequential read/patch/test cycles, unchanged full10 tests and diff check pass;25 commands/11 patches/all36 original native execution results, CLI and canonical final independently verified. One app dispatch/one user/zero observed approval replies; both Stops confirmed/no jobs. Prior-candidate comparison, direct AI-project development, renewal/recovery and ordinary CLI activation remain separate |
| phase-c-matched-development-review.json | First identical isolated JSONL attempt, GPT-6 High: a33c3e96 fails after one initial/one approval-reply dispatch, before native tools; native Stop/no pending calls, host Stop unconfirmed and preserved. eeaf9007 completes two commands/one two-file patch/all3 original returns, unchanged8 tests and independent CLI/canonical diff/results/final review; one initial/zero approval replies, both Stops/no jobs. This attempt alone does not establish a paired quality/latency result; no causal annotation claim. Latest-history/owner reconciliation is required before reuse; its later released-host failure is preserved separately |
| phase-c-same-thread-chat-handoff-review.json | eeaf9007 actual same-work/thread/turn/CLI continuation, source6→successor3 native calls/all9 original returns; grounded native history/context cursor hashes and controller3→4 receipt verified. Actual non-user successor starts once with zero user/tool bootstrap messages; source starts once, no approval reply. Independent17 tests and baseline nine-test failure, successor20 CLI checks, actual diff/CLI/canonical final pass; both Stops/no jobs. One bounded renewal; backend-list relookup inconclusive, no physical UI/debit/repeated-rotation claim |
| phase-c-released-host-restart-negative.json / phase-c-release-checkpoint-inputs.json / phase-c-release-operator-android.json | Zero-Chat eeaf9007 restart/claim failure preserved without journal migration. Corrected18e5fba2 source passes Provider101 Rust/115 Bun/36 Python and full358 Rust/135 Python; fmt/clippy/release. Armed actual Android operator covers both entrances and release-aware missing-lane semantics, same-endpoint claim/recorded host Stop with no app action, same-thread next-user continuation and retained handoff/stop/recovery/failure gates. Correction made zero actual Chat requests; no default, public release or activation change |
| phase-c-active-interruption-review.json |18e5fba2 actual pending native worker interruption, failed original receipt reconciled after Provider restart, acknowledged released Stop reused without app action, genuine same-work/thread resume. Worker executed once; independent kernel/progress/completion checks, CLI/canonical calls/returns/final verified. One initial/one resume Chat, zero approvals, final pending/jobs0 and both Stops. Broader host-loss/reboot/other interruption boundaries remain in G |
| phase-c-direct-ai-operator-review.json |18e5fba2 actual development directly in clean AI main43abdc8: new missing-option-value regression fails before minimal source patch; independent saved-baseline failure/current success. All original tests/verifier/AGENTS preserved. Full verifier first hits existing tmux focus failure, then focused/full unchanged reruns pass50 checks/zero warnings/failures. Eleven commands/two patches/all15 original results settled, actual diff/CLI/canonical final verified; one initial Chat/zero approvals, both Stops/no jobs. Two AI files remain uncommitted/uninstalled |
| phase-c-fresh-comparison-review.json / phase-c-final-readiness.json | Fresh separate identical baseline/task projects, a33c3e96 vs18e5fba2, GPT-6 High. Each performs baseline failure→source/README patch→unchanged8 tests/independent12 CLI cases, two commands/one patch/all3 returns with actual diff/CLI/canonical final and both Stops/no jobs. Additional valid400-digit integer: previous OverflowError/exit1 violates its contract; current exact integer/exit0. Two initial Chats/one approval reply;106.264s vs113.651s is one sample, not a general speed/quality/debit winner. Current owned service/tunnel/desktop read back with zero new model requests; CLI exited, fresh reconciliation required before resume |
| provider-project-discovery-android.log / provider-project-create-minimal-body.log / provider-live-project-native.log | Actual authenticated project discovery/create/reuse/readback without inference; separately actual project-bound GPT-6 with three native effects, four app dispatches/two user objects/three returns and saved membership |
| phase-c2-post-c-audit.json | Post-C transcript/effect audit: preserve valid prior C and requested AGENTS edits; correct CDP ownership/UI detour/default-model inference, distinguish ten cycles from five user turns and build comparison from D route comparison. Invalid Chat-origin work stopped without app/native effects; failed/partial saved Chats retained separately; C2 now accepted |
| phase-c2-correction-inputs.json | Exact2edc2215 binary/source/test hashes; Provider101 Rust/116 Bun/36 Python, full358 Rust/135 Python and fmt/clippy/release pass. Actual Android additionally proves positive app settlement before same-Chat next turn and Stop without late dispatch, retaining both entrances/native tools/handoff/recovery gates; zero Chat for these gates |
| phase-c2-continuity-review.json | Actual GPT-6 High, one saved source Chat with five independently reviewed development turns (11→17→23→29→36 tests), source-authored assumed-full summary, distinct system-only saved successor, exact current receipt before actual sorting development/43 tests/seven independent contract groups; original sixth native CLI return on same work/thread.18 commands/12 patches/all31 original execution returns, zero pending/jobs, native/host Stops. Seven accepted app dispatches/zero consent replies; two earlier dispatches excluded/preserved. Both saved Chats readable after Provider stop and local-cache release; native results complete, saved backend tool bodies omitted. Owned trial Provider/tunnel stopped; desktop/profile retained. Actual capacity/debit/long-duration rotation unmeasured |
| phase-d-route-decision-review.json | Same2edc2215/native runtime, identical CSV-ledger baseline/prompts, actual GPT-6 High two-turn local-function vs operator route comparison. Both retain original/prior tests, all20 original native calls/results each and saved same-Chat/native-thread finals. Tests22/24, common35 assertions and5000-digit probes pass; additional131073-digit contract fails local CSV field limit and passes operator, preserved without supervisor repair. App23 vs2; actual29 includes four unintended supervisor dispatches/three read calls, interrupted and excluded. Only first latency pair531.087s/407.921s comparable; no general latency/debit/model-quality causality. Both Providers/CLIs and owned tunnel stopped, saved Chats/profile/host preserved. Select operator delivery target plus explicit connector-free local-function fallback; effective defaults/public/device activation unchanged |
| phase-d2-comparison-review.json / phase-d2-evidence | Four exact9fef/Core/runtime/frontend/account/prompt/baseline Codex-entry six-task real-development runs accepted; all common/regression/native-result/CLI/saved-source/successor/Stop audits. Detailed UTC/monotonic timing, app/control/native counts, bytes, sampled CPU/RSS and supervision costs; all preliminary/failed attempts included. Actual billing/allowance and ordinary ChatGPT-first comparison unobserved/deferred; no installed/default/activation change |
| ai-development-proof.json | Direct ~/prj/ai actual development, failure/repair, same-thread recovery and independent49-check/166-TUI validation; detailed scope below. Installed AI runtime unchanged |
| provider-owned-auth-native.log / owned-auth-transition.json / provider-restored-auth-readonly.log | Actual authenticated Rust-owned app/private pipes/native tools, four starts/three effects; original profile inode and controller restored. Subsequent readonly identity preflight makes no inference |
| installed-full76-inventory-readback.json / installed-payload-readback.json | Full76 active/Full60 previous; complete declared payload readback at recorded time. Inventory equality is not a fresh signature check or Rust Provider activation |
| desktop-panel-icon-taps.json / provider-font-physical-proof.json | Six actual X pointer minimize/restore actions, official installed app/browser icons and Korean glyphs observed. Pointer timing is not physical Android touch acceptance; qualification helpers are not Provider runtime dependencies |

Failed TUI728cf26 compilation and ff2682d lint candidates are not admitted by the
later15da596 success. Historical310f0f7 proof cannot authorize changed inputs.
Earlier MCP tunnel401, target selection, approval, private-content and retained-
capability failures remain private diagnosis; R11 supersedes them only within its
actual bounded scope. Runtime-key references, exact real tunnel child argv, owned
immutable service snapshot and renderer bridge revision were checked before submit.
Readiness, schema discovery and construction alone are not connector execution ACK.
Ignored Android/live gates count only when armed and actually run; failed/zero-test
required gates remain open.

### Direct AI development acceptance

Baseline humtr/ai main1f958e5 was clean; its full verifier passed48 checks before
model edits. GPT-6 High independently found that session filters following a
reference were ignored, wrote a failing regression and observed its failure. The
first attempt stopped on the Provider's partial-final phase defect before its
implementation patch reached native execution. That failure and state were kept.

After the Provider repair, the same native thread resumed, read current Git/test
state, patched lib/ai_resource.py, passed the new regression/full verifier and
reviewed its diff. The supervisor did not implement the AI patch. Independent
review reproduced failure on baseline code in memory and success on the model's
patch, then independently ran all49 checks/166 TUI cases, zero warnings/failures.
AI commit43abdc8 is pushed to verified owned humtr/ai; installed AI runtime unchanged.

There were26 app dispatches across two explicit user tasks,24 tool returns, no Pro
or automatic replay. First attempt had2 user objects (one visible). Cold resume
imported history and reported29 user objects, not29 new visible user requests.
Warm returns and cold history import differ; neither count establishes allowance
deductions. Initial/intermediate/final reports and actual commands/patches/tests
were observed. This is one bounded maintenance proof, not hours-long/reboot/all-tool
qualification. Unsupported box components appeared in that historical trial transcript; later
stream-safe formatting fixtures address known boxes but are not a rerun of it.

### Remaining acceptance and delivery

This matrix is the outstanding qualification authority, not a second work order.
Use the A–H sequence above. Actual Chat trials need a declared scope/expected
submissions first. Do not turn completed fixtures/live proof into mandatory reruns
without changed inputs, failures or an unresolved boundary.
G applies to the paths/components retained after F integration, informed by D's
historical route evidence; replacement may retire an old
path-specific gate with its feature. Shared safety, supported upstream behavior,
protected state and signed lifecycle gates cannot be retired as path optimization.

| Area | Evidence already retained | Open gate and timing |
| --- | --- | --- |
| Design and functional definition | E complete; F1–F9 bounded implementation/integration reviewed above | G: apply the settled design to retained product composition and release acceptance; no required E decision remains open |
| Idempotency contract and hints | Common operation/duplicate/recovery logic qualified; Rust12 true/4 false values applied on candidate40 with other hints unchanged; actual native23/23 plus retained loss/restart/summary faults | G: retain those invariants in packaged paths. Backend response omits idempotentHint; internal use, causal speedup and billing benefit remain unmeasured |
| Linux app/host management | E capability research and F shared host CLI/MCP completed; actual private-pipe start/shutdown, lazy status/auth, explicit ensure, endpoint/credential reuse and RFB; shared app retained | G: broader host loss/provision/login/device composition; noVNC visual/touch/IME polish remains lower priority |
| Common bridge and usage routing | E settled shared roles; F actually exercises managed MCP/local, current direct MCP and current general MCP through one work service | G: package and qualify retained entrances with installed Core/TUI/profile combinations. The matched F9 work-CLI timing sample is distinct from original-TUI submission proof |
| Coordinated agents and orchestration | F actual independent review/correction/dependency, isolated writers/merge conflict, per-task/sibling/tree Stop, same-thread bidirectional transfer and retained quality/cost evidence | G: supported installed compositions, larger-duration/resource bounds and user-facing lifecycle. H alone retires replaced paths; no bespoke dashboard prerequisite |
| Shared saved-Chat reading | F common CLI/MCP discovery/read, unrelated/work-linked sources, duplicate titles, four branches, redactions and complete ranges; current40 long-source equality and prefix rejection | G: packaging/access/lifecycle combinations and backend capability changes; preserve the zero-inference, source/branch/cursor contract |
| Direct entry and delegated return | Earlier direct two-turn acceptance plus F actual general delegation/question/final return and equal-input four-route implementation; original false results and separately measured receipt corrections retained | G: installed entrypoint/composition qualification. No repeat of accepted development solely to erase an old verifier failure |
| User continuity and control | F existing-Chat input with pre-bound delivery cursor, preserved original cursor, exact saved/CLI/MCP result; agent transfers acknowledge receipt on the same native work; Stop/unknown boundaries covered | G: broader host-loss/approval/context/TUI/device transitions. Old missing delivery bindings remain strict; observed original output is preserved rather than replayed |
| Host lifecycle/visibility | Authenticated Rust boot/native effects, synthetic concurrent lanes/sibling stop, readonly journal status, Phase A exact-owner/restart and kernel-confirmed service-stop proof | G: real host-loss/cancellation/concurrency/reboot, Core-process replacement/cold recovery and fresh login/provisioning. Journal liveness is not current model progress; service exit is not native-job stop confirmation |
| Context and TUI | Seven prior native cases, no-Manager PTY /model, separate real Chat API re-entry; B synthetic same-thread incremental handoff/current input, controller fencing and pending-result/failure/service-restart reconciliation; C ten-cycle actual TMPDIR development, actual source-bound non-user successor/receipt/continued development and active pending-call interruption/restart/reconciled resume with original CLI/canonical results/final; C2 five genuine same-saved-Chat user turns, source-authored summary/current receipt and actual saved-successor coding, both Chats persist after local-cache release; initial G candidate40 Android PTY native/Chat/native with and without Manager, synthetic responses; R11 actual one-dispatch original-TUI Chat task after accepted cold resume | G: installed TUI downshift/re-entry, broader host-loss/approval/checkpoint/handoff interruption, repeated long-duration Chat rotation and retained-route lifecycle boundaries. These bounded actual renewals and active-command recovery do not establish every boundary |
| Tools/approval/question | Actual native discovery/correction, independent reviewer allow/deny effects and project isolation, blocking question result on original call; actual delegated conversation approval smoke; ten-cycle TMPDIR development with25 commands/11 patches/all36 original execution returns | G: broader function/custom/batch and host search behavior, real reviewer judgment/project instruction priority, physical question UI. Quoted executable prefix matching remains an upstream limitation |
| Existing MCP return | Actual R11 effect/result/report/final; Phase A endpoint/owner controls and exact-call regressions; C operator ten-cycle development, actual same-thread successor/continuation, active pending-call interruption/recovery, direct ~/prj/ai regression/fix/full-verifier and fresh identical-input comparison, with previous-output quality defect retained | C/C2 and D complete at recorded scope. G: apply lifecycle/product gates to model/tool adapters and managed MCP/local transports retained under F. B synthetic two-entry control and R11 two-command return retain their separate scopes |
| Cost/presentation | Stream-safe MemoryCite/box projection, exact native results/warm prefixes, D2/D3 counts/resources, direct-entry timings and F9 matched four-route timing/request/byte evidence with separately retained correction/failure costs | G: delivery/presentation gates for retained paths. F9 is one sample per route, not a universal speed winner; actual allowance debit stays unknown without measurement. No mandatory optimization loop |
| Core update information/prompt | User-confirmed85→86 success; Core bytes unchanged; release-date/other display details observed as unknown | Low priority with the next substantive Core change: [deferred Core follow-up](#deferred-core-follow-up), including component information and a prompt on each eligible bare launch; preserve upstream/noninteractive behavior |
| ChatGPT project setup/management | Existing history configuration, authenticated project list/read/create and bounded membership proof; E selected project/context/placement contracts and F qualified selected operations | G: retained product access/lifecycle combinations. Broader setup UI retains lower priority: [project setup and management](#chatgpt-projects-and-history). Preserve existing chats/context and optional-component independence |
| Viewer and desktop | E app/host management research; F Rust private RFB/HTTP/control fixtures and actual owned fresh-profile RFB; existing notification/terminal Linux viewer opener; old desktop font/icon observations | noVNC usability retains lower priority: [development host management and viewer usability](#deferred-linux-chatgpt-development-host-management). G when shipped: physical current viewer/keyboard/scroll/login; old observations do not qualify a changed viewer. Viewer stays optional for inference |
| Packaging/composition | Current88257d2a passes84 composition/113 signed-remote steps. Exact c65d3f8 CI, public Base89/Full90/Provider1/2 signing/readback and actual Termux immutable/stable install/update/rollback18 commands pass; Provider2/Core Full90 promote non-forced. Four earlier installed entrypoint development routes retain their independent actual proof | G: broader declared component/cold-recovery/in-flight version-data boundaries. Public delivery is closed, not inferred from local fixture signing. No implicit desktop download/login or permanent operating activation |
| Experiment retirement/publication | Source/provenance/user1m changes and immutable original results preserved; G public signed delivery accepted in public-delivery-review.json, including recovered Core CAS and Provider-only promotion | G: finish remaining declared runtime acceptance; permanent device/default activation is separate. H: retire only accepted replacements after unique-source/license/user-state review; retain profile-pinned/current/rollback artifacts and no uncertain auto-fallback |

Native prefix-rule matching misses double-quoted executable paths in the reviewed
upstream shell-command/src/bash.rs command_name parser. The allow/deny fixture uses
a plain owned program path; do not generalize it to quoted paths or rewrite model
commands in Provider to conceal that limitation.

R11 used the independent Termux Codex Provider connector/dedicated tunnel. Tdev's
key-file reference was reused; tunnel IDs/clients/targets remained distinct and
Tdev was unchanged. Process-only DNS relay preserved end-to-end TLS and resolver
files. Readiness metadata can diverge from an already-running tunnel child:
reconnect an owned changed alias only after settlement and inspect actual argv.
Trial service/tunnel/proxy and settled hooks were stopped/released; GUI/login,
native journals and saved conversations remain. No permanent migration followed.

Base/Full automation retains independent signed heads, immutable GitHub Releases,
exact Base references and latest-successful-Base repair target. A successful new
Base remains independently usable when Full fails; repairing the add-on targets
that latest successful Base, not the previous Full's older Base. Keep current/
previous and referenced bases. Core owns signed edition/date/base display without
Manager. Source integration, public publication and device activation differ.

Published Manager window/notification/title/tmux and Full60 quota-switch proofs
retain their original scopes: one-tap notifications never launch/resume Codex;
stock APK uses one registered dedicated window; live bare↔tmux migration/nesting
is not supported. Rotation defaults off and waits for settled tools/writer state;
Chat limits are not a Codex-account rotation target. The new composition must
preserve these features. Existing physical observations are not new device proof.
