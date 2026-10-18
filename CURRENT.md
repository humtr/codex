# Operating state and product completion

Reviewed 2026-10-18. Implementation authority is `rewrite/rust-core`; `main` pins
accepted exact source for publication. SPEC owns behavior and trust boundaries,
README owns user instructions, and this file owns current state and remaining
work. Git retains superseded plans and trial narratives.

## Operating baseline

| Component | Authority and evidence |
| --- | --- |
| Upstream | Qualified 0.163.0, source `83086188ae0614bd882818ff1aa23f3194962012`; `release/upstream.json` |
| Public/user Full | `local-relay-hosted-0-163-0-54f926437ad5-s60`; source `54f926437ad5093bb11e8d53c3502b810e514071`; published 2026-10-17T12:42:39Z |
| Base used by Full60 | `local-base-relay-hosted-0-163-0-b18cc2a2f5f4-s58`, reused byte for byte; previous user Full57 retained |
| Published TUI | Source `1c413cf`, qualification 39027733603: 48 maintained native tests, 94 retained upstream checks and lint |
| Installed opt-in Chat adapter | Payload `c3642b19a87cdace158f`; sourceBase `9a5436a` plus the user's 1m profile change; JS/Bun plus Python; signed-app local-function returns |
| Provider source integration | Active runtime/tests/licenses copied selectively from experiment `ab41733` to `providers/chatgpt`; later compaction guard and user's 1m edits retained here |
| Rust Provider | Source candidate now includes Responses HTTP/SSE, CDP, app-local tool return, private lane journal and standalone serve/status/auth commands and direct Rust app startup; native-client, project-bound GPT-6 and fresh-app pipe proof qualified; authenticated owned migration and cross-provider opaque context pending |
| Mutable pilot catalog | Last read-only inspection found all 5 old entries at 1,000,000 tokens, null explicit auto-compaction threshold; generated settings, not verified capacity |
| Experimental worktree | `experiment/web-termux` at `08e9856`; user's uncommitted 1m profile/test changes remain intact; worktree/branch not closed |

Full60 source acceptance 39033120658, Core qualification 39033121990 and publication
39033383447 passed. Main pin `b0702ba` selects source54f; promotion `b5e1d9f`
publishes Full60. The reusable main workflow remains its accepted b18 input.
`chat-full60-operating-diagnosis.json` previously verified the installed signature,
8 payload hashes/modes and 527 pilot package files without activating a candidate.
These are retained observations, not a fresh device readback for every source edit.
The current audit separately reads the pilot runtime selection and verifies all
526 immutable payload hashes against its marker with zero mismatches. Its server
PID/start/executable identity is live. Its only runtime-source difference from
recorded sourceBase9a5436a is `desktop_provider_profile.py` with the preserved1m
value. The earlier45072/source7bb installed-baseline description was stale.

Changes described below have not replaced the installed launcher/provider or
published a new generation. Settings tooling must say when saved values are not
yet consumed by the operating adapter.

## Components and approved architecture

Core, Manager, TUI and Provider are separate optional-composition boundaries.
Every Termux Codex execution passes through Core; minimum operation is upstream
plus Core. Manager, TUI and Provider do not form a mandatory launch chain.

| Component | Owns | Does not own |
| --- | --- | --- |
| Upstream | Inference, tools, approval/reviewer engine, canonical transcripts, native threads/turns/compaction/writer locks | Our signed generation lifecycle or Android window identity |
| Core | Qualified Termux executable/environment/FDs, execution-home/shared-layout binding, signed server binding, install/update/rollback and release provenance | Account/window/quota/model policy or Provider transport |
| Manager | Profiles, quota/rotation, notifications/windows/title/tmux convenience and task discovery/handoff policy | Another executor, canonical history or runtime pointer authority |
| TUI | Upstream transcript rendering and optional `/switch`, permissions and `/model` integration | Provider transport/lifetime, another backend supervisor or a duplicate model catalog |
| Provider | Rust configuration/catalog, Responses/tool-return mapping, Chat app auth context, CDP and owned-host/lane lifetime | Native tools/reviewer, canonical history or Core runtime selection |

