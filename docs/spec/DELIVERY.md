# Runtime, release, and Manager delivery contracts

**Status:** Normative detail of [SPEC.md](../../SPEC.md).
This document defines release-independent artifact, installation, update,
rollback, device compatibility and qualification behavior. Exact upstream
release identities and prior patch/admission reports are evidence outside
this contract, not a condition requiring a new specification on every upgrade.

> **Stage scope:** Active Stage 1 standalone delivery (`install.sh`, `codex update --upstream|--path|--rollback`, `package-root`, adaptive `termux-fd-remap`/`termux-uds-fd35`, and `codex doctor`) is documented alongside deferred Stage 2 optional extensions (`--signed` Ed25519 verification for published Wrapper and optional self-built addon bundles, `publish`, Manager/TUI composition, and shared-server snapshots). Under local upstream patching, official OpenAI upstream archives (`codex-package-aarch64-unknown-linux-musl.tar.gz`) remain verified against OpenAI metadata + SHA-256 and locally patched into `bin/codex` on-device without remote CI re-signing; cryptographic signing applies to the Wrapper and optional self-built addon distributions hosted by this repository. Stages and implementation status live in [CURRENT.md](../../CURRENT.md) and [docs/plan/STAGE2_PLAN.md](../plan/STAGE2_PLAN.md).

## Base qualification

### Independent unsigned base qualification

The reusable producer also has an explicit `base_only` qualification mode. It
constructs only the qualified upstream runtime, Wrapper, Code Mode host and browser
helpers through the existing release-builder. Manager and TUI are absent;
their compilation, downloads and qualification do not gate this mode. Complete
base inventory, digest/mode checks and actual Android/AArch64 Wrapper/runtime smoke
remain required. A base result is not an integrated product qualification.

This mode is read-only with respect to publication. It refuses `publish=true`
before artifact acquisition; signing, Release/Pages staging, stable promotion and
device activation are never reachable. Its unsigned CI artifacts have distinct
base-qualification identities and cannot replace an integrated candidate. It
adds no update channel, installed pointer, runtime mode or public CLI command.
The main publication path still requires the complete qualified backend/frontend
pair and retains the current generation on an unsupported update.

Base qualification may run from a repository push or manual dispatch
with read-only repository permissions. Source must equal the event's immutable
commit. `rebuild_current` selects the version and source archive digest from the
authenticated stable descriptor independently of TUI pins; otherwise the
ordinary official upstream discovery is used. An unsupported upstream artifact
still fails existing builder qualification, rather than weakening patch admission.
Skipped integrated construction distinguishes a non-newer release from missing
frontend qualification or an archive mismatch; it must not report all three as
already up to date. Independent base qualification and feature-preserving public
promotion are different guarantees; section4.6 defines their final boundary.
### Optional upstream compatibility canary and base/full lanes

Under local-patch Stage 1 delivery (`codex update --upstream`), the user's local
Wrapper resolves official OpenAI releases directly and requires no CI-hosted
patched runtime. Any CI upstream workflow (manual `workflow_dispatch` or optional
low-frequency read-only canary) has read-only repository permissions, resolves a
stable upstream version and official archive digest, and runs standalone
qualification without auto-deploying, filing issues, or signing/re-hosting
upstream binaries.

When optional Stage 2 self-built addons (`Manager`, enhanced `TUI`, `Provider`)
are distributed alongside Wrapper, base and full release lanes remain
independent: qualified Wrapper/base publication never waits for optional addon
porting, and failed optional addons leave the installed runtime untouched. Base
distributes Wrapper for the locally adapted upstream default UI; full adds admitted
self-built `Manager` and enhanced `TUI` bundles. The opt-in Chat provider remains
independent. Distributed Wrapper/addon artifacts require actual Android
execution, affected behavior proof, Ed25519 signature verification (Phase 2.2),
immutable assets, readback proof, and non-forced activation.

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
provider gate and do not rebuild unchanged Wrapper/Manager or make Bun a Wrapper
prerequisite. Qualify the Rust package and minimum app-side bridge together.
After qualifying the active Provider Rust package and retiring old experimental
paths, legacy differential source/tests remain in preserved experiment history;
the active gate covers Rust, the app bridge, CDP qualification and package tools. Missing Rust/device/bridge gates are unfinished, not a reason to fall back
to legacy admission. The provider gate must not require Bun/Python merely to
execute the production adapter. Scope Cargo lock/toolchain/
shared-build changes by their actual consumers; do not exclude changed shared
inputs from Wrapper qualification. Classification still requires a successful accepted predecessor;
otherwise the conservative full Wrapper gate runs. Changes to Wrapper/shared build inputs
or mixed unknown paths retain full qualification. Provider qualification is separate
from Wrapper acceptance and must be required before future signed provider admission.

Base and full (Stage 2 optional lanes) have independent signed public heads: `update-index`
is the full head and `update-index-base` is the base head (Stage 1 standalone Wrapper
does not read remote release indices and fetches upstream directly).
The default signed update head follows the authenticated baseline of the selected
Wrapper/addon distribution; an explicit update URL remains an operator override.
A locally patched upstream generation does not create a signed public baseline.
Full users must never be silently downgraded when only base qualifies. Both public
heads and retained last-known-good assets stay available in their immutable
publication origins; current releases do not replace historical origins.
Promotion coordinates both heads with non-forced compare-and-swap and increasing
release sequence; a stale concurrent run re-evaluates rather than overwriting
another lane. No device activation or user-profile/provider mutation is implied
by public publication. Qualification-only workflows remain non-publishing.

In **Stage 1**, Wrapper (`crates/wrapper`) depends **only** on `crates/release-builder`
and `libc`, with zero dependency on `Manager`, enhanced `TUI`, `Provider`, or
`codex-runtime-binding`. When reintroduced in **Stage 2** (`Phase 2.3`),
`codex-runtime-binding` is dependency-free shared **source**, statically compiled
into its callers, for the bounded shared-server execution-record codec. Wrapper still
never imports `Manager`, frontend, or `Provider`, and creates no installed shared
library, service, registry, or optional-component runtime prerequisite. Wrapper
configuration queries remain bounded and optional; failure skips conveniences while
preserving independent execution.

Independence has distinct observable levels:

- **Build and qualification:** Wrapper/base construction and Android execution do not
  compile, acquire or qualify Manager, enhanced TUI or provider. Unsupported native
  runtime revisions still require their own Termux qualification.
- **Installed execution and lifecycle:** a valid Wrapper-only generation supports
  ordinary upstream execution and Wrapper doctor/update/rollback. Unavailable optional
  assets in an already-installed generation do not disable those operations.
  Doctor may separately report missing authentication or conveniences. Fresh
  candidate admission still rejects every missing/changed declared asset.
- **Component reuse:** the builder accepts an independently admitted Manager;
  local reconstruction already carries forward its bytes after the Wrapper-API
  probe. A compatible unchanged Manager need not be rebuilt to change Wrapper.
  The integrated CI build may still compile both; that is not a Wrapper dependency.
- **Feature-preserving public updates:** stable remains one signed combination.
  Preserving /switch and the enhanced /model/permission UI requires its frontend
  to pass the qualified backend protocol/behavior gate. Exact backend-version
  admission remains until a broader pair is actually proven. Failure retains the
  accepted generation; base qualification alone never silently removes features.
  There is no automatic downgrade from full to base; each edition follows its
  own signed public head.

