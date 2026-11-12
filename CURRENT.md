# Current product state

Updated: 2026-11-12 UTC. Observed facts and proof limits only.
Requirements: [SPEC](SPEC.md) and [functional annexes](docs/spec/INDEX.md).
Unfinished acceptance: [HANDOFF](HANDOFF.md).
The [finite integration record](docs/development/RELEASE_FRAMEWORK_IMPLEMENTATION.md)
owns the W-task DAG, file Writers, provenance and change accounting.

## Source and publication authority

The sole ongoing development branch is
[rewrite/rust-core](https://github.com/humtr/codex/tree/rewrite/rust-core).
Integration starts from `7b98296a411a2c435bb9f34ee51f286f33f3491b` in the
independent `integration/release-framework-20261112` worktree. Feature commits
are fast-forwarded to development; the protected primary checkout remains
`e4f9ab5d` on its older 0.167.0 lineage.

The last integration checkpoint evaluated across Source, Core/Runtime, Manager,
Provider and stock Android is `26952f732a673ed2dd78187121d6374ddcfe9c88`:
all five gates passed. Its full TUI run failed when compiling
the focused test target; the production CLI build and upload succeeded.
At checkpoint `3d81ae14`, Manager, Provider and stock Android passed; Source
Acceptance exposed an oversized debug-binary rollback fixture. Test-only fix
`a596efe9` preserves the production byte bound. Exact-source checkpoint
`aa863b505bae6b5a6d525e364f0373e4e2a27844` subsequently passed Source,
Manager, Provider and stock Android. Its Core/Runtime qualification failed at
one mandatory Guardian assertion after Core 32/32 and PTY 76/76 passed. The final
execution-proof gate correctly refused the candidate despite successful raw
Runtime production and Android smoke. Its full TUI run failed before compilation
because the external dependency mirror could not be reached. The earlier 3d
TUI compiled and executed 180 focused tests: 179 passed and one fixture failed
while parsing its TOML document. These failures have distinct causes; neither
run qualifies the complete frontend. Read actual HEAD/Checks before reusing a
checkpoint.

Remote `main` remains `b2567063d2a7a333a718d2495e2c9f8c928d822c`,
the separate publication/approval authority. Neither branch has GitHub branch
protection. The exact-main release-intent guard is closed: the read returned
HTTP 404 for release-intent-v1. No formal release signing, sequence reservation,
publication, stable-index
change or operating activation was performed.

The recipe now selects Codex **0.167.2**, upstream source
`ce21072c90d32c40af7a55b60b2fb35c84a43a6a`. During integration, official upstream
discovery advanced development to `ec51fe4a5f5153a441cf78b1e8ce108050274e80`
at 00:48:38 UTC on November 12. It changes only the release recipe and is a
direct child of checkpoint J. Integration preserves it in merge `795e0cae`.
The current Runtime producer is integrated as `1e203874` from `c174ae83`,
paired with Provider consumer `d4028958` from `4e6b31e3`. These extend the
previous `f2aef205`/`e8a08631` process-tree pair with mixed-generation fences. Both Core-only
`--frontend upstream` preparation and separate Enhanced TUI preparation passed
on the exact 0.167.2 source. The normalized lock contains the same 1,308 external
identities and 171 workspace version updates; its SHA256 is
`5e1781c98407bf0cbf67bb39a2dffffa6d2a259871a47fe99946de22c716a782`.
Assembly receipt: `99ebd9a19fba7691da29ed034e9a3c0f7a62e1ad29cbf57a467e232f989d2ca9`.
This source preparation does not prove hosted Core compilation or execution.

The earlier basic-Runtime run
[43138521772](https://github.com/humtr/codex/actions/runs/43138521772)
failed Core compilation after PTY 66/66: the late-history path named RolloutItem
in the wrong crate and lacked HasLegacyEvent in scope. Those production-reference
corrections are proved at `dabd08ff52a7846e58ed913d8191626e9a4d315f` by terminal
successful run [43142830623](https://github.com/humtr/codex/actions/runs/43142830623),
completed 03:21:40 UTC. All 34 named gates passed: Core 24, full PTY 66,
full rollout 163, protocol 2, app-server protocol 1, Native hooks 1 and hook
matching 4; both schema checks and final cache save passed. Original proof
artifact 13911434762 is 114,366 bytes, ZIP SHA256
`391c3ec12206b327c816005e3fb8a71c96260a208b09edebf2dc6941faae8987`.
This earlier compiler checkpoint does not qualify the later process-tree code.

Parent OS wait alone cannot prove detached descendants ended. The final producer
uses the existing pre-thread spawn helper to supervise each execution. Positive
root wait, exclusive child reaping through ECHILD, a private completion frame,
EOF and successful helper wait are required together. Stop/pruning targets only
the selected scope; root status remains distinct from descendant lifetime.
Device helper/PTY focus passed 15/15 with zero ignored and strict production-lib
Clippy. An earlier full Android PTY run passed 52/54: both failures were reproduced
on pristine upstream and remain failures, not accepted candidate passes.
Final focused receipt: `76fa362e3b2843446577360257184812ded8e12c594a95a5403a8e795b0afbbe`.
Independent current producer reviews:
`95d4feef81d7b80aaf806fb2a0360acdfdece9a35230356e0ff474ae3cb32d01` and
`670455ed2c3ea85e5fe801d5185bf1b20c93a68dc03aa43ce9208c27bb879819`.
The unchanged helper/PTY subset can reuse its focused receipt; new Core and
Guardian cases require hosted execution. Exact integrated Core/Runtime and TUI
execution remains an acceptance boundary.

The compatibility audit found that an old Provider could ignore the new proof
and treat root exit as terminal scope. The current producer retains the legacy
running-session header while descendants remain, labels root status separately,
and delays canonical terminal history until verified tree closure. Native
inference uses the versioned private ingress: an old Provider returns 404 before
admission, inference or effects. Core, live switches, Memory and Guardian clients
use the configured Provider identity; absent Native URL fails before generic
auth/API resolution. The route itself grants no execution authority. Ordinary
OpenAI routes and original output bytes remain unchanged; complete late-output
capture is not a new guarantee. No fallback reexecutes the original request.

The bot's unchanged-producer Source and Core/Runtime gates passed separately;
they do not qualify the new input-inventory and completion changes. The observed operating
S165 installation remains **0.167.1**. Official upstream comparison identifies
two GCP incremental-tool changes and the version release; receipt
`98e166ac76a9786457308377ec1c8d413a3e7b49da959fa1e71306674fd8cf45`.
After unshallowing, Native/plan
histories have common ancestors with development; main retains separate ancestry.
Native integration a55d5bb8, PR #29's current alias and the account draft
2dd07d42 (through 8f97affc) are already integrated. No primary checkout reset,
branch deletion or whole obsolete-lineage replacement occurred.

At the 03:14:44–03:18:45 UTC inventory, there were 112 local/tracking refs,
95 worktrees, 45 actual remote heads, 166 local tag names and 257 remote tag
names. No common-name tag SHA mismatch or open PR was found. All refs have a
classified disposition; 73 unchanged and 16 unchanged transfer dispositions
were reused, and 23 current/owned classifications refreshed. Actual remote
heads are distinguished from stale local tracking refs. No branch was deleted.
Receipt: `db0207c8fb8048701a29fcf2e8e804ca81a5f226f415e48afe4f23cc56b15408`.
The finite integration record carries this snapshot and later integrated-source
corrections; it is not a claim that no ref can subsequently advance.

The observed bridge outage ended at 02:44 UTC. Exact original-call requery
recovered root590, Models693, Native1115 and Release823 before continuing their
owned work. This did not restart an operating service or replay a command under
a new identity. Root's original shell receipt is bound by
`316427ff407a5038dfd015e65e5a643b4f2a485d4de0e2ae085ea485c7c621d7`.
Local and remote contingency changes were compared and reconciled in `90c42fd7`;
its tree exactly matches `dabd08ff` and preserves both histories. All 63 local
workflow/document checks passed on that tree. The expired temporary outage
notice and its navigation pointers were retired after this reconciliation;
Git history preserves the notice and unverified WIP checkpoints.

Gate commit `4cd0e80e` requires all 53 named original-execution and descendant
oracles in both independent Runtime proof and complete TUI qualification: 32
Core, 15 PTY, one Guardian and five protocol/hook/history cases. Full PTY and
rollout suites remain mandatory. Its 70 workflow/artifact/document tests, YAML
parse and diff checks passed, receipt
`6b14a915fb878534e9b4352f74f2a9bc34d4e053a0bfb1a13958ad9b2143b2e5`.
This gate-plumbing proof is not execution of the new 53 upstream Rust tests.
Full TUI uses explicit manual dispatch with all full gates and the unchanged
explicit-intent publication relay. The temporary outage-only workflow is retired;
independent Core Runtime qualification remains a permanent source gate.

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

The integrated producer base (`15301ff6` / `a4e31865`, worker source `0008`)
passed four real Protocol/Rollout/AppServerProtocol tests in the frozen selected
0.167.1 workspace. Selected sources and external dependency identities were
preserved; unreachable workspace members and an unused global V8 patch were
omitted only in that disposable check. This is not full-workspace or 0.167.2
qualification. Log SHA256: 5133da4396dcbf9f5279a18ed34b084ba585d783671c1069dfe960113983e4d4.
The Core/Hooks continuation stopped before those tests at two Android mxc-sdk
E0433 dependency errors, exit 101; no substitute implementation was introduced.
Receipt: 002cc9122eee54d065ed224bfecc5cf340c7e847cfe997a5ba9a6dda521e6a10.

Provider consumer snapshot 18a7969046a76e776f4553e2ee9f79dccebb035cd8b7ed616a0250ee3e1d498c
passed all targets: 412 primary tests, zero failures, 34 existing ignored cases;
recursive child summaries are not counted again. Strict Clippy then reported
five diagnostics and stopped before the formatter step. Result receipt:
4e0563e45b5a398206ee37d71ff5dcb0f1c2b2606fc6c3e8dacf3f37167900dd.
These snapshots precede the final per-tool admission/Hook and portable-PTY
signal-wait corrections; their success does not accept those later changes.

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

Separately, qualified G Android worker artifact 13893816894, executable
68df8ecb6918153d28808e3a029cc53ae77579cffa20557d29dadc97ea4d5b12,
passed actual private stdio without Provider/Core: replay/conflict, restart,
cancellation preserving unknown intent, process-group cleanup, concurrent
one-effect execution, 64 KiB bound and private file modes. Three workers exited
0; original Native sentinel and operating wrapper/Core stayed unchanged.
Receipt: a7d828a8444baec9b47d4a1d427a7475e4b668ca8a3c87fa7b898576946ddff6.
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

| Source | Workflow | Run | Conclusion |
| --- | --- | --- | --- |
| 7b98296a411a2c435bb9f34ee51f286f33f3491b | Source | [43012469220](https://github.com/humtr/codex/actions/runs/43012469220) | success |
| same baseline | Core/Runtime | [43013017085](https://github.com/humtr/codex/actions/runs/43013017085) | success |
| same baseline | TUI | [43013363622](https://github.com/humtr/codex/actions/runs/43013363622) | success |
| 787fcd2bc6708941ffbdd6a193bae35d3b8478a5 | Full TUI | [43104655110](https://github.com/humtr/codex/actions/runs/43104655110) | success |
| 26952f732a673ed2dd78187121d6374ddcfe9c88 | Source | [43118366133](https://github.com/humtr/codex/actions/runs/43118366133) | success |
| same J checkpoint | Core/Runtime | [43118366004](https://github.com/humtr/codex/actions/runs/43118366004) | success |
| same J checkpoint | Manager | [43118366040](https://github.com/humtr/codex/actions/runs/43118366040) | success |
| same J checkpoint | Provider | [43118366092](https://github.com/humtr/codex/actions/runs/43118366092) | success |
| same J checkpoint | Stock Android | [43118366005](https://github.com/humtr/codex/actions/runs/43118366005) | success |
| same J checkpoint | Full TUI | [43118617817](https://github.com/humtr/codex/actions/runs/43118617817) | failure: focused test compilation |
| same J checkpoint | Full composition, attempt 1 | [43118366010](https://github.com/humtr/codex/actions/runs/43118366010) | refused missing exact-source prerequisite |
| ec51fe4a5f5153a441cf78b1e8ce108050274e80 | Source, upstream discovery only | [43130011419](https://github.com/humtr/codex/actions/runs/43130011419) | success |
| same upstream-discovery source | Core/Runtime | [43130014157](https://github.com/humtr/codex/actions/runs/43130014157) | success; completed 2026-11-12 01:28:06 UTC |
| 5f05d2d50ab15b749536f54b66332c15d0b6134f | Preliminary Native Runtime proof | [43138521772](https://github.com/humtr/codex/actions/runs/43138521772) | failure: Core production compilation after PTY 66/66 |
| dabd08ff52a7846e58ed913d8191626e9a4d315f | Earlier Native Runtime compiler/proof checkpoint | [43142830623](https://github.com/humtr/codex/actions/runs/43142830623) | success; 34 required checks, full PTY/rollout, final cache complete |
| bfec2858798bfdea18843112019c3f3a1b9e6fc7 | Earlier Provider consumer checkpoint | [43143270459](https://github.com/humtr/codex/actions/runs/43143270459) | success; 429 distinct tests, predates final schema-4 tree proof |
| 3d81ae14cbfa567c05d7cb4b8287113b0c7f74b1 | Source | [43148496588](https://github.com/humtr/codex/actions/runs/43148496588) | failure: debug executable exceeded rollback fixture bound; test-only fix a596efe9 |
| same 3d checkpoint | Core/Runtime | [43148496770](https://github.com/humtr/codex/actions/runs/43148496770) | success; original 50 required checks, full PTY 76 and rollout 163 |
| same 3d checkpoint | Manager | [43148496548](https://github.com/humtr/codex/actions/runs/43148496548) | success; 139 actual tests |
| same 3d checkpoint | Provider | [43148496475](https://github.com/humtr/codex/actions/runs/43148496475) | success; 437 distinct tests, prior consumer |
| same 3d checkpoint | Stock Android | [43148496525](https://github.com/humtr/codex/actions/runs/43148496525) | success; real emulator Activity/session checks, synthetic Core/frontend |
| same 3d checkpoint | Full composition, attempt 1 | [43148496578](https://github.com/humtr/codex/actions/runs/43148496578) | refused pending exact-source Core prerequisite before assembly |
| same 3d checkpoint | Full TUI | [43148525665](https://github.com/humtr/codex/actions/runs/43148525665) | failure: focused 179/180, TOML fixture document parse |
| aa863b505bae6b5a6d525e364f0373e4e2a27844 | Source | [43152378817](https://github.com/humtr/codex/actions/runs/43152378817) | success; Core 272 distinct cases and strict workspace lint/format |
| same aa checkpoint | Core/Runtime | [43152378677](https://github.com/humtr/codex/actions/runs/43152378677) | failure: mandatory Guardian assertion after Core 32/32 and full PTY 76/76; final admission refused |
| same aa checkpoint | Manager | [43152378731](https://github.com/humtr/codex/actions/runs/43152378731) | success; 139 distinct cases |
| same aa checkpoint | Provider | [43152378678](https://github.com/humtr/codex/actions/runs/43152378678) | success; 440 distinct cases; opt-in actual-Native cases separate |
| same aa checkpoint | Stock Android | [43152378689](https://github.com/humtr/codex/actions/runs/43152378689) | success; real API-30 Activity/session, synthetic Core/frontend |
| same aa checkpoint | Full TUI | [43152405151](https://github.com/humtr/codex/actions/runs/43152405151) | failure: dependency mirror connection, before source compilation |
| same aa checkpoint | Full composition, attempt 1 | [43152378691](https://github.com/humtr/codex/actions/runs/43152378691) | refused pending exact-source Core prerequisite before assembly |

The 3d Core/Runtime workflow completed successfully. Its original 50 named
checks each passed once: Core 30, PTY 15, protocol two, rollout one, app-server
protocol one and hooks one; the full PTY suite passed 76 and rollout passed 163.
The later Core URL and Guardian cases are not covered by those 50 checks.
Original job-log SHA256: `df04665cd6879ce843da412c5f88023ff29cf46ee8b77d0ffc169c426e6a023c`;
named-check receipt: `b95789baadcaeb20849f33231f94a32800c1298ff12705c2c16844d2f1f6fe19`.
Source job130952343696 reused the canonical
Android Core artifact and reverified it rather than rebuilding. The failing
Source workspace job copied its oversized hosted debug test harness as a prior
installed Core. Fix a596efe9 instead shares the existing bounded bootstrap
launcher in the three affected fixtures, preserves genuine installed ELF branches
and all rollback/digest/mode assertions, and leaves the production 64 MiB bound
unchanged. Seven focused tests passed, including three with a separately built
genuine Android Core; strict lint/format passed. Original failure log:
`669a22a273e9385155ac2238093272ac9fad6aae2eeeb14e69b8b424f24b392d`.

At aa, Source Acceptance passed 269 ordinary Core tests plus three separately
executed genuine-Android admission cases: 272 distinct passes, one existing
ignored smoke. Strict workspace Clippy and formatting passed. Manager passed
139 (60 library, 78 profile commands, one window); Provider passed 440 (340
library and 100 integration), with its existing 34 opt-in cases still separate.
The original Source workspace log is bound by
`b9c71425410def5af7744246fdd936f47f84c726069e42cefaa3034704914eb3`.

The 3d TUI failure occurred before ConfigBuilder or the profile flag producer:
a table document was passed to the TOML value-expression parser. Test-only
`785fe070` (origin `37911fb8`) uses the document parser with the exact same
fixture text and preserves every selection, destination and permission
assertion. The pinned parser reproduced exit 1; the corrected document parser
and actual upstream CLI table roundtrip returned 0, with all 43 registry package
identities matching the original lock. This local proof does not execute the
whole named TUI test. Receipt:
`bd391ee1f1e1a03df45da34d6de8b78b185c545f3b5a75b1214c68570657c072`.
The original TUI failure log remains
`9d1e0859943347e645a2d90f1d00b17b93820398f9c8b2a4b50d7b2a1a9bc2f0`.
The aa Runtime proof's new Guardian fixture observed three total HTTP requests
where it asserted one. Its log records that aggregate failure, not their methods.
Independent source inspection identified the unchanged pool's two test WebSocket
prewarm requests alongside the inference POST, and existing Guardian correlation
metadata. These must be checked explicitly; the one-inference-request and Native
authority boundaries are retained. The original failed log SHA256 is
`af3fb2e2041baddbecbd832af86e9566f50f61fd08c28c9d277a656f9c92b972`;
source review: `ddd22f826ee2439c1fdbd85d9fa7f8855d6806f5fcb6be1fb296c39d49afb714`.
Later package tests did not execute. At the 04:49:52 UTC API readback, both the
mandatory Native job and final execution-proof admission were terminal failures;
raw production/smoke success was not promoted to qualification. Metadata receipt:
`1150dba4df4a1471aa08054698402aa239b19e5da66eff66812eaf3d22ae911f`.
Fixture-only correction `5857e515` (origin `274190f3`) now checks exactly one
inference POST plus the pinned pool's two WebSocket upgrade GETs, all on the
expected route. It preserves normal Guardian correlation metadata and checks
that no Native execution authority is present; missing Native URL still produces
zero requests. The existing scoped mock synchronizes the upgrade observation.
Both independent reviews found no production behavior change. Actual embedded
ReleaseBuilder compilation, basic/provided source assembly, scope, lock and
format checks passed all thirteen steps. Only one expanded test file changed;
all other 105 logical-file images/maps and every baseline image are identical.
Handoff receipt: `9ff0828d88829857b9fd2d78a8b478c25eeb0bf32acf230f98aa33aa8dcc89a6`;
assembly: `43d8f326deaba21229bc0e1ade7e4146e328898074f53593ba22eef5d6a2e0ae`.
The corrected Rust test remains pending actual hosted execution. No mandatory
selector, inference-count assertion or production retry policy was weakened.
The independent aa tooling failure was curl exit 28 while connecting to the
kernel mirror, before source compilation; no timeout or test was weakened.
Its original log is `66025d8a07817a39c07cd8075d33eb56152550d706c35ed34c7e2179b2a4c79e`.

G full TUI completed 2026-11-11 23:41:42 UTC: focused 172/172; mandatory
independent-fault oracles; Native/config/hooks/API; Retain TUI 53/53, engine
43/43, apply-patch 2/2; strict lint and proof upload/cache all passed.
The exact pin and compaction regressions passed. Optional mutation flags were
false and their steps skipped; promotion was false. This qualifies G's TUI,
not J's later typed-reentry changes or a Full composition.
Proof artifact 13899854078 passed original ZIP digest/inventory/semantic checks:
b2d0256d8a30c1146351c1f04e1ed67770793e90f8fbdea1405c124aec13a86d.

J run 43118617817 completed with failure at 2026-11-12 00:19:10 UTC. The
focused target reported E0063: its shared App test constructor omitted the new
termux_routing_flags field. Zero focused tests executed; all later mandatory
gates were skipped by the failure. The narrow constructor correction initializes
ordinary fixture routing to false, matching the existing test constructors.
Exact-upstream patch application/format, seventeen frontend Python tests and ten
documentation tests passed locally. Hosted compilation/execution remains required.
Original job-log SHA256:
726ba05376f260d5f83fb91ea70ccbc9645f16417b754320b45d1afa141bd3be.

The earlier d2 pin failure (52/53) was fixed by observing its distinct pending
refresh, keeping every pin/unpin/persistence assertion; G verifies that fix.
Separate GitHub startup/finalization failures did not establish test success.
Superseded I failed only the subsequently fixed formatting gates. Original failed,
cancelled and prerequisite-refusal receipts are retained; no assertion was weakened
or mandatory test skipped. Later Owner/Runtime/Provider source still needs all
affected exact-source qualifications and final Full acceptance.

Before every push, recheck development and main: the existing main schedule may
advance upstream discovery independently. Its observed November 12 update to
rust-v0.167.2 is preserved above; the combined integration still needs its own
exact-source qualification. No main, stable index, signing key, release, installed generation,
operating account/service or active user Writer was changed.
