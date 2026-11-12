# Release framework integration — finite implementation record

This is the bounded W0–W20 integration record, not a second product
specification or a new authorization gate. It is retained while final source,
CI and live acceptance are being closed. Stable requirements belong to SPEC
and the scoped annexes; observed proof belongs to CURRENT. After acceptance,
Git history preserves the completed plan and its source provenance.

## Baseline, authority and ownership

Official input: `7b98296a411a2c435bb9f34ee51f286f33f3491b`, upstream
0.167.1 / `0a94398fa60c4fac0e7659e4213a0a36e613dd46`.
One integration worktree and owner-isolated feature worktrees share this
baseline; later focused worktrees start from frozen integration checkpoints. No primary-checkout reset, cleanup or branch switch is allowed.
One Writer owns each overlapping file; transfers are explicit.

**Decision A:** rewrite/rust-core is the one continuing development source.
Main remains separate publication/approval authority: signed indexes and
promotion consume that line. Its unrelated ancestry and existing approval
chain make an unconditional merge into development inappropriate. There is
currently no GitHub branch-protection rule to substitute for the implemented
exact-main authorization gate. Signing and activation stay separate.

The original assumption that e4f9ab5d and plan3ef675ad had no common ancestor
was caused by a shallow repository. The complete history establishes common
ancestors 27103ec0 and 0aebdd7f respectively. Selective patch review is still
necessary because the old checkout pins 0.167.0. It must not replace the
selected official recipe. The initial 0.167.1 input remains historical evidence.
During execution, discovery commit `ec51fe4a` advanced the official branch to
0.167.2 / `ce21072c90d32c40af7a55b60b2fb35c84a43a6a`; merge `795e0cae`
preserves that direct descendant of J. Apply and qualify the combined candidate
on the new exact upstream. Existing S165 remains 0.167.1 and is not rewritten.

## Implementation order

```mermaid
flowchart TD
    W0["W0 source, install and ownership"] --> W1["W1 inputs and metadata"]
    W0 --> N["W15 and W20 Native ownership"]
    N --> E["W15 Runtime execution proof"]
    E --> R
    W0 --> P["W17 authenticated catalog"]
    W1 --> R["W2 and W3 component production"]
    W1 --> V["W5a component revisions"]
    R --> C["W4 Full compatibility"]
    V --> C
    C --> S["W5b publication reservation"]
    P --> T["W19 Provider-scoped model selection"]
    N --> D["W16 terminal display"]
    N --> T
    U["W11–W14 Core interfaces"] --> A["W9 integrated contracts"]
    T --> A
    D --> A
    M["W18 independent MCP shell"] --> A
    S --> A
    A --> Q["W10 component CI, then Full acceptance"]
```

W6 Retain and already-accepted W7/W8 baseline evidence constrain every affected
TUI change. W15's completed operating P0 is not a prerequisite blocking other
work. Publication reservation metadata does not authorize signing/publication.
W20 reuses Core Owner discovery and upstream Native resume; it introduces no
Owner database or alternate Writer controller. The diagram orders source
responsibilities; actual Full admission follows successful same-source Core,
Manager and full-TUI qualification. Runtime execution-proof changes invalidate
Runtime production inputs and require affected consumer qualification again.

For the current W15 correction, Models is the sole assembler/Writer of
native_route.patch and its Core/Protocol/rollout changes. Release has a narrow
lease for the upstream PTY process.rs/pty.rs/pipe.rs hunks and actual-wait tests,
which it hands back to Models; Release also owns the required CI selectors.
Native owns Provider proof parsing, original receipt settlement and unloaded/Stop
observers. Integration owns norms and final assembly. These are existing
execution/receipt paths, not a new executor, history store or approval gate.

## Work packages and evidence gates

