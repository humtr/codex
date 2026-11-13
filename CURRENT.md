# Current product state

Observed: **2026-11-13 14:22 KST** (source/CI/Termux read-only observations).
This file records verified facts and evidence boundaries, not implementation obligations.
Requirements: [SPEC](SPEC.md) and [functional annexes](docs/spec/INDEX.md).
The single execution plan is [HANDOFF](HANDOFF.md); the W0-W20
plan-versus-implementation reconciliation is in
[the finite integration record](docs/development/RELEASE_FRAMEWORK_IMPLEMENTATION.md).
The deferred design candidates are linked from HANDOFF.

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

## Latest exact-development-source CI: 0ba05892

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

The new user-prescribed C1-C8 **0.167.2 end-to-end acceptance job** has
not been executed as part of this documentation reconciliation. Its exact
actor/generation and report route must be observed, not invented.

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
- S167 selected/active image, source `0ba05892` and staged Provider S27
  have separate identities; signing/staging a Provider does not certify Full
  publication or install a new Core.
- Session records, active Writers, unresolved Native effects, accounts,
  runit service images, signed generations and Android shared storage
  were not mutated by this review.

The full 2026-11-12 evidence snapshot (including unique receipt and hash
details) remains recoverable at `git show 0ba058922:CURRENT.md`.
Previous superseded worktree ledgers remain original owner evidence, not
a live instruction to replay operations.