The optional Stage 2 TUI patch contains only upstream call sites/types, supported-permission
integration, startup compatibility and actual test access; standalone policy lives
under `frontend/tui/src`. Upstream lacks a qualified general extension API
for these UI hooks. Moving them into Wrapper or a separate launcher would not remove
revision maintenance. Official-runtime binary adaptation (`crates/release-builder/src/runtime_patch.rs`)
uses adaptive rule-based path and UDS cave remapping (`termux-fd-remap`, `termux-uds-fd35`)
verified per binary in `bin/*` (no hardcoded version offsets); a Stage 2 source-built
optional frontend's permission hooks and FD remap are separately qualified. Never weaken
adaptive verification merely to admit a broken or unverified binary layout.

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

Ordinary Stage 1 upstream adaptation requires no compiler, source checkout,
remote build or private signing key. Source-build and missing-artifact orchestration
below applies only to explicitly selected self-built component flows; it must not
become an implicit fallback or prerequisite for ordinary `codex update --upstream`.

In optional Stage 2 component flows, Wrapper owns the complete update lifecycle:
discover installed-component updates, resolve compatible dependency groups,
reuse unchanged artifacts, acquire or build
missing artifacts, verify and atomically activate each inseparable group. Manager,
TUI, Provider and tunnel remain selected only when the user selected them. Full is
the complete selected composition, not a reason to rebuild unchanged components.
Existing Base/Full generations and independent Provider selections migrate without
losing credentials, profiles, conversations, rollback artifacts or live jobs.

Installed update and local-build policy, source preparation, build orchestration
and result admission are Rust responsibilities. Selecting local build must not
install Python merely to execute wrapper-owned lifecycle code. The release-builder
library owns the common implementation; Wrapper calls it in-process and remote
executors call its CLI. CI environment wiring and independent qualification may
use Python, but must not maintain a second implementation of these product rules.

Runtime source production consumes the exact release input plus a disposable
pristine official checkout, applies the maintained semantic source transformations,
audits lock normalization,
builds with the selected official toolchain and emits the existing six-file raw
runtime artifact contract. `build-runtime` and Wrapper's in-process consumer use the
same Rust implementation. Release/control JSON inputs retain the16-KiB bound;
raw transfer verification checks inventory, regular files, digests and source/audit
identity. Final output is snapshotted before the existing static AArch64 ELF check
and an isolated, bounded version probe. A failed compiler cannot admit an older
cached executable. Executor setup owns OS packages and compiler acquisition;
Generation assembly also has one typed request shared by CLI and Wrapper, covering
the official companion archive, optional source runtime/revision and selected
Manager/TUI artifacts. Wrapper preserves the builder's cause-specific error through
its public error chain instead of replacing every failure with a generic local
adaptation error. The same existing admission checks apply to every caller.
local execution and remote runners call the same Rust producer. Recipe fingerprints
bind its source/preparation/normalization code, exact upstream tuple, lock audit,
toolchain and build settings; a Wrapper-only commit cannot invalidate that raw
runtime recipe. Signed component metadata's `runtime_inputs_sha256` additionally
binds (when Stage 2 signed builds and `runtime-binding` are active) the Rust
release-builder/`runtime-binding` adaptation sources and builder
Rust toolchain when reusing already-adapted published runtime bytes. Legacy metadata
without that field remains admissible for installation but does not authorize
this producer reuse. Cached subsets are reauthenticated after executor transfer
before candidate composition. No previous Wrapper is fetched as a runtime dependency.
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

Optional upstream compatibility canary runs may be dispatched manually (or on an
optional schedule) without granting publication secrets or creating a runtime
dependency for local `codex update --upstream`. Consumers authenticate Wrapper and
optional self-built addon distributions against the admitted signing key (Phase
2.2) and authenticate upstream OpenAI archives directly against official OpenAI
release metadata and SHA-256 digests. GitHub CI is an optional executor/publication
adapter, not an online prerequisite for upstream updates. A request ACK admits no
executable.

Explicit `codex update` checks the official stable release independently of the
Wrapper distribution channel. A current Wrapper bundle does not establish that
the installed upstream version is current. An explicitly selected Stage 2
Wrapper/addon update may offer a grouped choice for unavailable self-built
components: keep those items, build locally including missing dependencies, or
request a remote build. Compatible independent updates may still proceed; the
displayed plan states that scope. This optional choice must not defer or gate a
valid standalone upstream update. Configured build policy provides its
noninteractive/default choice. No transport error silently starts a build;
signature/digest/compatibility failures remain failures. A selected frontend is
not silently removed. Remote requests reuse the same target job and report its
actual build/qualification/publication state and terminal failures.

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
of an upstream release's version. Wrapper acquires the listed official archives through
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
Wrapper's selected local build uses binary adaptation when its known policy is
available, otherwise the bundled qualified native source recipe matching the
selected official version/archive. `release/upstream.json` owns the common
source/version/archive/dependency contract for every executor. The on-device
compiler profile comes from `release/native-sdk-inputs.json`; remote executors use
the selected hosted toolchain. Qualification remains specific to the executor;
sharing source selection does not claim an untested SDK build works. An unavailable
source recipe is an explicit unsupported target, not permission to run an
unverified binary policy. Resolve that selection
before acquiring its upstream archive or provisioning build tools. Wrapper owns required
Termux package preparation, official source acquisition, SDK/dependency caches,
progress and the existing signing/activation transaction. Source construction uses
the shared Rust producer with an explicit child environment; it never mutates the
parent process environment. Private cached SDK/libraries are rechecked against
their recorded complete tree before execution/reuse. A cached raw runtime must
pass the same source/version/digest/dependency artifact admission as a new build.
The local cache identity binds both the shared producer and Wrapper's build-environment
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

Official upstream archive adaptation produces trusted local derivatives from
verified OpenAI inputs and the installed Wrapper. It requires no local signing
authority, configured private-key reference, temporary signer, public release
sequence, or signed whole-generation descriptor. Record the exact archive digest,
adaptation policy/report and component digests through the existing local
generation metadata and atomic activation/rollback path.

Signing authority belongs to the explicit publication of self-built Wrapper/addon
bundles. An authorized local developer may publish such a qualified bundle using
the same bounded explicit signing input as another authorized producer; ordinary
upstream update must not acquire that authority or require a signing key.
Model login, GitHub login and checkout presence are not signing authority.
Signing does not imply public upload. Publication follows its configured
permission and non-forced promotion policy, reuses admitted unchanged objects,
and requires renewed metadata admission on public sequence conflicts.

Missing-artifact policy is `ask` by default. The owner may configure one value
(`ask`, `keep`, `local`, or `remote`) in Wrapper's `config/update-policy`; an explicit
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
and signs the resulting adapted generation for delivery. The installed Wrapper
never executes an upstream self-updater. Explicit local build prepares only
recipe-required tools and invokes the bounded release-production routines.
Wrapper acquires the official archive through its shared bounded, resumable download
cache, keyed by the digest from official release metadata. It reports transferred
bytes and preserves untrusted partial bytes for the next explicit command; only
the verified complete archive reaches generation assembly. The builder owns the
256-MiB archive bound shared by acquisition and archive admission. It then adapts
the raw runtime, qualifies the result and signs only that local
candidate with the selected local-build signer while preserving the official
forward update_key. Local build never accepts a raw archive as an activation
candidate and never bypasses signed local admission. The builder's optional Stage 2 `fetch` operation (not exposed in Stage 1 where `codex-release-builder`
exposes only `build`) accepts one explicit stable `MAJOR.MINOR.PATCH` version, constructs only the
canonical official archive URL above, and downloads that archive with bounded
HTTPS transport into one absent local regular-file output.
`fetch` performs no channel discovery, mirror selection, source fallback, signing, generation
activation, or live-state mutation. In Stage 1 standalone builds, `codex-release-builder build` accepts the
version, upstream archive, SHA-256 digest, generation identity, Wrapper artifact, creation
metadata, and an absent output directory, emitting a `codex-local-generation` directory
without `manager_artifact_digest` or `upstream_doctor`. (Optional Stage 2 `--manager <ABSOLUTE_FILE>`
artifact carrying and signed `publish` remain decoupled Stage 2 extensions.)
It performs no discovery, signing, activation, or live-state mutation.