| Work | Defect / scope and implemented decision | Writer and principal source | Acceptance still required after focused local proof |
| --- | --- | --- | --- |
| W0 | Correct shallow ancestry, current S165 installation and mixed live generations; preserve dirty source | Integration; Git/process/installed inventory | Final remote HEAD and protected-asset comparison |
| W1 | Input identity follows actual producers, recipe, ABI, flags and lockfiles; qualification is separate | Release; component_inputs.py, component_artifact.py, qualified_component.py | Exact-source artifact/proof receipts |
| W2 | One canonical Android Core producer reused by source and runtime consumers | Release; reusable Core workflow and consumers | Hosted producer/cache plus consumer gates |
| W3 | Manager independent build and interface qualification | Release; manager workflow; Manager source | Hosted exact-source Manager qualification |
| W4 | Unsigned Full assembly binds successful same-source Core/Manager/full-TUI evidence | Release; full_qualification.py, release_control.py and workflow | Exact-source Full run after prerequisites |
| W5 | rN input revisions and composition revision separate from global sequence; immutable signed history | Release; component_revisions.py, release_reservations.py | Pure tests passed; no formal reservation/signing authorized |
| W6 | Preserve upstream TUI behavior; new model/switch paths have explicit Retain oracles | Models/Native; TUI patch and tests | Full hosted TUI tests |
| W7 | Existing current alias is already integrated; retain authority/symlink guards | Integration; current_alias.rs, existing tests | Reuse source tests; no redundant implementation |
| W8 | Baseline 7b98296 TUI success accepted; new changed paths need new exact-source proof | Integration | New full TUI qualification |
| W9 | Stable SPEC, single scoped norms, current evidence and finite handoff | Integration; SPEC and docs | Final links, dependency checks, exact evidence |
| W10 | Read-only qualification separated from exact-main approved signing/publication | Release/Integration; workflows, release_control.py | Safe push and final successful run IDs |
| W11 | Explicit --core/--full, no repeated cancellation, authenticated edition, Core-only build independent of TUI | Integration; main.rs, manual_update.rs, update_edition.rs | Local Core 271/0 passed; hosted CLI acceptance |
| W12 | Offline Core-owned version JSON/details, optional components and legacy unknown revisions | Integration; termux_version.rs, component_metadata.rs, provider.rs | Local Core 271/0 and installed read-only candidate passed |
| W13 | No-bwrap is normal platform information; explicit unsupported requests and real failures remain failures | Integration; Core Doctor path and DOCTOR norm | Local Core/Doctor passed; exact hosted qualification |
| W14 | Signed Core/Full fixtures, rollback, mismatched edition, noninteractive/version coverage | Integration; Core tests | Local signed fixtures/menu passed; exact hosted results |
| W15 | Original P0, verified scopes/reservations and cold receipt continuity; distinguish real OS wait/pre-backend rejection from synthetic exit/Stop/history absence | Native Provider; Models Runtime patch; Release PTY and CI under explicit leases | Warm overlap 5.760s and scope suite pass; Owner correction integrated; actual cold completion and typed producer/consumer/Stop acceptance remain |
| W16 | Exact Owner/home display, default-off auto, short descriptor-transition locks and unsafe-route rejection | Integration/Release after file-lease transfer; Manager terminal/window/task, Provider callback | Actual stock Android 66edd08d success; final source/device boundary retained |
| W17 | Fresh authenticated ChatGPT catalog, independent inference auth, stable stored Provider/model IDs | Models; models/control/projection and Provider docs | Candidate hosted tests; reuse operating live model loop only within stated boundary |
| W18 | One independent shell, original-call replay, conflicts, bounded output/process cleanup; common annotations unchanged | Integration; termux_shell, worker, candidate.py, connector refresh and qualification | Standalone candidate worker and actual connected replay during bounded Native Busy passed; final-source Provider CI remains |
| W19 | OpenAI upstream-only; ChatGPT account catalog-only through picker/direct input/reentry/effort; preserve upstream command-name completion | Models; termux_chat_models, input preparation, TUI patch | Candidate full TUI, provider transitions and unsupported-model errors |
| W20 | Decode legacy owners safely; verify actual execution-home Writer locks; preserve current Native selection/thread; consume initial intent once, fence Native children and reject unsupported private handoffs | Native/Models; runtime-binding/native, profile_transport, profile_manager, app_server_session and quota replay | Owner correction154 affected tests/readbacks pass, including Manager139; provisional J same-thread switch and old-d2/S165 safe refusals passed; original d2 regression preserved; final Runtime/composition and hosted19 named gates remain |