`runtime-binding` is the small Core-defined execution-record contract consumed by
optional tools; Core links no Manager code. The remaining runtime/configuration
coupling is listed below. Existing `frontend/manager-tui`
paths and signed markers keep their compatibility spelling until a qualified
migration warrants changing them. Public naming is TUI.
The current public Full remains its previously admitted pairing. Source now
admits independent TUI composition, with signed Android execution proof below;
this does not retroactively change the installed generation.

The model catalog belongs to the Provider configuration as data. TUI reads that
catalog; Manager remains independent of model/effort lists. Data changes can add
models or correct host-model/effort mappings without a binary rebuild. Protocol
or app-bridge changes can still require code. Model listing is not execution or
entitlement proof. GPT-5.6 is excluded by the latest user decision; GPT-6 and Pro
remain, with no automated Pro inference. Preserve the hidden legacy Instant alias.

## Audit findings and current implementation

The audit covers the plan revisions beginning at `d1e7cba`, the selective Provider
integration and commits through `5521293`, including source, workflows and recorded
proof. Corrective changes are being implemented against that exact history.

- **TUI:** `095f8c5` fixed the 65-character account fixture, but native run
  39093637388 still failed 1 of 48 tests. Its Policy tab fit entirely at 24 columns;
  the test wrongly required an ellipsis even where no text was clipped. Account
  and Custom tabs still require truncation. `93d7491` corrects this assertion;
  native acceptance39097582554 passed all48 focused tests,94 retained upstream
  checks and the lint gate. The earlier failed run's skipped checks are not proof.
  The candidate is not installed or published; published TUI authority is unchanged.
- **Core:** explicit `CODEX_TERMUX_CORE_ONLY=1` bypasses optional preparation,
  configuration and TUI while retaining execution-home/shared-storage/runtime
  validation. Effective settings now use immutable per-launch snapshots. The
  shared base directory contains Core defaults only; opaque optional fragments
  are admitted by the real upstream loader in an empty, no-inference environment.
  Syntax/type/envelope/hung failures retain Core defaults before any dispatch.
  Server identities bind exact config/requirements as well as home/runtime/catalog;
  equal settings reuse a server without rewriting live FD34. One current
  `config-v1` execution-record codec serves Core and Manager, without historical
  experimental decoders or staged reader/writer releases.
- **Independent TUI:** Core and release builder no longer require a Manager
  artifact to admit or run a qualified same-version TUI. `/switch` reports its
  optional profile capability unavailable; native UI remains live. Signed Android
  `core-settings-android.json` passed57 lifecycle steps, real config RPC/FD34
  normal→direct→normal live-sibling isolation, four admitted optional-artifact
  failures, install/update/rollback and an actual TUI-without-Manager PTY. These
  are disposable-root executions, not a production/device activation or Chat trial.
- **Rust settings:** initial `23b7be4`/`4975616` supplied a standalone binary, not a
  Rust inference adapter. The CLI could not construct project mode one field at a
  time; show/effective were identical; model capabilities were fixed Rust values;
  1m was incorrectly called a qualified ceiling; setters omitted transport fields;
  writes lacked cooperating-writer/editor-conflict handling. Corrective source
  supplies data-driven catalogs, atomic multi-field set/unset/edit, effective
  origins, bounded private I/O and conflict/no-op tests. Saved draft settings still
  do not configure the installed JS adapter.
- **Evidence and CI:** source4975616 Provider gate 39094807348 and source acceptance
  39094807266 passed; source5521293 acceptance 39095182319 and Core gate39095212491
  passed. The first new check-script expectation failed at23b7 and was corrected
  at497. Initial manual Android CLI observations lacked a retained automated driver;
  the replacement `tests/config_cli.rs` exercises the real binary on Android and CI.
  Provider cache assertions now check their own cache. Pin its Rust toolchain
  separately from Core. Generic smoke is never feature-specific acceptance.
- **Experimental comparison:** the main integration added the later compaction
  anchor guard, preserved 1m values and explicit license assets. Their absence from
  the older experiment baseline is not evidence that experiment removed them.
  Keep these improvements during selective integration; preserve unique probes,
  proof and user edits before branch closure.
- **Installed baseline:** actual selected pilot is c3642b19/sourceBase9a5436a,
  not the previously recorded45072/source7bb. It lacks the two later source
  compaction guards and new context observations. Source tests do not install
  these fixes. Preserve the running pilot until a bounded qualified transition.

