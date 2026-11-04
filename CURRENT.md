# Product state and completion roadmap

SPEC.md owns behavior and component boundaries. AGENTS.md owns development rules
and must be reread after recovery. This file distinguishes source implementation,
automated proof, actual Android execution, connected-client proof, public signed
delivery and device activation. None substitutes for the next boundary.

## Current baseline and takeover verdict

### P0: Active Steer batch — delivered and Jan service activated

The maintained TUI sends active Steer through upstream's original Native writer.
Image preparation retains its draft if the destination turn changes. Provider
AppHost owns same-Chat continuation and validates the original submitted prefix,
new user input and required Native results. Operator waits for a canceled
predecessor task to retire before admitting its successor, then revalidates the
existing Stop/controller/input/effect fences. MCP owns confirmed-Stop settlement
and capability retirement. No Core or upstream Native executor was added
(SPEC.md4.5–4.6); temporary diagnosis code was removed.

The accepted release source is e8e2c1b; Provider source is da5bf62. Clean work is
in `$TMPDIR/steer-batch-v8lgrrmr/source`. The original dirty0814be7b workspace,
including unrelated account-management work, remains preserved. After recovery
reread AGENTS.md and inspect the actual branch, diff and unresolved findings.

Frontend evidence reuses successful native run41834657202 atdb307ec:144 actual
focused tests, upstream features, format/lint and artifacte286df94. Expected-count
correction143→144 and literal job-budget90→120 do not change compiled inputs.
The qualifier checks the retained log against the current count and still binds
source, commands, toolchains, dependency audit, successful run and artifact.
Reuse/controller regressions and the targeted false-rejection mutation passed.
Unnecessary rerun41850957815 was canceled at the user's objection; canceled
run41842123715 retains its successful baseline/Steer-fault step but is not used
as successful build authority. No unchanged native CI is restarted.

Provider's full gate passed186 library tests plus integration, strict Clippy,
release and Bun/Python checks. The cancellation-versus-retirement fault failed
its regression and was restored byte-for-byte. Diagnostic-free Provider8a392dee
and frontende286df94 passed actual Android held-Stop Steer, Queue/Shift+Left,
Native tools/cold resume/Stop/control, signed package/TLS/rollback and Base,
Manager-without-TUI and Full compositions. Both Steers occurred once on the
original Native turn, without a false queue-edit hint. Chat responses in these
fixtures were synthetic; no paid model or physical UI observation is claimed.

Provider25 is signed, completely publicly read back and accepted by14 disposable
Android consumer steps, then promoted non-forced at maind64e70e. Signing
run41854609332 failed only release creation with403; the existing owner published
the retained, reverified signed assets without rebuilding/resigning. Its failed
workflow status is preserved. Full publication41858194993 succeeded with the
same accepted source/frontend after re-evaluating the independently changed main
pointer. Base147 and Full148 passed public readback, disposable update/rollback
and non-forced promotion. Public assets and previous generations are retained.

The device updater selected Full148/Base147 and signed Provider25, exited0 and
passed installed version/readback checks. At that package boundary31 protected
files and all4 operating Provider owners were preserved. The subsequent bounded
Jan activation deliberately changed its runit executable, the same tunnel's MCP
command and only the authentication executable field in8 referencing profiles.
Original configuration bytes are retained under the owned live-cutover root.
The default Provider and existing Native/app/browser owners were preserved.

The CDP409 was competition for a single connection already owned by the old Jan
Provider. Serial maintenance stopped that service, used the existing temporary
qualification relay for canonical `scripts/d3_refresh.mjs`, then stopped the
relay with confirmed kernel exit. Signed Provider25 now owns the original direct
app connection; no permanent relay or new refresh API was added. The refresh
called app refresh_actions, invalidated client caches and verified all17 complete
registered/MCP input schemas, including inference. A new connected Native work
completed create/start/attach/context acceptance/tools, command execution, exact
retry, independent readback and finish. The append happened once; final Native
status is idle/completed with no pending calls or unconfirmed effects.

Evidence is under
`~/.cache/codex/acceptance/goal-defects-20261104/`: final Android/mutation/public
consumer results, `device-update-final-after.json`, `d3-provider25-live.log` and
`provider25-connected-native-final.json`. Earlier failure records remain
historical observations, including `live-client-remaining-boundary.json`; they
are superseded by the successful activation/refresh/Native evidence above.
Manifest-owned inactive fixture payloads were archived with restore records;
compiler cache remains within the15GiB budget.

Original works5c43343f...,385d5be4... and a65b56a1... remain
reconciliation_required/notLoaded on their last observation. They were not
stopped, claimed or replayed. This batch repairs the current connected path; it
does not claim recovery of those original turns or universal P0/NC-01..29
acceptance. The reserved Pro trial and user chats/credentials/jobs are untouched.

| Boundary | Verified state | Qualification limit |
| --- | --- | --- |
| Accepted source | rewrite/rust-core e8e2c1b; Provider da5bf62 | Original dirty0814 workspace is independent |
| Selected Core | Full148 / Base147 / upstream0.166.0, local-relay-hosted-0-166-0-e8e2c1b2e10d-s148; previous Full144 retained | Selection is distinct from existing retained Native server144 and physical UI |
| Selected optional Provider | Signed Provider25, chatgpt-da5bf622bc49-s25 | Package selection and running process were checked separately |
| Operating Jan Provider/MCP | Both executables are installed signed Provider25, SHA2568a392dee5143966f63624eeb32c89417a012b4bae2474c2035ead7c55438efb3; same tunnel and app session | Default chatgpt service retains its2328b656 development image |
| Linux ChatGPT consumer | Canonical d3_refresh on the live owned Jan desktop and @TermuxBridge asdk_app_6ae9d933635081919b50090c57464b81;17 complete schemas agree | Actual client API path; no physical UI observation or autonomous model-choice claim |
| New Native work | 2d7a652d6c357e6bb14642adced0170d1930198586a14284; thread01a1a76d-952c-7e41-8db9-4ff21d226ffd; completed and idle | Attached inference, zero additional model requests; two Native commands, exact retry, one append |

The historical takeover sections below retain earlier evidence and unresolved
scope. The current table supersedes their older operating-generation claims.
Revalidate process image/start-time before another live action.

### Actual handoff boundary and subsequent integration

The original development rollout is
`01a177ab-ad95-77c3-8f3f-5b9d083162f6`; the later runtime qualification rollout
is `01a16fac-ff74-7bf1-8b48-9d0c1ba8c3a8`. Their original files remain under
`~/.codex/sessions/2026/10/26/` and `2026/10/24/` respectively. The later rollout
continued into November; directory date alone does not bound its contents.

The actual last committed source edit in the first rollout is `cc6ea6d`
(2026-11-02 06:30 UTC): settle conflicting Native calls before dispatch. Its last
uncommitted source edit at06:37 UTC adds the NC-14 RED regression in
`providers/chatgpt/src/native_effect.rs`. `807a21d` subsequently integrates that
regression and its non-login Git-read implementation; `d3f3b1e` closes the
omitted-login bypass. Thus807a21d is a later integration point, not the original
session's last clean modification boundary.