## Source provenance and independent review

- e4f9ab5dc3045cbde8317774da95eb98db7646dd: original Native continuation
  claim repair; compared against the current producer and integrated with
  origin attribution. No operating recovery was repeated.
- c0a8189815c70f0c6678bc9374bcd352423dca57: independent MCP worker; direct
  descendant patch integrated, then actual independent-process replay and
  lifecycle tests added.
- 3ef675ad721c0a08a50891394cf03df3abb66b2d: planning evidence revalidated
  against real producers; its approval gate and 0.167.0 assumptions are not
  carried forward.
- 9a236db89a3778a1d28a488d093ddee9943759d3 and parent089ec7f6: prior P0
  recovery and janmori101 connector-refresh evidence folded into CURRENT
  and scoped Provider guidance; no duplicate dated handoff is created.
- a55d5bb8 Native integration, 0aebdd7f documentation baseline, PR #29 and
  2dd07d42 account-draft ancestry are already integrated. Preserve actual
  current consumers instead of replaying obsolete files.

Worker commits retain cherry-pick origin attribution. The integration branch
is the reviewable source; individual work branches are temporary and cannot
substitute for final official-source integration.

## Tests and proof reuse

The three baseline 7b98296 CI receipts, earlier P0 repair receipts, original
15 claim tests and successful connector refresh are reusable only for their
original source/account/device boundaries. New effects, owner parsing,
catalog selection, terminal display, metadata consumers and CI producers are
retested. No assertion is weakened and no required gate is deleted or skipped.

Local TUI compilation lacks the pinned rusty_v8 dependency. Hosted G full TUI
passed focused/Native/API and every mandatory Retain gate, including the corrected
pin refresh and compaction tests. Its success does not qualify J's later typed
reentry tests or subsequent Runtime changes. Actual candidate PTY switch and
provider-model continuity with synthetic responses passed separately. Old-runtime
compiler and incomplete earlier switch trials remain original failure evidence,
not accepted new behavior. Existing Stop/empty-jobs tests alone cannot certify
actual process termination; the new Runtime proof must pass positive and negative
consumer cases before final availability acceptance.

Live model evidence used the operating Provider and one original Chat/Native
thread, preserving approval and sandbox configuration. Synthetic account
fixtures are separate. Android stock tests prove Activity/session behavior on
the pinned emulator APK with synthetic Core transport and frontend; they do not prove a
physical device or real model inference.

The first W1 recheck at 26952f73 measured one conservative cache miss: e961f25c-to-26952f73 changed only Manager's dedicated
profile test formatting, which changed Manager's declared input identity while
Core's remained identical. The audited package-tree inventory is retained; it is
not described as the minimal activated Cargo graph. This may consume a new rN for
identical output bytes. No incomplete test exclusion or alternate dependency
resolver is introduced. Audit SHA256:
`32b8a5971c0e0e44409b1d70e995f010721b272999db281c4b17f4eb20e2d207`.
Its no-missing-input conclusion was incomplete: the real 0.167.2 transition
exposed release-builder::local_recipe directly embedding release/upstream.json
inside Core while the inventory excluded it. Source ea5f5e55 adds that blob to
Core only and proves stale-artifact refusal; Manager's recipe independence stays
intact. Two original red cases, three focused green cases, fifteen component/
consumer passes and three current-descriptor passes are retained in CURRENT.
The producer-helper change is itself an input and requires fresh qualification.

## Change accounting and rollback

The original 1,370–3,020 product and 500–800 documentation LoC range was a
pre-investigation estimate for W0–W14, not a commitment or completed count.
Original W16 estimates and the 559-line shell patch are not added again to
actual totals. Final accounting is computed once from baseline to integrated
HEAD, with product, test, CI/support and documentation categories. Embedded
upstream patches additionally report semantic Rust lines so patch wrappers
are not mislabeled as product code. Pure moves and overlapping W assignments
are counted once. The final file inventory and actual counts are updated
after source acceptance.

### W20 measured subset