## Product completion scope and work order

The deliverable is a stable Core plus optional Manager/TUI and opt-in Rust ChatGPT
Provider, preserving native features and current user data. Complete independent
source work while slow hosted tests run, but admit releases in dependency order.
No arbitrary checkpoint cadence or extra planning hierarchy is required.

| Order | Work | Required result |
| --- | --- | --- |
| 1 | Correct this audit, `/switch` fixture and config/catalog defects; update contracts and checks | Consistent docs; executable CLI tests, failure/negative controls; 48 native TUI tests plus 94 retained upstream checks/lint; narrow one-line rendering and navigation |
| 2 | Finish Core/optional-component boundary | Manager-absent/broken/hung paths; same confirmed home; optional config syntax/type isolation; immutable effective-config server identity; safe pre-dispatch failure; no post-handoff replay; signed Android normal→direct→normal with live sibling |
| 3 | Establish a Rust Provider vertical path using disposable fixtures | Independently written request parsing/SSE/tool mapping, bounded HTTP/CDP and versioned minimal app JS bridge; original call/result IDs; actual native tool loop and public transcript; no Bun/Python product wrapper passed off as migration |
| 4 | Connect one configuration/catalog authority to runtime and TUI | Global file use without Manager, invocation/per-model/shared precedence and effective origins; generated upstream catalog/profile outputs; data-only model changes; `/model` same-provider model/effort changes without restart; native sibling isolation |
| 5 | Repair context and native↔Chat transitions | Previous-native-model compaction and opaque checkpoint handoff; 1m override; warm deltas; one canonical checkpoint application; manual/automatic/interrupted/repeated compaction and cold resume, both directions |
| 6 | Implement remote project destination | Read-only app-operation discovery; configured project resolve/reuse/create/place/readback scoped to account/workspace; serialized uncertain creation; changes affect new lanes; existing chats preserved |
| 7 | Complete lifetime and development qualification | JSON/freeform/tool discovery/search/batches, questions/reviewer, errors→repair→tests, concurrent lanes, actual output loss, stop acknowledgment, reconnect/latest-state resume, real sustained development directly in `~/prj/ai` after no-MCP completion, and reboot; no duplicate effects |
| 8 | Qualify independent MCP return and optimize cost/output | Connector `Termux Codex`, configurable display/identity, actual original-call ACK; no automatic unqualified fallback; lazy tools, setup/warm latency, safe public markup; separate user/tool/start/retry/allowance counters |
| 9 | Finish experiment integration and retire superseded runtime | Port useful drivers/fixtures/provenance with their implementation, preserve all unique/user work; one Rust Provider authority; only necessary app JS/viewer remains; no product Bun/Python dependency solely for adapter; signed rollback targets preserved |
| 10 | Package, release and activate | Authenticated Rust/bridge/assets inventory and compatibility; Base unchanged by optional-only inputs; Full includes admitted Manager/TUI, Provider opt-in; install/reinstall/update/rollback tests; sign/public readback/disposable update/non-forced CAS, then bounded device activation |

After the Rust no-MCP path is complete, run the actual development qualification
against `~/prj/ai` itself, as the user explicitly requested on2026-10-18. Do not
create a temporary worktree, clone or copied project for this trial. First read
that repository's AGENTS/contracts, record branch/HEAD and staged/unstaged/untracked
state, and preserve unique uncommitted source before model edits. Git history
alone does not preserve uncommitted or untracked files. Give the ChatGPT-backed
Codex a meaningful real improvement task; it must analyze the repository, implement,
run relevant tests, diagnose and repair failures, then review the resulting diff.
Do not implement the change in the supervising agent and present it as model work.
The supervisor must launch and follow the run, independently inspect its actual
diff and execute the applicable checks; the model's own success report is not
acceptance. This trial starts after no-MCP completion, not on the legacy adapter
as a substitute for qualifying the new Rust path.
Keep inference submissions/tool returns/retries separately counted, with a stated
live budget and no automated Pro. Observe the native initial/intermediate/final
transcript, exact effects, stop/reconnect/latest-state resume and any failed tool
paths. Preserve existing user work, credentials and operating AI settings/runtime;
source development does not authorize incidental deletion or automatic activation.
Report the actual task, source diff and tests, failures and remaining limitations;
a synthetic or disposable-project run does not satisfy this acceptance gate.

