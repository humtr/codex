# Runtime, release, and Manager delivery contracts

**Status:** Normative detail of [SPEC.md](../../SPEC.md).
This document defines release-independent artifact, installation, update,
rollback, device compatibility and qualification behavior. Exact upstream
release identities and prior patch/admission reports are evidence outside
this contract, not a condition requiring a new specification on every upgrade.

## Base qualification

### Independent unsigned base qualification

The reusable producer also has an explicit `base_only` qualification mode. It
constructs only the qualified upstream runtime, Core, Code Mode host and browser
helpers through the existing release-builder. Manager and TUI are absent;
their compilation, downloads and qualification do not gate this mode. Complete
base inventory, digest/mode checks and actual Android/AArch64 Core/runtime smoke
remain required. A base result is not an integrated product qualification.

This mode is read-only with respect to publication. It refuses `publish=true`
before artifact acquisition; signing, Release/Pages staging, stable promotion and
device activation are never reachable. Its unsigned CI artifacts have distinct
base-qualification identities and cannot replace an integrated candidate. It
adds no update channel, installed pointer, runtime mode or public CLI command.
The main publication path still requires the complete qualified backend/frontend
pair and retains the current generation on an unsupported update.

Base qualification may run from an exact rewrite/rust-core push or manual dispatch
with read-only repository permissions. Source must equal the event's immutable
commit. `rebuild_current` selects the version and source archive digest from the
authenticated stable descriptor independently of TUI pins; otherwise the
ordinary official upstream discovery is used. An unsupported upstream artifact
still fails existing builder qualification, rather than weakening patch admission.
Skipped integrated construction distinguishes a non-newer release from missing
frontend qualification or an archive mismatch; it must not report all three as
already up to date. Independent base qualification and feature-preserving public
promotion are different guarantees; section4.6 defines their final boundary.
### Automated upstream preparation and base/full publication

Upstream handling must resolve one exact official version/archive/source tuple,
prepare its Termux compatibility inputs, and drive separate base and full release
lanes automatically. A scheduled discovery controller resolves a newly published
stable upstream version, exact source and official archive digest, updates the
single input authority without force, and explicitly starts its read-only proofs.
Qualified base publication must not wait for Manager/enhanced-TUI porting or proof.
Unsupported transformations stop with actionable evidence and keep the last known
good head; source edits are never invented by a publishing script. Failed optional
frontend work cannot block source/Core qualification or basic publication. Device
activation follows its existing user/update policy, not a forced running-session
replacement. Base contains Core plus the adapted upstream default UI;
full adds admitted Manager and enhanced TUI. The opt-in Chat provider remains
independent. Both lanes require actual Android execution, affected behavior proof,
signing, immutable assets, complete public readback, disposable update/rollback
proof and non-forced promotion. Source preparation or a green host build alone
cannot authorize publication. An unsupported or ambiguous transformation stops
that candidate with actionable evidence and preserves the accepted installation.

Version/source/toolchain and qualified frontend references have one machine-
readable authority. Preparation must not guess binary offsets or accept an
unqualified version by removing its guard. Prefer repeatable source/semantic
transformations; any generated artifact policy remains bound to exact inputs and
Android proof. Historic policies needed by retained generations remain supported.

Qualification build/test jobs remain read-only. An isolated completion relay may
only enqueue the accepted-input workflow with its run ID; it has no content-write
permission or signing key. The receiver waits for that run to finish, then checks
owned exact-source qualification independently. Completion delivery is not proof.
Native trigger metadata and the exact isolated relay are excluded from recipe
comparison only while build/test code consumes no event/dispatch input. The literal
wall-clock budget of the native build job is scheduling metadata, not a compiler
input. Its change alone may reuse a completed successful run; it cannot admit a
failed, canceled or incomplete run. Other global build settings and every build/
test command remain bound. The required focused-test count is an acceptance
criterion: reuse checks the retained log against the current required count,
without treating an earlier misdeclared count as a source change. Source blobs,
upstream/toolchains, dependency audit, executed tests, lint and artifact identity
must still agree. Native builds start through
the accepted-input controller, reusing a matching build already in progress.

Provider-only source changes under providers/chatgpt use the independent optional
provider gate and do not rebuild unchanged Core/Manager or make Bun a Core
prerequisite. Qualify the Rust package and minimum app-side bridge together.
After qualifying the active Provider Rust package and retiring old experimental
paths, legacy differential source/tests remain in preserved experiment history;
the active gate covers Rust, the app bridge, CDP qualification and package tools. Missing Rust/device/bridge gates are unfinished, not a reason to fall back
to legacy admission. The provider gate must not require Bun/Python merely to
execute the production adapter. Scope Cargo lock/toolchain/
shared-build changes by their actual consumers; do not exclude changed shared
inputs from Core qualification. Classification still requires a successful accepted predecessor;
otherwise the conservative full Core gate runs. Changes to Core/shared build inputs
or mixed unknown paths retain full qualification. Provider qualification is separate
from Core acceptance and must be required before future signed provider admission.

Base and full have independent signed public heads. The existing update-index-v1
remains the full head for compatibility; the base head is update-index-base-v1.
The default update head follows the authenticated public baseline of the selected
generation, including a local-derived generation; an explicit update URL remains
an operator override. Full users must never be silently downgraded when only base qualifies. Both public
heads and retained last-known-good assets stay available in their immutable
publication origins; current releases do not replace the legacy Pages site.
Promotion coordinates both heads with non-forced compare-and-swap and increasing
release sequence; a stale concurrent run re-evaluates rather than overwriting
another lane. No device activation or user-profile/provider mutation is implied
by public publication. Qualification-only workflows remain non-publishing.

codex-runtime-binding is dependency-free shared **source**, statically compiled
into its callers, for the existing bounded execution-record codec. Core depends
on release-builder and this primitive, not on Manager, frontend or provider.
It creates no installed shared library, service, registry or optional-component
runtime prerequisite. Core configuration queries remain bounded and optional;
failure skips conveniences while preserving independent execution.

Independence has distinct observable levels:

- **Build and qualification:** Core/base construction and Android execution do not
  compile, acquire or qualify Manager, enhanced TUI or provider. Unsupported native
  runtime revisions still require their own Termux qualification.
- **Installed execution and lifecycle:** a valid Core-only generation supports
  ordinary upstream execution and Core doctor/update/rollback. Unavailable optional
  assets in an already-installed generation do not disable those operations.
  Doctor may separately report missing authentication or conveniences. Fresh
  candidate admission still rejects every missing/changed declared asset.
- **Component reuse:** the builder accepts an independently admitted Manager;
  local reconstruction already carries forward its bytes after the Core-API
  probe. A compatible unchanged Manager need not be rebuilt to change Core.
  The integrated CI build may still compile both; that is not a Core dependency.
- **Feature-preserving public updates:** stable remains one signed combination.
  Preserving /switch and the enhanced /model/permission UI requires its frontend
  to pass the qualified backend protocol/behavior gate. Exact backend-version
  admission remains until a broader pair is actually proven. Failure retains the
  accepted generation; base qualification alone never silently removes features.
  There is no automatic downgrade from full to base; each edition follows its
  own signed public head.

The retained TUI patch contains only upstream call sites/types, supported-permission
integration, startup compatibility and actual test access; standalone policy lives
under frontend/tui/src. Upstream lacks a qualified general extension API
for these UI hooks. Moving them into Core or a separate launcher would not remove
revision maintenance. The official-runtime binary adaptation remains exact-digest
and version-bound for FD/socket/config paths and no-sandbox permission choices;
the source-built frontend's FD remap is separately qualified. Never remove either
gate merely to admit a newer upstream version.

Responsibility realignment is complete when these ownership and independent
execution/lifecycle guarantees pass. It does not claim that future upstream
compatibility, public integration/promotion, same-provider in-place selection or
provider feature acceptance is already complete. Those changes belong to their
respective components and require affected-path proof.
## 6. Artifact and patch qualification

Upstream freshness, request acceptance, build/port qualification, signed public
publication and device activation are separate states. A controller run's success
is not proof that its downstream source/runtime/frontend jobs passed or that the
requested version is installable. An unsupported adaptation is a blocked port,
not an indefinitely pending build; preserve the installed release and report the
failed version/source/stage and original job URL. A failed pipeline is not repaired
by retrying identical inputs or compiling the same unsupported tuple on a device.

Core owns the complete update lifecycle: discover installed-component updates,
resolve compatible dependency groups, reuse unchanged artifacts, acquire or build
missing artifacts, verify and atomically activate each inseparable group. Manager,
TUI, Provider and tunnel remain selected only when the user selected them. Full is
the complete selected composition, not a reason to rebuild unchanged components.
Existing Base/Full generations and independent Provider selections migrate without
losing credentials, profiles, conversations, rollback artifacts or live jobs.

Installed update and local-build policy, source preparation, build orchestration
and result admission are Rust responsibilities. Selecting local build must not
install Python merely to execute wrapper-owned lifecycle code. The release-builder
library owns the common implementation; Core calls it in-process and remote
executors call its CLI. CI environment wiring and independent qualification may
use Python, but must not maintain a second implementation of these product rules.

Runtime source production consumes the exact release input plus a disposable
pristine official checkout, applies the maintained semantic source transformations,
audits lock normalization,
builds with the selected official toolchain and emits the existing six-file raw
runtime artifact contract. `build-runtime` and Core's in-process consumer use the
same Rust implementation. Release/control JSON inputs retain the16-KiB bound;
raw transfer verification checks inventory, regular files, digests and source/audit
identity. Final output is snapshotted before the existing static AArch64 ELF check
and an isolated, bounded version probe. A failed compiler cannot admit an older
cached executable. Executor setup owns OS packages and compiler acquisition;
Generation assembly also has one typed request shared by CLI and Core, covering
the official companion archive, optional source runtime/revision and selected
Manager/TUI artifacts. Core preserves the builder's cause-specific error through
its public error chain instead of replacing every failure with a generic local
adaptation error. The same existing admission checks apply to every caller.
local execution and remote runners call the same Rust producer. Recipe fingerprints
bind its source/preparation/normalization code, exact upstream tuple, lock audit,
toolchain and build settings; a Core-only commit cannot invalidate that raw
runtime recipe. Signed component metadata's `runtime_inputs_sha256` additionally
binds the Rust release-builder/runtime-binding adaptation sources and builder
Rust toolchain when reusing already-adapted published runtime bytes. Legacy metadata
without that field remains admissible for installation but does not authorize
this producer reuse. Cached subsets are reauthenticated after executor transfer
before candidate composition. No previous Core is fetched as a runtime dependency.
Qualified output identity is still its admitted content digest, never the recipe
hash or a claim of reproducible compilation. Host preparation, source/build
success and Android execution proof remain separate completion boundaries.

Compiler preparation selects the tools required for the target rather than
requiring a Linux distribution as the build interface. Prefer a qualified direct
Termux toolchain over a separate PRoot environment when both provide the required
build behavior. A private SDK's loader/library adaptation must be explicit and
bound to its original inputs; it must not alter system libraries or resolver
configuration. SDK library search must remain scoped to its compiler/linker;
it must not shadow Termux's runtime when Cargo invokes native child tools.
Experimental SDK execution alone does not qualify a complete
component build or make that environment an installed dependency.
Build-time protocol generation honors an explicitly prepared `PROTOC` compiler;
without that selection it retains upstream's vendored default. Cargo tracks the
selection so changing the compiler reruns generation. Native executor preparation
must qualify that compiler against the actual protocol inputs; a Linux executable
that fails Android's syscall policy cannot be treated as an available build tool.
The override changes build-tool selection, not the protocol or its runtime behavior.

Remote preparation checks official upstream metadata every two hours and responds
to owned component-source changes. Only changed build inputs trigger preparation;
equivalent pending work is reused. Consumers first use authenticated cached or
published artifacts, without a model login or repository write permission. Local
and remote executors use the same versioned build/qualification contract. GitHub
CI is an optional executor/publication adapter, not the update state authority.
Published artifact authority remains the admitted signing key regardless of
transport or executor. A request ACK admits no executable.

Explicit `codex update` also checks the official stable release. A current wrapper
channel alone does not mean the installed upstream version is current. Ready
signed releases install through ordinary admission. If the official release is
newer and a required compatible artifact is unavailable, interactive update offers
one grouped choice: keep those items, build locally including missing dependency
installation, or request an immediate remote build. Compatible ready independent
updates may still proceed; the displayed plan states that scope. Configured build
policy provides the noninteractive/default choice. No transport error silently
starts a build; signature/digest/compatibility failures remain failures. A selected
frontend is not silently removed. Remote requests reuse the same target job and
report its actual build/qualification/publication state and terminal failures.