At integration source `6450e0f7`, non-overlapping W20 semantic changes cover
12 logical files: product **+165 / -35**, tests **+465 / -0**, and scoped
norms **+22 / -1**. The Owner/IPC portion is product +159/-35, tests +321/-0;
upstream historical-quota replay and checkpoint acceptance add product +6/-0
and tests +144/-0. Those expanded upstream hunks are already in the TUI patch
total. W15 hook/claim machinery, W16 terminal display, W19 catalogs, shared
fixture publication and global W9/W10 bookkeeping are not counted again here.
The later current-selection correction f915344e/f1491afb adds product
+88/-4 and tests +324/-0, including semantic app_server_session Rust and
the no-resume readback assertion. These are measured additional deltas, not
an independent total to add to the global patch accounting. The final subset
reconciles the file union and the updated ownership norm after source freeze.
Replacement lines are included in additions/deletions, not extra LoC.

Rollback for development is a reviewed revert of a coherent feature commit,
followed by affected exact-source qualification. No rollback here mutates a
signed generation, active pointer, account, original result or Writer.
A failed publication proof prevents signing rather than changing already
issued S164/S165. Full-to-Core tests preserve previous Full and enforce
anti-rollback sequence rules; S164 cannot silently replace newer S165.

## Branch disposition

The following inventory comes from all observed local/remote refs after full
ancestry recovery. Worktree presence is an ownership constraint, not proof
that its process is idle. No branch or worktree is deleted. Integrated refs
can be retired only after their active owner/worktree and CI/release/PR
consumers are released; publication/rollback/evidence refs remain preserved.
Unique operating snapshots and experiments are not wholesale merge inputs.