Release production in Stage 2 (Phase 2.2) signs **self-built distributed bundles**
(Wrapper `codex`, `codex-release-builder`, `install.sh`, and any distributed
optional self-built addons such as `manager`, enhanced `tui`, or `provider`), rather
than re-hosting or signing locally patched upstream `bin/codex` binaries. It
exposes one non-installed `publish` operation over a verified self-built bundle
directory:

```text
codex-release-builder publish --bundle <ABSOLUTE_DIRECTORY> \
  --release-sequence <POSITIVE_DECIMAL> \
  --release-base <HTTPS_BASE_URL> \
  --private-key <ABSOLUTE_FILE> \
  --openssl <ABSOLUTE_EXECUTABLE> \
  --output <ABSENT_ABSOLUTE_DIRECTORY>
```

When enabled in Stage 2, `publish` signs the Wrapper/addon bundle artifacts built by
this repository (`wrapper`, optional `codex-release-builder`, `install.sh`, and
optional `manager` / `tui` / `provider` addons as declared), leaving official upstream
OpenAI tarballs and on-device patched `bin/codex` `package-root` generations verified
via OpenAI SHA-256 + local Wrapper derivation without requiring remote signatures on
locally patched binaries. The bundle release identity must be a safe URL path
component and the supplied canonical HTTPS release base must end in that encoded
identity. The release sequence is a positive decimal and the base follows the
canonical HTTPS URL grammar used by Stage 2 signed Wrapper/addon acquisition.

The operation derives one raw Ed25519 public key from the supplied private PEM
using the explicit OpenSSL executable. It emits a non-rotating publication:
the derived key is written as `release_public_key`, `release.sig` signs the
exact `release.manifest` with that key, and no `release-authority.sig` is
created unless an explicit authority rotation is issued. Therefore an operator
producing an ordinary public signed Wrapper/addon release with this command
supplies the current Wrapper `update_key` private key. The supplied private key
is read only for derivation/signing, is never copied into the output, and is
never written to repository or device state.

The Stage 2 signed Wrapper/addon bundle output contains only self-built
distributed artifacts (no upstream `bin/codex` binary redistribution required):

```text
<output>/update-index
<output>/update-index.sig
<output>/releases/<bundle_id>/release.manifest
<output>/releases/<bundle_id>/release.sig
<output>/releases/<bundle_id>/install.sh              (optional installer payload)
<output>/releases/<bundle_id>/wrapper
<output>/releases/<bundle_id>/codex-release-builder   (optional)
<output>/releases/<bundle_id>/manager                 (optional Stage 2 addon)
<output>/releases/<bundle_id>/tui                     (optional Stage 2 addon)
```

The index contains the four canonical records (`codex-update-index`, `channel`,
`bundle_id`, `release_base`) and final newline, and its sibling
`update-index.sig` signs those exact index bytes with the same key. The
publication manifest uses `codex-release`, binding control fields, monotonic sequence,
`wrapper-api`, `schema`, and the SHA-256 + regular-file octal mode inventory of every
present distributed bundle file (requiring `wrapper`, plus optional declared
self-built tools/addons). `publish` performs no network upload and does not assume that
locally patched upstream `bin/*` assets are hosted alongside the signed Wrapper bundle.
the URL is hosted by OpenAI.

Ordinary Stage 2 signed acquisition, `--force`, `--rollback`, `--signed --path`, and `--signed --remote`
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

Candidate construction and Android/AArch64 smoke are read-only and have no
signing secret or public mutation authority. Signing, immutable Release/Pages
staging, and public consumer readback are separate jobs on the same admitted
Wrapper/addon bundle. Official upstream canary checks run on manual dispatch (or
an optional low-frequency schedule) as read-only verification; ordinary local
`codex update --upstream` never depends on CI schedules.

All candidate/component transformations and corrective bindings must complete
before the unsigned transfer archive is created. Package the final admitted
candidate once immediately before artifact upload; never transfer an earlier
archive of a directory that was subsequently changed. The archive must preserve
the admitted file inventory, bytes and modes, including the deferred Manager
marker until native smoke removes it. Smoke packages only its final qualified
candidate, and signing/public readback must consume that same qualified payload.
This invariant applies equally to ordinary, Wrapper-only and Manager-only releases;
a successful directory check cannot qualify stale exported bytes.

For any authorized official producer, an explicit version selector must be one
stable `MAJOR.MINOR.PATCH` value; otherwise official discovery resolves
`https://api.github.com/repos/openai/codex/releases/latest` (or explicit release metadata
at `https://releases.openai.com/codex/releases/<version>/release.json`), binding the
`rust-v<version>` tag and SHA-256 digest for the exact
`codex-package-aarch64-unknown-linux-musl.tar.gz` asset. The metadata is a
version/digest selector, not an activation authority. The producer must download
the exact versioned archive and compare its digest to the metadata-bound digest
before adaptation. Missing, malformed, non-stable, mismatched, or unavailable
metadata fails closed; no mirror, package manager, mutable raw runtime, or
upstream self-updater is accepted.

Local upstream adaptation never reads maintainer keys, `CODEX_TERMUX_UPDATE_PRIVATE_KEY`,
or repository release secrets, and never generates an ephemeral signing key. It
does not invoke `gh`, create a publication store, dispatch a workflow or advance
a public stable pointer. The prior device-local signing authority,
`config/build-signing-key` and signed `codex-local-derived` generation format are
retired whole-generation delivery designs, not requirements of this contract.
Their prior records belong to [release history](../history/GENERATION_ADMISSION.md).

Stage 2 `--signed --path` and `--remote` authenticate distributed Wrapper/addon
bundles under their admitted publication authority. They cannot admit arbitrary
self-signed bundles through the unsigned upstream path. Conversely, ordinary
`--upstream` and explicit upstream `--path` retain local generation admission;
they require neither a remote re-signature nor a local private key. Preserve
rollback artifacts and enforce each artifact owner's applicable rollback guard
without making an optional bundle's signing state a veto of standalone upstream
updates. Temporary archive/build material is private and bounded. Local upstream
adaptation never invokes, installs, selects or repairs `bwrap`.

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
publication ref. For Stage 2 signed bundles packaged as flat GitHub Release assets
under `<generation_id>`, `release_base` uses the matching
`https://github.com/humtr/codex/releases/download/<generation_id>/` asset base,
while `asset-map.json` maps nested `package-root` relative generation paths to
canonical physical Release asset names without altering verified relative file layouts.

Base (`wrapper-*`) owns Wrapper, adapted upstream runtime/default UI, code-mode host,
and browser helpers. Full (`full-*`) pins one exact authenticated base and adds Manager/enhanced
TUI plus its own descriptor and signatures. Optional Provider generations follow
`provider-gpt-*` (or configured provider naming). Each full result retains one complete
signed inventory and one atomic current/previous activation lifecycle. There
is no device overlay, second updater, mutable latest-base dependency or component
selection policy in Wrapper. Manager/TUI-only changes must not advance the base head
or rebuild unchanged base executables; full-combination Android/runtime proof is
still required. Wrapper/shared input changes invalidate base reuse.
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
fingerprint. The fingerprint covers Wrapper/builder/binding sources, relevant build/
lock dependencies, upstream/archive and Wrapper toolchain, not Manager/TUI policy.
The producer verifies both the sidecar and base file inventory before reuse;
version equality or a successful unowned workflow alone never permits reuse.