The local executor checks the recipe's required tool versions, storage and target,
installs only missing required tools through the appropriate package manager, and
builds only the affected artifacts. Selecting local build authorizes those stated
dependency operations, not an unrelated system-wide upgrade. Source adaptation
preflights its known semantic transformations and reports unsupported upstream
changes explicitly. Actual Termux and Android execution qualification is required
before declaring a local recipe supported.
Compiler preparation verifies both object compilation and required dependency
generation before a long source build. Before adapting source or starting Cargo,
the shared runtime producer compiles and links a small standard-library consumer
for the exact runtime target, verifies its static AArch64 ELF, and executes it
with the existing bounded isolated runtime probe. The probe must return the exact
expected text, empty stderr and successful status. It creates both its isolated
HOME and CODEX_HOME directories privately before launching the child, then removes
that complete owned home on success or failure. An explicitly prepared Cargo
target linker is also used for this direct rustc invocation, so readiness and the
subsequent build test the same linker selection. Matching Rust version/commit strings and an
existing target-library directory alone do not establish compiler/stdlib
compatibility. A failed probe preserves the compiler's actual diagnostic, leaves
the checkout unchanged and removes its temporary probe inputs/output.
The shared runtime producer selects size optimization,16 codegen units, globally
disabled LTO, stripped symbols, no debug information and no incremental output.
This selection applies to the complete Cargo release profile, including dependencies;
disabling LTO only for the final CLI can leave dependency LLVM bitcode for a
different-version native linker. Local and remote executors use this same profile.
Its source definition participates in the existing runtime input fingerprint;
previous differently compiled outputs cannot inherit the new raw-cache identity.
The selected C/C++ compiler configuration
is a build input; a different compiler variant cannot inherit another variant's
raw-cache identity. An executor launcher's zero status is insufficient when its
worker died or the required output is absent. Preserve the actual compiler/worker
failure and admit only the complete verified artifact from the selected inputs.
Native SDK preparation uses the owner-shipped exact tool input catalog, independent
of an upstream release's version. Core acquires the listed official archives through
its authenticated digest cache; the shared Rust builder rechecks those digests
before extracting or executing any input. The catalog pins Rust and matching target
std, private musl/libgcc and Zig by URL/digest/byte bound. Its inputs are part of
the runtime build identity. One C/C++ adapter normalizes cc-rs target arguments
to Zig's target spelling, keeps host includes after target headers and preserves
the upstream musl build's sanitizer/preprocessor behavior for native dependencies.
APK contents are admitted by the owner-pinned complete
archive digest, not by a claim that APK signatures were checked. Extraction runs
only after every input passes admission and only into a newly reserved private SDK.
An existing destination is preserved. Failure removes the SDK created by that call.
Compiler interpreter/search adjustments stay inside that SDK. musl aliases belong
beside its compiler libraries, never in Cargo's exported target-library directory.
Readiness requires the same real compile/link/static-ELF/execution probe used by
runtime construction. SDK assembly alone does not qualify the remaining C library
dependencies, source build, installation or public release.
Native C-library preparation reads the selected immutable upstream commit's musl
build scripts rather than copying dependency versions/hashes/options into another
catalog. It accepts their supported literal assignment/Configure contract, reports
changed or ambiguous syntax before acquisition/build, and never executes their host
apt/sudo setup. The Rust producer snapshots and verifies the selected OpenSSL/libcap
archives before extraction, builds static target libraries with the prepared SDK,
and uses a native host compiler only for libcap's build-time generator. Configuration,
headers, pkg-config files and libraries stay in new private output/build roots;
existing roots are preserved. A failed build removes its new incomplete library
prefix while retaining the explicit build directory/console diagnostics for analysis.
Only libcap's C static library is requested; unrelated Go discovery is disabled.
The resulting libraries must link and execute a static target probe that checks
OpenSSL's selected version/SHA-256/TLS-context operations and reads/frees this
process's capabilities. No network connection or capability modification is needed.
Core's selected local build uses binary adaptation when its known policy is
available, otherwise the bundled qualified native source recipe matching the
selected official version/archive. `release/upstream.json` owns the common
source/version/archive/dependency contract for every executor. The on-device
compiler profile comes from `release/native-sdk-inputs.json`; remote executors use
the selected hosted toolchain. Qualification remains specific to the executor;
sharing source selection does not claim an untested SDK build works. An unavailable
source recipe is an explicit unsupported target, not permission to run an
unverified binary policy. Resolve that selection
before acquiring its upstream archive or provisioning build tools. Core owns required
Termux package preparation, official source acquisition, SDK/dependency caches,
progress and the existing signing/activation transaction. Source construction uses
the shared Rust producer with an explicit child environment; it never mutates the
parent process environment. Private cached SDK/libraries are rechecked against
their recorded complete tree before execution/reuse. A cached raw runtime must
pass the same source/version/digest/dependency artifact admission as a new build.
The local cache identity binds both the shared producer and Core's build-environment
construction. Hold an exclusive nonblocking lease on that cache for reuse or
construction; an independently active cache owner is a cause-specific failure.
Before recovering incomplete output, also detect surviving native children whose
owner exited and released its lease. Preserve their source and outputs.
Only fully admitted source output enters the ordinary generation builder/signing
and rollback path. The selected official source, archive and build inputs come from the
single maintained release recipe. Selection is not evidence of qualification.
Local and hosted source builds require separate input-bound verification;
a qualified hosted artifact may be reused only through exact raw-runtime,
Android and signed-publication admission. Evidence for one version or executor
does not automatically qualify another.

Ordinary local results use the existing device-local derived signing authority and
retain the admitted public update key/baseline. A configured signer whose public
key is authorized for the release may sign qualified developer output locally.
Core's optional `config/build-signing-key` contains one `file:/absolute/path`
reference (at most4096 bytes, with an optional final newline). It is consulted only
for a selected local build, including the manual updater's local policy. The
referenced owner-only Ed25519 key must match the installed state's admitted
`update_key`; malformed, unavailable, insecure or unauthorized configured keys
fail before archive acquisition, without an ephemeral-key fallback. Core preserves
the external key and reference. The signed result retains local-derived provenance,
the authenticated public baseline/sequence and ordinary rollback behavior even
when its signer equals the public authority; provenance is not inferred from key
inequality. Without this explicit configuration, keep the fresh temporary signer.
Model login, GitHub login and checkout presence are not signing authority. Signing
does not imply public upload; publication follows its configured permission and
non-forced promotion policy. Public upload reuses already admitted identical
objects and transfers only absent changed objects plus metadata/signatures.
Unchanged binaries are never rebuilt merely to resign a composition. Public
sequence conflicts require renewed metadata admission, not forced overwrite.

Missing-artifact policy is `ask` by default. The owner may configure one value
(`ask`, `keep`, `local`, or `remote`) in Core's `config/update-policy`; an explicit
`CODEX_TERMUX_UPDATE_BUILD_POLICY` overrides it. Invalid policy fails before update
activation. `ask` requires an interactive terminal when a missing artifact is
actually found; noninteractive callers must select a policy rather than dispatch
implicitly. `--wait` and `--no-wait` explicitly select remote request with their
existing waiting semantics. A keep decision reports the missing target, preserves
that dependency group and performs no dispatch or tool installation. Local builds
pin the official version/digest already presented to the user.

Normal `codex` entry never requests a build. During manual waiting, `c` continues
with the installed version while the CI request remains pending; `q` leaves the
updater. `--wait` explicitly waits; `--no-wait` requests/reuses the build then
continues on a terminal (returns pending status on noninteractive streams).
The update lease is released before requesting or waiting and reacquired only
for ordinary signed activation. An uncertain dispatch is not resent. A known failed request permits a new
explicit update invocation, after preserving its original run receipt.
An active rollback hold, forced held release and explicit custom update source
retain their existing bounded semantics and do not request public CI builds.
Anonymous users can install ready public releases; requesting a missing build
requires GitHub CLI authentication with permission on the owned repository.

The sole upstream input for the first supported target is the exact versioned
OpenAI asset
`https://releases.openai.com/codex/releases/<version>/codex-package-aarch64-unknown-linux-musl.tar.gz`,
where `<version>` is an explicitly selected stable `MAJOR.MINOR.PATCH` value.
Release production supplies that version, a local regular-file copy of the
archive, and its exact lowercase SHA-256. Mutable `latest` or channel names,
discovery, mirrors, and source fallbacks are not artifact authority.

Acquisition and adaptation are release-production work. The wrapper release
pipeline retrieves the exact official package, supplies its pinned digest to
the real non-installed workspace executable named `codex-release-builder`,
and signs the resulting adapted generation for delivery. The installed Core
never executes an upstream self-updater. Explicit local build prepares only
recipe-required tools and invokes the bounded release-production routines.
Core acquires the official archive through its shared bounded, resumable download
cache, keyed by the digest from official release metadata. It reports transferred
bytes and preserves untrusted partial bytes for the next explicit command; only
the verified complete archive reaches generation assembly. The builder owns the
256-MiB archive bound shared by acquisition and archive admission. It then adapts
the raw runtime, qualifies the result and signs only that local
candidate with the selected local-build signer while preserving the official
forward update_key. Local build never accepts a raw archive as an activation
candidate and never bypasses signed local admission. The builder's
`fetch` operation
accepts one explicit stable `MAJOR.MINOR.PATCH` version, constructs only the
canonical official archive URL above, and downloads that archive with bounded
HTTPS transport into one absent local regular-file output. It prints the exact
lowercase SHA-256 for the subsequent build invocation. `fetch` performs no
channel discovery, mirror selection, source fallback, signing, generation
activation, or live-state mutation. The builder's `build` operation accepts the
version, archive and digest, generation identity, Core artifact, creation
metadata, and an optional regular executable Manager artifact through
`--manager <ABSOLUTE_FILE>`, plus an absent output directory. When supplied,
the Manager is copied into the generation root and bound by
`manager_artifact_digest`; when omitted, the descriptor uses `-` and no
Manager file is emitted. It performs no discovery, signing, activation, or
live-state mutation and emits an unsigned Core-bearing generation source for
the `codex-release-v4` signing and delivery path. Core's explicit local build carries
forward the currently authenticated Manager artifact, when one is present,
so a qualified Manager is not silently lost on an upstream update.
`codex-release-v2` remains implementation history and is not retained as a
release compatibility path.

Release production then exposes one non-installed `publish` operation with the
exact form:

```text
codex-release-builder publish --generation <ABSOLUTE_DIRECTORY> \
  --release-sequence <POSITIVE_DECIMAL> \
  --release-base <HTTPS_BASE_URL> \
  --private-key <ABSOLUTE_FILE> \
  --openssl <ABSOLUTE_EXECUTABLE> \
  --output <ABSENT_ABSOLUTE_DIRECTORY>
```

`publish` accepts the current generation layout: a real directory containing
`generation.meta`, the executable Core artifact `core`, `runtime`, and the
root-level `codex-code-mode-host`, with an optional root-level `manager`; all
present entries are regular non-symlink files. A v3 source without `core` is
accepted only for legacy Manager-less publication compatibility. The descriptor must
be a qualified `codex-local-generation-v2` with the supported Android/AArch64,
Core API, persistent-schema, and root-companion bindings. The generation
identity must be a safe URL path component and the supplied canonical HTTPS
release base must end in that encoded identity. The release sequence is a
positive decimal and the base follows the same bounded canonical URL grammar
as `codex update --remote`.

The operation derives one raw Ed25519 public key from the supplied private PEM
using the explicit OpenSSL executable. It emits a non-rotating publication:
the derived key is written as `release_public_key`, `release.sig` signs the
exact `release.manifest` with that key, and no `release-authority.sig` is
created. Therefore an operator producing an ordinary public/non-rotating signed release
with this command must supply the current Core `update_key` private key; this
operation does not implement key rotation. Core's local-derived path is a
different in-process authority boundary: it preserves public update authority and
uses the selected local signer, temporary by default or explicitly authorized
through `config/build-signing-key`. The `publish` command's supplied
private key is read only for derivation/signing, is never copied into the output,
and is never written to repository or device state.

The complete publication output is:

```text
<output>/update-index-v1
<output>/update-index-v1.sig
<output>/releases/<generation_id>/release.manifest
<output>/releases/<generation_id>/release.sig
<output>/releases/<generation_id>/generation.meta
<output>/releases/<generation_id>/core
<output>/releases/<generation_id>/runtime
<output>/releases/<generation_id>/codex-code-mode-host
<output>/releases/<generation_id>/manager                  optional
```

The index contains exactly the four records and final newline defined below,
with the supplied generation identity and release base, and its sibling
`update-index-v1.sig` signs those exact index bytes with the same key. The
publication manifest containing `core` uses `codex-release-v4`; v4 keeps the
v3 control fields and inventory grammar but requires the executable `core`
asset. A v3 publication without `core` remains readable for already installed
legacy generations, but it is not a valid new Manager-bearing update. The
release manifest contains a lexicographically sorted inventory, lowercase
SHA-256 digest, and four-octal-digit regular-file mode for each present
generation file: `generation.meta`, `core`, `runtime`, and
`codex-code-mode-host`, plus `manager` when the optional Manager artifact was
supplied. The `core` digest must equal the generation descriptor's
`core_artifact_digest`; a Manager entry requires `core`. The local
`releases/<id>` tree is the directory to map to the URL represented by
`release_base`; `publish` performs no network upload and does not assume that
the URL is hosted by OpenAI.

Ordinary signed acquisition, `--force`, `--rollback`, `--local`, and `--remote`
never build or sign a release or read a release private key. The manual updater's
explicit missing-artifact phase may select local construction or the remote
request adapter as specified above; local construction alone may read its explicitly
configured signer. None of these routes stages, deploys or promotes public stable.
The presence of
`CODEX_TERMUX_UPDATE_PRIVATE_KEY`, the former device-local maintainer key path, or
an authenticated `$PREFIX/bin/gh` account does not select a signer or authorize
publication. Rollback-held and ordinary launch outcomes request no build. An
explicit manual update can check official freshness even when the wrapper's signed
channel is current; its configured missing-artifact policy owns any build decision.

