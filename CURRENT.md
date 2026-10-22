# Operating state and product completion

Reviewed 2026-10-22 UTC. `rewrite/rust-core` owns implementation; `main` pins
accepted exact sources for signed publication. SPEC owns product contracts,
README user instructions, and this file the operating baseline and remaining work.
Superseded development narratives remain in Git at `e0f0927` and earlier commits;
original private evidence remains under `~/.cache/codex/acceptance`.

## Current status and next action

A–D, E design, F1–F9 bounded integration, G qualification/public delivery and H
source retirement are complete at their recorded scopes. H is committed and
pushed as `e0f0927`; its Rust runtime/embedded assets are unchanged from accepted
G source `6aa228034346f43b39590afe83aa9232e9e72207`.

The operating device update is also complete: signed Full93 (previous Full86),
ChatGPT Provider release sequence3, generated Rust profile, same-account native
host and existing Termux Bridge tunnel on a permanent path.0 new model requests.
The next work is the separately deferred optional backlog below; G/H needs no
replay or additional release. Existing clients remain pinned until normal exit;
new launches use the selected Full93.

Provider is the component name; `chatgpt` is its CLI identifier. Release
sequence3 identifies one publication, not a product named “Provider release sequence 3”. The bridge
is the Provider's app/native tool-and-result connection; MCP is a protocol it
uses. The app displays that MCP connection as **Termux Bridge**.

Keep accepted implementation, signed public selections and actual running
processes distinct. Existing sessions retain their pinned executable; a new
selection does not upgrade their process in place. No completed reviewer,
program effect, Chat trial, signing or promotion is scheduled for replay.
Unknown `providers/chatgpt/target/` content remains untouched.

## Operating baseline

| Plane | Verified state |
| --- | --- |
| Implementation | Accepted G source6aa2280; H source retirement e0f0927 on rewrite/rust-core. H Provider167 Rust/67 JavaScript/81 Python; full364 Rust/141 Python and Clippy pass. Provider release sequence 28 opt-in tests and Core1 ignored test remain explicitly ignored |
| Public Core | Base89; Full93/source5da0254d. Provider-only main promotion6e89e76 leaves Core pointers unchanged. Earlier Full90 admission retains its own evidence; it is not evidence for different Full bytes |
| Operating Core | User-authorized signed Full86→Full93 update exit0; previous Full86 retained. Upstream0.163.0. Original Full86/88 and account/shared-profile backup retained in device-runtime-update-full93-provider-s3; next launches use Full93, existing clients remain pinned |
| Public/selected Provider | chatgpt-6aa228034346-s3, release sequence3, binary SHA2565ef3536c94137b24d86e10a9366a674baeca87e701c5ef30d824a83d8433d62d. Public immutable signing/readback, Android consumption and stable promotion accepted; operating installation exit0 |
| Host/profile | App26.1007.21434 / Chromium155.0.8059.40 under the signed Rust-owned private-pipe host; same authenticated account and no active lanes verified. Original login/history retained. No new Chat inference or physical viewer observation is claimed |
| Retired source | experiment/web-termux08e9856 and experiment/web-provider6aab6cd refs/history preserved; exact299-file snapshots, user1m profile/test edits, Cargo.lock deletion, licenses and restore records under phase-h-retirement-6aa2280.34 duplicate files and2 inactive worktrees retired |
| Retained dependencies | Immutable JS/Bun/Python installation c3642b19a87cdace158f and its526 verified files, rootfs, optional noVNC, current/previous payloads and original proofs remain preserved. Source retirement did not remove installed user data |

### Latest handoff boundary

Device evidence: `~/.cache/codex/acceptance/device-runtime-update-full93-provider-s3`.
`device-update-review.json` accepts the actual Core/Provider selections, kernel
owner plus immutable Provider hash, same-account host readiness, authenticated
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
| Device update | Separately authorized after G/H and completed: Full93 / Provider release sequence3, explicit profile/auth/history migration, owned-host/local MCP/tunnel and8-command operating smoke accepted;0 model requests |

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