For Rust migration, first port protocol/state fixtures and a real native-client
fixture loop, then app process/CDP control and original-call local-function return.
Keep app-local capability discovery/invocation/event delivery in the minimal JS
bridge; Rust owns policy, IDs, state, cancellation and reconciliation. Retained MCP
uses the same native executor boundary. Read-only app/project discovery may happen
early to identify actual host limits; no extra live task is submitted just to inspect.

Core isolation must bind the opaque effective config and requirements (including
absence) before another launch can replace them; merely hashing a later read of
the mutable global directory is insufficient. Reusing a server must never rewrite
its live FD-34 snapshot. Keep catalog/model policy in its existing owner. Use one current execution-record codec and fresh complete-composition proof for
discovery, profile deletion, retirement, install/update and rollback. The user
rejected historical experimental codec support and a staged reader/writer
rollout. Unsupported records authorize no operation; preserve their processes
and data, and verify native process/writer state before mutation.
Use the upstream configuration loader for syntax/type admission, without adding
a competing Manager policy parser to Core. Preparation failure can fall back only
before dispatch and with confirmed execution-home identity; never replay work.

The Rust candidate now has an HTTP/SSE endpoint, owned CDP client, minimal app JS
bridge and local-function lane engine. It does not yet replace the operating
adapter. Deterministic fixtures cover original-call batched results, warm context,
public transcript projection, lost-observer reconciliation without resubmission,
private endpoint proof and data-only model-route reload. They are not official-app
inference or native-executor effect-count proof. A fresh official app now boots and shuts down through Rust-owned PRoot/Xvnc and
private Chromium pipes without Bun/Python or touching existing profiles. Actual
Android Codex0.163 drives commands, a custom patch and dependent validation through
Rust, retaining the transcript with three exact effects and no repeated setup/user
prompt on tool return.
These are separate proofs: the native-client app replies are synthetic, and the
fresh official-app trial sends no inference. A later actual GPT-6 vertical trial
through the existing authenticated app's explicit relay also passed, as detailed
below. Authenticated owned-app migration, restart/reboot, project placement, opaque
compaction and full development qualification remain open. The legacy runtime
remains the equivalence reference. Port useful real-Core scenario drivers alongside each
boundary; do not defer all test migration until experiment closure. Preserve
attribution for translated/reused dependency code and never replay a live request
to both implementations for comparison.

## Context and easy configuration

The single draft file is `$XDG_CONFIG_HOME/codex/providers/chatgpt.toml`, defaulting
to `~/.config/codex/providers/chatgpt.toml`. It owns settings and `catalog.models`;
initial model defaults are a data resource, with editable configuration overrides.
The source CLI offers models and config show/effective/validate/set/unset/edit.
Its effective output reports `runtime_application=not_queried`; generated pilot
catalogs and registered homes still use the old adapter until migration is admitted.
The Rust candidate server consumes this file directly at each request admission;
its separate listener status never claims app connection or model/task progress.