#### GitHub-native transport and reuse compatibility

This is the contract for Stage 2 signed distribution, not a claim that standalone Stage 1
uses signed release indices. GitHub Release storage and GitHub Actions
execution are independent choices; neither ordinary download nor local construction
requires a second hosting service or gh/model login.

The native acquisition path reads the bounded physical asset map
from GitHub Releases and stages files under their exact `package-root` relative paths. Admit the
selected index/manifest and generation/high-water first; controls remain in the
selected generation. Validate map identity, requested inventory, manifest binding,
flat asset spelling and allowed repository/generation routes. Maps supply locations,
not trust: signed final sizes/digests/modes and ordinary complete-candidate
admission remain mandatory. Preserve Range/caches/errors and custom/private HTTPS
origins. Reuse one cohesive native transport responsibility and existing publication
map/target consumers rather than introducing another updater, server or changed
installed layout.

New-client acceptance must pass real GitHub acquisition including `package-root`
nested helpers, reuse from previous physical releases, interruption/Range refusal,
bad maps/signatures/digests, rollback, and custom HTTPS origins.
After admitting fresh signed controls and sizes, Wrapper reuses matching files from
the retained target generation or current selection before downloading payloads.
Retained bytes must pass the same ownership, file type, size, mode and copied-byte
digest checks. Reuse never skips complete candidate admission or repairs a corrupt
immutable generation. A rollback followed by an authenticated force retry must
work with the retained target's runtime origin unavailable.
Implementation, qualification, publication and device activation remain
separate states. [CURRENT.md](../../CURRENT.md) and [docs/plan/STAGE2_PLAN.md](../plan/STAGE2_PLAN.md) record sequence and proof status.

#### Signed composition and public qualification

Each Stage 2 Release contains its changed payloads and a bounded `asset-map.json`.
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
generations, fixed `humtr/codex` Release assets and exact admitted paths. It rejects
encoded/unsafe paths, queries, missing/foreign maps and indirect routing. It never proxies
payloads, chooses stable, handles credentials or controls device state. It needs
no secret/binding/storage or per-generation deployment. Administrative deployment
uses existing account credentials locally, never grants CI another credential.
Verify deployed source identity and public negative/positive requests before use.

Release CI stages immutable assets, verifies exact composition and complete public
HTTPS readback of their physical GitHub routes, and runs disposable Android fresh install/
update/no-op/rollback proof, then non-forced stable promotion. Generic acquisition
may copy a hash/size/mode-verified owned retained file into private staging without
hardlinks, then apply unchanged complete-candidate admission. Unavailable/corrupt
files fall back to authenticated download. This optimization never interprets
account, UI or rotation policy.

Both current lane heads, rollback/compatibility targets, every Base pinned by a
retained Full and every physical generation named by retained asset maps must be
preserved. Physical origins can be earlier Base or Full releases, independently
of the current logical Base. Deleting an unreferenced retired
release requires explicit retention scope and a reference check; no date-only
cleanup, published-asset overwrite or silent deletion is authorized.

The installed optional Stage 2 TUI (`frontend/tui`, when enabled in Stage 2) is an independently
developed optional frontend and is not locked to exact `upstream_package_version`
equality when enabled by `wrapper.toml` or `--with tui`. Local upstream
updates (`codex update`) operate exclusively on the upstream package and
Wrapper compatibility layer without consulting Manager or TUI version markers.

For any later authorized official stable promotion in Stage 2, the small
`update-index` and `update-index.sig` files are promoted together in one
Git tree and one commit on the selected stable publication branch only after the
selected release transport, complete readback, and disposable public-update smoke
are verified. Wrapper runtime does not perform this promotion. The branch update is non-forced
and is based on the exact previously verified head, so a concurrent publisher
cannot be overwritten. Automatic promotion must never expose a new index with an
old signature or vice versa. A failed, timed-out, incomplete,
path-renaming, readback, or pre-commit promotion never advances
the automatic stable pointer. An indeterminate final ref result is reported as
indeterminate rather than assuming either state. The account credential and the
release `update_key` private key are separate authorities; account authentication alone
cannot authorize a release for Wrapper.

All source files are snapshotted into private staging, revalidated as regular
files, and copied without following symlinks. Final modes are applied before
file synchronization; the complete index and release tree are synchronized
bottom-up and the absent output directory is atomically published with
`RENAME_NOREPLACE`, followed by output-parent synchronization. A failure
leaves no accepted output, preserves an existing destination, and does not
alter the source generation or any live Wrapper state. The official OpenAI
versioned archive remains the only upstream source authority; this publication
tree is wrapper-owned distribution content.

The release builder complete output boundary is durable: final file modes are set before the corresponding final file synchronization, the complete staging tree is synchronized bottom-up, the no-replace output publication is atomic, and the output parent is synchronized before the build reports success. A failed publication leaves no accepted output.

The accepted archive is gzip-compressed POSIX ustar. A per-entry POSIX PAX
header is optional and may contain only `mtime`; all other extended semantics
are rejected. There are at most 256 logical entries, paths are canonical relative
UTF-8 of at most 256 bytes, the compressed archive is at most 256 MiB, each
regular file is at most 384 MiB, and total regular-file payload is at most
768 MiB. Duplicate paths, absolute or dot/dot-dot paths, links, special files,
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

The archive must contain at least regular `bin/codex` and `codex-package.json`.
All extracted regular binaries in `bin/` (`bin/codex`, `bin/codex-exec-server`,
`bin/codex-code-mode-host`) must be valid little-endian 64-bit static AArch64
ELF files with no `PT_INTERP`. Package archive entries including `codex-resources`
(`voice`, `bwrap`, etc.) and `codex-path` (with `codex-path/rg` mapped to the
Termux `rg` wrapper) are preserved intact in `package-root` generations so
upstream resource discovery remains complete; Wrapper still refuses `bwrap`
Linux namespace sandboxing cleanly at runtime.

Rule-based adaptation applies equal-length non-overlapping FD path remaps
(`termux-fd-remap`: `/etc/resolv.conf` -> `/proc/self/fd/33`,
`/etc/codex/*.toml` -> `/dev/fd/34/*.toml`) and adaptive UDS cave relocation
(`termux-uds-fd35`: relocating `/tmp` socket base setup to `/proc/self/fd/35\0`
and shortening `\x0dcodex-daemon-\xc0\0` to `\xc0\0` UID-only naming) across every extracted
static AArch64 musl executable in `bin/` (`bin/codex`, `bin/codex-exec-server`,
`bin/codex-code-mode-host`) that contains matching unpatched patterns, verifying
zero unpatched source occurrences remain without hardcoding per-binary
occurrence counts or a single summed byte delta. Companion ELF executables in
`bin/` without matching sites remain unmodified valid static AArch64 ELF files.