| Change group | Kept / replaced / outstanding |
| --- | --- |
| `151a95b`, `ac50bbd` | NC contracts and Provider resource classification retained |
| `2069b33` then `a36dda9` | Optional Native ToolRegistry injection reverted; do not restore policy in Core/upstream |
| `2a0f74c`, `69db8a8` | Process-shared claims and Standalone local execution accounting retained |
| `5e71c19`, `fc2dc6f`, `3b08773`, `78e3745` | Durable results, dead unoffered owner reconciliation, pre-spawn rejection and stopped-work settlement retained; not evidence for every restart race |
| `cc6ea6d`, `807a21d`, `d3f3b1e` | Pre-dispatch conflict settlement and explicit non-login read classification retained; broader NC-14/24 execution-environment proof remains open |
| `819f22e`, `257aaac` | Removed product tmux paths remain removed |
| `49af7c3` through `f2084f3` | Merge, frontend binding and documentation integrated; Provider/runtime-binding source diff49af..f208 is empty |
| `605c9b5` | Later remote decision to defer permanent connector-refresh productization retained below |
| `4d80d43` | New Core fix: changed-component notes only; precise stale Provider sequence diagnostic. Source/local checks passed; signed public delivery is separate |

The one-off refresh checkout at
`$TMPDIR/bridge-refresh-live-20261102/source` is dirty overf208: `app.rs`,
`control.rs`, `operator.rs` and untracked `connector_refresh_once.js` add another
refresh entrypoint. It is **not canonical product implementation**. Preserve its
source and activation evidence, then retire its runtime override only after a
qualified clean Provider replacement. Do not integrate that duplicate API ahead
of P0/NC completion.

### Existing Native works reconciled from original execution

Both requested works were inspected using their exact original Native owner
socket, thread/turn and process identity. The generic home control socket points
to another server identity and must not be used as proof that a thread vanished.

| Work | Original evidence | Actual disposition |
| --- | --- | --- |
| `04daed623c604288e7b03d2807f640aa10e6effbedd31b69` | Original commandExecution ID matched the broker effect; exit0 and completed command, Native turn interrupted, retained opaque running-session claim | Existing Stop confirmed original owner idle, no background terminals and no remaining owned jobs; work is **stopped**, not falsely completed |
| `280427b3dd37da3e38c9b63c83b8bb1b5fec5e7fc6a3a747` | Attached-context trial's original Native turn interrupted; successful final was absent | Existing Stop confirmed physical settlement; work is **stopped**, not a completed attached trial |

No original command was rerun, no journal/claim was manually deleted, and no
unrelated process was killed. The shared Native effect ledger readback after
settlement contained zero claims. Earlier `b23e0aa...` offered/unreturned history
remains preserved; zero current claims does not retroactively establish its
unknown original tool outcome.

## Urgent updater repair

The stable21/device-high-water22 delivery inconsistency is repaired. The exact
existing signed Provider22 payload was recovered without changing its executable,
manifest, signature or source inventory. All71 signed source inputs match its
original Git commit2a837ccc4465; retained Android package lifecycle and actual-model
proofs bind the same executable. The admitted installed update authority matches
its signer. No account-management or viewer source changes were included.

The absent public release was staged, every public file downloaded and verified,
and14 isolated Android consumer steps passed using the installed Full142 Core:
public21 install/dispatch, pinned profile preparation and confirmed fixture stop,
public22 install/probe/catalog/reinstall, rollback retaining high-water22 and
forward reselection. Native auth/config/profile and operating selection were
preserved. The initial final guard also required every incidental Provider
process to remain alive; an unbound process disappeared. Its failed log remains.
The final proof verifies the authoritative operating service and existing child
by exact PID/start/image, protected files and actual fixture-service exit; the
unbound process's exit cause was not observed, and the consumer sent it no signal.

Existing `provider_promotion.py` promoted only the three Provider pointer files
non-forced at main commit `a089f236c4e4c289c38ce8275231a368811d3522`.
Canonical public stable index/signature then matched the verified22 package.
Two actual device `codex update` invocations exited0 with no Provider rollback
error, incomplete marker or repeated change notes. Full142/Base141, Provider22,
high-water22 and protected configuration remained unchanged; zero model requests.
Evidence: `~/.cache/codex/acceptance/provider-channel-recovery-20261103/`.
The unrelated `~/.codex/ais.config.toml` hash changed after the isolated consumer
check; its change source was not observed. This recovery neither wrote nor
removed either AIS file, and did not restore over that change.
This completes package/channel recovery, not replacement of the running unsigned
development service or qualification of its P0/NC behavior. No live runtime was
replaced, so no new connector refresh is claimed for this delivery-only batch.

Operating application remains incomplete: the live service and its runit
`CODEX_PROVIDER_BIN` both bind the `refresh-once-f2084f3-2328b656` development
image. Restarting that launcher alone will start the same development image.
Provider22 is the retained older signed package; applying it blindly would not
deliver the subsequent P0 changes. Qualify a clean signed forward package with
the required retained changes, then reconcile pending work, switch the service
and explicit profile bindings, run the existing full-schema client refresh and
prove a new Native work before claiming that the installed package is operating.

Update presentation changes are implemented in source, not built or deployed.
`codex update --details` (optionally with --wait/--no-wait) retains publication
times, component rows and Provider generation IDs; default output is compact and
omits an equal latest-upstream row. Provider selection produces its manifest,
changed flag and public origin; the command consumer renders one result after
successful selection/cleanup, identifying package installation and preserved
service/profile bindings. Pre-commit and duplicate status output were removed.
The public output oracle, publication workflow and Android consumers were updated
together, including original old-client output acceptance only on first upgrade.
Python publication11 and contract5 tests, Python parsing and Rust formatting
passed. The first Python run detected an identity-only no-op comparison; it now
compares manifest fields and executable inputs. Rust compilation, live new-output
proof and substantive runtime mutation checks remain pending for the final batch.
Provider owns live identity/lifecycle; Core does not infer app readiness or add a
service supervisor. New runtime status integration is not claimed.

