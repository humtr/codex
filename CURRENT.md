# Current product state

Updated: 2026-11-12 KST. Observed facts and proof limits only.
Requirements: [SPEC](SPEC.md) and [functional annexes](docs/spec/INDEX.md).
Unfinished acceptance: [HANDOFF](HANDOFF.md).
The finite integration record is [RELEASE_FRAMEWORK_IMPLEMENTATION](docs/development/RELEASE_FRAMEWORK_IMPLEMENTATION.md).

## Source and publication authority

The ongoing development branch is [rewrite/rust-core](https://github.com/humtr/codex/tree/rewrite/rust-core).
This integration starts from `7b98296a411a2c435bb9f34ee51f286f33f3491b`
in an independent `integration/release-framework-20261112` worktree.
Feature commits are fast-forward integrated into that official development
branch; the primary checkout remains on its protected older lineage. The
remote checkpoint `787fcd2bc6708941ffbdd6a193bae35d3b8478a5` passed Source,
Core/Runtime, Manager, Provider and stock Android qualification. Full TUI and
Full composition retain their separate exact-source gates. Later formal consumer
and current-selection changes require their affected exact-source gates; query
the actual HEAD and its Checks before relying on an earlier checkpoint.

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
  The complete Core suite passed: **271 passed, zero failed, one existing
  explicitly gated real-Termux smoke test ignored**, exit 0 (468.84 test seconds)
  at source d36ca465 using the repository's required single-thread test policy.
  An earlier caller override to two threads caused shared-fixture lock errors;
  that failed receipt remains preserved and the test policy was not changed.
  Whole-workspace all-target clippy with warnings denied and the Provider
  all-target lint gate both passed. The subsequent admission-argument grouping
  passed the real remote acquisition/reuse regression and fresh Core/Manager build.
  At exact build source 891c6595, actual isolated Termux PTY keep/remote menu
  cases both exited 0: one cancellation, zero/one local request respectively,
  lease release while waiting and unchanged activation. Real external requests
  were zero. Installed and candidate Core-only version executions both returned
  exactly codex-cli 0.167.1 with empty stderr; 20 signed-generation files, the
  public Core and current pointer remained unchanged.
- Manager display uses existing terminal/window machinery with verified
  Owner/home/foreground identity, one-shot construction, duplicate suppression
  and default-off automatic display. Correct stock SESSION_ACTION is a string.
  **134 Manager tests passed** at feature 66edd08d: 59 library, 74 profile
  integration and one window-launch test; zero failed/ignored. Seven display
  cases also passed under an alternate umask. They include real isolated PTYs,
  qualified Native Owner responses and simultaneous click/automatic commands.
  The earlier stock Android race exposed a generic profile-lock error before
  display idempotency. Window locks now cover only durable descriptor
  transitions, not Android dispatch/wait/focus. Exact pending identity and
  original Writer remain preserved; duplicate starters cannot delete an
  original route. Strict clippy, formatting and documentation gates passed.
  Stock run 43093224127 at 66edd08d passed actual Android API 30 Activity
  focus, foreground UUID, repeated/concurrent display, auto ON/OFF and
  foreign-home rejection. The Java assertions were not weakened. This fixture
  uses real Manager/Service/PTY/Activity with synthetic Core transport/frontend;
  it does not prove physical-device visibility or Native inference.
- Independent shell source `c0a81898` is integrated. The hardened candidate
  preserves original receipts, bounded output and owned process-group cleanup.
  A separate candidate-worker process without Provider state passed shell
  execution, exact replay, one-effect counter, argument conflict and unchanged
  Native-claim sentinel. Connecting only for discovery retained the original
  17 MCP tools plus `termux_shell`, 18 total. The Android workflow now preserves
  and verifies a separate unsigned worker candidate instead of discarding the
  executable built alongside the Provider. Eleven packaging tests passed.
  Connector refresh now selects an explicit owned `mcp_command` independently
  from the Provider/Desktop binary, preserves legacy omission behavior and
  fails closed for an invalid explicit wrapper. The actual client suite passed
  18 tests with 109 assertions; no operating refresh was performed.
- At exact remote source `66edd08d`, the Provider all-target suite passed
  **362 top-level tests**, zero failed, with 34 existing explicit asset/live/helper
  entries ignored. Nested subprocess results are not double-counted. Strict
  clippy and unsigned Android build passed; binary SHA256
  `c0bb92d6768b119f2b941a9c562d9e6015a1917424a73d8a3280cb5bc2e7e548`.
  The standalone worker test is included. Original failed receipts are retained:
  a sibling fork could hold a copied executable's write descriptor and cause
  ETXTBSY; fixture publication now uses a private child, without retrying effects,
  increasing deadlines or weakening ownership assertions.
- Provider/TUI source now keeps OpenAI's upstream catalog separate from the
  authenticated ChatGPT app catalog, including availability and actual effort.
  Internal Provider ID `chatgpt_desktop` and persisted model namespaces remain
  compatible. New artifact naming uses `provider-chatgpt-v...`; old
  `provider-gpt-v...` references remain readable.
  Cold OpenAI return also uses the upstream offline model manager, including
  account-scoped cache validation and bundled fallback, instead of manually
  parsing models_cache.json. Local pristine-upstream assembly and focused
  Python checks passed; combined Native/TUI source assembly at 891c6595 passed.
  Local TUI Rust compilation was blocked by an unavailable pinned rusty_v8
  dependency. Hosted run 43078797588 at d2ab3e0e built and executed the actual
  patched frontend and passed focused TUI, independent Owner/model fault
  oracles, Native inference/configuration, hook matching and Provider API gates.
  The subsequent Retain step failed; its recovered cause and test-only correction
  are recorded below. This run has not qualified the frontend. Candidate artifacts and earlier successful steps do
  not establish Full qualification.
- At b87d7e95 the complete Source Python/static gate passed **241 tests**
  (36 repository scripts, 190 release helpers, 15 frontend Python tests),
  exit 0. Documentation checks and ten regression tests passed; all 14
  workflow YAML files parsed locally. Ruby/Psych is unavailable in Termux;
  its unmodified hosted gate remains required.

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

The staged actual d2 frontend with candidate Core/Manager completed the
same-thread switch fixture, exit 0: source model samples two, destination one,
original prompt and checkpoint once, original Native tool effect once. Provider,
model, effort, approval, reviewer and sandbox settings remained intact. This
uses synthetic model responses and an explicitly mixed private candidate
composition, not a real OpenAI account transition or qualified final Full.
The later cf Core/Runtime + 66 Manager + d2 TUI fixture passed the existing
40-assertion task qualifier with the actual Enhanced executable observed and
the same UUID resolved across account-a, default and account-b. It preserved
active/disconnected Writers, refused cross-home conflict, confirmed Stop and
used only owned transfer fixtures. Receipt SHA256:
`7549caa51bd4afa74a0a3b2954e41437dfbe46e9ed3440882d065482f6d26341`.
Ordinary and named-permission automatic quota handoffs also passed; an
incompatible permission profile stopped before any destination inference.
Those three trials exercised the `/switch` menu but automatic handoff, not
manual destination-row selection. A separate actual manual trial then disabled
automatic handoff, selected `next` in the picker, verified the same UUID's new
Owner/home before destination inference and submitted one typed follow-up.
It preserved gpt-6.1-sol/OpenAI/high, named permissions and original tool effect
exactly once. Its source and current selections matched; this does not prove
preservation after a later model change. Manual receipt SHA256:
`744545926ef31dd2b1775643e88c315bf7b66baf2829f08ad0d3139c9af95e1c`.
Those trials used zero live model requests and no physical Android transport;
protected and input bytes were unchanged and owned processes finished.
A further changed-selection trial on the old d2 frontend reproduced a separate
regression: the same thread retained ChatGPT, its model, tier and turns, but
current `high` effort reverted to the source launch's explicit `none` after a
manual profile switch. This was the Native enum string `none`, not JSON null.
The target profile had different OpenAI/low/priority defaults; only two original
synthetic responses had occurred and no destination follow-up was submitted.
The failure receipt is `02d20ac216a71f5e0d224731fc1f700edbcae58f6817e7833501e0d35001ec73`.
Source f915344e/f1491afb now captures the current collaboration selection,
verifies the exact Native thread/model/Provider/effort without a resume effect,
and uses the existing private saved-selection mode to avoid stale destination
overrides. Ordinary resume tier options and permission/reviewer authority stay
intact. Four new Rust regressions cover current/default differences, nullable
versus explicit-none effort, Native readback and private/ordinary resume. CI
requires each actual named PASS; their hosted execution and the corrected
actual changed-selection trial remain acceptance gates.
Historical quota replay now restores display history without starting new
account recovery. The receipt digest is
`217048f8b2e750c6bfda0d21c7368fdd745cec820591c4afd8d8cf409caf1ff3`.

Native execution metadata previously rejected even an unrelated synchronous
request-user-input notification hook. The new per-tool schema-2 hook gate distinguishes actual
matching hooks from irrelevant hooks and retains opaque treatment for uncertain
hook effects. Stored unmarked/mixed/unknown bounded claims are conservatively
widened for new conflict decisions without rewriting their original snapshots.
Queued legacy Read calls are checked against their original turn facts for both
offer and settlement; original call IDs/fingerprints/results remain intact.
Old schema-1 execution receipts remain readable but do not grant new parallel
capability. An already-running old Provider may have unrecorded read effects;
new source cannot retroactively arbitrate those. Coordinated future activation
is a separate operating boundary. The patch applies to pristine upstream; four
hook-engine regressions passed in the hosted frontend gate. The earlier
old-runtime compiler trial had no compiler effect and was settled without
stopping other work. New actual compiler trials exposed two continuation
boundaries: a running empty poll lost its bounded scope, and a completed
Native item could release the claim before its last output was collected.
The fixes retain original call/session/PID/start/thread/turn evidence on the
existing receipt, without synthesizing results or creating another registry.
Pending, reused, foreign, collected or unproved session identities stay
conservative.

At Provider source ca1dd504 with exact qualified cf849ac3 Core/Runtime, a
Core-only fixture with no Manager/Enhanced ran two real clang processes with
**5,760 ms overlap**, two simultaneous bounded claims and two 2,960,160-byte
ELF outputs. Both original final polls returned exit 0; a conflicting opaque
call was refused before effect and its old ID never executed later. A legal
new call then ran on the same owned work. Original results were preserved,
unconfirmed effects returned to zero and the shared ledger returned to its
original empty digest. No model request was used. Evidence:
`terminal-poll-integrated-cf-r1/compiler.json` and `compiler-receipts.json`.
Provider-restart restoration of retired receipt chains was separately reproduced
after actual cache eviction. The integrated fabf0606 fix reads only the admitted
immutable source chain, including later collectors/reused session identities;
it never reactivates retired execution tokens or transfers a Controller.
Thirty-four Native concurrency and four source-discovery cases passed the
focused producer tests. The integrated fabf0606 Provider suite passed **364
top-level tests, zero failed, 34 existing explicitly gated entries ignored**.
A one-line test-only reference-slice correction in 54e3328a then passed strict
all-target clippy and all four terminal-session regression tests. These counts
are not added to earlier repeated suites. The warm-owner compiler proof is
not relabeled as a live restart trial.

The existing selected-controls qualifier also passed on a Core-only fixture:
21.18 seconds, exit 0, no Manager/Enhanced and zero model requests. Its checked
AST preserved all 32 block assertions and seven helper assertions. Actual
Native execution proved same-thread Controller transfer with receipt fencing,
two independent Writers, a queued third writer under one parent, independent
roots at a per-parent parallel limit of one, Stop preserving a sibling,
restart-stable Stop, safe notLoaded-thread reload, original-start/call-count
preservation on time extension, dependency evaluation and StopTree of five
works including the coordinator. Receipt SHA256:
`711b857bb0aa5be56429cf028b27646f9c83abcf41477de38070cd8d893aac70`.
After this and the Busy trial below, a read under the existing ledger lock
confirmed zero claims and the original empty digest. No manual settlement or
ledger mutation was used.

A separate actual Core-only observer fixture passed in 4.40 seconds, exit 0.
The same Native thread/turn remained active after its original observer
connection closed; owner-context reentry and a same-Owner observer recovered
the original socket. A different execution home's resume was explicitly
refused while the original Writer's kernel lock, inode, process and record
remained unchanged. The original response completed once; closing the final
subscriber then produced verified safe Owner absence. One loopback synthetic
response, zero live Provider requests and zero Android dispatches were used.
Receipt SHA256: `a135b830efc406cfe0e9cb71496287569118ca6ef046da0f838bcb776ae43da8`.
This verifies Native observation, not public Manager/Enhanced screen display.

A separate Core-only basic-UI trial passed through the ordinary Core dispatcher,
without a Core-only override, Manager or Enhanced TUI. The actual process image
was the selected qualified Runtime. Typed input caused exactly one real command,
whose original Native result contained `OWNED_PERMISSION_PROOF:PL_MAIN`, exit 0.
The same thread displayed `PL_MAIN_OK`; UI exit was 0 after 8.55 seconds.
Two responses came from the loopback fixture, with zero real Provider inference
and zero physical Android dispatch. Cleanup succeeded and operating Core,
activation and current-generation pointer stayed unchanged. Receipt SHA256:
`0e0506ca81fbe03bc19e91f7126956fe52470c9ad90acbb3cd3684bd29344446`.

## Physical terminal test boundary

One pre-isolation failed unit fixture recreated its owned display descriptor
for `/not-executed` while inheriting the actual Termux AM transport. A subsequent
read-only probe observed terminal count increasing from one to two, consistent
with an inert test surface; the exact Android session could not be mapped.
No further display/focus/close was issued to identify or remove it. Its original
private descriptor and bounded evidence remain preserved; the complete
environment was not logged. Signed installation bytes and activation remained
unchanged. Direct display unit probes now run in transport-isolated subprocesses,
so even failed admission cannot dispatch to operating Android. A reproduced
dangling-route case also now fails before any new descriptor or Android request.

## Actual candidate TUI and independent MCP evidence

The real d2 candidate PTY passed OpenAI → ChatGPT → OpenAI, foreign-provider
direct-model rejection, cold resume and follow-up on the same thread with
upstream High effort restored. Four responses were synthetic. A separate
zero-input PTY trial proved that Provider/model/effort were durably applied
before the first prompt: zero Responses/Chat submissions/checkpoints and two
settings audit entries. Receipt digests are respectively
`9ceef3690c2bf5910832f8157696aa5407952675ffad50698cbe8c5a4ba7ac79` and
`921c3016abb6e4ceda7407baf35a66c21186f3913035b0a30fa52846c8452993`.
These prove actual TUI/Native routing with synthetic inference, not a live
authenticated candidate Full. The qualifier observes actual terminal cells
and authoritative Native metadata rather than matching stale ANSI output.

A later actual d2 PTY trial exercised Native `/compact`, then the scoped model
picker, foreign direct-model rejection and a follow-up on the same thread.
Provider/model/effort and prior canonical turns were preserved; it exited 0 with
exactly three synthetic Responses and zero real Provider inference. Protected
operating Core/activation and all input binaries were unchanged. The earlier
trial's automatic-title request caused a strict request-budget failure; the
existing qualifier's explicit thread-name setup was restored before the new
isolated trial, rather than relaxing the budget. Successful receipt SHA256:
`3256d30da5e50ec754c26c8c5575cb0be4287a94ef603051c81c2a1e3b01178f`.
This is actual PTY/Native compaction on the recorded unqualified composition,
not final Full or live App acceptance.

The currently connected independent MCP `termux_shell` returned
`DIRECT_TERMUX_MCP_INTEGRATION_OK`, exit 0. Repeating the exact call ID/arguments
returned the identical original PID/output/timestamp; changed arguments with
that ID returned `termux_call_id_payload_conflict`, executed=false. The catalog
contains exactly the previous 17 tools plus that one shell. This wire evidence
uses the existing installed worker; the separately tested hardened candidate
was not activated. Shared claims were empty at that initial observation.

The independently qualified G Android worker was then acquired by immutable
artifact 13893816894 and verified against its exact source and closed inventory.
Executable SHA256 `68df8ecb6918153d28808e3a029cc53ae77579cffa20557d29dadc97ea4d5b12`
passed real private stdio execution without Provider or Core: exact replay and
conflict refusal, worker-restart receipt recovery, timeout/background-child
cleanup, cancellation preserving the original unknown intent, simultaneous
worker calls with one effect, and the 64 KiB output bound. All three workers
exited 0 with empty stderr, owned children were inactive and the Native sentinel,
operating wrapper and Core stayed unchanged. Execution receipt SHA256:
`a7d828a8444baec9b47d4a1d427a7475e4b668ca8a3c87fa7b898576946ddff6`.
This is actual candidate Android execution; its connected ChatGPT tunnel was
not activated or refreshed.

A subsequent actual connected call, `cr12-root-294`, ran while an owned real
clang command held a verified bounded Native claim. A conflicting command
against the same output had returned native_resource_busy, executed=false.
The independent shell returned `DIRECT_TERMUX_MCP_WHILE_NATIVE_BUSY`, exit 0.
Exact call-ID/argument replay returned the original PID, timestamp and output;
the effect marker remained one line. The original compiler then completed and
its original session result was collected with exit 0; the refused call never
executed. Timing control used only that compiler's verified pidfd, with an
independent 45-second resume watchdog; explicit release resumed it after
26.83 seconds. The Native Runtime/Writer stayed running. Protected operating
Core, activation and current pointer remained unchanged; model requests were
zero. The wire proof SHA256 is
`cca529071712d3daae62d9e3cbecab1b3032bcd5edcab9efbeadc1cc993babcc`.
This proves installed independent-MCP availability during candidate Core-only
Native Busy, separately from the hardened candidate worker's process tests.

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

Two additional actual operating-App trials used private execution homes:
one without ordinary `auth.json`, one with a synthetic invalid OpenAI API key.
Both received actual `chatgpt-desktop/app/gpt-5-6-instant` responses. Explicit
OpenAI routes in those same homes returned 401 missing bearer / invalid API
key; the retrying owned CLI probes received SIGTERM at the 25-second bound
(exit -15), rather than completing naturally. This does not claim a real revoked refresh-token test or actual
OpenAI account switch. Receipt SHA256:
`9891a5517af8dd78cd72ee121f217251db8a06f4b1edba9573d5b14ae056346a`.

A read-only live-candidate preflight found an actual protected boundary:
the operating Provider captures its installation HOME and verifies its own
registry/Runtime; the authenticated AppHost retains its existing owner even
with zero active lanes. A private candidate installation cannot borrow that
Writer or AppHost identity. No candidate model request, registry link/copy,
login or service replacement was attempted. Receipt SHA256:
`9eac4dba78908b6b22c819e91e4056a61a94277648633ef37e2f53522604c4c4`.
Candidate live acceptance therefore needs a separately authorized activation
or a separately authenticated private AppHost; synthetic TUI and operating
live evidence remain distinct.

The user's confirmed connector refresh in the **janmori101** Linux app
(17 to 18 actions, termux_shell registered) and actual independent MCP
shell success remain accepted prior evidence. Native execution-home labels
and the earlier wrlab OpenAI token_revoked observation do not establish the
ChatGPT inference account or invalidate that refresh.

## Remote CI evidence and publication boundary

Accepted baseline and integration checkpoint receipts:

| Exact source | Workflow | Run | Conclusion |
| --- | --- | --- | --- |
| `7b98296a411a2c435bb9f34ee51f286f33f3491b` | Source Acceptance | [43012469220](https://github.com/humtr/codex/actions/runs/43012469220) | success |
| same | Core/Runtime Qualification | [43013017085](https://github.com/humtr/codex/actions/runs/43013017085) | success |
| same | TUI Native Qualification | [43013363622](https://github.com/humtr/codex/actions/runs/43013363622) | success |

The 7b98296 receipts supersede the earlier f9dfc3f failure only as baseline
proof. Integration checkpoint `787fcd2bc6708941ffbdd6a193bae35d3b8478a5`:

| Workflow | Run | Conclusion |
| --- | --- | --- |
| Source Acceptance | [43104266073](https://github.com/humtr/codex/actions/runs/43104266073) | success |
| Core/Runtime Qualification | [43104266214](https://github.com/humtr/codex/actions/runs/43104266214) | success |
| Manager Qualification | [43104266119](https://github.com/humtr/codex/actions/runs/43104266119) | success |
| Provider Qualification | [43104266147](https://github.com/humtr/codex/actions/runs/43104266147) | success |
| Stock Android window | [43104266064](https://github.com/humtr/codex/actions/runs/43104266064) | success |

At cf849ac3, the actual Core producer built SHA256 `48c041e4…f4596f` once. Source Acceptance
restored and reverified the same bytes/input/original production run and skipped
cross-compilation. Raw Runtime production was reused from immutable successful
artifact 13887791090 after the current verifier checked its API identity, ZIP
digest, complete inventory and production inputs; that raw-production receipt
is not itself qualification. Consumer-specific Core/Runtime gates then passed.
Manager at 66edd08d qualified independently as binary
`16caee8493f22f47fee77658127627522af8b2d92334c9560f1807da018e4c89`.
The cf849 stock run 43090735539 failed the same-thread click/auto race and led
to the corrected, successful 66edd08d Stock run. The failed run's earlier
Activity steps were not treated as whole-run success.
Full run 43090735816 refused missing exact-source prerequisites at dispatch,
which preserves the admission gate. Later exact-source runs must satisfy all
required gates before the new source is reported accepted.

At the preceding dfd53e61 checkpoint, Source Acceptance 43098958949 also
completed successfully with actual Core cache-byte revalidation.
The full-TUI request 43097766900 and its one accepted rerun failed at GitHub
startup before any runner/test job was created. The reported internal-error
request IDs are retained; this is distinct from d2's actual Retain failure.
The original d2 run eventually reached completed/cancelled after the failed gate.
Its recovered log identifies `pin_action_updates_shared_section_state`: 53 selected
Retain tests, 52 passed and one failed at the expected PinToggled event. This is
a new d2 failure, not the earlier f9 failure. Source analysis found that the
fixture leaves a distinct asynchronous overview-refresh completion unsettled
before its next pin action. The corrected fixture keeps the original pin/unpin
and persistence assertions, additionally processes the exact matching successful
refresh and checks refreshed UI state. Hosted execution remains required.
The proof ZIP's 1,045,338 bytes uploaded successfully; GitHub then returned HTML
instead of a FinalizeArtifact API response. That separate external failure does
not justify changing upload limits or weakening tests. A bounded failure reporter
now preserves the next actual command's original
status and allowlisted test summaries before artifact upload; all original
Retain commands and assertions remain required.

An actual provisional Full payload probe exposed a different concrete defect:
Runtime/TUI with an empty environment exited 0 but emitted a missing-HOME alias
warning, which the strict stderr oracle correctly rejected. The corrected
producer gives only those two probes a fresh private HOME and a validated short
TMPDIR. Actual retry exposed the upstream refusal to create helpers beneath
TMPDIR, so the home now uses an explicit owned protected parent outside that
canonical temporary root. Both failures and the deferred qualification marker
were retained. Thirteen Full tests, thirteen workflow tests and ten documentation
tests passed after integration, without inherited user settings or weakened
stderr checks. Exact-source Full CI remains a separate gate. The corrected helper also passed
all four actual primitive probes on the explicitly provisional cf Core/Runtime,
66 Manager and d2 TUI payload: exit 0, exact stdout, empty stderr, unchanged
candidate digest and cleaned private HOME. This uses a disposable test signing
key and does not establish formal Full qualification. Execution receipt SHA256:
`275ef701761bceeb3b2f0d1c4d0d823b87b848cae0a9ccd6b64c07b9ceb2aee6`.

A new TUI regression sends live and restored ContextCompaction notifications
for both Providers, then checks same-thread model/effort/catalog/history and
scoped picker/direct selection without inference. CI now requires its named
PASS receipt. Rust formatting/source-signature review passed; this newly added
case still requires actual hosted compilation and execution. The pinned upstream
autocomplete is command-name completion (`/mo` to `/model`); direct model argument
resolution is separately tested and is not called a model-name completion engine.

The formal-path consumers now defer frontend public inputs/references until
exact Full admission and use immutable qualified Android artifacts across branch
cache boundaries. The Full proof's actual ZIP digest, byte count, closed inventory
and exact source/component runs are revalidated. Core-only prospective bindings
and publication queues contain no optional frontend. The final reservation still
compares every actual candidate payload and normalized composition before signing.
The Full input consumer additionally preserves the original inputs-v1 plan
and downloads each of its six immutable artifact IDs. Same-name replacement or
a newer run cannot replace those selections; source/run/role, archive digest,
byte count and bounded regular-file ZIP inventory are checked before assembly.
The proof reader retains that original source-and-runs-bound plan for private
reassembly. At f9d43ee2, 67 distinct Full/input/controller/reservation/intent and
qualified-component tests passed, plus thirteen workflow and ten documentation
tests. The earlier eight input/transfer tests remain separate. A real 567-byte G
Manager proof archive (artifact 13893541451) passed the new transport and matched
its original G source/run/component digest. None is formal signing or publication
acceptance.

That new consumer also acquired actual G Core and Manager qualification artifacts
in 19.11 seconds without compiling: Core artifact 13893267008 retained original
cf849ac3 production/run 43090735720 and binary 48c041e4...f4596f; Manager artifact
13891669517 retained G production/run 43104266119 and binary
`0cbc83d22087d0dcc410361195cd48ad52d2c2aa59025f96e23f5f45e45cad40`.
The consumer implementation was 9b88a644; qualified source was detached G.
Acquisition receipt SHA256:
`cc9015cc8176bcd82197368db9336109f96e10a5efc6c255b80e72b576308948`.
The existing exact-main guard remains closed; no formal publication was attempted.

The read-only pre-push audit found no exact-main release-intent-v1, no
automatic Provider publisher and no workflow_run publisher. The existing main
schedule still runs pinned upstream discovery before the guarded release
controller; a new upstream could advance development independently, so remote
HEAD must be rechecked before each push. The independently checked upstream
release was still rust-v0.167.1. No main, stable index, signing key, release,
operating install, account or active user Writer was changed.