Official source production remains an explicitly selected build responsibility.
The repository's `codex-release-builder` `fetch`, `build`, and `publish` commands
may be orchestrated by an authorized producer: `fetch` selects and verifies the
exact official upstream metadata/archive, `build` performs the required Termux
adaptation and qualification, and `publish` constructs the signed immutable
release/index publication tree using an explicitly supplied signing authority.
Local update executes the same qualified recipe through its optional build tools;
ordinary launch/download does not. The non-installed builder CLI is also available
to developers and remote executors. Neither build selection nor signing grants
network publication authority. Any public promotion requires the complete
source/execution/readback gate, independent of the executor.

The maintained release workflow runs from one exact accepted source selected by
the main publication caller. Candidate construction and Android/AArch64 smoke
are read-only and have no signing secret or public mutation authority. Signing,
immutable Release/Pages staging, public consumer proof and stable CAS are separate
jobs on the same admitted payload. Upstream preparation is scheduled every two
hours; manual dispatch remains available for non-publishing qualification or
explicitly authorized publication. Unchanged preparation inputs cause no build.

All candidate/component transformations and corrective bindings must complete
before the unsigned transfer archive is created. Package the final admitted
candidate once immediately before artifact upload; never transfer an earlier
archive of a directory that was subsequently changed. The archive must preserve
the admitted file inventory, bytes and modes, including the deferred Manager
marker until native smoke removes it. Smoke packages only its final qualified
candidate, and signing/public readback must consume that same qualified payload.
This invariant applies equally to ordinary, Core-only and Manager-only releases;
a successful directory check cannot qualify stale exported bytes.

For any authorized official producer, an explicit version selector must be one
stable `MAJOR.MINOR.PATCH` value; otherwise the bounded official
`https://releases.openai.com/codex/channels/latest` metadata selects the
`rust-v<version>` tag and digest for the exact
`codex-package-aarch64-unknown-linux-musl.tar.gz` asset. The metadata is a
version/digest selector, not an activation authority. The producer must download
the exact versioned archive and compare its digest to the metadata-bound digest
before adaptation. Missing, malformed, non-stable, mismatched, or unavailable
metadata fails closed; no mirror, package manager, mutable raw runtime, or
upstream self-updater is accepted.

Without an explicit `config/build-signing-key`, device-local construction never
reads the former maintainer key path, `CODEX_TERMUX_UPDATE_PRIVATE_KEY`, or any
repository release secret. It generates one fresh Ed25519 key in private
owner-only temporary storage, derives its public verifier, signs only the local
immutable candidate and deletes that temporary key before activation can succeed.
With the explicit configuration it uses the authorized external key described
above and never removes it. Both routes remove the complete private staging tree
on success or ordinary failure. They
never invoke `gh`, create a publication store, dispatch a workflow, or advance a
public stable pointer. The local-derived generation uses the authenticated public
baseline release sequence rather than allocating a new public sequence. Its
signed generation provenance is exactly the semicolon-delimited record
`codex-local-derived-v1;upstream_version=<VERSION>;archive_sha256=<SHA256>;public_generation=<ENCODED_ID>;public_sequence=<POSITIVE_DECIMAL>`; the signed
generation descriptor independently binds the exact upstream version, source
digest, Termux patch report, Core digest, and compatibility identities.

The local-derived special admission is reachable only from this in-process
official-source construction path. `codex update --local` and `--remote` retain
ordinary public-authority admission and cannot turn an arbitrary self-signed
directory into local-derived state. An active authenticated rollback hold blocks
local-derived construction so the single bounded `previous` slot cannot evict
the public generation that the hold protects. Temporary archive/build material
is private, bounded, and removed before success is reported. Local-derived work
never invokes, installs, selects, or repairs `bwrap`.

Local-derived construction never invokes `$PREFIX/bin/gh` or official publication.
The separate manual freshness route may submit an authenticated missing-build
request to the configured remote executor. Remote signing remains the default;
an explicitly configured authorized developer signer may sign qualified local
output. Keys stay in the configured local signer boundary and never enter model
prompts, downloaded artifacts or request receipts. Signing and installation do not
imply public upload or promotion; publication needs its separate configured
permission and the complete source/execution/readback gates.

Periodic preparation authorizes a lane's publication only for independently
authenticated strictly newer qualified upstream inputs. Base does not wait for
enhanced-TUI qualification; Full still requires its admitted frontend/backend pair.
Equality is a no-op; older/malformed/unavailable or unqualified inputs
cannot publish. Manual publication requires its explicit authorization input.
Scheduled and authorized manual releases share qualification, production-key
signing, immutable Release staging, complete HTTPS readback,
disposable update/runtime/doctor/no-op proof and non-forced exact-parent CAS.

When such a producer is authorized to publish, its complete official candidate
targets the fixed wrapper repository `humtr/codex` and the selected stable
publication ref. A release whose
complete signed file inventory is flat may use
immutable GitHub Release assets under a tag equal to the validated generation
identity; the signed index's `release_base` is then the matching
`https://github.com/humtr/codex/releases/download/<generation_id>/` asset base.
GitHub Release asset names are not authority to rename signed relative paths: if
any authenticated release file contains a `/` path component, the automated
Release-asset publisher must fail closed before creating a release or advancing
the index.

New nested-path publication uses immutable GitHub Release assets and retains the
fixed `https://codex-releases.humtr.workers.dev/<generation_id>/` logical identity
in signed records for `core-v*` (Base), `full-v*` (Full), as well as legacy
`local-relay-hosted-*` and `local-base-relay-hosted-*`
generations. Core resolves that identity to the direct GitHub routes described
below; the Worker remains available to supported older clients. Historical
signed Pages URLs and existing assets remain unchanged. The publication validator
chooses canonical identity by namespace. New publication never redeploys the legacy
Pages site, raises its size guard, or evicts a retained public generation.

Base (`core-v*`) owns Core, adapted upstream runtime/default UI, code-mode host and the two
browser helpers. Full (`full-v*`) pins one exact authenticated base and adds Manager/enhanced
TUI plus its own descriptor and signatures. Optional Provider generations follow
`provider-chatgpt-v*`. Readers retain the exact signed `provider-gpt-*` and
`chatgpt-*` identities for acquisition and rollback; producers never rename or
resign an existing package. Each full result retains one complete
signed v4 inventory and one atomic current/previous activation lifecycle. There
is no device overlay, second updater, mutable latest-base dependency or component
selection policy in Core. Manager/TUI-only changes must not advance the base head
or rebuild unchanged base executables; full-combination Android/runtime proof is
still required. Core/shared input changes invalidate base reuse.
If new base publication succeeds while full integration fails, the new base stays
stable and users retain the previous qualified full combination. This serving
fallback never becomes the automatic repair target: repair/rebuild of add-ons
always targets the latest authenticated successful base. Each build pins that
base immutably. Automatic publishers are serialized; promotion rechecks its exact
main snapshot, including the base head. A superseded snapshot cannot promote:
the next qualification resolves the latest successful base afresh, rather than
modifying the in-flight composition or automatically publishing against an older
base. A failed job calls the owner rather than entering a blind rebuild loop.
Base is never rolled back to satisfy optional UI.
Final unexpected automation failures produce one deduplicated user attention
notification identifying failed stage/logs, retained base/full and latest repair
target; expected negative qualification cases are not failures. The notification
path cannot execute repair, weaken proof or modify installed state.

An exact-input component sidecar is signed with the existing production key and
binds generation, manifest digest, full source SHA and the base build-input
fingerprint. The fingerprint covers Core/builder/binding sources, relevant build/
lock dependencies, upstream/archive and Core toolchain, not Manager/TUI policy.
The producer verifies both the sidecar and base file inventory before reuse;
version equality or a successful unowned workflow alone never permits reuse.

#### GitHub-native transport and legacy compatibility

This is the contract for the new Core, not a claim that the installed Core already
implements it. The Worker-backed logical origin remains the existing-consumer
boundary until migration is qualified. GitHub Release storage and GitHub Actions
execution are independent choices; neither ordinary download nor local construction
requires a second hosting service or gh/model login.

The native acquisition path reads the existing bounded physical asset map
from GitHub and stages files under the unchanged signed logical paths. Admit the
selected index/manifest and generation/high-water first; controls remain in the
selected generation. Validate map identity, requested inventory, manifest binding,
flat asset spelling and allowed repository/generation routes. Maps supply locations,
not trust: signed final sizes/digests/modes and ordinary complete-candidate
admission remain mandatory. Preserve Range/caches/errors and custom/private HTTPS
origins. Reuse one cohesive native transport responsibility and existing publication
map/target consumers rather than introducing another updater, server or changed
installed layout.

New-client acceptance must pass real GitHub acquisition with Worker access denied,
including Base/Full nested helpers, reuse from previous physical releases,
interruption/Range refusal, bad maps/signatures/digests, rollback and unchanged
custom origins. Historical signed identities and immutable assets are preserved.
After admitting fresh signed controls and sizes, Core reuses matching files from
the retained target generation or current selection before downloading payloads.
Retained bytes must pass the same ownership, file type, size, mode and copied-byte
digest checks. Reuse never skips complete candidate admission or repairs a corrupt
immutable generation. A rollback followed by an authenticated force retry must
work with the retained target's runtime origin unavailable.
Existing Core clients still using the logical Worker origin require a proved
signed Core upgrade/bootstrap route; retain that real compatibility consumer
until the supported-client floor and explicit retirement policy permit removal.
Worker source readback alone proves neither new direct delivery nor legacy
migration. Implementation, qualification, publication and device activation remain
separate states. CURRENT's consolidated plan owns sequence and proof status.

#### Signed composition and public qualification

Each Release contains its changed payloads and a bounded `asset-map-v1.json`.
Composition may reuse any same-path payload from an earlier authenticated public
head when its signed digest and mode match the new inventory. Controls and
generation.meta always belong to the new Release. Full still requires the exact
input-matching Base; common payload routes are flattened from that Base's map so
they point to physical assets even when the Base itself reuses older payloads.
Existing missing maps permit no optional previous-head reuse; a malformed map,
wrong manifest binding, invalid signature or non-earlier head is an error.
Inherited routes to the candidate itself are rejected before removing copies.
Every staged payload is checked before omitting any generated copy. The transport
map is not trust authority; the complete new signed inventory authenticates every
logical file. The redirect adapter accepts only GET/HEAD, canonical bounded local
generations, fixed humtr/codex Release assets and exact admitted paths. It rejects
encoded/unsafe paths, queries, missing/foreign maps and indirect routing. It never proxies
payloads, chooses stable, handles credentials or controls device state. It needs
no secret/binding/storage or per-generation deployment. Administrative deployment
uses existing account credentials locally, never grants CI another credential.
Verify deployed source identity and public negative/positive requests before use.

Release CI stages immutable assets, verifies exact composition and complete public
HTTPS readback of their physical GitHub routes, separately maintains the supported
legacy logical-origin consumer, and runs disposable Android fresh install/
update/no-op/rollback proof, then non-forced stable promotion. Old clients retain
their ordinary HTTPS fetch and signature validation; they may download common
files again. Core's generic acquisition may instead copy a hash/size/mode-verified
owned retained file into private staging without hardlinks, then apply unchanged
complete-candidate admission. Unavailable/corrupt files fall back to authenticated
download. This optimization never interprets account, UI or rotation policy.

Both current lane heads, rollback/compatibility targets, every Base pinned by a
retained Full and every physical generation named by retained asset maps must be
preserved. Physical origins can be earlier Base or Full releases, independently
of the current logical Base. Deleting an unreferenced retired
release requires explicit retention scope and a reference check; no date-only
cleanup, published-asset overwrite or silent deletion is authorized. Immutable
Release delivery avoids the Pages aggregate-image cap, not all hosting limits.

Until the stable compatibility floor is newer than R10, the public stable target
must itself use the exact R10 browser-helper bridge layout even when its Core is
newer **and** its signed descriptor must carry the bounded legacy activation
doctor signal `upstream_doctor=unsupported` with exact
`creation_metadata=r10-browser-helper-bridge-v1`. Both parts are required for a
retained sequence-7 R10 client to consume the final stable generation directly:
the historical R10 activation gate otherwise treats user credential/provider
doctor health as candidate integrity and rejects a credential-free candidate
before activation. The signal does not weaken public doctor semantics in a
corrected Core: only that exact bridge marker plus `unsupported` combination is
remapped on the public doctor route to real upstream diagnostic execution, while
activation-time candidate integrity remains the signed version/runtime probe.
The official producer must therefore retain this signal on every public stable
candidate while the R10 compatibility floor remains supported; dropping it is a
backward-compatibility failure, not a canonicalization cleanup.
The admitted TUI may extend this exact numbered bridge with helpers/2,
helper_count=3 and its same-version termux-manager-tui-v1 identity/digest. Manager
is mandatory for that extension; no other additional helper is admitted. Historical
two-helper releases retain their exact behavior. Stable publication, staging,
Pages retention and public readback require either exact legacy7-file inventory or
exact extended8-file inventory, preserving the legacy activation-doctor signal.
The retained R10 client must directly consume the extended signed target through
its real public update/rollback path before any stable promotion. An ordinary
nested-browser frontend is valid local construction, not a stable floor replacement.
A newer canonical local generation may continue to use `browser/open/curl` and
`browser/manual/curl`; canonical nested paths are not an R10-readable public
stable target.