The user requires postponing rebuilds until the planned implementation work is
ready, then one combined build/qualification/delivery pass before their actual
ChatGPT Pro development trial. Do not rebuild or activate merely for log polish;
source changes and operation of the retained existing runtime are distinct.
The work root is `$TMPDIR/ul-kfer6lif`; original inputs/results remain there.
A clean isolated `release-source` worktree at0814be7 excludes the optional
account-manager/Manager-return expansion preserved in the original worktree.
The initial combined gate stopped at three Core test compile errors (obsolete
install_stable argument and missing Installation Debug); those are corrected.
The repeated full gate passed: Core238/1 existing ignored, Manager51 and67,
window1, builder46 and3, Python suites, fmt and strict Clippy. The optional
account-manager/Manager-return expansion remains in the original workspace and
is excluded from the clean release source. The clean code/test candidate af5c0ce passed the frozen Provider gate, including
183 library tests/2 existing ignored, integration tests, strict Clippy, release
build, Bun/Python suites and CLI imports. The release artifact probe returned
the exact Provider payload identity. Accepted binaries and original Native
results are retained under `$TMPDIR/ul-kfer6lif/accepted-adapter-binaries`.
An existing CI bot concurrently cleared the stale frontend binding in
release/upstream.json at5fe6469. Non-forced push refused the divergent tip; that
metadata-only change was reviewed/merged, preserving both histories, at
29baba056c5d38e12eaecc6b42d7b5867327082d. Code/build evidence remains bound to
af5c0ce; the only merged file is release/upstream.json, not Provider code.
Hosted frontend build/mutation proof41730542386 remains pending; its build has
reached focused native TUI regressions. No new Provider activation or signed
public delivery is claimed.
The extra frontend fix recognizes exact Paginated blank-history errors while
retaining the original stable metadata-only rollout proof.


The repeated changelog defect is fixed in source. Core's existing release display
compares authenticated per-component file entries (bytes/modes); a Full repack
alone does not emit Core/Manager/TUI changes. Unchanged components retain their
identity/publication time. Upstream notes appear only when a newer official
version is available or this invocation changed that version. An unchanged
Provider suppresses its notes. No Manager dependency or alternate updater was
added. README and SPEC now agree with this behavior.

Local affected regression suite:6 passed. Complete Rust workspace gate:
Core238 passed/1 existing ignored, plus51/67/1/46/3 passing tests in other
workspace targets; fmt and strict Clippy passed. The initial nested TMPDIR
attempt had six shared-server/TTY failures because fixture Unix socket names
were112–114 bytes. The same gate passed with an exclusively allocated short
owned root; this was not counted as a product fix. A disposable mutation that
always marked components changed caused the new regression to fail, then was
restored. Source success is not signed public delivery or user installation.

The first actual publication attempt41564306489 built/smoked/signed/staged
Base141, then failed its public consumer: the checker still required unchanged
notes and its workflow collected original publication timestamps only when
`Core changes:` appeared. Both consumers now follow changed file inputs and
publication identity independently. Old installed Core first-update output is
still checked against its actual prior contract; the newly installed no-op is
strictly quiet. The new11-case publication suite and the complete Python gate
passed, and a disposable unconditional-notes mutation was detected/restored.
The original source CI41563110102 first failed on Android substrate HTTP503;
one failed-jobs rerun passed. Provider source CI41563109958 and independent
Core/runtime41563110366 passed. Failed staged Base141 is retained and was not
promoted; Full was correctly blocked from using an unpromoted new Base.

## P0 completion work

### Sequential defect-candidate follow-up

The current user request authorizes addressing documented defect candidates one
at a time while preserving existing Native work. `goal.md` is the small active
execution pointer; this file remains the evidence/acceptance authority. These
checks do **not** accept any complete NC-01..29 row or authorize an unqualified
runtime replacement:

- **Provider work/lane alias errors:** `operator.rs` now resolves exact `work_id`
  before AppHost aliases and propagates invalid/corrupt lane records instead of
  reporting `operator_work_unavailable`. Focused regression 1 passed; the original
  error-swallowing behavior in disposable source failed that regression and the
  copied source was restored byte-for-byte. All Provider library tests passed
  (188 passed, 2 existing ignored), strict Clippy, fmt and diff checks passed.
  Source-only result; no live binary replacement or post-replacement client gate.
- **Paginated blank-history:** the exact `thread/turns/list` error matcher and
  bounded negative cases already exist in published source `0c2a567...`;
  current-device fresh TUI display proof for this change is not claimed here.
- **AppHost read-only observation:** the CDP fixture
  `login_observation_reads_primary_without_granting_composer_admission` passed.
  Running Jan Provider image `52eb22fd...` served actual read-only `chats list
  --limit 1` with exit0. No composer permission was granted by the fixture;
  a full current Linux multi-window/ownership matrix is still unqualified.
- **Raw `codex exec`:** on the current Full144 runtime, a disposable-workspace
  `--profile chatgpt exec` with its configured `chatgpt-desktop/gpt-6` model
  exited1 before inference with `native_thread_owner_unavailable`. `Pool` only
  discovers a thread loaded by an owned Core app-server through
  `thread/loaded/list`. Review of `operator_native.rs::find_thread` confirms it
  cannot locate the independently executing `codex exec` writer; metadata-only
  `prepare_thread` is not authority to replace that running writer. **Blocked
  for this no-Core/no-upstream-change batch:** an actual supported writer
  rendezvous must be established before implementing this entrance. Do not
  relocate the writer, replay its request or replace Native ownership with
  another executor. A separate invocation overriding its model to `gpt-6` was
  caller-invalid (`unsupported_desktop_route`), not a second provider defect.
- **Android `ENFILE`:** no destructive exhaustion was attempted. A new same-UID
  snapshot counted 178 processes and 4369 file descriptors; kernel-global
  `file-nr`/`file-max` remain inaccessible. That snapshot cannot attribute the
  earlier system-wide exhaustion to a leak. **Evidence-blocked:** the original
  failure's system-wide file-table usage and owning processes were not captured;
  no speculative retry, limit increase or Core/Provider lifecycle change.

Current focused evidence is retained under
`~/.cache/codex/acceptance/goal-defects-20261104/` (some payloads private).

### P0-1: durable Native continuity and consuming-client correctness

Original defect: an observer could stop after the first Native tool return;
later calls on the same work failed `operator_work_unavailable`. Retain the
current durable broker/result handling, per-work controller, Native observer,
original tool results and first-tool continuation changes. Do not replace Native
history with a second database, saved-Chat crawler or another model execution.

**Now proven on the operating SDK path:** after full schema refresh, a fresh
attached work accepted original task/context, recorded complete public progress,
ran two distinct real append commands plus a read, reused an exact call's original
result without a second append, and reached confirmed Native completed status.
`effects.log` is precisely `first\nsecond\n`. Separate client connections were
used; create/start allocation was not counted as model inference. This closes the
fresh connected-client smoke and the first-tool continuation case at this scope.

**Still required for P0-1 acceptance:**

1. Repeat continuation across a controlled Provider/observer restart in a
   disposable owner; reconcile the old controller and unknown call before reuse.
   In this takeover's interrupted first fixture, the old controller was fenced;
   a fresh successful work does not prove that fixture recovered.
2. Prove same-work multiple rounds and original Native results after Stop,
   lost observer, daemon restart and exact uncertain delivery; no delayed execution
   of an already rejected conflict.
3. Verify actual Android ordinary-model input and active TUI display separately
   from stored Native history. Reuse existing Provider21/22 entrance evidence
   only where binary, account, route, branch and client inputs still match.