Set related values in one transaction. Edit structured catalog entries through the
same validator. Show defaults versus file origin, preserve no-op files and reject
invalid/stale edits. Use one validator from any future TUI/Manager convenience view.
Model data changes can map a new ID/effort without code; they do not grant access
or prove the signed app supports that route. Host protocol changes remain code work.
Invocation model/effort routing and next-request file reload have deterministic
proof. Model-specific context settings now override file-wide defaults and are projected
into immutable upstream catalog snapshots; ordinary invocation overrides remain
upstream-owned. Effective output exposes each model's values/origins. Candidate
`catalog` emits current upstream JSON, and explicit `prepare --home PATH` writes
one generated shared profile with no bearer plus a no-replace per-home link.
It authenticates the endpoint and may start the same Rust executable on demand
using the already configured/provisioned rootfs; it never installs an app, submits
Chat or restarts a live owner whose readiness failed. Concurrent startup serializes.
Read-only `status` does not start anything. Actual Android empty-profile proof in
`provider-auth-bootstrap-android.log` confirms two concurrent auth commands share
one service, shutdown retains its profile, restart reuses the endpoint and rotates
the credential; existing-profile access and Chat submissions are both zero. The
initial proof exposed a concurrent directory-creation race, fixed by revalidating
the winning private directory. An occupied old port fails before app startup.
Actual Android Codex0.163 now uses this generated `--profile chatgpt`, command-backed
auth and native catalog for commands, freeform patch and dependent validation.
`provider-real-native-catalog.log` records three exact effects, four app-wire
submissions and no Chat inference; app replies are synthetic. TUI source now reads
the Rust auth contract and catalog-defined models/efforts, without fixed families
or Python launcher references. The catalog gate at310f0f7 passed40 Rust cases,100 Bun and28 Python cases; targeted
CLI/profile tests and the generated-profile Android test also pass. Python repository
gates pass18 script,106 workflow and11 frontend tests. The candidate X server uses
`-rfbport -1` (per its installed manual) and a private Unix viewer socket; X11 TCP
is disabled. Actual owned-host startup/shutdown was repeated on those arguments.
The native fixture reports two user-role objects (one visible) in its initial
synthetic host submission and no repeated user objects on tool return. These
counts do not establish Chat allowance deductions. Hosted TUI qualification at
310f0f7 passed in run39107948384; actual Android TUI composition/application and
production activation remain pending.
Lane identity now binds the authenticated user/account/workspace and owned host
generation. App-wire regressions cover account mismatch, refusal to replay pending
calls into a replacement host, and settled cold reconstruction only at a new native
user turn. The updated Android generated-profile/native-tool fixture retains three
exact effects and zero Chat submissions (`provider-real-native-identity.log`).
Actual read-only inspection of the logged-in app now verifies identity and local
function/stream capabilities through Rust CDP without installing hooks or submitting
Chat (`provider-authenticated-app-readonly.log`). A separate real GPT-6 no-MCP
vertical trial used the Rust Provider, generated profile and Android Codex0.163:
two native commands, custom patch and dependent validation, three exact effects,
four actual app dispatches, two user-role objects (one visible) and three tool-role
returns; no new user objects on warm tool returns and no Pro/automatic resubmission.
The final `idle` journal and native transcript were rechecked without another
inference after a test-only uppercase/lowercase assertion error. The owned bridge
was released/disposed; existing app/authentication and its remote trial chat remain.
See `provider-live-no-mcp.log`, `provider-live-no-mcp-recheck.log` and the private
`provider-live-no-mcp-native.jsonl`. These counts do not establish allowance units.
The settled disposable source/home/journal were archived as
`provider-live-no-mcp-state` under the same private acceptance root, preserving
the proof while removing its temporary working root. The earlier Provider gate
passed45 Rust cases,100 Bun and28 Python cases plus formatting/lint/release build
(`provider-final-identity-lifetime.log`); ignored live/Android cases are separately
qualified above, never counted as executed by that offline gate.
This trial explicitly used the existing authenticated app's private relay; owned
Rust app startup has separate fresh-profile Android proof. It is not yet a complete
operating migration, long-development qualification or a reason to start the AI
trial before the remaining no-MCP admission work. Authenticated owned-app migration,
project/context transitions and unsettled-host-loss reconciliation remain open.

The 1,000,000-token value is the user's policy override, not measured Chat capacity.
Do not impose it as a maximum or 32k as a minimum. Native token triggers, opaque
model/hash compaction, encoded/decoded HTTP ceilings and true host context are
separate boundaries. An unset explicit token threshold uses upstream defaults.
The successor-prompt reply→compaction→new-Chat cycle needs observation of trigger,
checkpoint acceptance, lane retirement, actual chat creation and retransmission.
A larger catalog value alone is not a repair. Preserve canonical history and pending
results; apply each acknowledged checkpoint once. Deterministic warm-prefix and
repeated-summary guards already exist in JS and must move with the Rust path.

Native context repair now recognizes actual upstream compaction metadata and
explicit V2 triggers. Responses Lite declarations are transport metadata, not
conversation history; their removal during native compaction must not trigger
`native_context_prefix_changed`. Transient summary prompts/replies do not extend
the accepted prefix. Apply an offered checkpoint only after actual canonical
history replacement, exactly once; restore native policy on continuation.
`provider-compaction-final-android.log` proves actual Android Codex manual compact
commits its readable summary and continues the same native thread (three synthetic
app replies, zero Chat submissions). App-engine fixtures cover V2 `ocx1:` summaries,
restart/acknowledgment and unchanged-history guards. This does not decode foreign
OpenAI checkpoints or qualify automatic/model-downshift/cross-provider compaction.
The latter remains an explicit unfinished transition boundary.