For any later authorized official stable promotion, the small
`update-index-v1` and `update-index-v1.sig` files are promoted together in one
Git tree and one commit on the selected stable publication branch only after the
selected release transport, complete readback, and disposable public-update smoke
are verified. Core runtime does not perform this promotion. The branch update is non-forced
and is based on the exact previously verified head, so a concurrent publisher
cannot be overwritten. Historical explicit publication procedures may retain
their previously accepted ordering, but automatic promotion must not expose a
new index with an old signature or vice versa. A failed, timed-out, incomplete,
path-renaming, Pages-deployment, readback, or pre-commit promotion never advances
the automatic stable pointer. An indeterminate final ref result is reported as
indeterminate rather than assuming either state. The optional best-effort flat
Release-asset publisher remains separate, uses bounded child waits, never uploads
the private key, and does not weaken this automatic promotion boundary. The
account credential and the release `update_key` private key are separate
authorities; account authentication alone cannot authorize a release for Core.

All source files are snapshotted into private staging, revalidated as regular
files, and copied without following symlinks. Final modes are applied before
file synchronization; the complete index and release tree are synchronized
bottom-up and the absent output directory is atomically published with
`RENAME_NOREPLACE`, followed by output-parent synchronization. A failure
leaves no accepted output, preserves an existing destination, and does not
alter the source generation or any live Core state. The official OpenAI
versioned archive remains the only upstream source authority; this publication
tree is wrapper-owned distribution content.

The release builder complete output boundary is durable: final file modes are set before the corresponding final file synchronization, the complete staging tree is synchronized bottom-up, the no-replace output publication is atomic, and the output parent is synchronized before the build reports success. A failed publication leaves no accepted output.

The accepted archive is gzip-compressed POSIX ustar. A per-entry POSIX PAX
header is optional and may contain only `mtime`; all other extended semantics
are rejected. There are at most 256 logical entries, paths are canonical relative
UTF-8 of at most 256 bytes, the compressed archive is at most 256 MiB, each
regular file is at most 384 MiB, and total regular-file payload is at most
512 MiB. Duplicate paths, absolute or dot/dot-dot paths, links, special files,
unknown entry types, malformed headers, and nonzero trailing content are
rejected. Archive ownership and modes are not output authority.

The official archive is admitted by its **layout-version semantic contract**,
not by a version-specific exhaustive resource inventory. Every tar entry must
still use one canonical relative UTF-8 path, be a regular file or directory
(except the already bounded PAX mtime header), be unique, and remain inside the
entry-count, per-file, and total-payload ceilings. Symlinks, hardlinks, special
files, path traversal, non-canonical paths, malformed headers, duplicate paths,
and unsupported PAX metadata fail closed. Unselected archive files are streamed
and discarded; they are never materialized in the candidate generation.

`codex-package.json` is parsed semantically as one bounded JSON object. It must
bind layout version `1`, the requested version, target
`aarch64-unknown-linux-musl`, variant `codex`, entrypoint `bin/codex`, resources
directory `codex-resources`, and path directory `codex-path`. Object member
ordering and insignificant JSON whitespace are not authority. Unknown extension
members may be ignored only when they are valid bounded JSON values and do not
duplicate a member name; changing any required semantic field or the layout
version fails closed.

The archive must contain exactly one regular `bin/codex`, exactly one regular
`bin/codex-code-mode-host`, and exactly one regular `codex-package.json`.
Those two binaries are the only selected upstream executables and must be
little-endian 64-bit static AArch64 ELF files with no `PT_INTERP`. Other bounded
regular files/directories, including additions or removals below the declared
resource/path directories and future non-selected package resources, do not
change the Termux generation and therefore do not by themselves require a
wrapper release-builder change. Linux-side helpers such as `rg`, `zsh`,
`bwrap`, voice resources, or later package resources remain excluded unless a
separate normative contract explicitly selects them.

Historical binary policy identities and exact instruction/offset digests are
immutable, source-specific admission records, not a general recipe for new
upstream releases. An accepted generation retains the exact qualified legacy
adaptation bound to its signed manifest; unknown inputs fail closed.
The common FD-33/34 and owner-safe FD-35 runtime invariants remain mandatory,
and each newly selected official source requires exact preflight, native launch,
permission and release acceptance. Never apply a known legacy binary offset
to a new release by version-name guessing.

The minimal upstream adaptation consists of generation metadata, the adapted
runtime and an unmodified root-level codex-code-mode-host beside it. A current
Core-bearing builder output additionally contains its bound Core and declared
browser helpers, plus optional admitted Manager/TUI; exact signed inventories
above govern complete bundles. Historical zero-helper descriptors remain retained
compatibility, not the current full output. The companion's root-level placement
is required because upstream resolves it beside its executable, not through PATH.
Every output is create-new in private staging and the absent destination is
published complete-or-absent. Failure never publishes a partial generation,
changes an existing destination, signs or activates content, or writes outside
the selected output parent.

The release builder accepts only a regular executable Core artifact of at most
64 MiB. It must enforce this bound before and during its private snapshot, so a
source that grows after initial inspection still cannot produce an unbounded
copy. The resulting descriptor binds the exact snapshot digest.

An upstream runtime is accepted only when all declared inputs and outputs are
bound in a generation manifest:

- upstream package identity and version;
- immutable source artifact digest;
- expected platform and architecture;
- exact patch-policy identifier and patch report;
- resulting runtime and helper digests;
- Core and optional Manager artifact digests;
- Core API and persistent schema compatibility;
- qualification result and creation metadata.

Binary adaptation must verify expected source occurrences, reject already
patched or unexpected layouts, and compare the result with the declared patch
policy. Upstream layout drift fails before activation.

Archive extraction must reject absolute paths, traversal, escaping symlinks,
special files, duplicate conflicting entries, and writes outside staging.

## 7. State and generation model

Code/artifact generations and mutable user state are separate.

The Core state/generation layout is:

```text
$PREFIX/bin/codex                                      stable public entrypoint
~/.local/lib/codex/core/generations/<id>/             immutable complete generation
  generation.meta                                     versioned local descriptor
  release.manifest                                    signed release/integrity inventory + release key
  release.sig                                         candidate-key Ed25519 signature over exact manifest
  release-authority.sig                               rotation-only current-key signature over exact manifest
  core                                                authenticated Core launcher for v4 bundles
  runtime                                             patched upstream executable
  codex-code-mode-host                                first-target companion beside runtime
  manager                                              optional Manager executable
  helpers/<index>                                      optional helper artifacts
~/.local/lib/codex/core/generations/.acquire-*/        private incomplete remote source; never activatable
~/.local/lib/codex/core/publications/<id>/             retained old Core publication cache; no official producer authority
~/.local/lib/codex/core/release-public-key.pem         bootstrap-only initial trust seed; never update fallback
~/.local/share/codex/core/activation-state            authoritative generation + bounded trust state
~/.local/share/codex/core/downloads-v1/<sha256>.part    private bounded untrusted/verified payload cache
~/.local/share/codex/core/activation-journal[.tmp]    crash-recovery transaction state
~/.local/share/codex/core/activation-state.tmp        atomic state publication temporary
~/.local/share/codex/core/core-entrypoint-rollback    one retained prior Core launcher
~/.local/share/codex/core/core-entrypoint-rollback.meta
                                                      rollback binding for that launcher
~/.local/share/codex/core/config/                     Core defaults and shared-profile source directory
~/.local/share/codex/core/config/snapshots/<id>/      immutable per-launch effective system settings
~/.local/share/codex/core/{servers,catalogs}/<id>/    bound native server record/socket/config snapshot
~/.local/share/codex/manager/                         Manager-owned mutable state
```

The Manager v1 state root is independent of the Core root:

```text
~/.local/share/codex/manager/
  profiles/<PROFILE_ID>/profile.meta                 Manager profile record
  default-profile-v1                                 explicit default account preference
  profiles/<PROFILE_ID>/home/                        selected upstream CODEX_HOME
  notifications/config-v1                            notification configuration
```

`profile.meta`, `default-profile-v1` and `config-v1` are versioned Manager records and
contain no tokens, cookies, OAuth values, private keys, or session bodies.
Manager creates profile directories with mode `0700` and record files with
mode `0600`. It publishes a new profile tree with create-new atomic rename;
configuration record replacements use a private same-directory temporary,
final-mode synchronization, atomic rename, and parent synchronization. A
profile directory is accepted only when its path components are real
directories rather than symlinks and its `PROFILE_ID` passes the command
grammar. The `home/` child becomes an upstream `CODEX_HOME` only for a child
launch; Manager does not interpret the files written there by upstream Codex.
The default profile is the existing upstream default home and is never copied
into this tree.

Manager does not persist or consume an implicit last-selected execution account.
Only explicit `profile default` owns the new default-profile-v1 preference. Existing
`state-v1` selection-history records are retired: they are ignored and left
unchanged, including malformed or symlinked remnants. They never select an
account or block execution. No migration or automatic deletion is introduced.

Manager profile records use exact UTF-8 text formats with a final newline.
New `profiles/<PROFILE_ID>/profile.meta` contains exactly:

```text
codex-manager-profile-v2
```

The existing exact private v1 form remains readable:

```text
codex-manager-profile-v1
id\t<PROFILE_ID>
```

The derived profile-home path is not duplicated inside metadata. Unknown
records, duplicate records, invalid UTF-8, or a mismatched ID invalidate that
record and never cause a path to be followed. Notification commands alone write
`notifications/config-v1`.

Manager does not persist a session transcript, a second session database or a
parallel discovery index. Upstream owns discovery/resume and conversation state;
the declared compatibility links above do not transfer that ownership. Auth-state
migration is outside ordinary Manager operation.

The authoritative state format is `codex-activation-state-v3`. It owns one
forward `update_key`, one `current` generation with its exact `current_key`, and
at most one `previous` generation with its exact `previous_key`. Ed25519 public
keys in state are the canonical 32 raw key bytes encoded as exactly 64 lowercase
hex digits. The `previous` generation and `previous_key` are present or absent as
one pair. There is no separate `verified` pointer, keyring, discovered-key set,
or unbounded key history.

Only content that has already passed release admission and candidate probes may
become `current`. Ordinary runtime selection uses only current; it does not
verify signatures, search keys/generations or implicitly select previous. The
separately bounded bare all-TTY update advisory may authenticate the channel as
specified in Section3, and best-effort artifact maintenance has its own explicit
retention boundary below. Neither exception makes network/OpenSSL/maintenance a
prerequisite for executing a valid current generation. The generation directory name is one safe path component
and generation content is complete before it can become `current`. Before
ordinary launch consumes that generation, Core must verify the selected
generation directory and every selected asset parent directory is a real
directory, and every selected runtime, Manager, helper, and descriptor file is
a regular non-symlink file. Ordinary launch must not follow a symlink from the
generation root to content outside that generation.

For durability, complete means that every regular file has its final bytes and final mode written and synchronized, every generation directory is synchronized after its children in bottom-up order, the complete candidate directory is atomically renamed into the generation root, and the generation root is synchronized after that rename. A failure before the candidate rename leaves no activatable generation. A failure after the rename but before generation-root synchronization may retain that exact complete candidate without changing authoritative state; a retry may reuse it only after signed installed-generation verification and repeating the required tree and root synchronization. A differing, incomplete, or unverifiable existing directory is a conflict and is never activated.

A generation is complete or absent. Candidate construction occurs outside the
active path. Ordinary public forward activation publishes one complete new
state: the candidate becomes `(current, current_key)`, the old current pair
becomes `(previous, previous_key)`, and `update_key` becomes the candidate
release key. For a non-rotating public release the old and new update keys are
equal; for a key rotation they differ. Local-derived activation is the sole
exception: its selected signer's public verifier becomes `current_key`, the former
current pair becomes the retained previous pair, and the existing official
`update_key` is copied byte-for-byte into the new state rather than being
rotated or replaced.

The authoritative journal format is `codex-activation-journal-v3`. Its before
and after records contain the entire bounded trust-and-generation state, not only
generation identities. Activation, rollback, and recovery therefore treat
`update_key`, both generation identities, and both generation verifier keys as
one transaction. After process kill, power loss, short write, full storage,
permission failure, or a stale journal, recovery must resolve to exactly one
complete old or new trust-and-generation state and must never synthesize a mixed
state.

A v4 coordinated update also retains one regular executable
`core-entrypoint-rollback` and its exact `codex-core-entrypoint-rollback-v1`
binding for the launcher active before the forward transition. This cache is
not a trust source: its digest and associated generation are verified before
rollback, and it is atomically replaced only while the activation lock is
held. Existing v3 generations may lack this cache; a v4 forward transition
must create it before changing the stable entrypoint.