4. Verify Provider-submitted inference returns its original complete response
   and tool results to the requesting Chat, not merely a queue ACK or a final
   stored in another thread. This belongs to P0/A1, not future worker expansion.

Completion means the original request, every public progress/tool/result/final
and the same Native work stay consistent through those boundaries. Bridge-supplied
reports are not independently observed host messages. Hidden reasoning is not
part of public-record equivalence.

The user's operational acceptance goal is to delegate subsequent real work to
ChatGPT Pro after P0 completion. Recoverable tool, observer, process and connection
failures must not abandon that work: preserve its original results and effect
disposition, recover/continue the same work and return the full outcome to its
caller without duplicate effects or blind replay of an uncertain command. A
single successful smoke or a manually rescued unrelated work does not meet this
goal. Pro naming in configuration alone does not establish actual Pro execution.

### P0-2: execution and resource boundaries

Standalone — **단독 개발** uses the local read/write execution boundary and must
not create a Codex conversation or notification merely because ChatGPT inspected
local files. Integrated — **통합 개발** shares Native work, tools, control,
history and orchestration regardless of CLI or ChatGPT entrypoint. MCP versus
local Function is delivery; `inference=attached|submitted` selects existing
reasoning versus a new Provider model request inside Integrated. Neither
redefines workflow mode. Old runtime current/managed aliases remain removed;
necessary offline state conversion supports actual retained rollback only.

Current enabled MCP catalog has17 Integrated/control tools. Standalone local
tools are not implicitly granted; enabling their explicit configured capability
is separate. Renderer local-function Standalone has no qualified current caller.
Never substitute read-only access for the requested read/write capability.

Provider resource admission is at `native_effect.rs`, `native_claims.rs`,
`mcp.rs` and `local.rs`; upstream owns execution, approvals, canonical storage
and writer control. Claims cover the actual offer/result or live-process interval,
not a work's lifetime or repository cwd. Ordinary Core CLI/TUI and external
editors do not traverse this ledger. Those paths must not be advertised as
protected by optional Provider policy. Resource classification must account for
actual shell/environment effects, not only an innocent command string.

### NC-01..29 independent acceptance status

**Partial** means useful implementation/proof exists with the stated gap;
**open** means the required capability lacks sufficient implementation or proof.
The refreshed Linux SDK also ran two distinct actual Native works in one
workspace: a live read succeeded while the other work had a running opaque
process; a bounded patch returned persisted `native_resource_busy`,
`executed=false`; after observed process exit0, exact retry retained that
rejection and a new call alone applied the patch. Both works completed.
No row is accepted for the full product merely from the generic unit suite.
Shared code is not29 independently demonstrated real-path cases.

| Case | Current implementation / evidence | Remaining capability or real-path qualification |
| --- | --- | --- |
| NC-01 | Partial: work creation has no blanket cwd writer mutex; per-work controller/history | Shared-workspace two-work client proof passed; extend to parent/child and both transports |
| NC-02 | Partial: Read claims do not conflict; source browsing/read tests | Actual simultaneous independent readers, including orchestrator inspection |
| NC-03 | Partial: Read may coexist with writes, no immutable-view assertion | Actual Linux SDK read/opaque-writer overlap passed; further view labelling and transport combinations |
| NC-04 | Open: callers can use pinned Git views; no general repeatable-view acquisition | Implement/use a verified consistent content set or snapshot and prove failure rather than mixed-version success |
| NC-05 | Partial: bounded physical file claims; actual SDK simultaneous disjoint Native writes passed | Enforced write sets and full transport coverage |
| NC-06 | Partial: same-file publication serialized; unique surviving old anchors can rebase | Parallel preparation plus guarded nonoverlapping merge demonstrated on Native; not blind parallel writes |
| NC-07 | Partial: changed/missing/nonunique old anchors rejected before offer | Actual overlapping-hunk conflict with both original histories preserved |
| NC-08 | Partial: SDK exact retry and two concurrent retry observers on actual TUI/MCP/Native passed; duplicate reservation regression/mutation accepted | Ordinary-model requalification after activation; restart retry and current-version checks for distinct operations |
| NC-09 | Partial: path entry/inode preconditions; Add refuses existing target | Create/create, rename/edit and delete/edit races on actual Native path |
| NC-10 | Partial: parent/name entry identity avoids whole-directory mutex | Real sibling writes; parent replacement racing admission |
| NC-11 | Open: unknown recursive/metadata changes use opaque fallback | Exact descendant/metadata scope or isolated guarded publication; prove rename/delete versus live descendants |
| NC-12 | Partial: canonical paths, dev/inode and parent revalidation | Hardlink/symlink/path-swap/bind-mount races; record Android filesystem limitations honestly |
| NC-13 | Open: physical path resources exist, generic Git writes opaque | Nested/submodule/worktree data and actual common Git metadata ownership |
| NC-14 | Partial: explicit login:false, inert syntax and nonoptional-lock/fsmonitor-disabled status admitted | Prove effective shell/startup/environment and Git configuration are inert; real status/diff/log during writer; default login stays opaque |
| NC-15 | Open: no blanket work-creation exclusion | Independent indexes/HEADs/refs of real worktrees without over-serializing |
| NC-16 | Open: generic Git publication opaque | Resource-specific index/ref/lock coordination, expected HEAD/index and own-only staging/commit proof |
| NC-17 | Open: per-work roots supported; arbitrary build commands opaque | Independent Native builds with isolated outputs and exact source evidence admitted concurrently |
| NC-18 | Partial: product build caches have existing locks/ownership | Actual shared compiler/package store semantics versus named generated destination |
| NC-19 | Partial: owned TMPDIR subtrees and claims; no root ownership transfer | Actual parallel Native task temp effects; opaque fallback must not become a global build bottleneck |
| NC-20 | Partial: cleaner manifests and live-owner checks | Native recursive cleanup versus live descendants; preserve unknown/other-owned outputs |
| NC-21 | Partial: physical path claims across different roots | Real independent repository writers plus a genuine shared-resource negative case |
| NC-22 | Partial: distinct Native home/server/host/activation leases retained | Concurrent account/config/socket/runtime mutation and correct authority boundaries |
| NC-23 | Open: release publication retains signing/target authority | General Native external-side-effect scope; disjoint paths do not grant remote publication authority |
| NC-24 | Partial: unknown commands opaque; pre-dispatch durable conflict response; no later silent offer | Actual pre-dispatch conflict, original rejection retry and no late effect passed; OS/staging isolation and non-login environment proof remain |
| NC-25 | Partial: patch base/version/parent checks detect some outside changes | Unmanaged editor races and publication-time uncertainty; do not claim invisible writers prevented |
| NC-26 | Partial: SDK sequential tools plus actual TUI/MCP/Native two-tool continuation/final passed with zero-model fixture; targeted duplicate-admission fault detected | Activate qualified fix and ordinary-model development proof; independent same-work simultaneous effects |
| NC-27 | Partial: only unresolved effect claims remain; two original works normally reconciled | Real concurrent waiting/reconciliation records; unrelated work still runs |
| NC-28 | Partial: fenced Stop, owner identity, zombie/unoffered and original-result recovery | Fault matrix for dropped response, restarted owner, detached Chat, stale PID and physical process settlement |
| NC-29 | Partial: ordered short ledger, persisted rejection/results, exact retry | Simultaneous compound conflicts/partial effects; complete original outcomes and actionable owner/wait diagnostics |