| Ref | Observed SHA | Disposition | Worktree present |
| --- | --- | --- | --- |
| `local:archive/linux-app-operating-source-20261106` | `97118e775bc6` | other owner/operating snapshot: preserve | yes |
| `local:archive/mobile-desktop-attempt-20261106` | `8de07907b110` | other owner/operating snapshot: preserve | no |
| `local:backup/user-dirty-20261109` | `0401e4af6207` | other owner/operating snapshot: preserve | no |
| `local:backup/user-dirty-20261109-base` | `0814be7b51dd` | integrated legacy; check consumers before deletion | no |
| `local:design/entry-architecture-20261105` | `604df76bb30f` | integrated; retire after owner/worktree release | yes |
| `local:docs/release-framework-handoff-20261111` | `9a236db89a37` | evidence selected; retain owner worktree | yes |
| `local:docs/review-actions-20261110` | `e16bb12defc9` | integrated; retire after owner/worktree release | yes |
| `local:draft/thread-model-catalog-20261106` | `8d095cd754fa` | unique historical experiment: preserve, no automatic merge | no |
| `local:experiment/profile-tui` | `c4d9a41d3827` | integrated legacy; check consumers before deletion | no |
| `local:experiment/web-provider` | `6aab6cd9c8e3` | unique historical experiment: preserve, no automatic merge | no |
| `local:experiment/web-termux` | `08e985693f0a` | unique historical experiment: preserve, no automatic merge | no |
| `local:feat/termux-mcp-shell-20261111` | `c0a8189815c7` | patch integrated; retain owner worktree | yes |
| `local:fix/accepted-caller-repin-20261111` | `c642dd208735` | publication authority/history: preserve | yes |
| `local:fix/active-steer-20261104` | `f8d1a7f47c62` | integrated; retire after owner/worktree release | yes |
| `local:fix/attached-native-development-loop-20261106` | `cc535d18a897` | integrated; retire after owner/worktree release | yes |
| `local:fix/chatgpt-resume-concurrency-20261108` | `f93256083902` | integrated; retire after owner/worktree release | yes |
| `local:fix/current-generation-compat-link-20261111` | `a8cb66c6f2b0` | integrated; retire after owner/worktree release | yes |
| `local:fix/hosted-preflight-0161` | `ce371e090f86` | unique historical experiment: preserve, no automatic merge | no |
| `local:fix/mobile-desktop-20261106` | `8de07907b110` | other owner/operating snapshot: preserve | yes |
| `local:fix/native-route-transition-20261105` | `987dee84a5c3` | other owner/operating snapshot: preserve | yes |
| `local:fix/native-startup-settlement-20261108` | `317c7fe41d15` | integrated; retire after owner/worktree release | yes |
| `local:fix/p0-413-native-repair` | `0902eb69b319` | integrated legacy; check consumers before deletion | no |
| `local:fix/p0-stream-resume-identity` | `49e292b011e0` | integrated; retire after owner/worktree release | yes |
| `local:fix/release-notify-admission-20261111` | `c19b61bb3d1d` | integrated; retire after owner/worktree release | yes |
| `local:handoff/pro-local-baseline-20261106` | `b8c14252ee33` | other owner/operating snapshot: preserve | yes |
| `local:integration/codex-native-20261110` | `a55d5bb8fffa` | integrated; retire after owner/worktree release | yes |
| `local:integration/release-framework-20261112` | `66edd08d3487` | current isolated integration writer | yes |
| `local:legacy/monolith` | `bf30a7dc94d4` | release/compatibility: preserve | no |
| `local:main` | `53176ed79097` | publication authority/history: preserve | yes |
| `local:plan/release-framework-20261111` | `3ef675ad721c` | evidence selected; retain owner worktree | yes |
| `local:prep/0.164-native-review` | `27a918678644` | integrated; retire after owner/worktree release | yes |
| `local:prep/pro-baseline-20261106` | `6018ad990f14` | unique historical experiment: preserve, no automatic merge | no |
| `local:prod/download-size-v1-base-9300` | `9300a68852c8` | release/compatibility: preserve | no |
| `local:refactor/docs-contract-20261110` | `0aebdd7f2614` | integrated; retire after owner/worktree release | yes |
| `local:refactor/naming-and-delivery-rules` | `306d842f86aa` | integrated legacy; check consumers before deletion | no |
| `local:release/frozen-full-1670-caller` | `6624418979e1` | publication authority/history: preserve | yes |
| `local:release/frozen-full-1670-s163` | `d6f090e835a7` | release/compatibility: preserve | yes |
| `local:rewrite/rust-core` | `e4f9ab5dc304` | patch integrated; retain owner worktree | yes |
| `local:tdev-work/AUTHORITY-COMPACTION-V1-88d33545bf5e4bc6b24ebaeb6962ee5b` | `838c8ba7e773` | other owner/operating snapshot: preserve | no |
| `local:tdev-work/RUNTIME-ALIGNMENT-0-159-2-AUTHORITY-560ff9af76714c9baee22584368935f3` | `019f2066cfcb` | other owner/operating snapshot: preserve | no |
| `local:tdev-work/release-download-size-v1-a0dce69bd40e43f7b22f4d303b70efa5` | `ba0eacc4a077` | other owner/operating snapshot: preserve | no |
| `local:tdev-work/update-download-size-v1-9fcae140207f4608b7268cd77f0146f5` | `4fd14ed8aafc` | integrated legacy; check consumers before deletion | no |
| `local:wip/chatgpt-accounts-source-20261110` | `2dd07d422ad0` | integrated; retire after owner/worktree release | yes |
| `local:work/native-owner-20261112` | `90faa66a7a9a` | current isolated integration writer | yes |
| `local:work/provider-models-20261112` | `342dca6490b7` | current isolated integration writer | yes |
| `local:work/release-framework-20261112` | `68abe9a0486a` | current isolated integration writer | yes |
| `remote:origin/HEAD` | `b2567063d2a7` | publication authority/history: preserve | no |
| `remote:origin/archive/linux-app-operating-source-20261106` | `97118e775bc6` | other owner/operating snapshot: preserve | no |
| `remote:origin/archive/mobile-desktop-attempt-20261106` | `8de07907b110` | other owner/operating snapshot: preserve | no |
| `remote:origin/design/entry-architecture-20261105` | `604df76bb30f` | integrated legacy; check consumers before deletion | no |
| `remote:origin/docs/release-framework-handoff-20261111` | `9a236db89a37` | evidence selected; retain owner worktree | no |
| `remote:origin/draft/thread-model-catalog-20261106` | `8d095cd754fa` | unique historical experiment: preserve, no automatic merge | no |
| `remote:origin/experiment/profile-tui` | `2c9ec49685aa` | integrated legacy; check consumers before deletion | no |
| `remote:origin/experiment/stock-window-proof` | `ee28c904cb55` | unique historical experiment: preserve, no automatic merge | no |
| `remote:origin/experiment/web-provider` | `6aab6cd9c8e3` | unique historical experiment: preserve, no automatic merge | no |
| `remote:origin/experiment/web-termux` | `08e985693f0a` | unique historical experiment: preserve, no automatic merge | no |
| `remote:origin/fix/active-steer-20261104` | `f8d1a7f47c62` | integrated legacy; check consumers before deletion | no |
| `remote:origin/fix/attached-native-development-loop-20261106` | `cc535d18a897` | integrated legacy; check consumers before deletion | no |
| `remote:origin/fix/chatgpt-resume-concurrency-20261108` | `f93256083902` | integrated legacy; check consumers before deletion | no |
| `remote:origin/fix/current-generation-compat-link-20261111` | `a8cb66c6f2b0` | integrated legacy; check consumers before deletion | no |
| `remote:origin/fix/hosted-preflight-0161` | `ce371e090f86` | unique historical experiment: preserve, no automatic merge | no |
| `remote:origin/fix/native-route-transition-20261105` | `987dee84a5c3` | unique historical experiment: preserve, no automatic merge | no |
| `remote:origin/fix/p0-413-native-repair` | `0902eb69b319` | integrated legacy; check consumers before deletion | no |
| `remote:origin/fix/release-notify-admission-20261111` | `c19b61bb3d1d` | integrated legacy; check consumers before deletion | no |
| `remote:origin/fix/stopped-native-shared-claim-reconciliation` | `fec19d8e2c9a` | integrated legacy; check consumers before deletion | no |
| `remote:origin/handoff/pro-local-baseline-20261106` | `b8c14252ee33` | unique historical experiment: preserve, no automatic merge | no |
| `remote:origin/integration/codex-native-20261110` | `a55d5bb8fffa` | integrated legacy; check consumers before deletion | no |
| `remote:origin/legacy/monolith` | `bf30a7dc94d4` | release/compatibility: preserve | no |
| `remote:origin/lupi/review-20261109` | `53176ed79097` | publication authority/history: preserve | no |
| `remote:origin/main` | `b2567063d2a7` | publication authority/history: preserve | no |
| `remote:origin/prep/0.164-native-review` | `27a918678644` | integrated legacy; check consumers before deletion | no |
| `remote:origin/prep/pro-baseline-20261106` | `6018ad990f14` | unique historical experiment: preserve, no automatic merge | no |
| `remote:origin/prod/download-size-v1-seq23-source` | `b164e5b61cb4` | release/compatibility: preserve | no |
| `remote:origin/release/frozen-full-1670-s163` | `d6f090e835a7` | release/compatibility: preserve | no |
| `remote:origin/release/seq18-no-emoji-product` | `9300a68852c8` | unique historical experiment: preserve, no automatic merge | no |
| `remote:origin/rewrite/rust-core` | `66edd08d3487` | official development | no |
| `remote:origin/wip/chatgpt-accounts-source-20261110` | `2dd07d422ad0` | integrated legacy; check consumers before deletion | no |
| `remote:origin/work/capability-restoration-166` | `3c1e47e46cef` | integrated legacy; eligible after acceptance/consumer review | no |