`codex update` in standalone mode defaults to `--upstream`: it fetches
the selected official `openai/codex` release metadata and archive (`--version`
or latest), verifies the archive SHA-256 digest, unpacks the complete
`package-root` layout, preserves static AArch64 `codex-path/rg` when supplied
(falling back to a Termux wrapper script when absent), executes adaptive rule
patching across `bin/*`, carries forward the active Wrapper launcher, validates
`bin/codex --version` under Termux FDs, and atomically activates the new
generation without contacting addon release indices or requiring Manager
artifacts (`manager_artifact_digest` and `upstream_doctor` are omitted in
`codex-local-generation` descriptors; `bin/codex-code-mode-host` is likewise optional when
an upstream package omits it). `codex update --path <FILE_OR_DIR>` adapts an
offline `.tar.gz` archive or validates/installs a prepared generation directory;
`codex update --rollback` atomically restores the retained previous generation.

Every output is create-new in private staging and the absent destination is
published complete-or-absent. Failure never publishes a partial generation,
changes an existing destination, signs or activates content, or writes outside
the selected output parent.

The release builder accepts only a regular executable Wrapper artifact of at most
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
- Wrapper (and optional Stage 2 Manager) artifact digests;
- Wrapper API and persistent schema compatibility;
- qualification result and creation metadata.

Binary adaptation must verify expected source occurrences, reject already
patched or unexpected layouts, and compare the result with the declared patch
policy. Upstream layout drift fails before activation.

Archive extraction must reject absolute paths, traversal, escaping symlinks,
special files, duplicate conflicting entries, and writes outside staging.

## 7. State and generation model

Code/artifact generations and mutable user state are separate.

The Wrapper state/generation layout is:

```text
$PREFIX/bin/codex                                      stable public entrypoint
~/.local/lib/codex/wrapper/generations/<id>/             immutable complete generation
  generation.meta                                     local generation descriptor (`codex-local-generation`)
  wrapper                                             Wrapper launcher for this generation
  bin/codex                                           adapted upstream CLI executable
  bin/codex-code-mode-host                            optional adapted upstream Code Mode companion
  bin/codex-exec-server                               optional adapted upstream exec-server companion
  codex-package.json                                  upstream package manifest
  codex-path/                                         optional upstream helper tool directory (including `rg`)
  codex-resources/                                    optional upstream resource tree
  browser/open/curl                                   Termux browser open helper (`termux-browser-open`)
  browser/manual/curl                                 Termux browser manual helper (`termux-browser-manual`)
~/.local/lib/codex/wrapper/generations/.acquire-*/        private incomplete remote acquisition staging; never activatable
~/.local/lib/codex/wrapper/generations/.candidate-*/      private candidate build staging; never activatable
~/.local/share/codex/wrapper/                             state root (directory flock writer lock when writing activation files)
  activation-state                                        authoritative generation state (`codex-activation-state`)
  activation-state.tmp                                    atomic state publication temporary
  activation-journal[.tmp]                                crash-recovery transaction state (`codex-activation-journal`)
  .local-update-*/                                        private offline/upstream archive staging workspace
  config/config.toml                                      Wrapper-owned FD 34 compatibility config (`# codex-termux-wrapper`)
  config/snapshots/<id>/                                  Stage 2 immutable per-launch effective system settings
  resolv.conf                                             Wrapper-owned FD 33 DNS resolver file
  downloads/<sha256>.part                                 private bounded untrusted/verified payload cache (1 GiB total cap)
  publications/                                           optional staged/pruned local publication artifacts
  wrapper.toml                                            Stage 2 optional module & update mode config
  wrapper-entrypoint-rollback                             Stage 2 retained prior Wrapper launcher (when signed bundles enabled)
  wrapper-entrypoint-rollback.meta                        Stage 2 rollback binding (`codex-wrapper-entrypoint-rollback`)
  {servers,catalogs}/<id>/                                Stage 2 bound native server record/socket/config snapshot
~/.local/share/codex/manager/                             Stage 2 Manager-owned mutable state (absent in Stage 1)
```

The optional Stage 2 Manager state root is independent of the Wrapper root:

```text
~/.local/share/codex/manager/
  profiles/<PROFILE_ID>/profile.meta                      Manager profile record (`codex-manager-profile`)
  default-profile                                         explicit default account preference
  profiles/<PROFILE_ID>/home/                             selected upstream CODEX_HOME
  notifications/config                                    notification configuration (`codex-manager-notify`)
```

`profile.meta`, `default-profile`, and `notifications/config` are Manager records and
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
Only explicit `profile default` owns the `default-profile` preference. Any
legacy `state-v1` selection-history files from pre-cutover environments are
ignored and left untouched, never selecting an account or blocking execution.

Manager profile records use exact UTF-8 text formats with a final newline.
`profiles/<PROFILE_ID>/profile.meta` contains:

```text
codex-manager-profile
```
The derived profile-home path is not duplicated inside metadata. Unknown
records, duplicate records, invalid UTF-8, or a mismatched ID invalidate that
record and never cause a path to be followed. Notification commands alone write
`notifications/config`.

Manager does not persist a session transcript, a second session database or a
parallel discovery index. Upstream owns discovery/resume and conversation state;
the declared compatibility links above do not transfer that ownership. Auth-state
migration is outside ordinary Manager operation.

The authoritative state format is `codex-activation-state`. In Stage 1 standalone
mode it tracks `current` and optional `previous` generation identities alongside
placeholder `LOCAL_ZERO` (`00..00`) verifier key slots; when Stage 2 `--signed`
releases are active, those key slots hold canonical 32-byte Ed25519 public keys
encoded as 64 lowercase hexadecimal digits. The `previous` generation and
`previous_key` are present or absent as one pair. There is no separate `verified`
pointer, keyring, discovered-key set, or unbounded key history.

Only content that has already passed release admission and candidate probes may
become `current`. Ordinary runtime selection uses only current; it does not
verify signatures, search keys/generations or implicitly select previous. The
separately bounded bare all-TTY update advisory may authenticate the channel as
specified in Section3, and best-effort artifact maintenance has its own explicit
retention boundary below. Neither exception makes network/OpenSSL/maintenance a
prerequisite for executing a valid current generation. The generation directory name is one safe path component
and generation content is complete before it can become `current`. Before
ordinary launch consumes that generation, Wrapper must verify the selected
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

The authoritative journal format is `codex-activation-journal`. Its before
and after records contain the entire bounded trust-and-generation state, not only
generation identities. Activation, rollback, and recovery therefore treat
`update_key`, both generation identities, and both generation verifier keys as
one transaction. After process kill, power loss, short write, full storage,
permission failure, or a stale journal, recovery must resolve to exactly one
complete old or new trust-and-generation state and must never synthesize a mixed
state.

Stage 2 coordinated signed updates also retain one regular executable
`wrapper-entrypoint-rollback` and its `codex-wrapper-entrypoint-rollback`
binding for the launcher active before the forward transition. This cache is
not a trust source: its digest and associated generation are verified before
rollback, and it is atomically replaced only while the activation lock is
held.

Coordinated signed activation stages and verifies the complete signed generation and
its `wrapper` asset first, snapshots the current stable launcher into the bounded
rollback cache, atomically replaces `$PREFIX/bin/codex` with the candidate
Wrapper, synchronizes its parent, and only then commits the existing generation
pointer transaction. If the pointer commit fails, the old launcher is restored
before the failure is reported. A process kill between launcher replacement and
pointer commit leaves the new Wrapper running against the old complete generation,
which is backward-compatible within the accepted Wrapper API/schema boundary; the
next update may retry without accepting a mixed generation. Rollback swaps the
retained launcher cache and generation pointer as one locked operation and
preserves the exact one-pair boundary.

Wrapper optimizes for the shortest correct release path rather than speculative
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
the activation journal is a single pathname, Wrapper serializes activation and
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

### Stage 1 standalone installation (`install.sh`)

Fresh installation uses one self-contained POSIX shell installer ([install.sh](../../install.sh))
with zero reliance on legacy `install-online.sh`, `bootstrap/codex-bootstrap`, or
`<CORE_ARTIFACT>` positional arguments:

```text
install.sh [--prefix <DIR>] [--binary <PATH>] [--build] [--release <TAG>] \
  [--repo <OWNER/REPO>] [--version <X.Y.Z>] [--skip-upstream] [--help]