P0/NC next slices: operating ordinary-model tool continuation after qualified
activation; reuse accepted Provider/duplicate-admission/restart/two-work proof only at its
exact inputs, then bounded path/publication and Git resources; then isolated opaque
build/temp execution and parity across MCP/local Function. Do not impose one
long-lived workspace lease or restore ToolRegistry hooks to make tests pass.

## Unified development order and user-visible completion

The user's latest operating order takes precedence over the general feature
backlog below: (1) Provider package/channel recovery, now complete; (2) restore
the original configured Linux desktop and authenticate janmori101; (3) establish
its tunnel runtime and MCP connection using the verified wrlab precedent, then
prove CLI ChatGPT/Pro replies; (4) resume P0 and concurrency work to the sustained
Pro delegation acceptance above. Preserve both accounts and defer account-manager
expansion that is unnecessary for these outcomes.

Effort labels are relative batch sizes, not a delivery-date promise. Safe
independent source work may proceed during upstream429 or network waiting; a429
alone is neither proof of repeated model submissions nor proof that work failed.

| Order | Priority / size | Prerequisites and work | User-visible completion |
| --- | --- | --- | --- |
| 0 | P0 / small code, medium delivery | Quiet changed-only updater notes; repair public Provider sequence through existing signed pipeline; qualify clean runtime replacement | `codex update` succeeds for selected groups, preserves Full/Provider/profile/jobs and repeats no unchanged notes; public and actual installed versions agree |
| 1 | P0-1 / medium | Existing fresh-client smoke, durable observer/results and controlled recovery; full Native and consuming-client gates | Tools keep working after first result and restart; complete result returns to the same caller; no hidden replay |
| 2 | P0-2 / large | All29 NC cases above; preserve Standalone/Integrated scope; real path enforcement and concurrency | Unrelated work runs together; conflicting effect rejected/reconciled with its owner; no lost edits or blanket repository lock |
| 3 | P0/A1 / medium-large | 1 and relevant2 resource slices; existing input/catalog/bind/join implementation | Android/web/CLI same-task follow-up, full public text/tool/final and live TUI agree; Provider-submitted response returns completely; solo work leaves no unwanted Native history |
| 3a | P1 / medium | Existing Provider account-change policy and account-bound work/host/connector discovery; qualify the requested account transition below | Switch Linux app accounts while retaining a permitted live tunnel; continue selected Native work with refreshed account-specific bindings and precise pending-work/access diagnostics |
| 4 | P1 / medium | After P0 **and full NC acceptance**, reuse existing refresh implementation | Phase-neutral supported deployment integration automatically refreshes registered schemas and app caches after actual activation; no duplicate refresh API |
| 5 | P1 / medium | Stable input/assignment consumers from3; originality inventory | Retire confirmed obsolete names/formats/local packages/settings/audit copies within authorized scope, preserving unique user data, current/rollback and immutable public history |
| 6 | P1/A2 / medium-large | 3 and relevant5 cleanup | Shared MCP/Function work/context/intent/usage/Stop rules; separate discovery/transport/leases/original returns; replaced duplicates deleted |
| 7 | P1 / large | Existing selective lifecycle plus affected6 boundaries; independent tunnel preparation can proceed earlier | Core mandatory; Manager/TUI/Provider/patched tunnel individually selectable; Full selects all optional layers; update/reuse only changed compatible objects |
| 8 | P1 / medium, parallel where independent | Manager account/quota identity and isolated actual Native activity; does not depend on A2/workers | Optional automatic five-hour window start after reset using least proven activity; same-account/bucket dedup, no unnecessary requests and real window/restart proof |
| 9 | P1 / medium-large, parallel | Existing confirmed Android clipboard transport and maintained TUI; relevant attachment/model contracts | Real image plus text reaches upstream composer/model; multiple images, cancellation, snapshot/import/release lifetimes; file/share fallback labelled separately |
| 10 | P2 / medium | 6 usage/Stop owner | Delegated model proposes initial call allowance and reassesses actual progress; authorized extension or complete settlement, not fixed75/1000 policy |
| 11 | P2/A3 / large | 3/6 execution identity and10 controls; bounded discovery can precede implementation | Actual independent reasoning and child/temporary-thread lineage, status/result return and separate public records; no fake parent relationship |
| 12 | P2 / mixed batches | Respective already qualified runtime/authority boundary | Remaining first-run/Linux-host/project/history/viewer/IME/touch/setup/repository separation and long-duration polish |

Order0's signed delivery does not establish all NC acceptance. Order3 reuses the
already implemented input/catalog behavior; it is not a fresh wholesale rewrite.
Orders8/9 can use an external-rate-limited interval without blocking P0. Permanent
refresh product integration waits for P0/NC, but the **existing** D3 refresh is
still mandatory after each live runtime replacement now.

## Retained capability requirements and accurate completion limits