The checkpoint inventory at 66edd08d contains 94 local/remote refs and 83 registered
worktrees. The 16 orphaned transfer refs below were also reviewed: 15 are direct
ancestors; native-result29's two commits have exact stable-patch equivalents
141c0e71/5056cc30 already in development. Their names/SHAs have no current
main/development CI, release or entry-guide consumer and no open PR. The three
evidence worktrees remain protected, including recovery28's other-owner dirty
files (already integrated blobs, not missing product changes).
Local tag names remain 166 with no additions/deletions in this task; remote
names are 257 (258 raw lines include one peeled entry), with zero common-name
SHA mismatches. These are different inventories, not evidence of tag removal.

| Transfer ref | Observed SHA | Disposition | Evidence worktree |
| --- | --- | --- | --- |
| `remote:integration-transfer/acceptance-final` | `3140f7fc3239` | integrated transfer; retire after evidence-owner review | no |
| `remote:integration-transfer/barrier31` | `ea04c19208b0` | integrated transfer; retire after evidence-owner review | no |
| `remote:integration-transfer/codegate-v2` | `81049caeb05b` | integrated transfer; retire after evidence-owner review | no |
| `remote:integration-transfer/discovery29` | `0c5a70523f7a` | integrated transfer; retire after evidence-owner review | no |
| `remote:integration-transfer/fixture30` | `7ef80be546c6` | integrated transfer; retire after evidence-owner review | no |
| `remote:integration-transfer/gates31` | `12129a5ed37e` | integrated transfer; retire after evidence-owner review | no |
| `remote:integration-transfer/native-result29` | `fa6b46789fed` | patch-equivalent changes integrated; preserve evidence worktree | yes |
| `remote:integration-transfer/package-fixture26` | `e6f50f2977e1` | integrated transfer; retire after evidence-owner review | no |
| `remote:integration-transfer/preflight-20261110` | `f6dff75b72d0` | integrated transfer; retire after evidence-owner review | no |
| `remote:integration-transfer/provider-final` | `495c5213160e` | integrated transfer; retire after evidence-owner review | no |
| `remote:integration-transfer/qualification30` | `e0cca4c2d695` | other owner/evidence worktree: preserve | yes |
| `remote:integration-transfer/recovery28` | `5c9334b6aa53` | other owner/evidence worktree: preserve | yes |
| `remote:integration-transfer/result29` | `94e09c845b84` | integrated transfer; retire after evidence-owner review | no |
| `remote:integration-transfer/runtime-final` | `3c043bdd7e80` | integrated transfer; retire after evidence-owner review | no |
| `remote:integration-transfer/tui-proof-final` | `392b026f02a4` | integrated transfer; retire after evidence-owner review | no |
| `remote:integration-transfer/validation-final` | `18b90a5b34c8` | integrated transfer; retire after evidence-owner review | no |