```

- When `--binary`, `--build`, and `--release` are omitted, `install.sh` resolves
  1) unpacked bundle `./bin/codex` or `./codex` (verifying `SHA256SUMS` when present),
  2) local `./target/release/codex`, or 3) the GitHub Release asset
  `codex-wrapper-aarch64-linux-android.tar.gz` and `.sha256` from tag `wrapper`
  of `humtr/codex` over HTTPS (`curl`) + OpenSSL/sha256 verification. (Stage 1
  verifies `.sha256` / `SHA256SUMS` digests only without an Ed25519 signature;
  Phase 2.2 adds Ed25519 signature verification of the Wrapper bundle and
  `install.sh` self-update path.)
- On Android AArch64, `install.sh` runs a non-mutating `/system/bin/linker64`
  `--version` sanity probe on the staged binary before atomically publishing it to
  `$PREFIX/bin/codex`.
- Unless `--skip-upstream` was passed (or an initial generation already exists),
  `install.sh` invokes `$PREFIX/bin/codex update --upstream` (or `--upstream --version <X.Y.Z>`)
  so the newly installed Wrapper itself acquires, verifies, patches, and activates the
  initial `package-root` generation (`codex-local-generation`). Failure in `install.sh`
  cleans its temporary staging workspace without leaving a broken `$PREFIX/bin/codex`.

### Stage 1 standalone update lifecycle (`codex update`)

After installation, Wrapper exclusively owns `codex update`:

- `codex update` / `codex update --upstream [--version <MAJOR.MINOR.PATCH>]`: resolves
  official `openai/codex` release metadata (`api.github.com/repos/openai/codex/releases/latest`
  or `https://releases.openai.com/codex/releases/<version>/release.json`), downloads
  `codex-package-aarch64-unknown-linux-musl.tar.gz` into private `$state_root/.local-update-*` /
  `downloads/` via `$PREFIX/bin/curl` (15 s connect, 300 s idle transfer timeout, up to
  `256 MiB` compressed archive, `1 GiB` download cache cap), verifies the official SHA-256
  with `$PREFIX/bin/openssl`, unpacks and adaptively patches `bin/*` (`termux-fd-remap`,
  `termux-uds-fd35`) in `.candidate-*`, probes the exact `bin/codex --version`
  and `features list` under Termux FDs in a fresh account-free temporary home
  (bounded time and output; no personal-device allowlist or account test),
  and atomically activates (`codex-activation-state`, `LOCAL_ZERO` verifier slots).
- `codex update --path <FILE_OR_DIR>`: adapts an offline official `.tar.gz` package archive
  or validates/activates a prepared `package-root` generation directory (`generation.meta`).
  Raw standalone ELF files without `codex-package.json` or `generation.meta` are rejected.
- `codex update --rollback`: acquires the state-root directory lock, recovers any incomplete
  journal, revalidates the single retained `previous` complete generation, and atomically
  swaps `current` and `previous`.
- `codex update --check [--version <MAJOR.MINOR.PATCH>] [--json]`: uses the same
  official resolver and bounded HTTPS transport in an owned disposable temporary
  directory. Latest-release discovery is cross-checked against the exact-version
  official descriptor. Output binds the version, canonical archive URL/SHA-256,
  exact descriptor URL/SHA-256 and current Wrapper SHA-256. It reports current,
  available or older relative to an installed generation without activation,
  retention, artifact download, journal recovery or installed-state writes.
- Removed/Stage 2 flags (`--signed`, `--remote`, `--local`, `--build-local`) fail
  explicitly in Stage 1 with exit status `2` and zero state mutation. No compiler toolchain,
  signing key, or upstream self-updater is ever invoked in Stage 1.

Browser helpers occupy the fixed package-root paths `browser/open/curl` (`termux-browser-open`)
and `browser/manual/curl` (`termux-browser-manual`), with `helper_count = 2`. No indexed
`helpers/0` / `helpers/1` compatibility bridge is retained.

### Stage 2 optional signed Wrapper/addon channels and bootstrap trust (Phase 2.2, deferred)

When Stage 2 signed Wrapper/addon distribution (`--signed` or `wrapper.toml`
`signed_update = true`) is enabled in Phase 2.2, Ed25519 verification protects the
trusted computing base built and hosted by this repository (`wrapper`,
`codex-release-builder`, `install.sh`, and optional self-built addons such as
`manager`, enhanced `tui`, and `provider`) without requiring remote CI signatures
for OpenAI upstream tarballs or locally patched `bin/codex` `package-root`
generations (which remain authenticated by OpenAI SHA-256 + local Wrapper derivation):

- **`install.sh` & Wrapper self-update Ed25519 verification:** replaces Stage 1's
  checksum-only (`SHA256SUMS`/`.sha256`) release fetch with Ed25519 signature
  verification over distributed Wrapper/addon bundles using the pinned verifier key.
- **Automatic signed Wrapper/addon index:** fetches bounded `update-index` (or base
  `update-index-base`, overridable via `CODEX_TERMUX_UPDATE_INDEX_URL`) and `<URL>.sig`,
  verifying the canonical index (`codex-update-index`, `channel`, `bundle_id`,
  `release_base`) against `update_key` before touching bundle payloads.
- **Explicit signed remote/local acquisition (`codex update --signed --remote <HTTPS_BASE_URL>` / `--signed --path <DIR>`):**
  accepts canonical `https://` release bases (up to 4,096 bytes), fetches and verifies
  `<base>release.manifest` (`128 KiB` cap) and `<base>release.sig` (`1 KiB` cap, plus
  `<base>release-authority.sig` on key rotation) against the authoritative Ed25519
  `update_key` before staging Wrapper or self-built addon files.
- **Ed25519 verification & key transitions:** `release_public_key` is 64 lowercase hex digits.
  Non-rotating updates require `release_public_key == update_key`; key rotations verify
  `release-authority.sig` under `update_key` first, then `release.sig` under the new candidate
  key. On initial `--signed` trust enrollment (before signed keys exist in `activation-state`),
  Wrapper binds one pinned public key seed (`~/.local/lib/codex/wrapper/release-public-key.pem`,
  `16 KiB` cap); once signed keys exist, this seed file is never used as a recovery fallback.
  Bare `LOCAL_ZERO` (`00..00`) verifier slots from Stage 1 remain preserved for local
  `--upstream` / unsigned `--path` upstream generations without conflicting with signed
  Wrapper/addon self-updates.
- **Bounded transfer, staging & cache limits:** uses `$PREFIX/bin/curl` (`--location`,
  `--proto-redir =https`, `NO_PROXY`/`--noproxy` isolated environment, 15 s connect, 30 s
  control timeout, 1800 s idle transfer timeout for release files / 120 s for Provider files),
  `384 MiB` per-file / `768 MiB` total extracted payload bounds (`1 GiB` `state_root/downloads`
  total cache), staging under private `.acquire-<pid>-<counter>` / `.candidate-*`
  (and `64 MiB` Wrapper artifact bound / `64 KiB` descriptor bound).
