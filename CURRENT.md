# Current product state

## 2026-11-18 Provider v12 original Saved Chat reentry acceptance — PASS

The 0.167.2 Provider now runs as the private **v12** generation under
`/data/data/com.termux/files/home/.local/lib/codex/providers/chatgpt/generations/local-provider-chatgpt-v0.167.2-20261118-ledger-v12`.
The actual runit PID/executable was verified, and the pre-v12 v11
provider state and runit environment were independently backed up. The
Provider binary SHA-256 is
`2f2f6249b1e3fbdcbe671b04e0768359e165e28e95491144ae33bb35164bb1e4`.
This is a local development Provider activation, **not** a public build,
signed publication or a change to the stable 0.169 operating Core.
The actual stable `/data/data/com.termux/files/usr/bin/codex` SHA-256
remains `f5739302eff9b39a6883eca94f0ab4acfb8a79133f7493d392d8277828adeb3b`.

Reproduced v11 defect: after renderer restart, a completed original W2
Native turn was able to locate the original Saved Chat final anchor, but
`chat_adopt` rejected its original two consumed app dispatches with
`assignment_app_ledger_unavailable` when the new renderer retained a
smaller reserved ledger. The fix permits a **monotonic consumed-highwater
restore** only for a single authenticated, exact original Saved Chat and
fully settled prior turns. An active, changed or unauthenticated lane is
fenced; an already exhausted allowance stays exhausted and cannot be
refunded by recovery or a changed cap. No new saved Chat is a fallback.

Original W2 `work_id=7db41a86a57ae7eb92250430371e46ba37de004f3b34a6a8`,
Native thread `01a1ead3-da0b-7f33-8f60-0b7c78f9af2a` was explicitly
claimed after exact native history/context cursor reconciliation; no
replacement work/thread was allocated. Its new idempotent queued input
`ops1672-auto-e2e-20261118-b-W2-v12-saved-chat-reentry-001`
admitted exactly **one** new native turn
`01a1eb7c-e803-7ce3-92ec-0057ca3dba10`, which completed in the
**same original Saved Chat**
`6afca0fd-bb30-83e8-855f-fc03de6c9b1d`, with final text:
`V12_SAVED_CHAT_REENTRY_CONFIRMED W2 512 9559296`.
W2 was `completed`, Native `idle`, source app lane `idle`, cumulative
app dispatches **3** (the original **2**, plus **1** new), no previous
saved-Chat branches, no pending calls and zero unconfirmed Native effects.
The existing W2 `reference.json` SHA-256 remains
`6cd6e17dd15e4ca81dfa177aad376668262cf751d6cb071832366341a7584017`.
The original A/O/W1/W2 works all ended `completed`, Host active lanes **0**,
and original O consumption of A's ACK is preserved. The older W1 successor
Chat in the v10 failure remains historical evidence, not silently repaired.

Acceptance evidence is
`$PREFIX/tmp/ops1672-auto-e2e-20261118-b/evidence/v12-final-acceptance.json`,
SHA-256
`b08d30f1982f843a0fa532fb528a12bb99a73092fe8189e07a0e92f9f7996b16`.
Source/test commit `8f1adb5e3b01592cbb7ba2b5cdd6fac094ff17cb`:
App bridge JS **119 pass, 0 fail**, Rust library **361 pass, 0 fail,
5 ignored**, strict Clippy **0 warnings**, formatter pass, full Provider
binary build pass. This proves the **original Linux Saved Chat reentry**,
not an Android-current-chat unsolicited message or its automatic ACK.
Those Android recipient-boundary and additional fault-injection gates
remain NOT_TESTED; no other historical unfinished job is reclassified.

---

## 2026-11-18 earlier verified local E2E — prior to v12 reentry acceptance

The 0.169.0 operating entry at $PREFIX/bin/codex was protected; SHA-256
f5739302eff9b39a6883eca94f0ab4acfb8a79133f7493d392d8277828adeb3b.
The privately deployed 0.167.2 Provider for development was actual runit
v10 local-provider-chatgpt-v0.167.2-20261118-auto-budget-v10; its
running PID/executable was checked and v9 remained available for rollback.
Do not infer the operating Codex version from the Provider name or a
legacy symlink. No public release, signing or 0.169 activation occurred.