### Retirement prerequisites

An additional consumer scan covered current workflows, scripts, release inputs
and user/agent entry guides. No open PR was present at this read. Ref aliases
sharing an owned worktree SHA/name remain subject to that owner's release,
even when the remote-tracking ref itself has no worktree. The old integration
name in the TUI artifact test is an intentional negative branch-admission
fixture; README now points only to rewrite/rust-core.

| Integrated inactive ref | Remaining retirement condition |
| --- | --- |
| `local:backup/user-dirty-20261109-base` | preserve other owner |
| `local:experiment/profile-tui` | eligible after final source acceptance no open pr |
| `local:fix/p0-413-native-repair` | eligible after final source acceptance no open pr |
| `local:refactor/naming-and-delivery-rules` | eligible after final source acceptance no open pr |
| `local:tdev-work/update-download-size-v1-9fcae140207f4608b7268cd77f0146f5` | eligible after final source acceptance no open pr |
| `remote:origin/design/entry-architecture-20261105` | after related worktree owner release |
| `remote:origin/experiment/profile-tui` | eligible after final source acceptance no open pr |
| `remote:origin/fix/active-steer-20261104` | after related worktree owner release |
| `remote:origin/fix/attached-native-development-loop-20261106` | after related worktree owner release |
| `remote:origin/fix/chatgpt-resume-concurrency-20261108` | after related worktree owner release |
| `remote:origin/fix/current-generation-compat-link-20261111` | after related worktree owner release |
| `remote:origin/fix/p0-413-native-repair` | eligible after final source acceptance no open pr |
| `remote:origin/fix/release-notify-admission-20261111` | after related worktree owner release |
| `remote:origin/fix/stopped-native-shared-claim-reconciliation` | eligible after final source acceptance no open pr |
| `remote:origin/integration/codex-native-20261110` | after related worktree owner release |
| `remote:origin/prep/0.164-native-review` | after related worktree owner release |
| `remote:origin/wip/chatgpt-accounts-source-20261110` | preserve other owner |

No deletion is performed during source acceptance. Release/rollback and other-owner snapshots remain preserved.