The same audit repaired an owned-host shutdown defect: repeated stop could signal
the caller's process group after the stored PID became zero. A separate-process
regression now proves repeated stop leaves its caller alive. Normal host startup/
shutdown and authenticated migration remain separate gates.

## ChatGPT projects and history

Retain history by default during migration. Configured project mode should reuse
or create the named project before the first new user submission. A saved setting
is not host implementation. Bind exact project ID to authenticated account/workspace;
resolve duplicate/inaccessible/deleted projects explicitly, reconcile uncertain
creation and never silently fall back to root chats. Model/effort changes should
reuse a settled lane; project changes affect new lanes and do not move/delete old
conversations. The Rust fields are `history.mode`, `history.project_name`, optional exact
`history.project_id` and `history.create_if_missing` (true by default in project
mode). The installed pilot does not consume these settings.

Project instructions/files/memory can affect behavior. Verify instruction priority,
native reviewer/tool boundaries and sibling/guardian isolation. Do not alter global
personalization/sharing silently or assume project+temporary-chat support. Grouping
reduces clutter, not stored history or Chat allowance consumption. Explicit future
archive/delete/temporary modes need their own acceptance; never mass-delete chats.

[Official projects documentation](https://learn.chatgpt.com/docs/projects?surface=app)
was checked on2026-10-18 and describes user-facing project context. It does not
establish an API in the installed signed app. Current local-function source retains
normal chats without project binding; older text transport uses temporary chats.
The Rust bridge now invokes the installed app's authenticated project list, read
and create operations, with no token export. Read-only discovery passed with zero
Chat submissions (`provider-project-discovery-android.log`). Actual create/reuse/
readback of one `Termux Codex` project passed with zero Chat submissions
(`provider-project-create-minimal-body.log`); the project is preserved. Earlier
failed probes are retained and do not count as successful acceptance. Use the
app's minimal creation body and explicit no-retry policy. Creation uncertainty
is journaled before dispatch and reconciled by exact destination instead of
blind retry; definite server rejection has a separate recoverable error.

`provider-live-project-native.log` and its private native JSONL prove actual GPT-6
inference in that project through Rust and Android Codex: two commands plus a
custom patch/validation, three exact effects, four app dispatches, two user-role
objects (one visible) and three tool-role returns. No additional user object was
submitted on tool return. The bridge checks project ID before every stream and
this driver independently read back the saved conversation's project membership.
There was no Pro sampling or automatic request replay. Chat allowance deductions
remain unknown. The installed pilot and existing chats were not changed.

Source fixtures cover duplicate names, inaccessible/deleted exact IDs, account
scope, definite rejection and uncertain creation reconciliation. Project instruction
priority, guardian/sibling isolation and complete owned-app migration still need
qualification; successful HTTP metadata alone does not prove those properties.

## Confirmed operating Chat blocker

The retained Full60 diagnosis recorded 2 long-thread attempts (Extra High/High),
both failing400 `unsupported_desktop_route` before Chat submission. Native
ModelDownshift compaction sent previous model `gpt-6.1-sol` to the selected Chat
endpoint: 2 arrivals, 2 admission failures, 0 admitted requests/user submissions.
The stored transition also contains an opaque upstream checkpoint, not readable
ocx1 context; it was not a captured failed wire body. No-inference HTTP controls
prove both rejection boundaries. An alias rename does not repair opaque history.
Fresh Chat avoids that transition, but fresh real inference under Full60 was not
requalified by this diagnosis. The 1m edit alone proves neither repair nor capacity.
Proof: `chat-full60-operating-diagnosis.json`,
`chat-full60-cross-provider-admission.json`, `chat-full60-diagnosis-python.log`.

## Retained evidence and its limits

- Full60 delivered quota-aware `/switch`: usage/freshness/errors, pool enrollment,
  combined/5h/weekly/remaining/custom ranks, reserves/order/weights, cooldown,
  exclusions and session-off. Default rotation is off. Identity/quota/model/effort/
  permissions and settled tools/writer release gate handoff. No prompt/tool replay;
  Chat limits are not a Codex-account rotation target. `quota-qualified-device-matrix.json`
  and public57→60/rollback/forward proofs cover fixture effects, not real depletion.
- Published Base/Full automation has independent signed heads, immutable GitHub
  Release assets, exact base references, latest-successful-base repair target,
  public/disposable update proof and CAS. Release edition/date/base is signed and
  displayed by Core without Manager. Full60 Base fingerprint is
  `43e0b888ec06af28e2e98d55cdd30adb9f01e5ef9d3e8f691cfab7c6e2a97a3c`.
  Retain current/previous and referenced bases, not date-only cleanup.
- Historical Core-alone lifecycle has 30 signed build/install/update/rollback
  steps. `integrated-lifecycle.json`, `integrated-profiles.json` and
  `product-preparation-android.json` cover earlier layouts and profile/lease/writer
  safety. They do not qualify the new direct flag, malformed upstream settings or
  post-exec failures. Fresh/resume/btw permission proofs retain dangerFullAccess,
  independent reviewer and native commands without bwrap in Full60.
- Native and stock-APK window return, optional tmux, `/switch` window retention
  and `/title` have physical confirmations at the deployed inputs. A notification
  never launches/resumes Codex; one tap consumes it. Stock uses one registered
  dedicated window; live bare↔tmux migration is unsupported; nesting is refused.
  `stock-window-proof38137172382` and `notification-consolidation-device.json`
  retain those scopes; they are not fresh physical proof of this candidate.
- Installed local-function transport returns tools on their original host calls
  without one new Chat user-message per result. Retained independent MCP reaches
  host `confirm_action`; discovery is not result ACK. Use connector display
  `Termux Codex` for new proofs, read its identity from config and preserve exact
  native call/result IDs. Historical `Codex Native2` is provenance, not a required
  name. Existing authorized admin/computer-use access can be reused when needed.
- Legacy helpers for parser/SSE/queue/tool mapping/turn identity/summary derive
  from pinned miuuyy6.1.5/source92a356f with licenses and maintained patch; our
  app transport/lifetime is repository-owned. Source7bb202e had103 Bun/45 Python
  tests; the selectively integrated gate runs a smaller runtime-focused suite.
  Those numbers describe different scopes. Historical signed-app proof records
  2 user objects, 2 local returns, 3 starts and public initial/intermediate/final
  reports restored in cold TUI. Earlier32k/copy-ai trials do not qualify1m or Rust.
- Output-loss/reconnect, tool search/discovery, JSON/freeform repair, batches,
  driver-answered questions and compaction have deterministic/older bounded proofs.
  Physical questions, actual-host observer loss, hours-long work, reboot, new Core
  configuration isolation, projects and the current real-repository development trial remain.
  Show actual public progress, acknowledged stop state and latest resume; neither
  a heartbeat nor a saved result file alone proves the user-visible problem fixed.
- Count user objects, tool returns, starts, HTTP/setup/summary/retries and actual
  observed allowance deductions separately. Codex quota experiments do not prove
  Chat deductions. Preserve useful text from MemoryCite/unsupported markup without
  executing HTML or fabricating private reasoning. SIWC/public Responses remains
  a distinct research path, not inferred entitlement or automatic fallback.

The current context/project Provider gate passed56 Rust cases,101 Bun cases and28
Python cases, rustfmt, clippy and release build
(`provider-context-project-full.log`). Separately executed actual Android proofs
are `provider-native-fixture-final.log` (three exact native effects, zero Chat),
`provider-compaction-final-android.log` (same-thread manual checkpoint, zero Chat),
and the project-bound actual GPT-6 proof described above. Ignored live tests are
not included in offline counts. Core/Manager/TUI sources are unchanged by these
Provider repairs.

The direct `~/prj/ai` development gate is now being exercised through this Rust
no-MCP transport and its qualified authenticated relay, not the legacy Responses
adapter. Baseline `main` at1f958e5e078b8076a350fa8fac9f7664cbec61f7 was clean and
`verify/final-verify.sh` passed48 checks with zero warnings/failures before model
edits. The private task/baseline/transcript and subsequent independent review
belong to the existing acceptance root. GPT-6 High is bounded to one initial
user task and at most39 subsequent app submissions, no Pro or automatic replay.
Do not declare sustained development acceptance before the actual diff and tests
have been independently inspected. Complete owned-app migration/reboot, foreign
opaque handoff, independent MCP and signed optional packaging remain separate work.

## Verification and publication

Use offline boundary tests first. Runtime/state/security/installer changes and
release candidates need full affected acceptance and real Android execution.
Reuse evidence only for unchanged relevant inputs/scope. Live trials must explain
necessity, model, expected user/setup/summary/retry counts and mutation scope before
execution; no automated Pro sampling or uncertain-delivery replay. Current audit,
settings and native fixture checks need zero Chat submissions.

The 2026-10-18 corrective CLI run on Android passed 12 Rust unit/integration
tests, rustfmt, clippy and release build. Python checks passed 18 script, 106 CI,
11 frontend and 28 Provider tests; docs checks and diff checks passed. Additional
check/cache-scope regressions passed 3 tests. Logs are retained under
`~/.cache/codex/acceptance/development-audit-1t4txdwt`. This is local configuration
and source proof, not Rust inference, hosted-toolchain or deployment acceptance.
Exact source `b60f28f` subsequently passed Provider gate39098078633 with pinned
Rust1.98.0/Bun1.4.0 and source gate39098078609. Their logs and native TUI logs are
retained in that same acceptance directory. No Chat request was submitted.
`direct-core-android.json` additionally records 39 signed lifecycle steps with
real upstream0.163.0, unchanged Manager reuse, the direct flag and invalid-default
controls. The owned fixture was removed and its preservation sentinel survived.
The first new driver attempt used the launch command `profile use` where setting
a preference required `profile default`; it refused before opening a TUI and was
corrected before the successful run. This was a fixture error, not a product
failure or inference trial. The renamed unit test states its actual scope:
Manager calls/config projection, not unexercised UI behavior.

Core isolation source7d179af passed source acceptance39104609832 and independent
runtime qualification39104609914. The initial renamed-test CI wiring failure was
corrected: its focused gate uses the admitted Android Core and requires exactly
one executed test. Local Core/Manager/builder suites and clippy passed; current
signed Android proof `core-settings-android.json` has57 accepted steps, live FD34
and config-RPC sibling isolation, optional syntax/type/envelope/hung failures and
actual TUI without Manager. Its owned roots were removed; zero Chat submissions.
No installed generation or operating adapter was replaced.

Rust vertical-path logs use the same acceptance directory. Exact source13967e4
passed hosted Provider39105622016, source39105622017 and Core39105654233 gates.
The subsequent local gate `provider-owned-native-full.log` passed36 Rust cases,
100 Bun cases and28 Python cases, formatting, clippy and release build. The two
explicit Android tests were also executed separately (not counted as passed merely
because the offline gate marks them ignored). The stop/recovery work preserves cancellation intent, permits recovery
only on a new native turn after acknowledged stop, and reconciles the same app
round after observation loss. Opening/release are idempotent; cached public output
and original tool results are not resubmitted. Private pipe framing has fragmented/
out-of-order/closed-observer tests. `provider-real-native-client.log` records actual
Android Codex0.163, two dependent effects and native transcript with synthetic app
replies. `provider-owned-host-android.log` records official app26.1007.21434 startup,
unique app target and shutdown over Rust private pipes in a new disposable profile.
Neither submitted Chat or read an existing login. Full local/hosted regression gates
must cover the final changed inputs before release; the old installed adapter
remains the operating authority. Host-side web-search declarations are separated
from executable Core functions; real search/public-tool transcript qualification
is still pending. The new `host.rootfs` selects a provisioned rootfs at service
startup; it does not download an app or migrate authentication.

Final composition proof covers Base without optional components; Manager without
TUI; TUI/Provider without Manager; direct Provider `--profile` use; Full plus opt-in
Chat; missing/incompatible components; install/reinstall/update/rollback and bounded owned-state cutover without historical record-codec emulation. Native transcripts, exact effects, stop/resume and project
membership are observed at the actual path. Source integration, signed publication
and bounded device activation are different steps. Preserve credentials, profiles,
conversations, running work and experimental edits throughout.