The real Linux-account project was 코덱스 오케스트레이션, project ID
g-p-6afc6996acd88191b15bf36dd39c46eb. Native work/task:
ops1672-auto-e2e-20261118-b under $PREFIX/tmp of the same name.
All actors used janmori101, submitted GPT-6 Thinking xhigh Non-Pro:
- A d58b5333016667a3c1781ff852fb0a8fcfc4a9f383dab078
  / thread 01a1eacc-64b3-7b63-ae28-0310e0275991
  / saved Chat 6afc9f08-04cc-83ec-b9a1-1f0ce82807c4.
- O 72c39a92d967502a372b8b78cfb8f8d47f80a943cf570260
  / thread 01a1ead0-03f3-7ba3-bbf3-96c5b1611148
  / saved Chat 6afc9ffd-5a20-83e8-a54f-c33bd20f447b.
- W1 aa5b98685e1bd263245a64ce6364020f89e602a83fae0669
  / thread 01a1ead3-9a8d-7472-aef2-6308ea046e6a
  / original saved Chat 6afca0ef-780c-83ee-8a04-c3affc6ee86d.
- W2 7db41a86a57ae7eb92250430371e46ba37de004f3b34a6a8
  / thread 01a1ead3-da0b-7f33-8f60-0b7c78f9af2a
  / saved Chat 6afca0fd-bb30-83e8-855f-fc03de6c9b1d.

A independently created and started O. O independently allocated/started
two worker saved Chats and ended its turn. W1 generated 512 deterministic
CSV rows; W2 independently computed six statistics. Both returned an
explicit final Native marker and the Provider automatically posted a
deduplicated native_auto_result_v1 to the original O thread. O consumed
both in distinct model turns, checked source hashes, produced a final
report, posted original W1/W2 ACKs which both workers consumed, and
delivered FINAL_RESULT_V1 to the original Linux A saved Chat. A actually
consumed and verified this message and posted its own ACK. O consumed
the ACK in yet another original Native turn and recorded ack-received.json.
All four original works then showed completed, pending_calls 0,
unconfirmed_effects 0 and AppHost active model lanes 0.

The observed six statistics were count=512, sum=9559296, min=19,
max=37365, even_id_sum=4770176 and odd_id_sum=4789120.
Original CSV SHA-256:
a877eb75ade06befedf6065fb6d2ebb8d607e85b643f9110246a7f8a4c6b5b5f.
The job/evidence directory includes original queue client IDs, the
actual message-consumption history, W1 proof, W2 independent reference,
O report, A receipt, A ACK and supervisor independent verification.
The A in this acceptance was a Linux ChatGPT delegate, not the
currently active Android ChatGPT application; native Android automatic
queue consumption remains NOT_TESTED.

New work budget defaults: 3600 seconds per Native model turn (queue
waiting does not consume its next turn), max 100 turns, 2000 Native
calls, 2000 app starts and parallel max five. Old journal records use
21600 seconds/session fallback. Actual v10 authorized A-to-O and
O-to-W1 budget edits succeeded, an original call_id idempotently
returned the same receipt, a 2-turn/4-call ceiling rejected a queued
W1 turn before admission, and parent-controlled restoration retained
all consumed counters. The same original W1 queue call then ran a
read-only post-restart model turn and returned
V10_BUDGET_RECOVERY_CONFIRMED W1 512 9559296.

Known saved-Chat identity regression in v10: the post-restart W1 turn
retained native work/thread but created successor saved Chat
6afca515-3448-83ee-848c-88c0ed1ce4c7 instead of continuing the
original W1 saved Chat. The earlier saved Chat remains preserved,
not overwritten. Source repair requires authenticated chat_adopt with
the saved final-answer anchor and a real post-restart acceptance.
Do not call this aspect passed merely because the Native IDs match.

Build evidence before the latest identity repair: 358 Provider Rust
library tests passed (five ignored), strict formatting/Clippy,
v10 all-target Clippy and actual build all succeeded. The isolated
source includes commits eebabaa9b527164fb4b81359287a5cea7cd3d418
(auto-result and 1h/turn budgets) and
d388432a0f7b74e8eab8231160022897098f7e6e
(parent-only budget CLI). Historical reconciliation-required works
are not new-job unsettled effects and were not silently reset.