| Capability | Reuse / remaining work |
| --- | --- |
| /switch, accounts and0.166 profile binding | Enhanced frontend and Manager restored; do not treat the old Base128 capability gap as current. Full140 carries invocation settings. Qualify actual profile transition and protected sibling state after affected changes; upstream storage stays upstream-owned |
| /config notifications | Upstream event/UI policy retained with thin Manager Android delivery. Prior Android notification-list proof is useful; physical tap/focus/IME and actual user failure diagnosis remain separate. A Base installation does not gain Manager features implicitly |
| /copy and Ctrl+O | Full0.166 plus qualified Termux APK0.118.0+18705c1.origin supports acknowledged copy; upstream limit is100,000 raw UTF-8 **bytes**, not100,000 Korean characters. Preserve logical original text, target picker, negative/unknown acknowledgement and exact full payload; terminal soft-wrap selection is not the copy path |
| Clipboard image input / optional key copy | App-side image candidate exists, Codex consumer incomplete (order9). Optional Ctrl+C-copy remains default-off; Ctrl+Shift+C interrupt and physical/extra-key behavior need their own actual APK/UI proof |
| Editor and Termux APK maintenance | Verify inherited EDITOR/VISUAL and actual save/cancel; do not hardcode an editor. Exact reviewed official master/CI base, preserved custom title/origin/keyboard/clipboard, compatible signature/data and on-demand rebuild; no recurring APK watcher. Clipboard does not require X11; separate Linux ChatGPT host display does |
| KST / TZif | Implemented and scoped proof retained. Regress affected Core/TUI inheritance rather than redevelop timezone logic |
| Prompt-only chat input / prepared workflow | Existing inspected app send path retained. Model/effort derive from live app choices plus explicit config additions; preserve prior user selection when omitted, with settings-event/per-request provenance. No hardcoded model generation, silent Instant fallback or pretending missing effort provenance was a user action |
| Bridge-first entrances | No2-second saved-Chat crawler. Ordinary attached input uses Bridge original task/context; saved source_sync/source_join is explicit recovery, not background polling. Original conversation/branch/message/account IDs and complete pagination define any source proof |
| Complete response quality and duplicate display | Preserve complete final, progress and tool chronology with the selected presentation-only fold/expand. No reduced effort, summarization or extra inference as a display fix |
| Queue and source identity | Registration ACK, actual input receipt and execution consumption are different. Compare full local/registered/client schemas. Host hints do not choose a work or grant its controller |
| Annotations and idempotency | Fixed hints true/false/false remain the explicit development contract; idempotency follows each actual operation. Actual refreshed host declarations must be inspected; a local schema alone is insufficient |
| No-login tools and attribution | Model-free local/tools/update do not require Codex subscription login. Native account, ChatGPT account, connector, project and signing authority are distinct; a Codex process does not prove that account performed inference |
| Project/history setup | Verify actual remote creation/readback plus selected history/project/lane binding; changing config alone does not create or move a Chat. Preserve existing conversations |
| Services and HTTP2 | termux-services/runit is generic lifecycle supervision, not application readiness or Native authority. Retain scoped startup/stream fixes; verify exact process image/host/tunnel/MCP readiness and original error before diagnosing termination |
| Patched tunnel | Existing wrapper-owned Termux recipe/scoped runtime retained. Order7 delivers prebuilt Full and optional selection plus in-repo acquisition/patch/build recovery; no mandatory separate tunnel repository. Verify then-current upstream compatibility and active selected executable, not PATH name alone |
| Selective update/build lifecycle | Keep/local/remote choices, dependency preparation, authorized signing, two-hour remote discovery, artifact/cache/progress-aware transfer reuse exist at scoped proof. Complete independent optional compositions, service/profile transition and publisher-consumer handoff reliability. Major upstream changes remain maintainer work |
| Live updates and retention | New immutable generations serve future clients while old clients retain their images; running Core alone does not block update. Provider shared runtime/state/client activation is a separate lifecycle transaction. Preserve original signed Full134/Provider22 and rollback inputs |
| Originality scope | Review27 unresolved old inventory entries and actual consumers after input acceptance. User authorized cleanup concerns the referenced miuuyy repository and borrowed names/formats; do not remove unrelated dependency licenses or claim all unknown local records are dead |
| Rust / JavaScript / CI Python | Installed lifecycle/state/policy stays Rust; renderer JS is actual app API/React/stream access. CI/test orchestration Python is not another installed updater/executor or a reason to rewrite the runtime |
| Model budgets and Pro discovery | No universal75/1000. Retain measured consumed/unexecuted counts and parent/platform/Stop/concurrency authority. Pro is for discriminating discovery useful to ordinary models; prior remaining-call count needs exact provenance before spending, not routine smoke. No Pro call in this takeover |
| Workers and BTW | Sequential requests in one Chat do not establish independent model workers. Verify actual Native parent_thread_id/subagents/ephemeral lifecycle and original results before integrating external workers. BTW idea retains requested thread01a18ac9-4f81-7f72-bc2c-46404d87fe66 as a discovery clue, not assumed resumable durable work |
| Manual Android test text | Full UTF-8 prompt file, automatic clipboard attempt/readback and separated short fallback; avoid hard-wrapped CLI selection. Successful clipboard command exit alone is not physical paste verification |
| Device stability and cleanup | Use observed kernel/process ownership and real system evidence, not an assumed TUI leak/3GiB cap. No repeated ADB demand as a precondition to useful development. One heavy compiler job; owned temp/output roots; preserve results/source/current/rollback before cleaning generated payloads; reusable compiler cache around15GiB or less |

### Requested Linux app account transition

The requested operating transition is `wrlab001` to `janmori101`, retaining the
existing healthy tunnel where the destination account/workspace has access.
The original saved Linux desktop is restored and janmori101 login was verified.
Its dedicated runtime/connector admission passed the existing full-schema refresh;
this establishes that operating scope, not CLI model-response completion.
Provider owns live app identity, account-bound model/project/Chat/connector
resolution and the transition; tunnel configuration owns remote associations
and credentials. Native execution and canonical history remain upstream-owned.
Account names are the requested test case, not source-code routing rules.

Retain both accounts' admin credentials in separate named tunnel-client profiles,
and preserve independently saved Linux app login state when switching accounts;
admin credentials do not authenticate the Linux app. Local admin profiles
`wrlab001` and `janmori101` use the shared naming/storage convention in the
Provider guide, without changing the active default or running tunnel. Their
`file:` references point to `~/.config/codex/credentials/tunnel-admin/<account>.key`.
The existing key's remote display name is verified as `codex-admin`; new keys
use `codex-admin-<account>`, so the requested new name is `codex-admin-janmori101`.
The existing key was consolidated under `wrlab001.key` (directory0700/file0600),
with both old credential paths preserved as symlinks for existing consumers.
The destination key is saved with mode0600 and its named profile authenticated
the organization API (HTTP200, exact masked-key match to
`codex-admin-janmori101`). The previous ad-hoc destination filename is superseded;
use the guide's hidden-input registration procedure for any account.
Actual Android `tunnel-client` proof added three profiles and removed one in an
isolated XDG state root, preserving both remaining profiles, the active selection
and every key file; that fixture was reclaimed. The canonical `wrlab001` reference
authenticated the organization API (HTTP200, exact masked-key match to
`codex-admin`), both legacy paths retain identical bytes, and live tunnel health/
readiness/control-plane polling pass. Documentation checks and Bash syntax pass.
The dedicated `codex-chatgpt-janmori101` runtime uses its own saved service-account
key and state `~/.local/state/codex/providers/chatgpt-janmori101` (port45341); the
plain `@TermuxBridge` connector is active and the canonical D3 refresh compared
all17 full input schemas and invalidated app caches. Dual app-login switching
remains unqualified; saved admin authentication alone proves no app login.

Real app-SDK Native proof in `$TMPDIR/ul-kfer6lif/native-operating-proof.json`
completed two works: exact retries retained original effects/results, two
independent writes ran concurrently, conflicting writes were rejected before
effect, reads/Git status remained available, and a controlled Provider restart
retained original work/history and allowed fenced continuation. This is bounded
P0/NC evidence with zero model requests, not all29 NC acceptance.

The generated shared profile has now been prepared through the existing Provider
command using the Jan account configuration; it binds45341, Jan state and the
actual operating binary. The previous marked profile is preserved in the work
root. This does not replace either running service.

An18-second host-status observation was insufficient: a longer bounded Jan
observation completed after30.28 seconds with app_target_unavailable. The old
Provider requires a composer even for observations; direct authenticated CDP
inspection still works on Home/Plugins. The existing desktop helper opened a
verified composer without sending inference. The clean candidate separates
read-only authenticated-page observation from mutating composer admission.
Both original services remain alive; a universal CDP deadlock is not established.