- **`--rollback` across Wrapper/addon & upstream boundaries:** swaps retained `previous`
  state atomically while leaving rotated-out forward `update_key` authorities retired,
  and missing optional addon assets degrade only at the addon boundary without bricking
  `codex`, `codex doctor`, or `codex update --upstream`.

In summary, every Stage 2 signed Wrapper/addon update step must:
1. recover state/journal under the state-root directory `flock`;
2. verify index/manifest signatures (`update_key`, optional `release-authority.sig`, `release.sig`) and monotonic sequence anti-rollback before staging;
3. acquire verified regular files with expected octal modes (`wrapper` and declared addons executable, no special bits);
4. run non-mutating binary probe(s) before atomic replacement;
5. commit state atomically without leaving partial destinations on failure, keeping local upstream `codex update --upstream` independent.

### Bounded installed-artifact retention

Wrapper performs best-effort local artifact maintenance after explicit update
attempts, without pruning on ordinary execution. Failure or contention in
maintenance must not change upstream argv, streams, exit status or block an otherwise valid
launch. The one authoritative activation state and its existing recovery journal
own all retained pointer roles; malformed or unrecovered state disables pruning.
Keep the current generation, the one previous rollback generation, any generation
bound by an effective artifact-scoped rollback guard, at most one
complete higher-sequence staged candidate for the existing crash/retry reuse
contract (highest sequence, deterministic generation-ID tie-break), and complete
generations still used by live local executables or open generation files.
All other Wrapper-owned installed generations/publication caches are disposable.
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

Retire only an obsolete Wrapper-owned shared server whose private owner record,
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
checks, and staging, Wrapper must execute the exact candidate upstream
`--version` probe and bounded `features list` configuration probe in that qualified
Termux environment with disposable HOME/CODEX_HOME. A failed probe,
qualification failure, admission failure, or transaction failure rejects the
candidate before the new launcher/pointer pair is committed. Activation must
not require the full upstream `codex doctor` command to exit zero: credential,
provider, and external-network health belong to the diagnostic surface, not to
release-runtime integrity.

## 9. Doctor contract

`codex doctor` (`codex doctor [--color | --json]`) is read-only. It runs the raw
upstream doctor when supported and adds a Termux Wrapper diagnosis (plus optional
Stage 2 Manager status only when Stage 2 modules are active) without recursively invoking
the public launcher. Human output begins with the bounded, sanitized upstream doctor
output itself, preserving the upstream doctor's own header, layout, and safe
ANSI SGR sequences (or emits `"Upstream doctor output unavailable: <status>"` when
unsupported/failed), followed by `\n[Termux doctor]\n`, the `Codex Termux Wrapper Doctor`
header, and grouped diagnostics. `--json` emits one redacted envelope:

```json
{
  "schema_version": 1,
  "upstream": {"status": "healthy", "output": "..."},
  "termux_wrapper": {
    "status": "healthy",
    "generation_id": "...",
    "layout": "package-root",
    "runtime": {"status": "healthy"},
    "code_mode_host": {"status": "healthy"},
    "sandbox": {"status": "unsupported", "reason": "bwrap is not used"}
  },
  "summary": {"status": "healthy"}
}
```

(When optional Stage 2 Manager diagnostics attach, an optional `"manager"` object may
also be included; standalone Stage 1 omits `"manager"`.)

When human output is connected to a TTY and `NO_COLOR` is absent, Wrapper gives the
upstream doctor a bounded pseudo-terminal so its own headings, progress
cleanup, and ANSI SGR markup retain the upstream layout. `--color` is the
explicit exception for an interactive caller whose outer Termux/AI wrapper
injected `NO_COLOR`; Wrapper removes that variable only from the upstream child
and enables the same bounded PTY path. Non-TTY output remains plain, and
explicit `NO_COLOR` remains plain unless `--color` was requested. Wrapper
normalizes carriage-return/erase controls and preserves only safe SGR sequences
before composition. If the upstream doctor is unsupported or produces no
output, Wrapper emits the concise unavailable status line before `[Termux doctor]`.
The Termux doctor section emits `Codex Termux Wrapper Doctor · generation <id> · status <status>`,
the `Runtime`, `Support`, `Wrapper`, and `State` groups, and the `[Summary]` section,
with colored health rows when enabled. It reports the selected generation,
`bin/codex-code-mode-host` companion status (`Support`/`Runtime`), and the explicit
reason that Linux bwrap sandboxing is not used.

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
class when the bounded JSON envelope is valid, Termux Wrapper/runtime/code-mode
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

Generation descriptors written by `codex-release-builder` and `codex update` use
`codex-local-generation` and the `package-root` layout (`bin/codex`, optional
`bin/codex-code-mode-host`/`bin/codex-exec-server`/`codex-path/`/`codex-resources/`,
`codex-package.json`, `browser/{open,manual}/curl`, `wrapper`, `generation.meta`),
without reading legacy v1/v2 descriptor formats or `compat/` layouts.
## 9A. Termux compatibility extension

The upstream runtime remains the official `aarch64-unknown-linux-musl` Codex
artifact adapted by this product. Running that Linux-target executable inside
Termux does **not** activate upstream compile-time `target_os = "android"`
branches. Compatibility decisions therefore belong to an explicit Wrapper-owned
Termux boundary rather than to accidental Linux desktop behavior.

This extension does not reopen or weaken the accepted Wrapper completion
invariants. The resolver, signed-generation trust model, wrapper-owned update,
coordinated Wrapper/generation activation and rollback, profile/session/auth
boundaries and sandbox policy remain unchanged. Ordinary signed installation
requires no on-device compiler; explicit source-build paths, including the tunnel
recovery contract, are separate opt-in operations.
It also does not authorize an unbounded raw-runtime byte patch: the selected
version's exact termux-fd-remap policy and retained-generation policies remain
bound to their qualified artifacts. Any additional binary rewrite requires a
separate normative amendment that names its exact bounded substitutions and
qualification evidence.

### Timezone compatibility

Resolve the musl runtime's user-visible timezone mismatch through Wrapper's launch
environment. Respect explicit working POSIX/path TZ settings and resolve selected
IANA names, including the Android system timezone when TZ is unset, using the
active Android timezone data. The preferred method to qualify is a verified
zone-specific TZif supplied as `TZ=:/absolute/private-cache/zone.tzif` to the
existing runtime. Wrapper's common child environment boundary supplies the same
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
Wrapper must support the result without Manager or shell edits, and timezone
discovery/data failures must be diagnosable without preventing ordinary launch.

### Browser and external URL intents