Coordinated activation stages and verifies the complete signed generation and
its `core` asset first, snapshots the current stable launcher into the bounded
rollback cache, atomically replaces `$PREFIX/bin/codex` with the candidate
Core, synchronizes its parent, and only then commits the existing generation
pointer transaction. If the pointer commit fails, the old launcher is restored
before the failure is reported. A process kill between launcher replacement and
pointer commit leaves the new Core running against the old complete generation,
which is backward-compatible within the accepted Core API/schema boundary; the
next update may retry without accepting a mixed generation. Rollback swaps the
retained launcher cache and generation pointer as one locked operation and
preserves the exact one-pair boundary.

Core optimizes for the shortest correct release path rather than speculative
defense layers. Complete-or-absent generations, one atomic state transaction,
and recovery to one complete last-known-good state are the primary safety
invariants. Do not add a second trust updater, key-file transaction, fallback
key search, or recovery mechanism for a failure already covered by those
invariants. If a simpler base invariant makes an existing check, retry path,
pointer role, or fallback redundant, remove the redundant mechanism instead of
maintaining both.

One installer/updater transaction is the normal product model. Simultaneous
install or update attempts are not a first-class coordination feature and do
not by themselves justify locks, leases, fencing tokens, or a multi-writer
protocol. If attempts overlap, the required outcome is limited to preserving a
complete state boundary: one attempt may succeed while another fails or retries,
and recovery may return to the already complete last-known-good state. Because
the activation journal is a single pathname, Core serializes activation and
explicit recovery writers with one exclusive kernel-held lock on the state-root
directory. The lock is coordination only: it is not authoritative state, is
not read by ordinary launch, introduces no persistent lock record, and is
released by the kernel when the owning process exits. A contending writer
fails or retries before touching journal/state files. Launch must never observe
a mixed or partially constructed generation.

`previous` is the only rollback pointer and is not permission to build a
fallback ladder. Rollback is an explicit bounded activation-state transition;
ordinary launch never consults `previous` automatically. Rollback swaps only the
`(current, current_key)` and `(previous, previous_key)` pairs. It never rolls
back `update_key`, so a key removed from forward-update authority by an accepted
rotation cannot regain that authority merely because the user rolls back the
runtime generation.

## 8. Installation and update

The release bundle keeps one audited local `install.sh` delivery frontend and
adds one separate network-acquisition frontend, `install-online.sh`.
`install.sh` remains local-only and accepts exactly the two bootstrap forms
below, forwarding their original arguments without compiling, downloading,
signing, discovering releases, or implementing a second installation protocol:

```text
install.sh <CORE_ARTIFACT> <SIGNED_RELEASE_DIR> <BOOTSTRAP_PUBLIC_KEY>
install.sh upgrade-legacy <CORE_ARTIFACT> <SIGNED_RELEASE_DIR> <BOOTSTRAP_PUBLIC_KEY> <EXPECTED_LEGACY_ENTRYPOINT_SHA256>
```

`install.sh` must locate a sibling `bootstrap/codex-bootstrap` that is a
regular non-symlink executable and then `exec` it with the exact original
argv. A missing, symlinked, or non-executable sibling fails before any target
mutation. The frontend owns no trust, generation, entrypoint, or recovery
state; all validation and writes remain behind the audited bootstrap boundary.
The fresh form keeps the existing no-clobber rule, while the
`upgrade-legacy` form is the sole explicit legacy entrypoint handoff.

`install-online.sh [--variant base|full]` defaults to full and selects only the
fixed project head for that lane. It is only a bootstrap transport
frontend for a fresh target; it is not an update command, alternate trust
authority, package-manager installer, release producer, or legacy handoff. It
requires the existing Termux shell, `$PREFIX/bin/curl`, and
`$PREFIX/bin/openssl`; it never installs or discovers replacements for them.
Its production locators are fixed project HTTPS URLs, not caller-selected
mirrors.

Before trusting any network-selected generation, the online frontend retrieves
the bootstrap public key from the fixed project release locator and requires
the exact repository-pinned SHA-256 of those key bytes. It then retrieves the
canonical stable `update-index-v1` (full) or `update-index-base-v1` (base)
and sibling signature and verifies that
signature with the pinned bootstrap key before parsing the generation identity
or `release_base`. HTTPS transport alone is never release authority. The
verified index must have the exact stable four-record grammar already consumed
by Core, and the release base must be canonical HTTPS ending in the signed
generation identity.

The online frontend retrieves that generation's `release.manifest` and
`release.sig`, verifies the manifest with the same bootstrap key before using
its inventory, and accepts only the existing bounded v4 bootstrap inventory:
`generation.meta`, `core`, `runtime`, `codex-code-mode-host`, optional
`manager`, exactly one supported two-helper layout, and an admitted optional
`helpers/2` enhanced TUI in the full lane. Base requires its signed `local-base-`
identity and exact six-file inventory; full requires Manager and seven or eight
files. Selecting a lane cannot cross into another signed lane. It downloads only
those signed relative paths into a private `$TMPDIR` workspace with the Core
remote-acquisition per-file and total byte ceilings and applies only the
signed regular-file modes needed by the existing bootstrap admission. It does
not make downloaded file digests authoritative; the audited bootstrap/Core
admission re-verifies the signed manifest, descriptor, digests, modes, Core
binding, candidate probes, and activation transaction.

The network frontend obtains the audited local `install.sh` and
`bootstrap/codex-bootstrap` only from one immutable accepted repository commit
chosen by the immutable installer, recreates their sibling layout in the private
workspace, and invokes local `install.sh <CORE_ARTIFACT>
<SIGNED_RELEASE_DIR> <BOOTSTRAP_PUBLIC_KEY>`. It never writes
`$PREFIX/bin/codex`, the bootstrap trust pin, a generation, activation state,
or recovery state directly. All persistent writes remain owned by the existing
bootstrap/Core boundary. Failure removes only the private acquisition
workspace.

Fresh installation uses a small audited bootstrap because Core cannot install
itself before it exists. The bootstrap may only detect the environment,
retrieve or accept a local immutable release, verify it, stage Core, run a
self-test, and activate the initial generation.

The bootstrap command surface has exactly two local-only forms:

```text
codex-bootstrap <CORE_ARTIFACT> <SIGNED_RELEASE_DIR> <BOOTSTRAP_PUBLIC_KEY>
codex-bootstrap upgrade-legacy <CORE_ARTIFACT> <SIGNED_RELEASE_DIR> <BOOTSTRAP_PUBLIC_KEY> <EXPECTED_LEGACY_ENTRYPOINT_SHA256>
```

The first form is fresh bootstrap and never replaces a differing existing
`$PREFIX/bin/codex`. The second form is the sole supported legacy handoff. It is
not a `codex update` mode, Manager operation, package-manager action, or legacy
state migration. `EXPECTED_LEGACY_ENTRYPOINT_SHA256` is exactly 64 lowercase
hexadecimal digits. It identifies the one entrypoint the user has explicitly
authorized bootstrap to replace; it is not release authority, is not persisted,
and does not alter `codex-release-v3`. Candidate trust continues to come only
from the bootstrap public key and the exact signed release.

Fresh bootstrap uses the same authenticated Core publication boundary for its stable entrypoint: after self-test, Core creates a private same-directory temporary, writes the exact authenticated bytes, sets and verifies mode `0755`, synchronizes the file after its final mode, atomically publishes without replacing a differing existing target, and synchronizes `$PREFIX/bin` before invoking the stable entrypoint. If publication is interrupted after rename or its parent synchronization fails, activation has not been invoked; a same-Core retry must revalidate the target and re-establish parent durability before activation.

Bootstrap trust-seed publication is also owned by the authenticated Core. The shell bootstrap may snapshot and validate the supplied key, but it must not directly publish the persistent pin. Core writes the exact key bytes to a private temporary in the pin directory, sets final mode `0644` before synchronizing the file, verifies the parsed key, atomically publishes without replacing a differing existing pin, and synchronizes the pin parent before continuing. An existing matching pin is revalidated, repaired to mode `0644` when necessary, and re-synchronized; a differing, symlink, or special-file pin fails without replacement. A failure after rename but before parent synchronization leaves no activation state and is retryable only after the same-key verification and parent synchronization complete.

Before any bootstrap snapshot is consumed, each bounded input is checked and
copied through a bounded path: the bootstrap public-key PEM is at most 16 KiB,
the authenticated Core artifact is at most 64 MiB, `release.manifest` is at
most 128 KiB, `release.sig` is at most 1 KiB, and `generation.meta` is at most
64 KiB. A bound failure occurs before trust-seed, entrypoint, generation, or
activation publication. Core uses the same bounds for direct authenticated
inputs and for persistent descriptor loading.

Bootstrap classifies the target after resolving any recoverable activation
transaction:

- a **fresh target** has no authoritative v3 state or transaction residue and
  has no `$PREFIX/bin/codex`;
- a **same-Core bootstrap retry** has no authoritative v3 state or transaction
  residue and has one regular non-symlink entrypoint whose digest equals the
  authenticated Core artifact digest;
- a **legacy handoff target** has no authoritative v3 state or transaction
  residue and has one regular non-symlink entrypoint whose digest equals the
  explicit expected legacy digest and differs from the authenticated Core
  artifact digest;
- a **prepared legacy handoff** has that same legacy entrypoint and one complete
  recovered initial v3 state whose `update_key` and `current_key` equal the
  candidate release key, whose `current` equals the candidate generation, and
  whose `previous` pair is absent, and whose installed current generation
  verifies with `current_key`; and
- a **completed legacy handoff retry** has that exact initial state and an
  entrypoint whose digest equals the authenticated Core artifact digest.

On a prepared or completed handoff retry, the recovered v3 keys and the signed
installed generation are the authority. The supplied bootstrap key and release
must match that state exactly, but bootstrap must not use them to reinitialize,
reconstruct, replace, or weaken the authoritative state.

Any symlink or non-regular entrypoint, digest mismatch, incompatible bootstrap
key, other v3 state, mismatched prepared generation/key, retained previous pair,
or other transaction residue is a conflicting target and fails without
changing it. Bootstrap must not classify every non-Core file as legacy, scan for
another launcher, execute the legacy entrypoint, infer a legacy version from its
output, or inspect/import a legacy internal schema. It may inspect only the
entrypoint type, mode, and bytes needed to bind the explicit digest.

Legacy handoff is activation-first and entrypoint-last. Before modifying Core
state or the public entrypoint, bootstrap snapshots and verifies the same Core,
key, manifest, signature, descriptor, and Core-artifact binding required for
fresh bootstrap and completes the authenticated Core self-test. It then uses the
existing initial v3 admission, complete-generation staging, candidate probes,
installed-generation verification, and activation transaction to establish the
candidate as `current` with no `previous` pair. Only after that complete state is
recoverable may bootstrap create a same-directory private Core entrypoint
temporary, set and verify mode `0755` and the authenticated Core digest,
revalidate the legacy entrypoint against the explicit expected digest, atomically
replace `$PREFIX/bin/codex`, and durably synchronize its parent directory. That
completed parent-directory durability boundary is the handoff commit.

A completed handoff retry is successful only after it revalidates the authenticated Core target and synchronizes the entrypoint parent directory; an already-Core target does not bypass that synchronization. If the synchronization fails, bootstrap reports failure and remains retryable without changing authoritative state.

If interruption or failure occurs before the entrypoint commit, the legacy
entrypoint remains the public executable. A complete prepared initial Core state
may remain and is resumable only with the same authenticated release, bootstrap
key, Core artifact, and expected legacy digest. If interruption occurs after the
entrypoint commit, the new Core sees the already complete initial v3 state. A
completed handoff retry is idempotent. This ordering is the recovery invariant;
legacy handoff adds no second journal, backup launcher, trust source, generation
type, fallback path, or persistent-schema field.

Successful legacy handoff is one-way. It does not retain or automatically run
the old entrypoint, and ordinary launch never falls back to it. The initial v3
state has no `previous` pair, so `codex update --rollback` fails clearly until a
later successful Core update establishes one previous Core generation. Rollback
then remains exclusively a swap between signed Core generations and never
restores legacy code.

Legacy handoff may create or change only the stable Core entrypoint and the Core
roots declared in Section 7, plus private temporary files needed for that
operation. It must leave the resolver, auth, profile, session, Manager, package,
and all other non-Core state untouched. Except for the explicitly replaced
entrypoint, legacy-owned user/state files remain in place and are neither read as
migration input nor copied into Core state. Qualification compares protected-
state identities before and after the handoff rather than adopting a legacy
schema.

Installation from ready signed artifacts requires no on-device compiler. Only
explicit local-build selection may prepare missing compiler dependencies;
ordinary launch and download failures never start compilation or install tools.

After bootstrap, the Core owns every top-level `codex update` form. `install.sh`
is not an update dispatcher and must not bypass the authenticated local
admission, staging, probe, activation, or rollback path. The local, explicit
remote, and automatic channel forms use the same forward activation
transaction, and rollback remains the explicit swap of the one retained
complete previous generation. A bare `codex update` resolves the wrapper-owned
signed channel and public upstream freshness path. A missing newer release uses the
selected keep/local-build/remote-request policy. All channel and explicit transport
selectors preserve acquisition failures without starting a different operation. No path may install an
unpatched runtime or turn transport failure into official publication.

