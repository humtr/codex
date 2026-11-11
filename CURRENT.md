# Current product state

Updated: 2026-11-12 KST. Observed facts and proof limits only.
Requirements: [SPEC](SPEC.md) and [functional annexes](docs/spec/INDEX.md).
Unfinished acceptance: [HANDOFF](HANDOFF.md).
The finite integration record is [RELEASE_FRAMEWORK_IMPLEMENTATION](docs/development/RELEASE_FRAMEWORK_IMPLEMENTATION.md).

## Source and publication authority

The ongoing development branch is [rewrite/rust-core](https://github.com/humtr/codex/tree/rewrite/rust-core).
This integration starts from `7b98296a411a2c435bb9f34ee51f286f33f3491b`
in an independent `integration/release-framework-20261112` worktree.
Core, MCP-shell and terminal-display changes are now feature commits in this
integration. Exact-source remote acceptance remains pending.
Query the actual HEAD before relying on these observations.

The independently read remote `main` is
`b2567063d2a7a333a718d2495e2c9f8c928d822c`.
It remains publication/approval authority, separate from development.
The repository was initially shallow: after unshallowing, the Native patch
and planning histories have common ancestors with development; `main` still
has separate ancestry. Neither branch currently has GitHub branch protection.
No history replacement or primary-worktree reset was performed.

The recipe selects upstream **0.167.1**, source
`0a94398fa60c4fac0e7659e4213a0a36e613dd46`.
Native integration `a55d5bb8`, PR #29's `current` compatibility alias,
and the account draft lineage `2dd07d42` are already in development ancestry.
The account draft was integrated through `8f97affc`; it is not an unmerged
feature to restore over the current AppHost implementation.

## Independently observed operating installation

The new candidate Core's offline `termux version --json` read the existing
installation without activation or replacement. It exited 0 with empty stderr.

| Observation | Verified value / boundary |
| --- | --- |
| Selected signed generation | **Full S165**, `full-v0.167.1-7b98296-s165`, Codex **0.167.1** |
| Previous generation | `full-v0.167.0-c19b61b-s163`, retained rollback |
| Core | `74c9ef705d46a1c9a35711e8cb7a64adc107a60849994aa3ce324dc2063cdd55`, `core-api-v1` |
| Runtime | `ef50757ebe89bf672ab2bb8a6fa875362f7882ba18537ed9bf64196e54bf3d29` |
| Upstream source archive | `ef3e1443a7795d596f1601a533d7225a1030488d60c965af1f0610ab95889031` |
| Optional Manager and Enhanced TUI | Present and digest-verified in S165 |
| Selected Provider package | `chatgpt-da5bf622bc49-s25`, independently signature-verified; service/authentication readiness not queried |
| Core S164 | Signed release reference exists; no S164 generation directory was installed on this device |
| Component revisions | Absent in legacy S165 metadata; reported unavailable/null, never invented from sequence 165 |

The candidate Core binary for that read was
`cdca2946eaf840323f432b925fb7327662887ceae5e05a22ae82fa3829c0378a`
from the integration working tree. It was **not installed**. The operating
Core's earlier `codex termux version` still returned invalid command.
Sixteen protected generation files and the active pointer were identical
before and after the candidate read. Existing signed generations were not
rewritten, re-signed or renamed.

A bounded read-only process inventory found runtime executables from S159,
S163 and S165 concurrently (19, 16 and 5 processes respectively, including
servers/children, not a count of independent TUI windows). Several older
frontends inherit the current public Core path. This is coexistence evidence,
not authorization to stop them or proof that all version combinations work.

## Implemented source and local acceptance

- Release inputs, canonical Android Core production, independent Manager
  qualification, unsigned Full composition, component revisions and globally
  reserved publication sequences are integrated. Legacy signed formats remain
  readable. The exact-main release-intent guard stops development source
  acceptance from authorizing publication. No reservation or signed release was
  created by this integration.
- Core-owned offline version inventory, explicit Core/Full update selection,
  Core-only local-build independence, one cancellation choice and factual
  no-bwrap diagnostics are implemented. Signed isolated Core/Full fixtures,
  edition rejection, rollback preservation, missing optional components,
  sidecar revisions and upstream version routing passed focused tests.
  The complete Core suite passed: **270 passed, zero failed, one existing
  explicitly gated real-Termux smoke test ignored**, exit 0 (474.67 seconds).
  Whole-workspace all-target clippy with warnings denied and the Provider
  all-target lint gate both passed. The subsequent admission-argument grouping
  passed the real remote acquisition/reuse regression and fresh Core/Manager build.
- Manager display uses existing terminal/window machinery with verified
  Owner/home/foreground identity, one-shot construction, duplicate suppression
  and default-off automatic display. Correct stock SESSION_ACTION is a string.
  **130 Manager tests passed**: 57 library, 72 profile integration, one
  window-launch test; no failures/ignored tests. The new fixtures include
  actual isolated PTY and qualified Native Owner responses.
  Android Activity acceptance remains a separate CI/device boundary.
- Independent shell source `c0a81898` is integrated. The hardened candidate
  preserves original receipts, bounded output and owned process-group cleanup.
  A separate candidate-worker process without Provider state passed shell
  execution, exact replay, one-effect counter, argument conflict and unchanged
  Native-claim sentinel. Connecting only for discovery retained the original
  17 MCP tools plus `termux_shell`, 18 total.
- The integrated Provider library suite passed **253 tests**, zero failed,
  with five existing explicitly gated tests ignored. The new independent
  worker integration test passed. This is not an assertion that the ignored
  live boundaries were tested by that invocation.
- Provider/TUI source now keeps OpenAI's upstream catalog separate from the
  authenticated ChatGPT app catalog, including availability and actual effort.
  Internal Provider ID `chatgpt_desktop` and persisted model namespaces remain
  compatible. New artifact naming uses `provider-chatgpt-v...`; old
  `provider-gpt-v...` references remain readable.
  Local pristine-upstream assembly and focused Python checks passed.
  Local TUI Rust compilation was blocked before compilation by the unavailable
  pinned rusty_v8 dependency; exact-source hosted TUI qualification is pending.

## Native ownership and availability evidence

The earlier Busy P0 was already operationally recovered: original evidence
`NATIVE_RESOURCE_RECOVERED`, `SEQUENTIAL_EXEC_OK_2`, `NATIVE_REUSE_OK`
all exited 0 and 15 claim tests passed. Its source patch `e4f9ab5d` is now
selectively integrated; a range-diff confirmed patch equivalence. It is not
an unresolved operating incident.

W20's installed Owner lookup failure was independently reproduced. A legacy
server record naming an old runtime was parsed as a current Binding before
process identity was checked; its PID had been reused by a proot loader.
That unrelated stale registration poisoned Owner discovery. Candidate code
skips only a positively different/exited process, keeps uncertain/unsafe
records fail-closed, and preserves the private response schema.
Against the same operating registry it returned confirmed writer-free absence
for the unused test UUID and identified an actual live Owner correctly.
No operating registry or Writer was changed. Nineteen Owner tests, the Core
Owner contract test and eleven Manager task tests passed.

A real S165 frontend with candidate Core/Manager in owned isolated profiles
now passes Owner lookup and same-thread reentry. Its destination checkpoint
has **not yet completed** the full quota-switch fixture. Preserved process
evidence proves the checkpoint reaches the frontend argv. Source additionally
prevents replayed historical quota errors from mutating current recovery state;
the new frontend must still pass the whole route.

Native execution metadata previously rejected even an unrelated synchronous
request-user-input notification hook. The new hook gate distinguishes actual
matching execution hooks from irrelevant hooks and retains opaque treatment
for uncertain hook effects. The patch applies to pristine upstream; four
hook-engine regressions are wired into CI. A failed old-runtime compiler
trial had no compiler effect; its owned work was settled without stopping
other work. Safe parallel compiler acceptance requires the new runtime.

## Actual ChatGPT–Native development loop

The existing authenticated app's live catalog returned six visible choices
plus three hidden compatibility entries, including GPT-6 and GPT-5.6 Sol
Instant/Thinking/Pro. This demonstrates that two static entries are not the
server catalog. Thinking supplied standard/extended/max options.

An actual `chatgpt-desktop/app/gpt-5-6-thinking` request at effort `high`
used one original Chat and one Native thread. After an initial MCP discovery
failure with zero effects was preserved, a legitimate same-work follow-up
wrote and compiled C, returning `W19_NATIVE_FIRST` and exit 0. A further
same-thread turn modified the same file, recompiled and returned
`W19_NATIVE_SECOND` and exit 0 to the original Chat. Final state was completed,
Native idle, with no pending/active/unconfirmed effects.
All three persisted turns retained Provider, model, effort, on-request
approval and the existing sandbox setting.

This is **operating-binary live evidence**, not acceptance of the new TUI.
The operating Provider binary was
`e27e92c2442647da3a1997455198b4f5a64e700df6a4e6f37c70fb71b5eae44d`;
it differed from the Core-selected signed Provider package.
The masked receipt digest is
`c44dc713ce94b3962d2dba41a27eb0deecc9c494ac049297bad473d8595e2929`.
No login/token/service/hook setting was changed for the trial.

The user's confirmed connector refresh in the **janmori101** Linux app
(17 to 18 actions, termux_shell registered) and actual independent MCP
shell success remain accepted prior evidence. Native execution-home labels
and the earlier wrlab OpenAI token_revoked observation do not establish the
ChatGPT inference account or invalidate that refresh.

## Remote CI evidence and publication boundary

Only these baseline receipts are currently accepted here:

| Exact source | Workflow | Run | Conclusion |
| --- | --- | --- | --- |
| `7b98296a411a2c435bb9f34ee51f286f33f3491b` | Source Acceptance | [43012469220](https://github.com/humtr/codex/actions/runs/43012469220) | success |
| same | Core/Runtime Qualification | [43013017085](https://github.com/humtr/codex/actions/runs/43013017085) | success |
| same | TUI Native Qualification | [43013363622](https://github.com/humtr/codex/actions/runs/43013363622) | success |

These supersede the earlier f9dfc3f failure as baseline evidence, but do not
qualify new changes. New exact-source runs have not yet been accepted.

The read-only pre-push audit found no exact-main release-intent-v1, no
automatic Provider publisher and no workflow_run publisher. The existing main
schedule still runs pinned upstream discovery before the guarded release
controller; a new upstream could advance development independently, so remote
HEAD must be rechecked before each push. The independently checked upstream
release was still rust-v0.167.1. No main, stable index, signing key, release,
operating install, account or active user Writer was changed.
