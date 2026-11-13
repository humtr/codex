# Current product state

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