The automatic stable channel is represented by a bounded signed index. Its
default control URL is
`https://raw.githubusercontent.com/humtr/codex/main/update-index-v1`; a
deployment or test may supply the same canonical HTTPS URL through
`CODEX_TERMUX_UPDATE_INDEX_URL`. The URL is only a location hint: the index
bytes and its sibling `<URL>.sig` are verified with the recovered v3
`update_key` before Core trusts any field. The exact index format is:

```text
codex-update-index-v1
channel\tstable
generation_id\t<ID>
release_base\thttps://<host>/<path>/<ID>/
```

It has exactly these four records and a final newline. The generation identity
is a safe path component, the release base is the canonical immutable
generation base already defined below, and its final path component must match
the signed identity. After index signature verification, Core invokes the
existing signed remote-generation acquisition path. Index transport failure,
signature failure, malformed discovery, or release qualification failure is a
hard failure for that attempt; there is no fallback to upstream, a package
manager, an alternate mirror, or a raw package.

The signed index is a pointer to an already-adapted wrapper generation, not an
upstream source authority. The admitted upstream identity is the exact official
source commit and versioned official release archive tuple in release/upstream.json.
The hosted source producer compiles that source; the archive supplies the original
companion, and retained binary adaptations bind their original raw CLI. All paths
require qualification before signing. When the user selects local build, Core performs the exact official fetch/build/adapt
steps locally, signs the immutable candidate with the explicitly configured
authorized key or a fresh ephemeral device-local key, binds it to the
authenticated public baseline, and feeds it only to the
dedicated local-derived admission path. That path performs no official
publish/signing action. Core never downloads a raw upstream archive directly
into the active generation and never runs an upstream self-updater.

`codex update --build-local` is an explicit selector owned by Core. It is valid
only by itself. It resolves the same exact official stable metadata and versioned
archive used by the qualified release builder, verifies the metadata-bound
archive digest, builds/adapts in private staging, signs with the configured
authorized key or the default fresh ephemeral Ed25519 key, runs the normal
candidate probes, and atomically activates the result. A private-key environment
variable alone grants no signing authority; configured signing follows the
`config/build-signing-key` contract. No transport failure selects this route
automatically.

Local construction reuses the authenticated current generation's immutable Core
artifact when that generation declares one. The running updater may be an older
image whose stable pathname was atomically replaced; its process image is not
the selected build input. Historical generations without a bundled Core retain
their existing running-Core input until a coordinated generation is admitted.
Admission compares the constructed Core digest with the reauthenticated current
generation's declared Core; a file changed during construction cannot silently
be admitted as a new Core under the local-build authority.

A local-derived candidate records the authenticated public baseline generation
and sequence in its signed provenance and uses that same sequence in its signed
release manifest. This is not a new public sequence and grants no public signing
authority. Public anti-rollback after local-derived activation therefore compares
against the recorded baseline: a later authenticated public release must advance
that public sequence to supersede local-derived current. The dedicated
local-derived admission may install a different local generation at the baseline
sequence only because it is reached directly from the in-process qualified
official-source build; ordinary `--local`, `--remote`, and signed-channel
admission keep the equal-sequence/different-release rejection rule.

`codex update` must:

1. recover the authoritative v3 state and resolve one immutable signed wrapper
   release against its `update_key` (automatic update first verifies the signed
   channel index, while explicit remote update starts from its supplied base;
   explicit local build instead resolves and digest-binds one exact official
   upstream version before building);
2. enforce architecture, API, channel, and the release-sequence anti-rollback
   policy: a lower sequence is rejected; an equal sequence is a no-op success
   only when the authenticated candidate generation identity and signed release
   manifest exactly equal the installed current release, while an equal
   sequence with any different authenticated identity or manifest is rejected;
   only a greater sequence may enter candidate staging, probing, and activation;
3. download into a private staging location or accept an explicit local
   artifact;
4. verify the required current-authority and candidate-key signatures, exact
   digest/mode inventory, archive safety where applicable, and compatibility
   metadata;
5. resolve the exact official upstream version and package digest, then build,
   sign, and probe a complete candidate generation when local build is
   selected;
6. atomically publish the new trust-and-generation state;
7. retain one complete previous generation with its exact verifier key as
   rollback state;
8. report failure without damaging the active trust-and-generation state.

The retained two-helper coordinated v4 bridge is permitted for an already-installed
R10 Core whose signed release parser predates the TC-2 browser-helper paths. The
bridge is not a new trust or release format: it uses the recovered v3
`update_key`, the normal monotonic release sequence, the normal v4 Core binding,
and the exact signed digest/mode inventory. Its generation descriptor must bind
`creation_metadata` exactly `r10-browser-helper-bridge-v1`, `helper_count`
exactly `2`, and the two existing helper identities in order:
`termux-browser-open-v1` then `termux-browser-manual-v1`. Only for that exact
marker and helper contract are those identities stored and loaded from the
R10-readable signed paths `helpers/0` and `helpers/1`. The release inventory
must sign those exact two paths, digests, and modes. Missing or extra helpers,
identity/order mismatch, a marker/layout mismatch, or any attempt to use the
indexed layout without the exact marker fails closed. The separately admitted
three-helper TUI extension above is the only allowed extension of this bridge.

The bridge carries the same qualified browser helper bytes and policy as the
canonical TC-2 layout; it does not temporarily remove browser protection. A
new Core may retain read support for this exact bridge layout so the bridge can
run and can be the one retained rollback generation. Canonical non-bridge builds
use browser/open/curl and browser/manual/curl. Publication retaining the R10 floor
explicitly selects its exact numbered bridge layout as required above; no caller
may infer that layout from arbitrary files or rename signed paths implicitly. The bridge adds no alternate signing key, bootstrap path,
package-manager action, PATH widening, raw-runtime patch, or second activation
mechanism. Its purpose is retained R10-readable consumption and rollback until
an explicitly qualified compatibility-floor change, not a timed two-promotion
sequence that clients may miss.

The shared key/rotation contract below applies to retained codex-release-v3 and
current Core-bearing codex-release-v4. New coordinated Core bundles require v4;
v1 and v2 are not retained release compatibility paths. In addition to generation identity, monotonic
release sequence, supported channel, platform, architecture, Core API,
persistent schema, and the exact SHA-256 plus regular-file permission-mode
inventory of every load-bearing generation file, the manifest binds exactly one
`release_public_key`. That value is exactly 64 lowercase hexadecimal digits
encoding the 32 raw bytes of the candidate Ed25519 public key. Each canonical
inventory record continues to contain path, lowercase SHA-256, and four octal
permission digits. Special permission bits are rejected; every file must be
owner-readable and runtime/Manager/helper files must be owner-executable.

`release.sig` is always an Ed25519 signature by the manifest's
`release_public_key` over the exact manifest bytes. For an ordinary non-rotating
public update, `release_public_key` must equal the recovered authoritative
`update_key`; `release.sig` is then the only release signature and no
`release-authority.sig` is accepted. For a public key rotation,
`release_public_key` differs from `update_key`; `release-authority.sig` is then
mandatory and must verify over the same exact manifest bytes with the current
`update_key` before Core treats the candidate key as trusted, after which
`release.sig` must verify with the candidate key. A rotation is rejected if
either proof is missing or invalid. The dedicated local-derived path is not a
rotation: its manifest key is the fresh ephemeral local verifier, its signed
provenance must match the independently authenticated public baseline, and
activation stores that verifier only as `current_key` while preserving
`update_key`. No generic local/remote candidate receives this exception. Core
never accepts an adjacent key file, alternate-key search, network key lookup,
CA/PKI chain, key server, or unbounded keyring as authority.

Before forward admission, Core recovers the v3 state. Installed-generation
verification requires the signed manifest `release_public_key` to equal the
selected state verifier key before `release.sig` is verified with that key. Core
applies that rule to the installed current generation with `current_key` and uses
that signed current release for the release-sequence anti-rollback comparison.
An authenticated candidate below that sequence is rejected. An equal-sequence
candidate is returned as an already-current no-op only when its generation
identity and signed release manifest exactly equal the installed current
release; any other equal-sequence candidate is rejected before staging or
candidate execution. After staging and probing a greater-sequence public candidate, successful
public activation sets `update_key` and `current_key` to the candidate release
key, sets `current` to the candidate generation, and moves the former
`(current, current_key)` pair to `(previous, previous_key)` in the same atomic
transaction. When current is local-derived, its signed release sequence is the
integrity-bound public baseline sequence, so this comparison is still a public
anti-rollback comparison rather than a local sequence allocation. Dedicated
local-derived activation instead preserves `update_key` and stores only its
selected local signer's verifier in `current_key` as defined above.

`codex update --rollback` must recover any pending activation transaction,
require the one retained `(previous, previous_key)` pair, verify exactly that
previous generation with `previous_key`, and atomically swap the current and
previous generation/verifier pairs. It deliberately does not apply forward
release-sequence anti-rollback policy and deliberately leaves `update_key`
unchanged. When the generation rolled back from is authenticated local-derived,
rollback creates neither a public `update-hold` nor a rollback Core guard. When
the generation rolled back from is an ordinary authenticated public generation,
the accepted hold/guard rules are unchanged even if the retained previous
generation is local-derived. `--force` continues to require and target only that
authenticated public hold. Missing, malformed, mismatched, or unverifiable
rollback state fails without changing authoritative state; rollback never scans
generations, searches keys, restores a rotated-away key to forward authority, or
constructs a fallback ladder.

Initial bootstrap, whether fresh or an explicit legacy handoff, owns the only
permitted use of `~/.local/lib/codex/core/release-public-key.pem`. Before any v3
activation state exists, bootstrap may verify one admitted initial v3/v4 release
only when
the manifest `release_public_key` exactly equals that pinned key and
`release.sig` verifies with it; successful initial activation initializes
`update_key`, `current_key`, and `current` from that release with no previous
pair. Once v3 state has been established, Core update and recovery never treat
the bootstrap key file as a fallback or reconstruction source. Absence or
corruption of authoritative trust state after initialization fails closed rather
than re-authorizing an old bootstrap key. The two bootstrap forms share this one
initial trust rule and create no second bootstrap or update authority.

Automatic update checks must be bounded and fail open when a verified runtime
already exists. Ordinary `codex` launch must not depend on network success,
OpenSSL availability, key rotation, or silently run a package manager.

The updater must not depend on the same resolver implementation as the patched
upstream runtime without an explicit qualification proving that dependency.
Offline local-artifact installation and recovery are required before release.
On Termux, update and explicit rollback may use the already-present
`$PREFIX/bin/openssl` for Ed25519 verification and SHA-256; they must fail
clearly if it is unavailable and must never install a crypto package themselves.

`codex update --remote <HTTPS_BASE_URL>` adds only acquisition in front of the
same local admission/staging/probe/activation path. Core first recovers the
v3 state, so the authoritative `update_key` is known before any candidate trust
transition. The base is at most 4,096 ASCII bytes, begins exactly with
`https://`, ends in `/`, and contains no credentials, query, fragment,
whitespace/control byte, or backslash. It names one generation directory: after
signature admission, its final path component must equal the manifest generation
identity encoded as canonical UTF-8 URL-path bytes. Core may follow only HTTPS
redirects required by the selected release transport and never tries another
URL or permits a non-HTTPS redirect.

The remote control resources are `<base>release.manifest` and
`<base>release.sig`, plus `<base>release-authority.sig` exactly when the parsed
manifest key differs from the recovered `update_key`. Core may parse the bounded
manifest to select that fixed signature set, but the candidate key is not trusted
by parsing. For a non-rotation it verifies `release.sig` with `update_key`. For a
rotation it verifies `release-authority.sig` with `update_key` first and only then
verifies `release.sig` with the manifest candidate key. No generation content is
acquired before this control admission succeeds. The remaining remote resources
are the files named by the signed inventory. For the fixed mapped public identity,
Core also acquires the bounded GitHub asset map and resolves those files to physical
Release assets as specified above. Custom HTTPS origins retain the ordinary
logical-path transport.

For ordinary custom origins, inventory URLs are derived only by preserving `/`
separators and percent-encoding every UTF-8 path byte outside the RFC 3986
unreserved set. Every resulting resource URL is also at most 4,096 ASCII bytes.
After signed control
admission, Core reconstructs the exact inventory, verifies the assembled bundle
again through the same local v3/v4 admission, and feeds the existing staging,
probe, activation, and recovery path. This file-addressed release transport is
not an archive and does not weaken the archive-safety requirements for later
upstream-artifact work.

Remote acquisition creates directories with owner-only access, creates response
files owner-readable, and after each content digest succeeds applies the exact
signed permission mode before whole-bundle verification. Local admission checks
the same mode inventory before and after staging. Mode reconstruction therefore
does not depend on HTTP metadata, curl defaults, or a blanket executable bit.