---

## 2026-11-13 historical snapshot (superseded where inconsistent)

Observed: **2026-11-13 14:22 KST** (source/CI/Termux read-only observations).
This file records verified facts and evidence boundaries, not implementation obligations.
Requirements: [SPEC](SPEC.md) and [functional annexes](docs/spec/INDEX.md).
The single execution plan is [HANDOFF](HANDOFF.md); the W0-W20
plan-versus-implementation reconciliation is in
[the finite integration record](docs/development/RELEASE_FRAMEWORK_IMPLEMENTATION.md).
The deferred design candidates are linked from HANDOFF.

## New acceptance checkpoint — 2026-11-13 15:17 KST

The following are **new same-day observations**, superseding only explicitly
dated values below. The exact remote development HEAD remained
`d9c3ed76a0a03a2fecbac7e46b7aa2e89317e189`, and the separate
publication `main` remained `fa6a1856866b9944c08cc7511d85180a384c1995`.
The clean isolated acceptance worktree uses the exact `d9c3ed76` source;
the older primary checkout and its three untracked files were not modified.

**Operating device, read-only:** `codex --version` returned
`codex-cli 0.167.2`; `codex termux version --json` reported verified Full
S167 `full-v0.167.2-220cc2e-s167` with previous Full S165, and verified
selected ChatGPT Provider S27
`provider-chatgpt-v0.167.2-0ba0589-s27`. The runit Provider process was
PID **8454**, with actual `/proc/8454/exe` under that *new* S27 image.
The Provider filesystem `current` link still targeted the separately named
`provider-gpt-v0.1.0-df8a8c9-s27` path; this is **not** evidence that
the old image was running. Tunnel PID **8450** and Host-keeper PID
**8457** were active runit script processes. Neither service was restarted.
`codex doctor` also reported three thread-record issues, WebSocket fallback,
a stale/unreachable background app-server socket and unrestricted filesystem
sandbox; none was repaired or presumed causal. Only approximately **5.3 GiB**
free disk remained, so full local rebuilds require capacity preflight.

**New actual independent model experiment:**
`codex1672-e2e-20261113-a` used a genuine submitted GPT-6 O, saved Chat
`6af6a7d2-dd14-83ee-bbf1-acfd6b5cf806`, Native work
`05d5a8d63b45e49edd39d2083ea437bd9f4ac08893a6d437`,
thread `01a1d37f-1573-7c12-90ea-86417dda6496`. Its first model
turn completed but correctly reported that W1 allocation failed:
`assignment_parent_controller_required` without a bound O controller,
and `native_workspace_identity_invalid` when an absent explicit
execution-home path was supplied. A separate Native shell command returned
`native_resource_busy` and `executed=false`, while independent
TermuxBridge `termux_shell` created the private W1/W2 workspace directories
successfully. The subsequent manually initiated **same O work/thread**
model turn then created two genuine parent-bound submitted-inference workers:

| Child | Actual Native work | Actual Native thread | Allocation call |
| --- | --- | --- | --- |
| W1 | `229d2385385773dda334a9cca5bce847402c6c18f24fbced` | `01a1d383-e5d7-78e2-b75c-915434e47efa` | `e2e-a-w1-create-v2` |
| W2 | `b9228ebefee8c2ebd63e407f31321d0e78e59da5b8e958c0` | `01a1d383-f57c-7453-bafb-36285cc96e8b` | `e2e-a-w2-create-v2` |

Worker model start calls `e2e-a-w1-start-v1` and
`e2e-a-w2-start-v1` did **not** yield settled inference receipts.
Read-only events/wait and work-list observations continued to report
W1 `admitting`, W2 `queued`, `source_conversation_id=null`, and
no `completed_result`. Actual Worker replies, W-to-O return queue,
O message consumption, receiver ACK and automatic O-to-A delivery
are **not** proven. Do not create new Workers or redispatch the unknown
starts under new call IDs. An unrelated original Native work
`1d3d4bca273ecdc90c096535311475a2f9aebb679efff2a6`
retained one pending opaque effect; correlation with these admission
states is a hypothesis, not a proven root cause. That work was not touched.