An upstream request to open an HTTP or HTTPS URL on Termux must use one
Wrapper-qualified Termux opener capability rather than Linux desktop discovery.
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
configuration` is provider/account policy evidence, not by itself a Wrapper
compatibility defect, when Codex submitted a syntactically valid callback that
matches the selected MCP registration mode and the server simply has not
allowed that callback. Wrapper must not rewrite, relax, or bypass an
authorization server's redirect allowlist merely to turn that response into a
successful registration, and source qualification must not mutate external
OAuth account configuration.

For the selected upstream release, qualification must revalidate the actual
MCP registration path. Codex may use advertised CIMD when its prerequisites are
met or Dynamic Client Registration otherwise. Native loopback DCR can register
an HTTP callback on `127.0.0.1`/`localhost` with the active listener port and,
when issuer-bound responses are unavailable, a server-specific callback path.
The authorization server used for end-to-end qualification must therefore be
configured outside Wrapper to accept the exact callback form that the selected
upstream release legitimately requires. A provider-policy rejection before
that prerequisite is met is recorded as an external configuration blocker,
not patched in Wrapper.

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

No Termux wrapper update command delegates runtime installation or generation
replacement to upstream (`$CODEX_HOME/packages/standalone` or upstream
self-updaters). `codex update`, `codex update --upstream`,
`codex update --path`, and `codex update --rollback` remain exclusively
Wrapper-owned.

For execution, standalone Wrapper passes `codex app-server daemon` and
`codex remote-control` directly through to the active adapted upstream runtime
with FD 33 (`resolv.conf`), FD 34 (wrapper `config.toml`), FD 35 (temporary root
for Unix domain sockets fitting the 107-byte `sockaddr_un.sun_path` limit),
clean Termux environment/timezone (`TZ` TZif), and `daemon_settings::ensure`.
There is no hard block preventing upstream `app-server daemon` or
`remote-control` dispatch. Note that optional Wrapper-managed shared
app-server / catalog IPC contracts recorded below apply only if optional
Manager modules are reintroduced in a later extension layer; standalone Wrapper
starts no private `__app-server-*` daemon of its own on ordinary launches.

For interactive local upstream TUI/resume/fork launches, Wrapper may start one
shared app-server directly from the already qualified active generation before
unchanged upstream exec. This server receives the same FD-33 resolver, FD-34
system configuration, FD-35 Termux temporary directory and qualified child environment. Its private profile-local
socket is exposed at the upstream default control socket through one validated
Wrapper-owned socket symlink into a private short generation/profile namespace.
Both the Wrapper rendezvous and the upstream physical socket pathname must fit
the 107 encoded-byte bound. For an FD-35-backed shared server, the physical path must be
formed beneath the canonical private temporary root using verified UID/identity; the standard Termux
temporary root fits. Disposable proof must provide a separate private temporary
root short enough for this native path rather than nesting it under a long
fixture root. Wrapper does not bypass the native private socket ownership checks;
profile identities and authentication remain distinct. Wrapper binds a reused
server to its recorded PID, kernel socket peer PID, and exact qualified executable
through `/proc`, and
serializes startup with a kernel-held directory lock. Generation changes choose
a new namespace and preserve existing clients on the old server; they never
kill user turns as a side effect of launch. Server system configuration is a
private immutable snapshot of the effective Wrapper defaults/optional fragment and
requirements for that identity. Another launch never refreshes a live snapshot. A substituted socket,
record, directory or runtime is rejected. Startup is bounded; failure does not
invoke an installer or execute any package outside signed generations. Explicit
user embedded/remote/configuration selections retain their upstream semantics.

A same-generation Stage 2 optional frontend may advertise deferred native
startup through its non-mutating `--termux-backend-capabilities` query, which
must return `codex-termux-resolved-startup` before configuration,
terminal initialization or a server connection. Only a bounded, successful
probe enables `CODEX_TERMUX_BACKEND_STARTUP=resolved` with Wrapper's planned
socket; no server or secondary installer starts merely for this probe. The
frontend then resolves its upstream settings and asks Wrapper for the qualified
local backend. Unknown or older helpers retain their supported eager path.
The helper and Manager must belong to the selected qualified installation, not a
stale preview. Wrapper does not interpret model policy or override user-supplied
upstream profile/CLI options; an incomplete optional frontend leaves the
independently qualified Wrapper available.

A qualified Stage 2 local TUI may submit its upstream-resolved model catalog
to Wrapper's private `__app-server-catalog` operation on stdin (at most 4 MiB).
For deferred startup without a resolved catalog, the frontend requests the same
native server through private `__app-server-default`. Both private commands
also accept bounded --config VALUE/--profile VALUE pairs for the invocation;
their existing no-argument callers remain unchanged. The default command does
not consume stdin. Wrapper validates the argument transport but delegates the
meaning, precedence and policy of their values to upstream. Both operations return
only a validated socket and share the existing coordinator. Failure after deferred
selection is explicit; it never retries through an embedded executor or starts a
second native server. A catalog request remains distinct from an absent catalog;
an empty/invalid catalog is not a request for native/default behavior.
Wrapper returns only the exact qualified server socket, and uses the existing server
coordinator, environment, descriptor and retirement path. Catalog bytes, profile
home, runtime, invocation arguments and effective system config/requirements
jointly identify reuse; a changed catalog at the same source
filename must not reuse stale metadata. Wrapper stores an immutable private snapshot
and supplies its path only to backend startup, never to ordinary upstream argv.
It does not interpret profile TOML, resolve provider credentials or start another
executor. The frontend remains responsible for loading selected user/CLI/project
configuration with upstream code. Existing native servers and active siblings
remain intact. Remote/embedded launches are unaffected; failure to prepare a
requested catalog is explicit, never a fallback to an unqualified embedded backend.
Catalog or invocation-configured records live under Wrapper's private `catalogs` namespace and never
retarget the default `servers` rendezvous. Current Manager discovery
validates both families, including exact catalog content, runtime and peer PID;
generation retirement applies the same existing-client/writer protections to both.
The existing server-record codec has one implementation shared by Wrapper and
Manager: owner fields, family/namespace binding, opaque catalog/invocation bytes
and bounds. Records without invocation overrides retain their existing encoding
and namespace for their live consumers. An invocation record adds its explicit
marker and private argument snapshot; reuse validates complete bytes, not only
the routing hash. The snapshot shares the existing16 KiB configuration-record
bound, including its UTF-8 arguments and NUL separators. Unsupported/malformed
arguments fail before server launch.
Wrapper owns this execution-record contract; Manager consumes decoded bindings for
its task UX rather than independently defining Wrapper's catalog/storage format.
This is a dependency-free source library compiled into each executable, not an
installed shared runtime or a third management authority. Wrapper never imports
Manager, and Wrapper-only construction and execution require no Manager artifact.
Callers retain their filesystem trust checks and revalidate live process/peer
identity before use. The codec performs no writes, process control, model-schema
interpretation or provider selection. This introduces no registry, service or
second runtime authority.
Wrapper launches no upstream daemon updater and never creates a packages tree.
The historical public daemon/remote-control lifecycle fence remains in force.

Configuration-bound Stage 2 execution records add the `config` owner marker after
home/runtime, followed only by an optional `catalog` marker. They retain the
existing `servers`/`catalogs` families. Their namespace also binds the exact
opaque `config/config.toml` and optional `config/requirements.toml` bytes, with
absence distinct from an empty file. Each config file retains the 16-KiB bound.
Wrapper and optional consumers share this codec; it interprets no Manager/provider
policy. Reuse compares the complete requested snapshot, not only its short routing
hash. A live snapshot is immutable. Use one current codec, without historical
format emulation or a staged reader/writer release. Unsupported old experimental
records authorize no operation. Preserve their running processes and user data;
profile mutation and takeover still require native process/writer checks. Test
the complete new composition, its install/update/rollback, discovery and retirement
directly. Compatibility with superseded experimental codecs is not an admission
requirement.

(Upstream `app-server daemon` and `remote-control` run directly via upstream
passthrough with FD 33/34/35 Termux compatibility as specified above.)

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
exists. For an upstream artifact compiled as Linux, Wrapper may satisfy this
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

Official archives preserve static AArch64 `codex-path/rg` when supplied and fall back
to a thin Termux wrapper script delegating to `${PREFIX}/bin/rg` when absent; a
fresh Termux installation must still handle a missing host `rg` gracefully without
silently failing unrelated workflows. A feature that requires host tools must either
have a qualified helper/fallback or degrade explicitly with bounded
doctor/user-facing evidence. Wrapper installers and recovery paths must not install a
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