Remote transport is exactly the existing `$PREFIX/bin/curl`, independent of the
patched upstream runtime and compatibility resolver. Before network I/O, Core
requires curl, `$PREFIX/bin/openssl`, and a valid recovered v3 trust state; Core
remote update never falls back to the bootstrap key file. The curl child loads no
user config, inherits no environment/proxy settings, permits HTTPS only, uses
the Termux certificate file/directory, applies a 15-second connect timeout and a
30-second control transfer deadline. For payloads, the former whole-transfer
deadline instead bounds time without additional response bytes: 300 seconds for
official and local-build inputs, 1800 seconds for release files, and 120 seconds
for Provider files. A transfer does not expire merely because its total duration
is long while bytes continue arriving. Curl payload output is unbuffered so that
Core observes received bytes before deciding whether a transfer is idle. It follows
redirects only with `--location` and
`--proto-redir =https`, and writes response bytes only to a caller-created
regular file. The manifest limit remains 128 KiB and each signature file is
limited to 1 KiB; each generation-file response is limited to 512 MiB and the
sum of all response bytes is limited to 1 GiB. Curl and Core both enforce the
applicable remaining byte bound. A transport or bound failure is terminal for
that explicit attempt and reports its cause and observed bytes; curl failures also
report exit status, HTTP status when available and elapsed time.
There is no automatic build or retry. Authenticated payload identities permit
a private `state_root/downloads-v1` content-addressed cache outside activation
staging. It reserves the selected object's authenticated size (or file limit),
evicts only recognized inactive private objects under its directory lease and
retains at most the protocol's 1 GiB total bound. Unknown files are preserved.
Verified objects are reused and partial objects resume on the next explicit attempt.
Partial bytes never authorize execution: final authenticated size/digest checks
precede staging. Control documents are always freshly acquired and authenticated.
A server rejecting Range produces an explicit error; corrupt completed objects
are discarded and never admitted. Cache contention never permits concurrent writes.
An ignored resume can fail as curl's Range error or, when its full200 response
exceeds the remaining-byte limit first, as filesize-exceeded. Both invalidate the
partial object for the next explicit attempt. Preserve the actual curl/HTTP cause;
do not relax byte limits or retry automatically to hide it.

Acquisition uses one create-new `.acquire-<pid>-<counter>` directory beneath the
generation root and create-new output files beneath that directory. Core attempts
cleanup after every acquisition outcome, and remote success is impossible until
that cleanup succeeds. A cleanup error is terminal and preserves the authoritative
activation pointers. An uncatchable process kill or a filesystem cleanup failure
may leave a dot-prefixed partial directory; Core never scans, launches, verifies
as a generation, or activates such a path. Bounded installation maintenance may
remove an abandoned partial directory under the retention contract below;
it must not add a retry, fallback, or registry merely for it. Remote success reports
`activated remote generation <id>`; every failure preserves the authoritative
activation pointers.

### Bounded installed-artifact retention

Core performs best-effort local artifact maintenance before ordinary execution
and after an explicit update attempt. Failure or contention in maintenance must
not change upstream argv, streams, exit status or block an otherwise valid
launch. The one authoritative activation state and its existing recovery journal
own all retained pointer roles; malformed or unrecovered state disables pruning.
Keep the current generation, the one previous rollback generation, any generation
bound by an effective rollback guard/local-derived public baseline, at most one
complete higher-sequence staged candidate for the existing crash/retry reuse
contract (highest sequence, deterministic generation-ID tie-break), and complete
generations still used by live local executables or open generation files.
All other Core-owned installed generations/publication caches are disposable.
Abandoned acquisition, candidate and local-build staging directories are also
disposable. Conversation activity is not inferred from executable timestamps.

Use directory flock leases rather than another registry: production update
preparation/activation and launch selection hold a shared lease on the existing
generation catalog parent; pruning requires its exclusive nonblocking lease plus the
existing activation-state writer lock. The launch lease survives until exec's
close-on-exec boundary. In-flight updates therefore protect their whole staging
work without PID-name guesses or deletion of another updater's source.
Inspect local process executable/open-file references before deleting retired
generations or staging; preserve active clients and their companion/helper trees. Inspect
only live process groups: a confirmed zombie with no surviving sibling threads
holds no executable/open-file references and must not disable maintenance.
Incomplete visibility of a live process still disables pruning. Where a
non-dumpable process changes its proc-directory owner to root, use its native
status UID fields to identify a local process; directory ownership alone must
not hide an unreadable local process from this check. If a process
reference read fails during exit, re-read its native process status: only a
confirmed single-thread zombie/dead process or a vanished process may be skipped.
The kernel exiting flag alone never proves that handles are released. A
single-thread process with that flag may be observed until the same confirmation,
using a total scan waiting budget of at most ten milliseconds; an unreadable,
still-live, multi-thread or unconfirmed process disables pruning. Inspect
only owned real directories, never follow substituted parents or deletion-root
symlinks, and never descend into profile/auth/session or resolver state.
Revalidate authoritative state while locked. Removing stale files is idempotent
and does not manufacture an activation success or invalidate rollback.

Retire only an obsolete Core-owned shared server whose private owner record,
socket peer and executable bind to its recorded signed runtime, and whose open
FDs show only its listener socket and no conversation/writer-lock handle.
Coordinate with its existing namespace lock. Request upstream's graceful-only
SIGHUP shutdown; never force an active client or turn to stop. Live process/file
references protect its generation until shutdown finishes. Dead obsolete server
records/config snapshots may then be removed. Unknown ownership or incomplete
process visibility keeps the affected files and never authorizes termination.
A missing process executable path alone does not prove the entire process group
has exited. Removing a dead server record/config snapshot uses the same native
exit confirmation as generation retention, preserving records for surviving
sibling threads or unknown visibility.

Historical operator-authorized cleanup does not grant recurring permissions
to delete conversations, backup copies or unknown state. Retention and cleanup
are restricted to verified owned inactive artifacts and must preserve active
writers, signed current/rollback generations and canonical user records.

Automatic channel discovery uses the same curl, certificate, timeout, and
owner-only temporary rules. It fetches only the bounded index and its sibling
signature, verifies the exact index bytes with the recovered `update_key`, and
then delegates to the explicit signed remote-generation path. The index is not
an alternate trust source and never authorizes a raw upstream package.

Candidate activation integrity is narrower than public diagnostic health. After
signed descriptor/manifest/index admission, exact signature/digest/mode checks,
qualified Termux environment construction, release-sequence/anti-rollback
checks, and staging, Core must execute the exact candidate upstream
`--version` probe in that qualified Termux environment. A failed version probe,
qualification failure, admission failure, or transaction failure rejects the
candidate before the new launcher/pointer pair is committed. Activation must
not require the full upstream `codex doctor` command to exit zero: credential,
provider, and external-network health belong to the diagnostic surface, not to
release-runtime integrity.

For the bounded backward-compatible transition from the public sequence-10 R10
bridge Core, and only for exact `creation_metadata =
"r10-browser-helper-bridge-v1"`, the signed descriptor may declare
`upstream_doctor = unsupported` as a legacy activation-capability signal. The
legacy Core may use that signal only to omit its historical activation-time
upstream-doctor health gate. The corrected Core must map that exact marked
transition back to supported public-doctor behavior and actually execute the
upstream doctor. The release builder must reject this compatibility signal for
any other creation metadata. It is never a fabricated healthy result and never
weakens signature, inventory, mode, version-probe, anti-rollback, atomic
activation, LKG, rollback, or CAS requirements.

## 9. Doctor contract

`codex doctor` is read-only. It runs the raw upstream doctor when supported and
adds a Termux Core/Manager diagnosis without recursively invoking the public
launcher. Human output begins with the bounded, sanitized upstream doctor
output itself, preserving the upstream doctor's own header, layout, and safe
ANSI SGR sequences; Core does not prepend an `[Upstream Codex doctor]` wrapper
heading or a duplicate synthetic status line. `--json` emits one redacted
envelope rather than concatenated documents:

```json
{
  "schema_version": 2,
  "upstream": {"status": "healthy", "output": "..."},
  "termux_core": {
    "status": "healthy",
    "generation_id": "...",
    "layout": "root-code-mode-host-v2",
    "runtime": {"status": "healthy"},
    "code_mode_host": {"status": "healthy"},
    "sandbox": {"status": "unsupported", "reason": "bwrap is not used"}
  },
  "manager": {},
  "summary": {}
}
```

When human output is connected to a TTY and `NO_COLOR` is absent, Core gives the
upstream doctor a bounded pseudo-terminal so its own headings, progress
cleanup, and ANSI SGR markup retain the upstream layout. `--color` is the
explicit exception for an interactive caller whose outer Termux/AI wrapper
injected `NO_COLOR`; Core removes that variable only from the upstream child
and enables the same bounded PTY path. Non-TTY output remains plain, and
explicit `NO_COLOR` remains plain unless `--color` was requested. Core
normalizes carriage-return/erase controls and preserves only safe SGR sequences
before composition. If the upstream doctor is unsupported or produces no
output, Core emits only a concise status diagnostic for that missing upstream
portion before the Termux section.
The Termux doctor section follows the legacy wrapper presentation: a `Codex
Termux Wrapper Doctor` status header,
`Runtime`, `Support`, `Wrapper`, `State`, and `Store` groups, colored health
rows when enabled, and a bounded summary. It reports the selected generation,
root-level code-mode companion or migration state, and the explicit reason that
Linux bwrap sandboxing is not used.

Unsupported upstream or Manager diagnostics are represented explicitly and do
not fabricate success. Diagnostic failure returns nonzero while preserving a
valid machine report when `--json` was requested. After valid doctor argument
parsing, an upstream probe/setup failure is represented as a redacted
`unhealthy` upstream status with empty output rather than an error string, and
the command still returns its nonzero health-failure status. Upstream doctor
output is capped at 64 KiB, has terminal control sequences and credential-like
values redacted before composition, and is emitted as a JSON string in the
envelope. Usage errors remain distinct from health failures and API
incompatibility.

A credential-free disposable release proof therefore does not require doctor
exit zero. It may accept only the documented success or health-failure exit
class when the bounded JSON envelope is valid, Termux Core/runtime/code-mode
health is `healthy` for the exact activated generation, and the upstream
section proves that the real upstream diagnostic ran by reporting
`healthy` or `unhealthy` rather than `unsupported`. The exit code must agree
with the composed summary. Missing user credentials or external provider
reachability may make the upstream diagnostic unhealthy, but must not be
reclassified as candidate runtime-integrity failure.

Doctor must not expose tokens, OAuth data, cookies, auth-derived private data,
notification content, or unredacted session content. A filesystem snapshot
before and after doctor must be unchanged except for operating-system access
metadata outside product control.

Generation descriptors written by the current release builder use
`codex-local-generation-v2` and the root-level companion layout above. Core
continues to read an already-installed `codex-local-generation-v1` generation
with `compat/codex-code-mode-host` only as a bounded migration input; it never
produces that layout, and it never treats an unlisted root-level symlink as the
companion. The next authenticated generation is the required permanent repair
for such an old layout.
## 9A. Post-Core Termux compatibility extension

The upstream runtime remains the official `aarch64-unknown-linux-musl` Codex
artifact adapted by this product. Running that Linux-target executable inside
Termux does **not** activate upstream compile-time `target_os = "android"`
branches. Compatibility decisions therefore belong to an explicit Core-owned
Termux boundary rather than to accidental Linux desktop behavior.

This extension does not reopen or weaken the accepted Core completion
invariants. The resolver, signed-generation trust model, wrapper-owned update,
coordinated Core/generation activation and rollback, profile/session/auth
boundaries and sandbox policy remain unchanged. Ordinary signed installation
requires no on-device compiler; explicit source-build paths, including the tunnel
recovery contract, are separate opt-in operations.
It also does not authorize an unbounded raw-runtime byte patch: the selected
version's exact termux-fd-remap policy and retained-generation policies remain
bound to their qualified artifacts. Any additional binary rewrite requires a
separate normative amendment that names its exact bounded substitutions and
qualification evidence.

### Timezone compatibility

Resolve the musl runtime's user-visible timezone mismatch through Core's launch
environment. Respect explicit working POSIX/path TZ settings and resolve selected
IANA names, including the Android system timezone when TZ is unset, using the
active Android timezone data. The preferred method to qualify is a verified
zone-specific TZif supplied as `TZ=:/absolute/private-cache/zone.tzif` to the
existing runtime. Core's common child environment boundary supplies the same
selection to bare runtime, optional frontend and new shared-server launches.
The frontend uses that TZif through chrono's Local conversion, including absolute
resume-picker dates; it does not maintain another timezone database or parser.
Existing processes retain their launch environment until their ordinary restart.

Discover Android's active APEX tzdata first, then legacy updated tzdata and the
system image when preceding locations are absent. A present but unreadable or
invalid source is diagnosed rather than silently replaced by older data. Validate
the selected zone's index bounds and TZif structure before publishing it in a
private cache. Compare actual selected-zone bytes on each launch and atomically
replace changed data; no version-only cache hit may conceal a data update.
Explicit empty, POSIX-rule and file-path TZ settings remain under user control.
CURRENT records implementation and qualification separately; source changes do
not by themselves establish installed musl behavior.

Keep the upstream Local-based formatters and stored/server UTC timestamps. User
displays that explicitly format UTC require a separate narrow upstream-TUI fix;
do not add an unconditional offset or force Korean time on other users. Bare
Core must support the result without Manager or shell edits, and timezone
discovery/data failures must be diagnosable without preventing ordinary launch.

### Browser and external URL intents