Actual codex --profile chatgpt interactive TUI with gpt-6 returned JAN_TUI_READY
in about9 seconds, and the same canonical Native work completed with no pending
calls. A raw codex exec trial instead failed before app inference with
native_thread_owner_unavailable; its records are retained and that entrance is
not qualified. One explicit Pro development trial then failed at its first tool
handoff before creating hello.py. Its MCP Broker call was already reserved;
Operator attempted external-effect admission for the same call and rejected its
own claim. The fix retains MCP admission/settlement in McpBridge and external
AppHost admission only for local-function delivery. The new zero-model Android
TUI/MCP/Native regression passed two consecutive opaque commands, two exact retry
observers, original results and final delivery to the original live TUI. Its
original Native history/results and accepted executables are retained in
`$TMPDIR/ul-kfer6lif`. A targeted fault restored external admission for MCP in
the disposable source; it compiled, the regression failed at the first tool,
and no Native effect occurred. Source was restored byte-for-byte. Evidence:
`adapter-tools-native.json`, `adapter-mutation.json` and their original logs.
The first Provider gate is not accepted: its shell runner was edited while it
was being read and exited127 after the earlier checks/build. The frozen final
Provider gate passed with source/content unchanged during execution. The passing
Native fixture and intentional-fault fixture each had six manifest-matched
generated payloads archived/reclaimed through clean_development.py; original
results, Native records, accepted binaries and separate restore logs remain.
Compiler caches remain below the15GiB active budget.
The failed Pro work (`b7229ed73effd70279838a43e9b2542066bdfb18bb1bece1`,
`thread_id=01a1a37a-712a-7e42-80d5-c8af17bf12dc`) was stopped through the
existing control; app and Native stops were confirmed and `remaining_jobs` was
empty, but its offered MCP call (`0b489a44bef8d1be41c448250e91343f0ca0a4441f37bbd4ea07109bcfd12470`,
registered by `McpBridge` under `AppHost` marker
`lane_68a109542a0f7181aa6bca45eda80349ca35fb8c6fa6ab64` and `turn_token=c155ac7b06558a39f03d918837248b01198505428cdf0223`)
remained in `native-effect-claims-v1` (`pending_result=true`, `process_session=null`,
`resource_kind=opaque`) because `Operator::stop` only settled live process IDs
under `record.id` and never reconciled unexecuted offered MCP calls proven absent
from Native `thread/items/list`, while `codex-chatgpt-janmori101-provider`
remained pinned to the older `refresh-once-f2084f3-2328b656` binary. That
orphaned opaque claim subsequently blocked Native tool execution across later
`janmori101` sessions (`f5f9eb4f...`, `9c3f2e5b...`, `5bacb04d...`, `fc6f1510...`)
with `native_resource_busy`.
The Provider fix in `app.rs`, `mcp_bridge.rs`, `native_claims.rs`, `mcp.rs` and
`operator.rs` resolves `AppHost` `lane.marker` to its owning Operator `work_id`
across `binding`, `codex_status` and `codex_stop`, and reconciles offered MCP
calls absent from Native `thread/items/list` after confirmed Stop as
`{"error":"native_call_not_dispatched","executed":false}` through the single
authoritative `reconcile_persisted_integrated_receipts()` path while preserving
any call present in Native items or lacking a turn receipt. All 187 library
tests, strict Clippy, fmt and Python suites passed; a disposable mutation
removing `native_call_not_dispatched` from `verified_exec_not_spawned` caused
`nc_confirmed_stop_releases_original_session_and_preserves_unconfirmed_effects`
to fail (`left: 1, right: 0`) and was restored. No Pro retry occurred. User has
now reserved exactly one additional Pro opportunity, usable only after ordinary
model paths and relevant checks have no failures/unresolved findings. Do not
consume it for diagnosis, source review or incomplete qualification.

The former frontend blank-history matcher admitted only thread/read although
its complete-history consumer uses thread/turns/list for Paginated history.
The candidate adds that exact method/error pair with thread-bound messages while retaining
the canonical metadata-only rollout proof; arbitrary32600/32601 errors are
not emptiness. Extend the existing focused test rather than add a fallback
executor or change upstream history semantics.

Reuse `history.account_change="new_chat"`: `app.rs` already permits a successor
remote Chat at a new Native user turn after an idle/stopped round; existing
operator work also verifies its recorded identity and can reject a changed
account. Improve these actual consumers rather than adding another switch API.
Refresh account-bound model/project/connector caches and references from the
new authenticated session, preserving the user's selected Native work/home and
the old account's conversations. Resolve a destination project/Chat explicitly;
do not claim that a login change transfers access to the old cloud Chat.

Acceptance covers the actual Linux login transition and Native continuation
with a returned result through both selected transports. MCP requires destination
tunnel association/permission and connector authentication/discovery, with full
registered/live schema agreement using the existing refresh implementation.
Function (`local_function`) requires no Chat-side connector registration or
tunnel: verify the destination app's local-function capability, declarations,
actual invocation and result return on the original call. Report
`app_local_functions_unavailable` distinctly if the new account cannot use it;
do not silently switch transports. Chat-first local-function discovery remains
a separate deferred capability, not proof supplied by a Codex-origin trial.
Keep the tunnel process/profile when still valid; report missing remote access
or registration separately from app-login readiness. Reconcile pending/unknown
old-account requests before continuation, preserve their original result
identity, and never silently replay them to the new account. Verify switching
back without losing either account's state. This request adds the task only;
no live login, credential, service or work binding is changed here.

### Deferred connector-refresh production integration

Decision605c9b5 is retained: only after P0-1/P0-2 and all NC implementation/
acceptance, refactor existing `scripts/d3_refresh.mjs` into a phase-neutral
supported deployment component. One implementation owns refresh_actions,
app query-cache invalidation, exact connector/account/owner verification, full
registered/live schema agreement and actual read-only readiness. Source builds
alone do not refresh clients. Activation must bind the **current live** owned
Linux session and exact deployed binary/state; historical descriptors and the
Provider's internal CDP pipe are not interchangeable with a D3 relay.

The one-off `connector_refresh_once.js` is not a second permanent API. Its
`refresh_identity_changed` meant the memoized operation's connector/tunnel
arguments differed, not proof of a changed user account. Its
`connector_owner_changed` compared name as well as ID/tunnel. Actual authenticated
readback showed the same connector/tunnel but name `@TermuxBridge`; using that
verified current reference with the existing D3 function succeeded. Do not relax
owner checks to conceal stale references.

## Device interruption and evidence limits