**User-defined C1-C8:** C1 PASS (A observed real independent O model
reply); C2 PASS (O itself created two parent-bound worker works); C3
BLOCKED/UNKNOWN pending actual two completed model replies; C4-C6
NOT_TESTED without Worker result, queue consumption or ACK; C7
NOT_TESTED (no authenticated autonomous A return binding); C8
NOT_TESTED for automatic delivery. A directly inspected the O model
reply through its original work handle, which is *manual observation*,
not proof of unattended O-to-A transport. No C1-C8 full PASS.

**Exact-source CI at `d9c3ed76`, rechecked through GitHub Actions:**

| Qualification | Run ID | Current observed result |
| --- | --- | --- |
| Source Acceptance | 43372399358 | SUCCESS |
| Core / Runtime | 43372399294 | **SUCCESS**: run completed 2026-11-13 06:17:55 UTC; Native proof and Android executable smoke completed |
| Manager | 43372399387 | SUCCESS |
| ChatGPT Provider source | 43372399261 | SUCCESS |
| Android fallback synthetic | 43372399238 | SUCCESS |
| Unsigned Full | 43372399266 | FAILURE at initial prerequisite admission; not proof of assembly defect |

At the time of the early Full failure, Core was incomplete; a later
readback confirmed that Core `43372399294` **finished SUCCESS** at
06:17:55 UTC, but exact-source full TUI had no matching run at the latest
check. The Full workflow **at d9c3ed76** was triggered on development
pushes and immediately runs
`full_qualification.py acquire`, which requires already completed
exact-SHA Core, Manager and TUI runs. The above early failure is a
repeatable workflow dependency-order defect, **D-CI-01**; no Full rerun
should occur until Core and all other exact inputs are successful.
Prior `0ba05892` successes are historical, not automatically d9c acceptance.
The latest full TUI qualification of `d9c3ed76` has not been independently
verified, so leave it unknown.

**Independent Termux Shell execution proof (not Native):**
`termux_shell` call `codex1672-v2-r6-independent-clang-20261113-a`
actually created one C source in the isolated internal scratch, compiled
and ran it with `assert(2+3==5)` and stdout
`R6_SHELL_FIRST_1672` (compile/run exit 0), then modified the same
source to `assert(3+4==7)`, recompiled and ran stdout
`R6_SHELL_SECOND_1672` (compile/run exit 0). This confirms local
Clang and the **independent Shell route**, not the required actual
same-Native-thread tool execution.

**Follow-up development candidate:** after this evidence was reconciled,
the Full workflow's premature `push` trigger was removed from the
isolated source candidate, preserving exact-source `acquire/assemble/
probe/prove` and mandatory manual `workflow_dispatch`. Focused
regression tests in an isolated **home** worktree outside Termux
`TMPDIR` passed **28/28**; documentation tests passed **10/10**.
This is a *bounded mitigation*, not automated producer-completion
scheduling and not CI success for the new source commit. The initial
local Full test attempt under `$PREFIX/tmp` failed four intentional
home/TMPDIR separation checks (3 error, 1 failure); the same full suite
passed unmodified when run from the correct home test-worktree location.
No production safety code or test assertion was relaxed.

**Product proof boundaries:** the installed Provider route source advertises
stable `/v1/native/responses` and compatibility
`/v1/native-v4/responses`; actual valid SSE, 400/404/413 classification,
same Native thread edit/compile/run, real AutoReview allow/deny,
physical notification click, account switching and automatic runit
generation reconciliation were **not** qualified in this checkpoint.
The cached ChatGPT catalog listed GPT-6 models but was not a
fresh account-entitlement query. No signature, update, rollback,
runtime activation or operating restart was performed.


## Git authority and development changes