An upstream request to open an HTTP or HTTPS URL on Termux must use one
Core-qualified Termux opener capability rather than Linux desktop discovery.
The preferred local capability is the absolute Termux URL opener under
`$PREFIX/bin`; the adapter passes exactly one already-formed URL argument and
must not evaluate a shell command string. Opener selection is child-local: it
must not write user shell configuration, desktop configuration, profile state,
or upstream config files. If the capability is unavailable or fails, the
operation must fail soft at the browser-open boundary and preserve the
upstream flow that exposes or otherwise allows manual use of the URL; it must
not make authentication state or the runtime unusable.

The same policy covers primary login, TUI onboarding/history URL actions, and
MCP OAuth browser intents. A fix limited to one login call site is incomplete.

### MCP OAuth registration and provider-policy boundary

MCP OAuth client registration and browser opening are separate compatibility
boundaries. An authorization-server response such as
`invalid_client_metadata` with `redirect_uri is not allowed by the account
configuration` is provider/account policy evidence, not by itself a Core
compatibility defect, when Codex submitted a syntactically valid callback that
matches the selected MCP registration mode and the server simply has not
allowed that callback. Core must not rewrite, relax, or bypass an
authorization server's redirect allowlist merely to turn that response into a
successful registration, and source qualification must not mutate external
OAuth account configuration.

For the selected upstream release, qualification must revalidate the actual
MCP registration path. Codex may use advertised CIMD when its prerequisites are
met or Dynamic Client Registration otherwise. Native loopback DCR can register
an HTTP callback on `127.0.0.1`/`localhost` with the active listener port and,
when issuer-bound responses are unavailable, a server-specific callback path.
The authorization server used for end-to-end qualification must therefore be
configured outside Core to accept the exact callback form that the selected
upstream release legitimately requires. A provider-policy rejection before
that prerequisite is met is recorded as an external configuration blocker,
not patched in Core.

TC-2 acceptance begins only after that provider prerequisite is satisfied. It
then proves the full MCP OAuth path: registration strategy selection,
authorization URL production, the shared Termux browser opener, loopback
callback receipt and validation, token exchange, and credential persistence in
a disposable `CODEX_HOME`. If the server is proven to allow the exact callback
and registration still fails because Codex generated a callback inconsistent
with the server's advertised metadata or the applicable OAuth/MCP contract,
that becomes a client compatibility defect and may be patched only after the
normative contract is updated with the observed mismatch. Credential values,
client secrets, authorization codes, and tokens never become test evidence.

### App-server daemon and remote-control authority

No Termux command may create, select, execute, update, or trust an unmanaged
Codex installation under `$CODEX_HOME/packages/standalone` or
`$CODEX_HOME/packages/app-server-daemon`, and no Termux app-server path may
fetch or execute an upstream installer or self-updater. An upstream runtime may use
the latter daemon package root with a legacy standalone fallback; both remain
independent package authorities rather than signed Core generations. Bare
`codex update` and all runtime replacement remain Core-owned signed-generation
operations.

For interactive local upstream TUI/resume/fork launches, Core may start one
shared app-server directly from the already qualified active generation before
unchanged upstream exec. This server receives the same FD-33 resolver, FD-34
system configuration, FD-35 Termux temporary directory and qualified child environment. Its private profile-local
socket is exposed at the upstream default control socket through one validated
Core-owned socket symlink into a private short generation/profile namespace.
Both the Core rendezvous and the upstream physical socket pathname must fit
the 107 encoded-byte bound. For an FD-35-backed shared server, the physical path must be
formed beneath the canonical private temporary root using verified UID/identity; the standard Termux
temporary root fits. Disposable proof must provide a separate private temporary
root short enough for this native path rather than nesting it under a long
fixture root. Core does not bypass the native private socket ownership checks;
profile identities and authentication remain distinct. Core binds a reused
server to its recorded PID, kernel socket peer PID, and exact qualified executable
through `/proc`, and
serializes startup with a kernel-held directory lock. Generation changes choose
a new namespace and preserve existing clients on the old server; they never
kill user turns as a side effect of launch. Server system configuration is a
private immutable snapshot of the effective Core defaults/optional fragment and
requirements for that identity. Another launch never refreshes a live snapshot. A substituted socket,
record, directory or runtime is rejected. Startup is bounded; failure does not
invoke an installer or execute any package outside signed generations. Explicit
user embedded/remote/configuration selections retain their upstream semantics.

A same-generation, signed optional frontend may advertise deferred native
startup through its non-mutating `--termux-backend-capabilities` query, which
must return exactly `codex-termux-resolved-startup-v1` before configuration,
terminal initialization or a server connection. Only a bounded, successful
probe enables `CODEX_TERMUX_BACKEND_STARTUP=resolved-v1` with Core's planned
socket; no server or secondary installer starts merely for this probe. The
frontend then resolves its upstream settings and asks Core for the qualified
local backend. Unknown or older helpers retain their supported eager path.
The helper and Manager must belong to the selected signed generation, not a
stale preview. Core does not interpret model policy or override user-supplied
upstream profile/CLI options; an incomplete optional frontend leaves the
independently qualified Core available.

A qualified local TUI may submit its upstream-resolved model catalog
to Core's private `__app-server-catalog-v1` operation on stdin (at most4 MiB).
For deferred startup without a resolved catalog, the frontend requests the same
native server through private `__app-server-default-v1`. Both private commands
also accept bounded --config VALUE/--profile VALUE pairs for the invocation;
their existing no-argument callers remain unchanged. The default command does
not consume stdin. Core validates the argument transport but delegates the
meaning, precedence and policy of their values to upstream. Both operations return
only a validated socket and share the existing coordinator. Failure after deferred
selection is explicit; it never retries through an embedded executor or starts a
second native server. A catalog request remains distinct from an absent catalog;
an empty/invalid catalog is not a request for native/default behavior.
Core returns only the exact qualified server socket, and uses the existing server
coordinator, environment, descriptor and retirement path. Catalog bytes, profile
home, runtime, invocation arguments and effective system config/requirements
jointly identify reuse; a changed catalog at the same source
filename must not reuse stale metadata. Core stores an immutable private snapshot
and supplies its path only to backend startup, never to ordinary upstream argv.
It does not interpret profile TOML, resolve provider credentials or start another
executor. The frontend remains responsible for loading selected user/CLI/project
configuration with upstream code. Existing native servers and active siblings
remain intact. Remote/embedded launches are unaffected; failure to prepare a
requested catalog is explicit, never a fallback to an unqualified embedded backend.
Catalog or invocation-configured records live under Core's private `catalogs` namespace and never
retarget the default `servers` rendezvous. Current Manager discovery
validates both families, including exact catalog content, runtime and peer PID;
generation retirement applies the same existing-client/writer protections to both.
The existing server-record codec has one implementation shared by Core and
Manager: owner fields, family/namespace binding, opaque catalog/invocation bytes
and bounds. Records without invocation overrides retain their existing encoding
and namespace for their live consumers. An invocation record adds its explicit
marker and private argument snapshot; reuse validates complete bytes, not only
the routing hash. The snapshot shares the existing16 KiB configuration-record
bound, including its UTF-8 arguments and NUL separators. Unsupported/malformed
arguments fail before server launch.
Core owns this execution-record contract; Manager consumes decoded bindings for
its task UX rather than independently defining Core's catalog/storage format.
This is a dependency-free source library compiled into each executable, not an
installed shared runtime or a third management authority. Core never imports
Manager, and Core-only construction and execution require no Manager artifact.
Callers retain their filesystem trust checks and revalidate live process/peer
identity before use. The codec performs no writes, process control, model-schema
interpretation or provider selection. This introduces no registry, service or
second runtime authority.
Core launches no upstream daemon updater and never creates a packages tree.
The historical public daemon/remote-control lifecycle fence remains in force.

Configuration-bound execution records add the `config-v1` owner marker after
home/runtime, followed only by an optional `catalog-v1` marker. They retain the
existing `servers`/`catalogs` families. Their namespace also binds the exact
opaque `config/config.toml` and optional `config/requirements.toml` bytes, with
absence distinct from an empty file. Each config file retains the16-KiB bound.
Core and optional consumers share this codec; it interprets no Manager/provider
policy. Reuse compares the complete requested snapshot, not only its short routing
hash. A live snapshot is immutable. Use one current codec, without historical
format emulation or a staged reader/writer release. Unsupported old experimental
records authorize no operation. Preserve their running processes and user data;
profile mutation and takeover still require native process/writer checks. Test
the complete new composition, its install/update/rollback, discovery and retirement
directly. Compatibility with superseded experimental codecs is not an admission
requirement.

A daemon-backed app-server command, including `remote-control start`, may run
only if it is bound to the currently qualified signed generation and cannot
enter an upstream daemon-package or standalone installation/update loop. If
that binding is not available, the command must fail closed with a stable Termux-specific
unsupported result before creating unmanaged package state or performing
network I/O.
The foreground `remote-control` form that owns a private temporary socket is a
separate path and remains usable when otherwise qualified.

An app-server Unix control socket for a supported Termux path must use a
private, profile-distinct short runtime namespace. Its encoded pathname must
be at most 107 bytes so it fits the Termux/Linux `sockaddr_un.sun_path[108]`
bound including the terminating NUL. The short runtime identity may derive
from the logical `CODEX_HOME`, but it must not relocate, alias as authority, or
merge profile auth/config/session state. Different logical profiles must not
collide, and symlinks or pre-existing untrusted entries must not become socket
or profile authority. Default and Manager-created profiles use the same rule.

### Linux-target Android capability mismatches

The Linux-target runtime must not treat X11, Wayland, a desktop Secret Service,
or another Linux desktop facility as present merely because the compile target
is Linux. Each observed desktop-only capability is either adapted through a
bounded Termux/terminal mechanism or reported unavailable without corrupting
TUI state.

Image clipboard paste must not reach a usable Linux desktop or WSL clipboard
transport on a normal Termux session unless a separately qualified backend
exists. For an upstream artifact compiled as Linux, Core may satisfy this
without a new runtime byte patch only when release-specific source review proves
the image path is bounded to desktop `arboard` plus the enumerated WSL fallback:
the child-local capability projection must make X11 and Wayland transport
unreachable before runtime launch and remove inherited WSL selector variables.
Qualification must also reject a positive WSL kernel signature when the exact
upstream kernel probe is readable. If that source is unreadable under the same
credentials the child will inherit, qualification may instead rely on that same
probe remaining unavailable together with removal of every remaining upstream
WSL selector. The upstream Linux function may then execute only far enough to
fail before a desktop connection; no PowerShell/WSL subprocess fallback may
become reachable. In the
absence of a separately qualified clipboard backend the user-visible outcome is
a clear unavailable result. Text copy may use a terminal-mediated fallback such
as the already supported OSC 52 path; native clipboard failure alone must not
make the TUI unusable. Compatibility code must not bypass Android permission or
application boundaries. This capability fence does not widen the raw-runtime
byte-patch allowlist.

The release qualification for every newly supported upstream version must
inspect material `target_os = "android"` versus `target_os = "linux"`
behavior in the upstream source used by the artifact. A new Android-specific
upstream guard that affects a Termux-visible feature is a compatibility review
input even though the shipped binary is Linux-target.

### Host-tool and credential fallback discipline

Because the accepted package adaptation excludes upstream `codex-path/rg`, a
fresh Termux installation must not silently assume that system `rg` exists for
ordinary product correctness. A feature that still requires it must either
have a qualified signed helper/fallback or degrade explicitly with bounded
doctor/user-facing evidence. The bootstrap and Core recovery paths must not install a
package manager dependency automatically. Bundled zsh remains excluded unless
a later accepted feature makes it an explicit requirement.

MCP OAuth `Auto` credential storage must apply one coherent authority across
load, save, refresh, and delete. If keyring storage is unavailable and file
storage is the resolved fallback, logout/delete must be able to remove the
resolved file credential without requiring a functioning desktop keyring.
Credential contents never become compatibility evidence.

### Compatibility acceptance gate

A compatibility slice is accepted only with focused tests for its exact
boundary plus the existing protected-state and workspace regressions. Tests
use disposable roots and may not mutate the live resolver, launcher/runtime,
auth, profiles, sessions, Manager state, or network configuration. For any
app-server slice, acceptance additionally proves no standalone Codex tree or
upstream installer request is created, custom-profile socket paths stay within
the pathname bound, profile identities do not collide, and foreground
remote-control behavior outside the daemon path is preserved. For browser and
clipboard slices, acceptance covers every enumerated upstream surface rather
than one observed call site.

## Release execution

One maintained release workflow owns construction and publication. main is a
small immutable-source caller, with no second implementation or manually mirrored
producer. The caller selects an accepted exact implementation commit; nested
publication logic comes from that same source. Job-specific permissions separate
read-only construction/smoke from signing/public mutation. Signing secrets appear
only at the signing boundary. A manual publication needs explicit authorization;
scheduling authorizes only a qualified strictly newer upstream pair. An explicitly
authorized current-version rebuild uses its exact qualified upstream/archive and
normal next sequence, with all ordinary integrity/publication gates intact.

Remove completed one-shot acceptance/deployment code paths, old mutation workflows
and experimental preview construction. Existing immutable migration releases and
required consumer compatibility remain ordinary product support, not executable
historical producer modes. Preserve normal signature/inventory verification,
LKG continuity, candidate site size, complete public readback, disposable consumer
proof and non-forced exact-parent CAS. No public release or installed activation
is implied by a development-plane change.