Immediately before the reported phone failure, command creation and an existing
single-job Provider compiler both returned `ENFILE` / `File table overflow`
(os error23). This is distinct from the older Android LOW_MEMORY report and
from per-process EMFILE. After resume, a read-only same-UID sample counted2127
descriptors across66 processes; system-wide file table access is denied. That
post-restart snapshot proves neither the pre-crash global count nor a leak owner.
The disposable Linux host has confirmed clean process-exit records. Preserve
source/green test logs and resume only the failed compile; do not rerun accepted
tests or start another app host while compiling.

## Evidence, limits and next execution point

This takeover's original results are under
`~/.cache/codex/acceptance/takeover-20261103/`; owned source/client/Native fixtures
are under the root recorded in its `work-root.txt`. These locations contain
private context/control data: do not publish raw rollouts, capabilities or auth.

| Evidence | Establishes / does not establish |
| --- | --- |
| `local-session-change-index.json`, `runtime-session-change-index.json`, Git cc6/807/d3/49af..f208 | Original source mutation and integration boundary, not just commit dates |
| Both work prefixes `*-original.json`, `*-native-read.json`, `*-background-terminals.json`, `*-stop-v2.json`, `*-after-stop.json` | Original command/turn disposition and confirmed Stop; not retroactive successful finals |
| `d3-refresh.log`, `d3-refresh-client2.log` and their private refresh results |17 complete registered/live input schemas and actual read-only client calls; zero model requests |
| `native-client-execution.log`, `native-parallel-effects.log`, `native-parallel-finish.log`, fixture `native-client2/*-result.json` and its Native history | Multiple actual Native calls, exact retry, original outputs and completed work through Linux SDK; no physical Android model/TUI claim |
| `update-display-tests-final.log`, `display-mutation.json`, `display-mutation.log` | Six affected regressions, intended mutation detected and disposable source restored |
| `core-workspace-check.log`, `core-workspace-check-short.log` | Initial socket-path failures preserved; full workspace/fmt/strict Clippy passed with short owned TMPDIR |
| Historical `workflow-milestone-20261101/{integrated-android,standalone-android,package-android,linux-actual-model,startup-android}.json` | Earlier actual route/lifecycle/startup scopes; not qualification of the dirty current refresh executable or all29 NC cases |

Earlier detailed public delivery, device qualification and failed-trial records
remain recoverable in Git, including the
[pre-takeover CURRENT at605c9b5](https://github.com/humtr/codex/blob/605c9b5ea1c005794c7db404760502347ed891b6/CURRENT.md).
Their original files are retained. They are evidence at exact inputs, not a second
current roadmap. AUDIT.md retains its earlier independent architecture review
scope; this takeover does not claim to have reconstructed every external ChatGPT
conversation from first message to last.

Current request: publish/sign the latest qualified snapshot through the shortest
existing release path, so codex update installs the complete latest Core/Full and
Provider. The user resumed work and explicitly prioritized delivery over further
NC/features; do not spend the reserved Pro opportunity. Work/evidence root is
$TMPDIR/ul-kfer6lif/publish-current. The publication caller was owner-authorized,
non-forced repinned/read back at main b042b6b4c1acc00b8988a603dc5b869230cf7033.
Source0c2a567bb06f126db2156abd58463e23a324cdfd adds only the accepted frontend
binding to29baba0. Native frontend run41742926011 passed; candidate binary hash
0124aa08d66da161568d97065190326c9c0bf52b6995ee9b410748c79bcab303 is in
publish-current/frontend/codex. Source/core/provider exact CI gates passed.
Canonical publication run41750002705 has signed/read back/disposable-proved and
promoted the Base lane; Full is in progress. Its main caller source is034d39a.
Additional frontend mutation proof run41750031338 is running, with no models.
Local current Core/release-builder release binaries were built in check-target;
Provider binary remains the accepted af5 component-equivalent candidate under
accepted-adapter-binaries/codex-chatgpt-provider-release. Fresh actual Android
operator cold-resume/Stop/control qualification passed (operator-android.json).
Package/real-TLS lifecycle plus three installed compositions are running in
exec session97713; logs/reports are in publish-current. Next: finish those gates,
prepare truthful exact Provider source/qualification input, use canonical
provider_signing/publication/readback/promotion with retained authority and a new
sequence (stable is22), finish Full, run actual device codex update, explicitly
refresh both service/profile pins where settled, then canonical d3_refresh with
the live Jan desktop/connector and new Native create/start/attach/execute/finish.
Signed Provider23 (`chatgpt-0c2a567bb06f-s23`) was promoted at main
`3cfd0209a06103a3ba0a68c04f29b86e0d957d8e` and selected by device `codex update`
(before the Full#144 archive download interrupted with `curl Some(56)`).
For the subsequent Native execution blockage on `janmori101`, the updated
Provider binary `development/p0-effect-reconcile-29baba0-52eb22fd/codex-chatgpt-provider`
(SHA256 `52eb22fd85423cf5aa718d1133d2b033dedf9d692425c33cdd3ec8325c991f31`)
was deployed and activated on `codex-chatgpt-janmori101-provider` (port 45341),
the `codex-chatgpt-janmori101` tunnel runtime (`tunnel_6ae9d65bdcbc81918041b5feb7764787`)
and `chatgpt.config.toml` while preserving the default `codex-chatgpt-provider`
host process (PID 12855) and previous binaries. Calling `work stop` on
`lane_68a109542a0f7181aa6bca45eda80349ca35fb8c6fa6ab64` reconciled `0b489a44...`
to `{"error":"native_call_not_dispatched","executed":false}`, reducing
`unconfirmed_effects` to `0` and `pending_native_calls` to `0` without manual
journal deletion. Canonical `scripts/d3_refresh.mjs` against the live owned
desktop session (`ld-tqo0snje`, `@TermuxBridge` `asdk_app_6ae9d933635081919b50090c57464b81`)
passed (`attempt=1`, `ok=true`, `cache_refresh=true`, `expected=17`,
`schema_error=null`, `model_requests=0`). A fresh connected-client Native
round-trip (`work_id=0d212e415b87ee75179874529533223fec6dc7272256464a`,
`thread_id=01a1a4d6-6237-7763-a547-4c3e21e8fadc`) completed `create -> start -> attach -> accept -> tools -> report -> execute (two writes, exact retry, repository AGENTS.md read) -> finish -> status -> history (9 items)`
with `unconfirmed_effects=0` and `active=[]` (evidence:
`~/.cache/codex/acceptance/native-effect-recovery-20261104/`).
Signed Provider24 packaging/promotion of this fix remains separate from this
live `janmori101` recovery. The earlier interrupted Full144 download is retained
as a historical failed attempt; the current Core activation-state now selects
Full144 with Full142 as previous, verified after that attempt.
The user's subsequent instruction now prioritizes sequential disposition of the
above defect candidates; do not turn that into an indiscriminate NC rewrite or
consume the reserved Pro opportunity.
Provider/hosted frontend builds, mutation proof, actual ordinary-model development,
signed publication, runtime activation and post-replacement client refresh must
be reported separately; unfinished work stays unfinished.
Build/source, signed delivery, device activation and physical UI observations
must stay separate in the final handoff.
