# Current product state

Updated: 2026-11-12 UTC. Observed facts and proof limits only.
Requirements: [SPEC](SPEC.md) and [functional annexes](docs/spec/INDEX.md).
Unfinished acceptance: [HANDOFF](HANDOFF.md).
The [finite integration record](docs/development/RELEASE_FRAMEWORK_IMPLEMENTATION.md)
owns the W-task DAG, file Writers, provenance and change accounting.

## Source and publication authority

The sole continuing development branch is
[rewrite/rust-core](https://github.com/humtr/codex/tree/rewrite/rust-core).
The last fully enumerated qualification checkpoint before independent handoff
was `1f1f5cffa26bccf27428132e7482c1a0e959fbf4`, pushed and read back on
November 12 at 05:00 UTC. Follow-up starts from its local descendant
`578a386b88ef0cfecb268c439c485b650f457970`; a new exact-source
qualification must bind the subsequently integrated development HEAD. The isolated integration
worktree starts from `7b98296a411a2c435bb9f34ee51f286f33f3491b`;
the primary checkout remains `e4f9ab5d` on its older 0.167.0 lineage.

Remote main remains `b2567063d2a7a333a718d2495e2c9f8c928d822c`, the
separate publication/approval authority. Neither branch has GitHub branch
protection. The exact-main release-intent read returned HTTP 404 before this
push. No formal signing, sequence reservation, publication, stable-index
change or operating activation was performed.

The official recipe now selects Codex **0.167.2**, upstream
`ce21072c90d32c40af7a55b60b2fb35c84a43a6a`. Official discovery
`ec51fe4a5f5153a441cf78b1e8ce108050274e80` advanced the development
branch at 00:48:38 UTC; merge `795e0cae` preserves that valid descendant.
The user's original **0.167.1** baseline and the operating **Full S165 0.167.1**
remain distinct from the new recipe. No signed generation was rewritten.

Runtime producer `1e203874` and Provider consumer `d4028958` are the
current process-tree and versioned-ingress pair. Both basic Runtime and separate
Enhanced TUI source preparation passed against the exact upstream. The normalized
lock preserves 1,308 external identities and updates 171 workspace versions;
SHA256 `5e1781c98407bf0cbf67bb39a2dffffa6d2a259871a47fe99946de22c716a782`.
Source assembly is not hosted compilation or actual Native execution.

The subsequent changes are test-only: `34ac9668` retains an original private
Native fixture for inspectable proof; `785fe070` uses the TOML document parser
for a document fixture; `5857e515` distinguishes one Guardian inference POST
from the existing pool's two WebSocket preparation GETs. All original permission,
selection, request-count and no-Native-authority assertions are preserved.
The updated Guardian fixture and all mandatory 53 Native checks passed in the
complete 1f Core qualification, including the unchanged full PTY/rollout suites.
Enhanced TUI and actual Runtime/Provider joint acceptance remain separate.
Raw build or Android-smoke success alone cannot qualify either boundary.

Complete history corrected the initial shallow-repository ancestry assumption.
The old Native and planning lines have common ancestors with development;
main has separate ancestry. PR #29, a55d5bb8 Native integration and the account
draft through 8f97affc were already integrated. Selective source reconciliation
preserved the newer recipe and other owners' files.

At the completed 05:24–05:30 UTC branch audit, there were 114 local/tracking
refs, 99 worktrees, 45 actual remote heads, 166 local and 257 remote tag names,
zero common-name tag mismatches and zero open PRs. Audit SHA256:
`d47b958c7d81c81cc52d104412e6d7b00f0761eac7745893f9633bca2223277e`.
The finite integration record owns complete ref dispositions. Immediate deletion
eligibility and actual branch/worktree/tag deletion were both zero.

Superseded checkpoints, original failed iterations and their receipts remain in
[the immutable pre-consolidation record](https://github.com/humtr/codex/blob/1f1f5cffa26bccf27428132e7482c1a0e959fbf4/CURRENT.md).
This includes the recovered original calls from the temporary bridge outage;
no unresolved effect was replayed under a new identity.

## Independently observed operating installation

| Observation | Verified value and boundary |
| --- | --- |
| Active signed generation | **Full S165**, full-v0.167.1-7b98296-s165, Codex **0.167.1** |
| Retained rollback | full-v0.167.0-c19b61b-s163 |
| Core | 74c9ef705d46a1c9a35711e8cb7a64adc107a60849994aa3ce324dc2063cdd55, core-api-v1 |
| Runtime | ef50757ebe89bf672ab2bb8a6fa875362f7882ba18537ed9bf64196e54bf3d29 |
| Upstream archive | ef3e1443a7795d596f1601a533d7225a1030488d60c965af1f0610ab95889031 |
| Manager / Enhanced TUI | Present and digest-verified in S165 |
| Selected signed Provider package | chatgpt-da5bf622bc49-s25, independently signature-verified |
| Core S164 | Signed reference exists; no S164 generation directory installed on this device |
| Legacy component revisions | Absent in S165 metadata; null/unavailable, not invented from sequence 165 |

S165's existing Ed25519 signature was independently verified at 04:20:50 UTC,
using byte-exact private copies, the existing public key and the same OpenSSL
verification path as Core: exit 0, `Signature Verified Successfully`. Manifest
SHA256 is `bc45846a8367f051ad482e80e745355937d3a653664c6adfb37c316077d3fb14`;
signature SHA256 is `44f6a2abc309c0865c4d07d6d6b1fa5ddaef3eee49308ae337805449042f34c0`.
The public key matched activation's trusted key. Original inode/mode/hash/mtime,
current pointer and activation were unchanged; private-key reads, signing and
operating execution were zero. This does not verify absent S164 package bytes.
Receipt: `4e5d0f872e076096a5eac686057261fc96db3fc0eb9fc05d6beb743a4f10ac9c`.

Candidate Core cdca2946eaf840323f432b925fb7327662887ceae5e05a22ae82fa3829c0378a
read the installation with offline `termux version --json`, exit 0 and empty
stderr, without activation. Sixteen protected generation files and the active
pointer were identical before/after. A later Core-only version check preserved
20 files plus the public Core/pointer and returned exactly codex-cli 0.167.1.
The installed Core's earlier `codex termux version` returned invalid command;
a source implementation is not an installed feature.

A read-only inventory found S159, S163 and S165 runtime processes concurrently
(19, 16 and 5 including children/servers, not window counts). Older frontends
can inherit the current public Core path. Coexistence does not prove every
combination compatible or authorize stopping these processes.

## Release, CLI and qualification implementation

Core plus a qualified Runtime is the minimum product. Manager, Enhanced TUI and
Provider are optional components with separate production/qualification and
interface responsibilities. Core-only builds, updates and basic UI do not require
a same-version optional frontend. Full preserves its composition gate. Its current
six qualification inputs cover Core/Runtime, Manager and Enhanced TUI. The new
Full revision map selects Manager/TUI together; independent production and
qualification do not prove a new Manager-only or TUI-only installer. Legacy
signed seven/eight-file Full inventories retain their original interpretation. Provider
uses its independent signed package/installer; selected-Provider updates do not
install an unselected Provider. The repository-owned tunnel recipe remains an
explicit source-build path and is not included in the Full published inventory.
The ownership annex is aligned with the requested Core/Full contract; it no
longer treats a Full qualification as proof of implicit Provider/tunnel bundling.
Independent producer/consumer review: `88ad3e8b1f2b2ee4586e02a8eb4046c6499281a102e29e4db4e984d38d1d7f18`.

Integrated source includes component input identities, canonical Android Core
production/reuse, independent Manager qualification, unsigned Full assembly,
component rN, composition identity and globally reserved Core/Full publication
sequence. Legacy signed readers and rollback remain supported. The conservative
input inventory includes selected package trees and Cargo.lock dependency edges,
but is not a minimal activated compile graph. A Manager test-only change can
cause an avoidable cache miss/rN change without changing bytes; it does not
invalidate every component. The earlier audit
`32b8a5971c0e0e44409b1d70e995f010721b272999db281c4b17f4eb20e2d207`
missed Core's direct embedding of release/upstream.json through release-builder.
The 0.167.2 transition reproduced both an unchanged Core fingerprint and wrongful
acceptance of the old artifact. The integrated ea5f5e55 fix includes that exact
blob for Core; Manager remains independent of this recipe change. Two original
red cases became three focused passes, the component/consumer suite passed
fifteen tests, and the current 0.167.2 descriptor passed three additional cases.
The bounded external-include/build-script audit found no further omission in
Core, release-builder or runtime-binding. Receipt
`6ffd3098733ad271d81e6a2edcf4657f3caae3393ba11512df0a467f2f15ae76`;
audit `917598401dd7cbd126a68229c3ba180ed1bec55efa19142ae87b3c2647c38625`.
The changed producer helper itself is also a declared input; final qualifications
use its new identity rather than claiming every older artifact is reusable.

Core owns offline version/details/JSON, explicit Core/Full update selection,
one cancel choice and no-bwrap platform classification. Explicit unsupported
sandbox, approval/authentication/Native failures and data mismatches remain
visible. Base-format compatibility, generation identity and rollback are preserved.
The complete Core suite at d36ca465 passed 271 tests, zero failed, with one
existing explicitly gated Termux smoke ignored. Whole-workspace strict clippy
passed. Actual 891c6595 PTY update keep/remote cases exited 0 with one cancel,
zero/one local request, released waiting lease and unchanged activation;
external build requests were zero.

Full consumers require exact-source Core/Manager/full-TUI qualifications and
retain the original six-artifact input plan. Each immutable artifact ID/run/source,
ZIP digest/size and closed regular-file inventory is verified. Qualification is
not raw production, signing or publication. At f9d43ee2, 67 distinct Full,
input/controller/reservation/intent and component tests, thirteen workflow tests
and ten documentation tests passed. The actual provisional primitive probes
passed exact stdout/empty stderr after private HOME/TMPDIR isolation; the earlier
missing-HOME and temporary-root failures remain preserved.

| Actual evidence | Scope / receipt SHA256 |
| --- | --- |
| Immutable G Core/Manager acquisition | 19.11 seconds, no compilation; cc9015cc8176bcd82197368db9336109f96e10a5efc6c255b80e72b576308948 |
| Provisional cf Core/Runtime + 66 Manager + d2 TUI probes | Four real probes, disposable test signing only; 275ef701761bceeb3b2f0d1c4d0d823b87b848cae0a9ccd6b64c07b9ceb2aee6 |

Core artifact 13893267008 retained original cf849ac3 production/run 43090735720
and binary 48c041e4…f4596f. Manager artifact 13891669517 retained G production/run
43104266119 and binary 0cbc83d22087d0dcc410361195cd48ad52d2c2aa59025f96e23f5f45e45cad40.
Source Acceptance independently restored/reverified canonical Core bytes instead
of producing another copy. The final reservation still binds actual candidate
payloads and normalized composition before any separately authorized signing.

## Native Owner and switch

The installed Owner-query failure was reproduced read-only. A known legacy
record naming an old runtime was decoded as a current Binding before checking
its PID, which had been reused by a proot loader. That unrelated registration
poisoned lookup. The correction excludes only proven retired/reused known
records; unknown codecs, malformed current records, unsafe paths/peers and
ambiguous Writers remain errors. No operating registry was repaired.

A subsequent actual compiler trial exposed a second defect: Owner proof assumed
the installation's default writer-lock namespace and missed an independent
CODEX_HOME Writer. The ff5b41d0 correction, integrated as baab4437 with a separate
format-only f06fd10e commit, checks actual admitted execution homes, exact
Core-produced shared aliases and safely decoded retired namespaces.
Unknown held Writers remain unresolved even if another valid Owner exists.
The original profile-switch deadline was retained.

Affected tests passed **154**: runtime-binding seven, Manager 139, Core one and
Provider discovery seven (one existing helper ignored). Strict Core/Manager/
runtime-binding all-target clippy and both workspace/Provider format checks passed.
Original fixture/format failures are retained, not counted as passing evidence.

Candidate Core SHA256
31fc3fad74270f6a5560f2b071eaa9205a3d6d4beef0a5e5bc1fa32be4bcbb20
returned null for completed compiler A and the correct independent home/socket
for active B. It also returned safe null for the unused operating test UUID and
correctly found a separately observed live S163 Owner in wrlab; its PID/start,
installed Core and S165 pointer remained unchanged.

| Evidence | Receipt SHA256 |
| --- | --- |
| Owner source/tests | 7247395ebf4a442378a0894a8c7ceccd205c50fd0e08357464e1dbdd0cfc329b |
| Candidate A/B readback | 8f78238e7a2a76eeec482775d31d6b04b5359838e06a0f67e086d85541ffa94a |
| Operating read-only lookup | bcb75dcdba07cf63006d16cdcc2aa9957f8c85581eaa21de5dbf7773dbc8d623 |

The observed live Owner was not directly mapped to the user's original error
screen. That exact old TUI PID/thread remains unproven; do not substitute a
different live thread as evidence for it. The 03:35 UTC read-only follow-up
identified 42 same-UID generation processes across S159/S163/S165. The five
exact home/log paths were absent, and no private regular log descriptor was
open in those processes. No pipe/socket or conversation body was read. Receipts:
`c3462b34980c4b08b69cfa9305f95a9883e41173daf7701c6ee7c3f7c738190b` and
`bba42944f1ca53aabf5ca9a0e090e20ed64f8c48bedf508fb17937d9a2b783cc`.

Actual staged same-thread profile-switch tests passed with synthetic inference.
The later cf/66/d2 task qualifier passed 40 assertions, including active/
disconnected Writer preservation and cross-home refusal. Automatic quota
handoffs passed ordinary/named-permission cases and refused an incompatible
profile before destination inference. A manual auto-OFF picker trial verified
the target Owner/home before follow-up and preserved OpenAI/gpt-6.1-sol/high,
permissions and one original tool effect:
744545926ef31dd2b1775643e88c315bf7b66baf2829f08ad0d3139c9af95e1c.

A separate old-d2 changed-selection trial reproduced high reverting to the
Native enum string none after manual switching. Same thread, Provider, model
and tier survived; only two original synthetic responses occurred, with no
destination follow-up. Original failed receipt:
02d20ac216a71f5e0d224731fc1f700edbcae58f6817e7833501e0d35001ec73.

The integrated correction captures current selection, uses a typed same-PID
initial reentry intent before threads/dispatch and removes its mode/PID from
all five Core Runtime child producers. Initial reentry restores saved
Provider/model/effort/tier; later ordinary resume preserves its explicit choices.
Unbound legacy private profile/quota handoffs now fail before Manager mutation;
the old TUI may already have exited normally. Public profile/CLI semantics remain.
Seven actual process-environment cases and focused source tests passed; the
latest hosted TUI gate requires nineteen named positives. A subsequent private
composition using J's production TUI, corrected Owner Core and 4fb23c69 Provider
passed the changed-current-selection trial, exit 0. The same thread retained
ChatGPT/gpt-6/high/default despite the destination's OpenAI/low/priority defaults.
Four synthetic responses produced two real Native commands: distinct original
call IDs, one effect each, correct stdout/exit 0 and no reentry mode/PID in either
child. Withdrawing the model made no fifth request and preserved draft/history/
selection. Installed Core, activation and S165 pointer were unchanged.
Receipt: 0acccfe90f9ec25cd96cd7aea466f7fc58c1d45f04caf5f4466ef6560bf43a45.
This is provisional composition evidence, not full J Qualification, live account
switching or final Runtime acceptance. A separate new d2 legacy fixture then
verified unsupported-generation refusal before destination Owner or inference.
Its entire original canonical rollout hash and ChatGPT/high/default selection
remained unchanged; two synthetic responses, zero live responses. The old UI may
already have exited normally, so this does not claim an active old Writer survived.
Receipt: 6615198335d2f62bf56bbe07e503f212e445d0a63ae2d9a669db4e87a246a798.
The first negative fixture's raw-versus-adapted ELF assertion failed before
switching and is preserved separately.

The separate signed-S165 read-copy trial also passed, exit 0. Its real TUI
opened `/switch` and the target picker; the guarded candidate Manager refused
the unsupported private reentry before any destination Owner or inference. The
original rollout bytes and OpenAI/model/effort/tier selection were unchanged.
The executed ELF, private generation helper, metadata and original S165 copy
all matched `7a8c732b9bd5b651818b4bde07b3b207e1ae1e1a47eacccf18e4a6acfb1f6b50`.
This mixed composition used one synthetic response, no live model/account or
physical Activity, and no old fixture thread. It does not certify final Full.
Original signed files, operating Core/activation/current remained unchanged;
owned Runtime PID30746 exited. A loopback client-disconnect BrokenPipe is
retained in the raw log, not hidden as an extra successful request. Summary
`0513bac71065c1b4d083e56244c85746236530f21e36c1912f18eae9652f8bc7`;
result `2c42c097693a164faab36302fc866bea30419b13d79a55d3a33469608423c9e2`.

At 03:13:52 UTC the protected readback again matched all sixteen baseline
files byte-for-byte and preserved their modes, sizes and mtimes: activation,
public Core, keys and S163/S165 signed generation metadata/executables. S165's
current pointer, the primary e4f9ab5d HEAD and its exact three untracked entries
also remained unchanged. No operating command or mutation was needed.
Receipt: `fc35769aad077b2d54aebcf35ab6c27945a0915e927bf80c86865d5d5810ebde`.

## Native effects, recovery and parallel availability

The earlier P0 is already operationally recovered: NATIVE_RESOURCE_RECOVERED,
SEQUENTIAL_EXEC_OK_2 and NATIVE_REUSE_OK exited 0, with fifteen claim tests.
Its e4f9ab5d source patch is selectively integrated and patch-equivalent.

The integrated producer/consumer uses execution schema 4 and independently
verified resource-scope schema 3. Fresh process-effect release requires canonical
`process_tree_completed`; legacy parent-wait facts cannot release descendant
scope. Older schema records remain readable and replayable without granting new
parallel capability. Original results, late canonical End supplements and current
continuation hook facts retain the same execution identity. Remote/caller fields
cannot manufacture scope, and cached executable offers still need current
verified dispatch capability. Exact CodeMode handlers are refused before effects;
ordinary same-name tools are not treated as those handlers.

This qualification applies to the exact participating Runtime and Provider.
Unrecorded effects in an already-running older Provider cannot be retroactively
arbitrated by new source. No operating Provider was replaced for these checks.
The mixed-generation consumer delta has ten files byte-identical to frozen
`4e6b31e3`; its complete Provider tree is `1a32a6b76485f58c746cad173ddea14f04fbddf4`.
All-target tests passed **440 distinct cases** (340 library plus 100 integration),
zero failed and 34 existing gated live/asset/helper cases ignored. Recursive
child summaries are not counted twice. Strict all-target Clippy, both format
checks, 41 Python tests and diff checks passed. The actual loopback cut fixture
checks the original body/call/response, one POST and upstream EOF separately from
final Runtime execution. Receipt:
`1d93d344dbe7b1f095a41d8233becc4886fbb48b966698adb5d21352e9153c87`;
independent review `df913ddd5243c9fddbd94c0610d964ed0e890e4a15e955b1a4e932d23cb7a841`.
The original failing iterations remain preserved. Hosted final-source and actual
new Runtime/Provider joint execution are separate from these local passes.

| Actual execution | Result and proof boundary |
| --- | --- |
| Warm parallel compilers | ca1dd504 Provider + qualified cf Core/Runtime; two real clang processes overlapped 5,760 ms, two bounded claims and two 2,960,160-byte outputs; original final polls exit 0, conflicting call refused before effect, ledger returned to original empty digest |
| Core-only controls | 32 plus seven assertions, 21.18 seconds, zero-model; 711b857bb0aa5be56429cf028b27646f9c83abcf41477de38070cd8d893aac70 |
| Observer detach | Same active Owner/thread retained; a135b830efc406cfe0e9cb71496287569118ca6ef046da0f838bcb776ae43da8 |
| Core-only basic UI | No Manager/Enhanced, actual Runtime, one real command/exit 0, same thread, two synthetic responses; 0e0506ca81fbe03bc19e91f7126956fe52470c9ad90acbb3cd3684bd29344446 |

The cold source-bound trial is **not yet accepted**. Its first attempt retained
an interrupted original result and unknown Stop ACK without replay. Attempt two
recovered its original compiler exit-0 output after private Provider restart,
but unloaded completion still called a loaded-only job API. B finished; A's
original work/receipt remained preserved. This led to the Owner correction above
and the completion/settlement corrections now integrated; joint execution remains pending.

Independent pinned-source review proved additional unsafe assumptions:
synthetic termination status can precede successful OS wait; a CreateProcess/
Rejected error can follow spawn; absent asynchronously persisted history cannot
prove non-dispatch. Three existing-consumer negative tests compiled and failed
semantically before correction. The Runtime producer and Provider consumers were
corrected together in f2aef205/e8a08631 and extended by 1e203874/d4028958;
final joint acceptance remains separate. Stop ACK, empty jobs, terminal item or legacy exited
boolean alone is not accepted as real process settlement. Earlier control tests
that asserted only those weaker states cannot qualify the corrected boundary.
Original raw results and failed receipts remain unchanged.

HTTP 413 and CDP failures were injected at production interfaces; no deliberate
live ChatGPT 413 was induced. Actual Native observer/TUI disconnection preserved
Writers. Original-result/receipt replay was exercised; a live Chat final ACK was
not deliberately lost. No uncertain active Runtime was forcibly crashed.
Boundary receipt: 2d18be369bf0680876e4995124acd434e63e4daf11f93fe398e370d2642c0681.

## Provider, model and independent shell evidence

Stable IDs remain chatgpt_desktop and chatgpt-desktop/...; display naming is
Linux ChatGPT app. New provider-chatgpt-v... names preserve old provider-gpt-v...
readers. Ordinary Codex authentication, app inference identity, local service
token, Native execution home and source-conversation ownership are distinct.

OpenAI uses the upstream model manager, order, effort and account-scoped cache
without querying the Linux app. ChatGPT uses the authenticated app/server catalog,
availability and actual reasoning options, without two-model restriction or
foreign-provider fallback. Other providers retain their upstream catalog path;
no additional real third-provider inference was claimed.

The default synchronous Auto Review path keeps the current selected model and
supported effort under the upstream reviewer policy. Its existing auxiliary
AppHost lane is independent of development work and cannot grant Native effect
authority. Optional Guardian V2 instead requests a fixed Luna classifier model,
which the current ChatGPT catalog does not admit: HTTP 400
unsupported_desktop_route remains explicit. Upstream records failure/risk 1.0,
emits an ExtensionWarning and follows its existing synchronous-review or AskUser
decision; classifier failure does not create an automatic approval. No model
alias, approval-policy change or separate Doctor attestation is introduced.
The 53-gate ingress fixture is not live classifier acceptance. Six relevant
offline cases passed at 1f; the two actual reviewer opt-ins remain separate
until explicitly executed. Source/consumer review:
`3eb3ac1e95fecfbdb5b5e32dd4df519468fe61379d0511b42c7a4287a73e1486`.

| Actual candidate path | Receipt SHA256 / limit |
| --- | --- |
| OpenAI → ChatGPT → OpenAI, cold resume, foreign direct-model refusal | 9ceef3690c2bf5910832f8157696aa5407952675ffad50698cbe8c5a4ba7ac79; four synthetic responses |
| Zero-input Provider/model/effort persistence | 921c3016abb6e4ceda7407baf35a66c21186f3913035b0a30fa52846c8452993; zero model submissions |
| Native compact, picker/direct selection, same-thread follow-up | 3256d30da5e50ec754c26c8c5575cb0be4287a94ef603051c81c2a1e3b01178f; three synthetic responses |
| Actual preparation-source mutation at J | 6159de393b4997a1b3d894e6fed708351c3a4e0eb259cef8da2690d7c419970e; green → compiled semantic red → byte-exact restored green, zero real App calls |

These d2 PTY compositions were explicitly provisional, not final Full/live
account acceptance. Upstream completion is command-name /mo → /model;
direct model argument resolution is a separate tested path.

The independent MCP contract remains one termux_shell plus the original seventeen
tools and unchanged common annotations. The connected installed worker passed
real execution, exact original-ID replay and changed-payload refusal. A connected
call during an owned bounded Native Busy returned once; replay preserved its
original PID/time/output and one-effect marker. The original compiler later
exited 0; only that verified compiler's pidfd was briefly controlled with a
45-second resume watchdog, not the Runtime/Writer.
Wire receipt: cca529071712d3daae62d9e3cbecab1b3032bcd5edcab9efbeadc1cc993babcc.

The refreshed frozen worker from the `4e6b31e3` five-object Provider build closure passed
actual private stdio again: single tool/common annotations, no Core/Provider,
original-ID replay/restart/conflict, cancelled unknown intent and child cleanup,
timeout, concurrent workers with one effect, exact 65,536-byte bound, private
modes and unchanged sentinel. Three workers exited 0; five owned children were
inactive. Worker SHA256:
`7996824b875c2585cde841962233ff9ee2d03e50107eff0df86c9be6b702a250`.
Execution receipt: `ce23cf756b06393fa048396cb07ab1dd0cc5196c53c539d264da6f2b94b68ab6`.
This is an Android local build/execution. The earlier `21e82507` worker receipt
`7a60052783151faac51459ebe4fcb454cde0faab741a7e656da1cd1c7d6488b4` remains preserved.
This does not yet prove the new worker plus final Runtime under Native Busy.
The candidate was not activated in the connected tunnel. Refresh selects an
explicit independent mcp_command, preserves legacy omission and refuses invalid
explicit wrappers; eighteen client tests/109 assertions passed without a refresh.

## Live App and Android boundaries

The existing authenticated app returned six visible choices plus three hidden
compatibility entries, including GPT-6 and GPT-5.6 Sol Instant/Thinking/Pro;
Thinking offered standard/extended/max. One original Chat and Native thread
used gpt-5-6-thinking/high to write/compile C (W19_NATIVE_FIRST, exit 0), edit the
same file and recompile (W19_NATIVE_SECOND, exit 0). Original Provider/model/
effort/approval/sandbox settings survived, final work/ledger was settled.
Receipt: c44dc713ce94b3962d2dba41a27eb0deecc9c494ac049297bad473d8595e2929.

That is operating-binary evidence: Provider
e27e92c2442647da3a1997455198b4f5a64e700df6a4e6f37c70fb71b5eae44d
differs from the selected signed package. No login/token/service change occurred.
Private homes without auth.json and with a synthetic invalid OpenAI API key
both received actual app inference; explicit OpenAI calls returned missing/
invalid-key 401. The owned retrying CLI probes ended by bounded SIGTERM, not
natural successful completion. This is not a real revoked refresh-token/account
switch test: 9891a5517af8dd78cd72ee121f217251db8a06f4b1edba9573d5b14ae056346a.

Read-only preflight proved a private candidate cannot borrow the operating
AppHost's authenticated owner or installation-scoped Runtime registry.
No registry link/copy or candidate live request was made:
9eac4dba78908b6b22c819e91e4056a61a94277648633ef37e2f53522604c4c4.
Candidate live acceptance requires separate authorized activation or private
authentication. The confirmed janmori101 refresh (17 → 18 actions) remains
valid prior evidence; execution-home labels and wrlab's old OpenAI token_revoked
do not identify the app inference account.

Manager uses existing terminal/window machinery, verified Owner/home identity,
one-shot creation, duplicate suppression and default-off auto display. Real
isolated PTY/Manager tests and stock Android API 30 CI passed actual Activity,
foreground session UUID, repeated/concurrent click/auto ON/OFF and foreign-home
refusal. The stock fixture has synthetic Core transport/frontend: it is not
physical-device visibility, screen-lock/background or real Native inference proof.
A physical read-only dumpsys attempt at 03:31 UTC returned Permission Denial for
the app UID lacking android.permission.DUMP, despite process exit 0; adb was
unavailable. No screen result is inferred from that exit status. Receipt:
`b5ee8b32672ba10e5548002bbbd85161901bb30b007a6d20e2ee919cc7597cc2`.

**Physical test exception:** one early failed unit fixture inherited real Termux
AM and recreated an owned /not-executed display descriptor. A read-only probe
observed terminal count one → two, consistent with an inert test surface; exact
session mapping was unavailable. No further focus/close/relaunch occurred.
Descriptor/evidence remain preserved and installed bytes/activation stayed
unchanged. All later unit display probes isolate Android transport. Do not claim
zero physical UI effects for this entire integration.

Historical SQLite/rollout inconsistencies were diagnosed read-only and kept
separate from Busy/Provider causes. No database or JSONL repair, deletion,
reindex, thread-ID change or user-Writer termination occurred.

## Exact-source CI evidence

The current 1f1f5cff checkpoint has seven registered qualification runs.
The table records verified conclusions only; unfinished runs do not grant
admission. Full attempt 1 refused missing prerequisite evidence before assembly
and must be executed again after successful exact-source prerequisites.

| Source | Workflow | Run | Last verified conclusion |
| --- | --- | --- | --- |
| 1f1f5cffa26bccf27428132e7482c1a0e959fbf4 | Source | [43158448316](https://github.com/humtr/codex/actions/runs/43158448316) | failure: TLS fixture connection before stall oracle |
| same 1f checkpoint | Core/Runtime | [43158448288](https://github.com/humtr/codex/actions/runs/43158448288) | success; all 53 required checks and final admission |
| same 1f checkpoint | Manager | [43158448242](https://github.com/humtr/codex/actions/runs/43158448242) | success |
| same 1f checkpoint | Provider | [43158448224](https://github.com/humtr/codex/actions/runs/43158448224) | success |
| same 1f checkpoint | Stock Android | [43158448254](https://github.com/humtr/codex/actions/runs/43158448254) | success |
| same 1f checkpoint | Full TUI | [43158539818](https://github.com/humtr/codex/actions/runs/43158539818) | failure: two native_completion expect_used and two process_manager redundant closures; no signed advancement |
| same 1f checkpoint | Full composition, attempt 1 | [43158448095](https://github.com/humtr/codex/actions/runs/43158448095) | refused pending exact-source prerequisite |

Core/Runtime attempt 1 completed successfully at 05:33:12 UTC, including the
final required gate130995541165. Its original Native proof artifact13918770699
was downloaded once and its 13 files/hash-bound logs were verified. All **53
required names** passed. Actual package execution was **276/276**, including
full PTY76 and rollout163; the required names are a subset, not an additional
53 executions. Four additional hook tests and two schema commands also passed.
The corrected Guardian fixture proved one inference POST and the existing two
WebSocket preparation GETs. This is upstream Runtime proof, not Enhanced TUI,
physical Android or live authenticated App inference. Verification SHA256:
`20e4681ed4defffba6b3abb79f2a4cdedb057ae2c2442e85c1c55adbd6fcae42`.

Manager's immutable proof artifact 13916797569 was acquired through the strict
Full input verifier and matched its successful run/source and closed inventory.
Receipt: `488953c750cf9de088222a52a250a986180c44c0b281cee9883640cfa6cddca3`.
A matching binary/input digest does not authorize rewriting original producer
provenance. Final Full still requires all six verified inputs.

Reusable historical evidence is limited to its exact source and boundary:

| Source | Accepted evidence | Limit |
| --- | --- | --- |
| 7b98296a | Source [43012469220](https://github.com/humtr/codex/actions/runs/43012469220), Core/Runtime [43013017085](https://github.com/humtr/codex/actions/runs/43013017085), TUI [43013363622](https://github.com/humtr/codex/actions/runs/43013363622) | Original 0.167.1 baseline |
| 787fcd2b | Full TUI [43104655110](https://github.com/humtr/codex/actions/runs/43104655110): focused 172, Retain TUI 53, engine 43, apply-patch 2 | Predates current switch/Runtime changes |
| 3d81ae14 | Core/Runtime [43148496770](https://github.com/humtr/codex/actions/runs/43148496770): original 50 named gates, full PTY 76 and rollout 163 | Does not cover the later three ingress checks |
| aa863b50 | Source [43152378817](https://github.com/humtr/codex/actions/runs/43152378817): Core 272 distinct passes, strict workspace lint/format | Predates latest test fixtures |
| aa863b50 | Manager [43152378731](https://github.com/humtr/codex/actions/runs/43152378731): 139; Provider [43152378678](https://github.com/humtr/codex/actions/runs/43152378678): 440; stock Android [43152378689](https://github.com/humtr/codex/actions/runs/43152378689) | Opt-in Native/live cases remain separate |

The 3d TUI focused run passed 179/180 and failed while parsing its fixture's
TOML document. Its test-only correction preserves every behavior assertion.
The aa Native proof passed Core 32/32 and PTY 76/76, then failed its new
Guardian aggregate request assertion; final candidate admission correctly failed.
The aa TUI failed at dependency-mirror connection before compilation.
These are three distinct failures, not successful qualification. Original logs
and narrow correction/review receipts are retained in the immutable record above.

The 1f Source workspace failure occurred before the payload-stall oracle:
curl returned 7, no HTTP status and zero bytes. The fixture exposed its port
file before writing the value; a controlled pause reproduced the empty read.
Test-only `578a386b` publishes the completed private port file atomically,
preserving production code, timeouts and every download assertion. Six actual
download tests, strict all-target Core Clippy, formatting and diff checks
passed. The original CI did not retain its port/URL, so its precise scheduling
interleaving is not asserted. Independent review:
`0db42c867660457722655e91e4f8d2e09be51c457f234d98359089ba0b6389d2`.
The corrected source still needs hosted Source Acceptance.

At the independent 18:57–19:12 KST readback, the original integration worktree
remained on 578a386b with four original dirty documentation files and the remote
development branch remained 1f1f5cff. A separate exclusive Writer worktree at
`work/independent-accept-20261112` copied those four files byte-for-byte
(SHA256 verified) without clearing their original staged/unstaged state.
Bridge read-only inventory returned 131 Native works: 68 completed, 56 stopped,
6 reconciliation_required, and 1 interrupted, with none in an ordinary running
phase. These seven uncertain/interrupted works were not settled or replayed.
Installed S159/S163/S165 and isolated fixture processes remain distinct and
were not indiscriminately stopped.

The independent Writer traced four exact 1f TUI Clippy errors to the real
`crates/release-builder/src/native_route.patch` producer using official upstream
blob `9715d1b1a8d0c3e2f0c02c7b27858fa77bb75bd6`. Two typed completion
serialization `expect()` calls now emit explicit unqualified errors instead of
fabricating completion evidence; two `begin_attempt` closures use the equivalent
method directly. The patch retained its full-index upstream ancestry.
Patch reverse-application, rustfmt, `git diff --check` and documentation
contracts passed. Local release-workflow 14/14, runtime inputs 3/3,
runtime-production 12/12, reentry environment 2/2 and TUI producer 3/3
passed; a separate TUI artifact test bundle reached the 60-second execution
limit without a final result. No hosted Source/TUI/Full acceptance or candidate
live-model acceptance is inferred from these local checks.

No timeout, test selector, assertion strength, retry policy or approval policy
was weakened to obtain a pass. A Guardian route fixture proves ingress and
metadata boundaries, not authenticated application inference or all Auto Review
behavior. Each final source and actual product boundary must be checked separately.