- One continuing development source: [rewrite/rust-core](https://github.com/humtr/codex/tree/rewrite/rust-core).
  Remote and tracking ref were both `0ba058922fd1243f63a7cea2db0653348c64aa1d`
  at 14:19 KST, latest commit 2026-11-13 10:50:54 KST. That commit corrects
  a Provider capability-test lint issue; preceding `7831aae9` introduced
  authenticated Native protocol negotiation.
- `main` remained `fa6a1856866b9944c08cc7511d85180a384c1995`,
  the separate publication/approval line. A development push is not an
  authorization for signing, stable promotion or device activation.
- Relative to the original `7b98296a` 0.167.1 source baseline, the
  `0ba05892` development tree contains **161 commits**, touching **175 files**,
  with **41,094 added and 5,216 deleted Git diff lines**. These include
  imported upstream adaptation patches, tests, documentation and CI; they are
  not 41,094 newly authored product-code lines.
- Existing dirty worktrees remain owned by their original Writers. In
  particular the former integration workspace at `578a386b` retains four
  separate uncommitted document changes; the primary checkout remains
  `e4f9ab5d` with unrelated untracked files. None were cleaned or replaced
  during this documentation reconciliation.
- Official recipe was advanced from Codex 0.167.1 to **0.167.2**,
  upstream source `ce21072c90d32c40af7a55b60b2fb35c84a43a6a`.
  Neither a source SHA nor an uploaded CI artifact alone identifies the
  running binary.

## Actually installed / running Termux state

Read-only Termux commands returned:

| Observation | Result | Evidence boundary |
| --- | --- | --- |
| `codex --version` | `codex-cli 0.167.2` | Installed dispatcher |
| `codex termux version --json` | Full, S167, upstream 0.167.2; signature reported verified | Installed Core metadata; no update was performed |
| Core `current` | `full-v0.167.2-220cc2e-s167` | Observed symlink target |
| Provider in Core version output | `provider-chatgpt-v0.167.2-0ba0589-s27`, verified | Reported selected package, readiness not queried by version command |
| Provider `current` filesystem link | `provider-gpt-v0.1.0-df8a8c9-s27` | Legacy-named on-disk pointer; not equivalent to the running image |
| Runit Provider PID 21766 executable | `provider-chatgpt-v0.167.2-0ba0589-s27/codex-chatgpt-provider` | Direct `/proc/21766/exe`; at 14:22 KST |
| Runit Provider, tunnel, Host keeper | All `sv status` returned run; Provider 21766, tunnel 16304, Host 16103 | Process observation, not a reboot or update/rollback test |
| Existing prior generation | S165 Core runtime processes still existed | Existing live process does not mean active default edition |
| Signed/rollback history | Previous S165 / S163 generations retained per prior release evidence | No destructive change or fresh rollback drill in this review |

The Provider file link and the running service point to different S27-named
generation directories. The running PID matches the newer user-described
Provider image. Whether the link is a compatibility alias, a separate channel
pointer, or stale state requires consumer/installer ownership review before
classifying a defect. It must never be treated as proof that the runit service
is still running the old Provider.

The manual runit binding repair and v0.0.20 tunnel installation were reported
in the [runit task](docs/development/RUNIT_GENERATION_ACTIVATION.md).
**Automatic runit generation reconciliation across future update, rollback
and reboot is not verified/implemented.** No services were restarted and no
credentials changed during this review.

## Earlier exact-development-source CI: 0ba05892

The original 2026-11-12 Source TLS failure and TUI Clippy failures were
corrected on the development line. Their earlier failing run IDs are historical,
not current failures.

| Qualification | GitHub Actions run | Conclusion |
| --- | --- | --- |
| Source Acceptance | [43353147385](https://github.com/humtr/codex/actions/runs/43353147385) | **SUCCESS** |
| Core/Runtime | [43353147456](https://github.com/humtr/codex/actions/runs/43353147456) | **SUCCESS** |
| Independent Manager | [43353147332](https://github.com/humtr/codex/actions/runs/43353147332) | **SUCCESS** |
| Optional ChatGPT Provider source | [43353147336](https://github.com/humtr/codex/actions/runs/43353147336) | **SUCCESS** |
| Android window fallback (synthetic) | [43353147285](https://github.com/humtr/codex/actions/runs/43353147285) | **SUCCESS** |
| Full TUI Native (previously pending) | [43353819723](https://github.com/humtr/codex/actions/runs/43353819723) | **SUCCESS** |
| Unsigned Full composition | [43353147279](https://github.com/humtr/codex/actions/runs/43353147279) | **FAILURE** |
| Separate optional Provider signing/staging | [43354284442](https://github.com/humtr/codex/actions/runs/43354284442) | **SUCCESS**; not Full publication |

The failing Full job ended in its first admission stage:
`ValueError: no successful exact-source core qualification; rerun Full after its completion`.
The same SHA's Core and TUI jobs subsequently succeeded. The observed failure
is therefore an **early dependency/admission timing failure**, not evidence
that Full construction or Android execution itself is defective. One
exact-source Full requalification after all prerequisites is still necessary;
the successful Full run [43321313899](https://github.com/humtr/codex/actions/runs/43321313899)
belongs to older source `220cc2e4` and does not automatically qualify `0ba05892`.
A documentation-only successor SHA must not silently inherit source acceptance
without matching the actual producer input and CI admission contract.

Successful TUI CI proves its hosted build/tests, not a real Android
notification tap or the original user's exact `/switch` process/session
failure. The Android fallback CI uses synthetic transport, not actual
physical screen-lock and background Activity behavior.

## Actual independent ChatGPT session observations

Accessible saved Chat records were read without starting inference:

- Worker-1 saved conversation `6af5e376-5488-83e8-b721-065f8d639895`
  contains actual final `RESULT worker-1 323`.
- Worker-2 saved conversation `6af5e45e-e83c-83ee-ba10-be999d156063`
  contains actual final `RESULT worker-2 667`.
- The O2 saved conversation `6af5e41f-550c-83e8-816c-cee042d41a5d`
  documented W1 completion, W2 creation and queued ASSIGN, but did **not**
  confirm W2 model result reception/ACK; the later saved W2 final is separate
  evidence, not an orchestrator-consumed result.
- A distinct orchestrator experiment `6af5df99-14f8-83ec-9d7b-877e87565544`
  verified independent model/Native catalog observation but spawned zero workers
  when unrelated uncertain Native effects and missing parent authority blocked it.
- A separate saved automation experiment
  `6af5d09b-32cc-83e8-bacd-0fe27a826334` verified an independent model
  wakeup, **not** a verified original-assistant recipient.
- Original O1 source-integration and worker creation are real historical
  work, but no inspected receipt closes the full **A -> O -> W -> O -> A**
  delivery and acknowledgement chain. Queue insertion, Worker saved final
  and O consumption are distinct events.

The later **0.167.2 end-to-end acceptance job** was actually executed and
is recorded above. C1/C2 passed; Worker model completion and the C3-C8
delivery/acknowledgement chain remain unproved. The historical experiments
below are not reused as proof of this new job.

## Evidence boundaries retained

- W0-W20 source/CI implementation is substantially integrated; individual
  work items have separate device/live-model/recovery acceptance conditions.
  The [integration record](docs/development/RELEASE_FRAMEWORK_IMPLEMENTATION.md)
  maps the original work packages to current proof.
- The independent Termux `termux_shell` path works without a Codex Native
  writer and was used for these read-only observations. This is **not**
  evidence that a particular operating Native/Provider model round-trip passed.
- A capability HTTP 200 and malformed-payload HTTP 400 do not establish
  successful normal protocol-4 SSE inference. Native parent/child OS wait,
  Stop ACK, receipt reconciliation and true sibling execution require their
  own actual receipts.
- Automatic saved-Chat wakeup, durable worker report routing, adjustable
  budgets (all four dimensions), unattended job continuity and user-facing
  A-recipient ACK are **deferred designs**, not established product behavior.
- Genuine AutoReview invocation/allow/deny, cross-account refusal,
  original account switch, physical notification tap and private candidate
  authenticated execution are not promoted from synthetic/operating evidence.
- S167 selected/active image, source `d9c3ed76` (historically `0ba05892`),
  and staged Provider S27 have separate identities; signing/staging a Provider does not certify Full
  publication or install a new Core.
- Pre-existing user sessions, Writers, unrelated unresolved Native effects,
  accounts, runit service images, signed generations and Android shared storage
  were not modified. Only this new O/W acceptance scratch was created.

The full 2026-11-12 evidence snapshot (including unique receipt and hash
details) remains recoverable at `git show 0ba058922:CURRENT.md`.
Previous superseded worktree ledgers remain original owner evidence, not
a live instruction to replay operations.
