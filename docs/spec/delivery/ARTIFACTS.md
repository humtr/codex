# 6. Artifact and patch qualification

> Normative annex of [SPEC](../../../SPEC.md); [INDEX](../INDEX.md) is navigation only and [CURRENT](../../../CURRENT.md) is observed status.


Upstream freshness, request acceptance, build/port qualification, signed public
publication and device activation are separate states. A controller run's success
is not proof that its downstream source/runtime/frontend jobs passed or that the
requested version is installable. An unsupported adaptation is a blocked port,
not an indefinitely pending build; preserve the installed release and report the
failed version/source/stage and original job URL. A failed pipeline is not repaired
by retrying identical inputs or compiling the same unsupported tuple on a device.

## Component composition and update

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
Actions artifact transfer wraps the verified six-file raw directory in one
`runtime.tar` so the transport cannot discard executable permissions. The CI
transport helper admits only bounded, unique, flat regular members, restores
their recorded modes into an absent private directory, then invokes the same
Rust verifier. It does not infer executable modes or repair a rejected input.
The selected original producer source, run, job and immutable Actions artifact
ID/digest remain the workflow's responsibility. The downloader checks the exact
API-bound ZIP digest and byte count before admitting its single `runtime.tar`.
An advertised loose or mismatched Actions artifact is a transfer failure, not a
reason to rebuild, chmod-repair or choose another run. Remote raw reuse does not
trust a loose cache or a self-authenticating receipt stored beside its bytes.
Local execution and remote runners call the same Rust producer. The raw compiler
identity binds source/preparation/normalization code, the exact upstream tuple,
lock audit, dependency closure, Cargo compiler settings, toolchain and build steps.
Only audited exact effect-isolated transport/control steps are normalized away;
new or changed setup, environment and unclassified steps remain bound. Consumer
transport validation and identity calculation are not upstream compiler inputs.
Cargo compile caches remain optimization inputs that Cargo must revalidate.
A completed original producer job can supply identical raw inputs even if another
job of its parent run failed. Its source/run/job/artifact and file digests remain
in `termux-runtime-production`; this records production, not qualification.
Every current consumer reruns the Rust verifier and its exact-source qualification.
An absent or expired original artifact permits fresh production; it never authorizes guessed reuse. Signed component metadata's `runtime_inputs_sha256` additionally
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
## Source and executor qualification

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

## Accepted-source publication

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

Periodic preparation may discover and qualify independently authenticated new
upstream inputs. Publication additionally requires the immutable-main exact-source
release intent defined in [SIGNING](SIGNING.md). Base does not wait for
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

## Exact-input component reuse

An exact-input component sidecar is signed with the existing production key and
binds generation, manifest digest, full source SHA and the base build-input
fingerprint. The fingerprint covers Core/builder/binding sources, relevant build/
lock dependencies, upstream/archive and Core toolchain, not Manager/TUI policy.
The producer verifies both the sidecar and base file inventory before reuse;
version equality or a successful unowned workflow alone never permits reuse.
