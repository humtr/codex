# Codex Termux Rewrite Specification

Status: initial normative baseline  
Repository: `humtr/codex`  
Active implementation branch: `rewrite/rust-core`

## 1. Product definition

The product provides one public `codex` entrypoint that runs the official
upstream Codex CLI correctly on supported Termux environments.

It consists of two strictly separated layers in one repository and release
system:

1. **Core** — a minimal native Rust compatibility, execution, installation,
   update, diagnosis, activation, and rollback layer.
2. **Manager** — a separately implemented convenience layer reached through
   `codex termux` for profiles, sessions, notifications, and related Termux UX.

The current two-milestone program completes Core. Manager product contracts
are post-Core and remain optional for ordinary launch. MGR-1 through MGR-5 are
accepted source slices; MGR-5 qualifies the separately built Manager artifact
before it enters a signed generation. MGR-6 is an accepted distribution and
disposable-qualification slice for that optional artifact. MGR-7 is the
accepted remote-readback and operational-qualification slice for one explicit
Manager-bearing signed generation; it adds no source command or state. R10 is
the accepted coordinated Core + generation update slice opened by the bounded
live qualification finding that followed MGR-7.

This is a clean rewrite. Legacy source is historical evidence, not an
implementation dependency or migration base.

## 2. Product priorities

In descending order:

1. preserve a working, recoverable upstream Codex execution path;
2. never damage system resolver, auth, profile, session, or installed runtime
   state;
3. preserve upstream process behavior at the execution boundary;
4. make installation, update, activation, and rollback crash-safe;
5. keep Core small, fast to build, and fast to execute;
6. add Manager conveniences without giving Manager ownership of Core state;
7. optimize implementation velocity without weakening acceptance gates.

## 3. Public command contract

The launcher classifies only an exact first argument of `update`, `doctor`, or
`termux`. Every other invocation is passed to upstream Codex. `update` is a
Core-owned safety boundary: the installed wrapper must never execute the
upstream distribution updater, because that updater can install an unadapted
runtime on Termux. The wrapper release pipeline obtains the official upstream
package, applies the accepted Termux patch, qualifies it, binds the matching
Core artifact, and publishes a signed Core-plus-generation bundle; the
installed Core obtains and activates only that signed adapted bundle.

| Command | Owner | Required behavior |
| --- | --- | --- |
| `codex [UPSTREAM_ARGS...]` | Core | execute upstream with original arguments |
| `codex --version`, `codex -V` | upstream | print exactly the upstream version output |
| `codex update` | Core | resolve the signed stable wrapper release channel; only transport-level channel unavailability may fall back to an official-source local-derived build signed by a fresh ephemeral device-local key, activated while preserving the official `update_key`, and never published |
| `codex update --help` | Core | print the wrapper-owned update usage without invoking upstream or changing state |
| `codex update --force` | Core | retry exactly the authenticated held public release sequence for this invocation while bypassing only the local rollback-hold comparison; a valid matching hold is required and signature, digest, release-sequence, candidate-probe, and activation checks remain unchanged |
| `codex update --build-local` | Core | resolve the exact official upstream stable metadata/archive, build and probe one local-derived generation, bind it to the authenticated public baseline with a fresh ephemeral device-local Ed25519 signer, activate it without changing `update_key`, and never invoke official signing or publication |
| `codex update [INVALID_ARGS...]` | Core | reject unsupported updater options or combined selectors without invoking upstream or changing state |
| `codex update --local <DIRECTORY>` | Core | verify, stage, probe, and activate one compatible signed official/operator bundle through ordinary public-authority admission; it is not a local-derived import surface |
| `codex update --remote <HTTPS_BASE_URL>` | Core | acquire one immutable signed Core-plus-generation bundle and activate it through the local coordinated path |
| `codex update --rollback` | Core | atomically activate the retained previous complete signed generation; record a public update hold only when rolling back from an authenticated public generation, while rollback from a local-derived current creates no public hold or rollback guard |
| `codex doctor [OPTIONS]` | Core | combine upstream and Termux diagnostics |
| `codex doctor --color` | Core | explicitly request colored human diagnostics on a TTY, including when an outer Termux wrapper supplied `NO_COLOR` |
| `codex termux [COMMAND]` | Manager boundary | invoke the Manager artifact or report it unavailable |

`codex version` is not introduced. Wrapper/Core/Manager version rows must not
be appended to upstream `--version` or `-V` output.

### Manager command boundary

`codex termux` is a Manager boundary and is never passed to upstream. Manager
v1 is defined as four bounded command families:

```text
codex termux
codex termux help
codex termux profile list
codex termux profile current
codex termux profile create <PROFILE_ID>
codex termux profile use <PROFILE_ID> [--] [UPSTREAM_ARGS...]
codex termux session list
codex termux session list --all
codex termux session list --profile <PROFILE_ID>
codex termux session resume <SESSION_ID> [--profile <PROFILE_ID>] [--] [UPSTREAM_ARGS...]
codex termux notify show
codex termux notify set [NOTIFY_OPTIONS...]
codex termux repair plan
codex termux repair apply
```

`codex termux` with no command is equivalent to `codex termux help`. The
profile family was the first implementation slice. The accepted MGR-2 session,
MGR-3 notification, and MGR-4 repair commands follow their bounded contracts
below. An unavailable or not-yet-delivered Manager reports a bounded
Manager-unavailable result through the Core handoff; it never forwards an
unknown `termux` command to upstream.
Manager does not provide `codex termux install`, `codex termux update`, or a
second doctor/version authority. Installation, update, rollback, and top-level
doctor remain Core commands.

`PROFILE_ID` is one ASCII path-safe identifier of 1--64 bytes beginning with
an alphanumeric character and containing only alphanumerics, `.`, `_`, or
`-`. `default`, `home`, `termux`, `.`, and `..` are reserved aliases or
rejected names. `SESSION_ID` is treated as an opaque bounded upstream
reference after syntax validation; it is never used as a filesystem path.
Arguments after `--` are forwarded byte-for-byte to the Core entrypoint. A
Manager child launch preserves standard streams, TTY, signals, process exit
status, and raw argument bytes at the final Core execution boundary.

The generated upstream hook command may call the bounded internal Manager
endpoint `codex termux notify emit <EVENT>`. It is not a configuration or
upstream passthrough form, is not shown by Manager help, and accepts only one
canonical event name. Core remains the public-launch owner: Manager never
writes Core's config directory or asks Core to execute an arbitrary command.

Manager command failures use the shared public classes: `0` for success, `1`
for an operational or child failure, `2` for usage/validation/policy failure,
and `130` for an interactive cancellation or interrupt. A Manager must not
echo invalid raw arguments, credentials, session content, or arbitrary
environment values in an error.

The Core-owned doctor surface accepts exactly no arguments, `--json`, or
`--color`. `--json` and `--color` are mutually exclusive. `--color` is an
explicit human-output override for interactive diagnostics: when stdout is not
a TTY, output remains plain; when it is a TTY, Core removes only the inherited
`NO_COLOR` value from the bounded upstream-doctor child and uses the Termux PTY
capture path. It never changes the caller's environment or enables color in a
JSON envelope.

The Core-owned update surface accepts exactly no arguments, `--help`, `--force`,
`--local <DIRECTORY>`, `--remote <HTTPS_BASE_URL>`, or `--rollback` after
`update`. No top-level `codex rollback` command is introduced. A malformed
invocation beginning with one of those Core selectors is a Core usage error. The no-argument
form is the primary product path: it first tries the signed stable wrapper channel
and may then run the local release-production path defined in Section 8 when the
channel is unavailable at the transport boundary, is already current, or its
otherwise valid candidate is suppressed only by the local rollback hold. A
verified index with an invalid signature, malformed fields, an incompatible
release, a digest or mode mismatch, a candidate-probe failure, or an activation
failure is never converted into a local-build fallback. The explicit local,
remote, and rollback forms remain secondary diagnostic/recovery paths; local and
rollback remain offline, and the explicit remote form remains an immutable
signed-generation source. No top-level `codex update` argument is passed to
upstream, and Core never invokes a package manager or an upstream self-updater.

Rollback is an explicit Core operation, not an ordinary-launch fallback and not
a search through generation history. After a rollback activation commits, Core
atomically records a separate bounded exact-format `update-hold` file containing
only the authenticated generation identity and signed release sequence that were
current before rollback. Activation-state v3 is not extended. The hold file must
be a regular file, must reject malformed/oversized/symlink state, and must use
temporary-create, file sync, atomic rename, and parent-directory sync semantics.
A failed rollback must not create or change the hold.

The first rollback from a hold-aware Core to an older Core that does not advertise
exact `codex-update-hold-v1` capability must not lose hold enforcement. Core may
therefore retain the rolled-back-from signed Core launcher as a bounded rollback
control-plane guard while the runtime, Manager, helpers, and authoritative current
pointer roll back to the retained previous signed generation. The guard is a
separate exact-format Core-owned state record; it must bind the rollback target
generation, the held generation, the held signed release sequence, and the held
generation's signed Core digest. The stable launcher is accepted under this
exception only when the authoritative pointer pair matches the guard, the held
previous generation verifies under its retained verifier key, and the launcher
digest exactly equals that held signed Core digest. No unsigned or arbitrary Core
path is authorized. A target Core that advertises `codex-update-hold-v1` uses the
normal full Core rollback instead and does not retain the guard.

Core writes the guard intent durably before the pointer swap but treats it as an
effective hold only after the authoritative pointer becomes the exact guarded
rollback pair. This closes the commit-to-hold crash window without treating a
failed rollback as held. After the committed rollback, Core writes the normal
`update-hold`; a hold-aware target then removes the temporary guard, while a
legacy target retains it until a force retry or greater-sequence activation
restores a normal Core/generation pairing. Malformed, mismatched, stale, or
unverifiable guard state fails closed whenever it is needed to justify a launcher
mismatch.

Ordinary update applies signature, digest, and release-sequence validation before
the hold. After rollback, an otherwise eligible candidate that does not exceed the
held signed release sequence must not stage, probe, or activate; the hold therefore
prevents both the exact bad sequence and any intermediate sequence from displacing
the retained evidence. A differently named same-sequence repack cannot bypass it.
A greater release sequence is eligible normally. After a greater-sequence
activation commits, Core removes the now-obsolete hold and guard while still
inside the activation writer boundary; a failed activation must not clear them.
`codex update --force` requires a valid authenticated hold and may retry exactly
that held sequence for that invocation. It bypasses only the hold comparison; a
same-held-sequence force retry retains the normal hold while removing any now-stale
rollback-Core guard after the normal Core/generation pairing is restored. Force
never enters the transport-unavailable local-build fallback and cannot target an
unheld or greater sequence. Neither path weakens signature, digest, anti-rollback,
candidate-probe, or atomic-activation rules.

Internal release IDs, component digests, API versions, and schema versions are
still mandatory for update, diagnosis, and rollback. They may appear only on a
clearly Termux-specific status or redacted machine-diagnostic surface.

## 4. Architecture and ownership

### 4.1 Core

Core exclusively owns:

- public command dispatch;
- official upstream artifact qualification;
- Termux runtime adaptation and launch environment;
- FD 33 and FD 34 preparation;
- final upstream process execution;
- sandbox capability enforcement;
- immutable generation construction and integrity metadata;
- update, activation, last-known-good selection, and rollback;
- read-only Core diagnosis;
- composition of upstream and Termux doctor results;
- the typed boundary through which Manager requests Core operations.

Core must remain usable when Node.js, Manager, network access, update services,
or optional Termux APIs are unavailable.

### 4.2 Manager

Manager owns:

- `codex termux` command UX;
- profile selection and presentation;
- session indexing and selection;
- notification configuration and delivery;
- Manager-local state and UI;
- repair planning and requests to Core.

The Manager executable is built separately from Core and is an optional signed
generation asset. Core does not compile it, discover it from `PATH`, or probe
it during ordinary launch. Release-builder owns the bounded artifact probe;
Core consumes only the signed generation and its digest-bound Manager path.

Manager must not directly write Core generations, pointers, manifests, locks,
runtime state, resolver data, or activation journals. A Manager request that
would mutate Core state must use a versioned, runtime-validated Core contract.
The Manager may create and update only its declared state and an explicitly
created profile-home directory. It must not copy, parse, summarize, or rewrite
authentication files, cookies, OAuth material, arbitrary upstream state, or
session transcript content. Profile execution changes `CODEX_HOME` only in the
child environment and returns through the stable Core entrypoint; it never
changes the caller's environment or persistent Core state. Manager commands
that need an update, rollback, doctor, or other Core operation must request the
corresponding Core-owned command through that entrypoint and must not
reimplement the operation.

The Manager artifact is optional and independently qualified. Its absence,
incompatibility, or failure must not make ordinary upstream launch, Core
doctor, Core update, or Core rollback unavailable. Manager v1 has no network,
package-manager, OpenSSL, bwrap, resolver, or generation-discovery authority.
Its first profile slice is read-only except for the explicit `profile create`
operation and the normal upstream writes made after a profile launch.

TypeScript is the preferred Manager implementation language, but no Manager
runtime or dependency may become a prerequisite for ordinary upstream launch,
Core doctor, update, or rollback.

### 4.3 Shared contracts

Core and Manager may share only explicit versioned data contracts. Compile-time
types are insufficient; every external or cross-layer payload is validated at
runtime. Unknown incompatible schema versions fail without mutation.

## 5. Termux runtime contract

The first supported release target is `aarch64-linux-android` on a supported
Termux installation. Release users must receive a prebuilt Core and must not be
required to install Rust, Clang, or Cargo.

Core must derive runtime paths from the actual environment and must not embed a
single app data path as product authority.

Before final upstream execution Core must:

- construct the qualified runtime environment without leaking package-manager
  or preload variables;
- preserve stdin, stdout, stderr, TTY behavior, signals, and upstream exit
  status;
- open the selected resolver source read-only and make it available on FD 33;
- make the process-local managed configuration directory available on FD 34;
- ensure those descriptors survive the final `exec` boundary;
- use the selected official runtime and compatibility tool paths;
- report unsupported Linux sandbox requests clearly.

Core must never create, rewrite, chmod, repair, or delete
`$PREFIX/etc/resolv.conf`. Resolver diagnosis is read-only.

Linux namespace/bwrap sandboxing is not a Termux capability of this product.
Core must not claim that `read-only` or `workspace-write` Linux sandbox modes
are enforced. Ordinary supported launch uses the explicitly selected upstream
no-sandbox policy; unsupported sandbox requests fail clearly rather than
silently weakening the request.

Core never invokes, installs, downloads, or repairs `bwrap`. An explicit Linux
sandbox-policy rejection is a Core usage/policy failure with process status 2
and must occur before resolver/configuration descriptor setup, upstream
execution, generation construction, bootstrap publication, or activation-state
mutation. A failure of a development runner's own sandbox is infrastructure
evidence only and is not a product-path result.

## 6. Artifact and patch qualification

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
does not compile itself, install a Rust toolchain, or execute an upstream
self-updater. It contains the same bounded release-production routines needed
by the no-argument local fallback; those routines may fetch the official
archive, adapt the raw runtime, qualify the result, and sign it with the
already-authorized update key. The fallback never accepts a raw archive as an
activation candidate and never bypasses signed local admission. The builder's
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
the `codex-release-v4` signing and delivery path. Core's local fallback carries
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
using the explicit OpenSSL executable. It emits the non-rotating v3 form only:
the derived key is written as `release_public_key`, `release.sig` signs the
exact `release.manifest` with that key, and no `release-authority.sig` is
created. Therefore an operator producing an ordinary public/non-rotating signed release
with this command must supply the current Core `update_key` private key; this
operation does not implement key rotation. Core's local-derived path is a
different in-process authority boundary: it generates a fresh ephemeral key and
never reads an official release private key. The `publish` command's supplied
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

Official release production is not a Core runtime behavior. `codex update`,
`--force`, `--rollback`, `--local`, and `--remote` never build or sign an
official release, never read an official release private key, never invoke `gh`,
and never stage, deploy, or promote public stable. The presence of
`CODEX_TERMUX_UPDATE_PRIVATE_KEY`, the former device-local maintainer key path, or
an authenticated `$PREFIX/bin/gh` account is inert to Core update semantics.
Signed-channel exact-current and rollback-held results therefore remain consumer
outcomes; they do not trigger ambient upstream discovery or release production.

Official source production remains an explicit, non-installed tooling boundary.
The repository's `codex-release-builder` `fetch`, `build`, and `publish` commands
may be orchestrated by an authorized producer: `fetch` selects and verifies the
exact official upstream metadata/archive, `build` performs the required Termux
adaptation and qualification, and `publish` constructs the signed immutable
release/index publication tree using an explicitly supplied signing authority.
Those commands do not make `codex update` a producer and do not themselves grant
network publication authority. GitHub-hosted scheduling, secret-backed signing,
and Release/Pages promotion are separate later RALD phases.

The repository-owned hosted producer preflight is
`.github/workflows/auto-release-termux.yml`. Its source contract is a fixed
accepted commit SHA, never a moving branch head. When installed on the default
branch it may run every six hours and by manual dispatch, with one serialized
producer concurrency group and read-only repository permissions. RALD-3 is a
pre-sign dry-run boundary only: it may read and authenticate the current public
stable channel, read official OpenAI stable metadata, cross-build Core and
Manager for Android/AArch64, adapt one unsigned candidate, transfer that
candidate only as a short-lived Actions artifact, and run a Termux-compatible
Android/AArch64 executable smoke. It must not read an Actions signing secret,
sign a release, create or alter a GitHub Release, dispatch Pages, write `main`,
or advance the stable index. Secret-backed signing and all public mutation remain
RALD-4 and RALD-5 respectively.

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

By contrast, `codex update --build-local` and a transport-unavailable bare-update
fallback never read that key path, `CODEX_TERMUX_UPDATE_PRIVATE_KEY`, or any
repository release secret. They generate one fresh Ed25519 key in private
owner-only temporary storage, derive its public verifier, sign only the local
immutable candidate, delete the private key before activation can succeed, and
remove the complete private staging tree on success or ordinary failure. They
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

Core runtime has no official publication path and never invokes
`$PREFIX/bin/gh`. Any later authorized official producer must keep signing and
publication authority outside Core; GitHub authentication alone never authorizes
publication. When such a producer is authorized to publish, its complete official
candidate targets the fixed wrapper repository `humtr/codex` and the selected
stable publication ref. A release whose
complete signed file inventory is flat may use
immutable GitHub Release assets under a tag equal to the validated generation
identity; the signed index's `release_base` is then the matching
`https://github.com/humtr/codex/releases/download/<generation_id>/` asset base.
GitHub Release asset names are not authority to rename signed relative paths: if
any authenticated release file contains a `/` path component, the automated
Release-asset publisher must fail closed before creating a release or advancing
the index.

An explicitly authorized nested-path publication uses the repository's fixed
GitHub Pages workflow instead of changing signed release paths or Core fetch
semantics. A GitHub Release tagged by the generation identity is staging only:
top-level signed files keep their basename, while the exact two R10 bridge
helpers are uploaded under unambiguous staging names. The workflow accepts only
a stable `codex-release-v4` whose trusted public-key identity is the pinned
wrapper key, verifies `release.sig`, requires exact
`creation_metadata = "r10-browser-helper-bridge-v1"`, exact helper identities
and `helpers/0`, `helpers/1` signed inventory, verifies every signed file digest,
and reconstructs the exact signed release tree under
`<generation_id>/`. Because a Pages deployment replaces the whole site, the
workflow must first verify the currently signed stable index and current stable
release, then reconstruct both that current generation and the candidate in the
same Pages artifact. Staging names are never release authority and never appear
in a signed manifest. The candidate signed `release_base` is exactly
`https://humtr.github.io/codex/<generation_id>/`. The combined current-plus-
candidate site must remain below the GitHub Pages one-gibibyte site bound. Every
preserved-current signed payload is digest-verified during reconstruction, and
complete candidate HTTPS readback of the manifest, signature, signed index, index
signature, and every signed file must byte-match the local signed publication
before stable promotion. The fixed workflow file itself may be mirrored to
`main` through the Contents API; generation bytes must not be sent through the
Contents API.

Until the stable compatibility floor is newer than R10, the public stable target
must itself use the exact R10 browser-helper bridge layout even when its Core is
newer. This permits a retained sequence-7 R10 client to consume the final stable
generation directly. A newer canonical local generation may continue to use
`browser/open/curl` and `browser/manual/curl`; canonical nested paths are not an
R10-readable public stable target.

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
are rejected. There are at most 32 logical entries, paths are canonical relative
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

Patch policy `termux-fd-remap-v1` changes only `bin/codex` through these
equal-length substitutions:

| Source bytes | Required count | Replacement bytes |
| --- | ---: | --- |
| `/etc/resolv.conf` | 2 | `/proc/self/fd/33` |
| `/etc/codex/config.toml` | 1 | `/dev/fd/34/config.toml` |
| `/etc/codex/requirements.toml` | 1 | `/dev/fd/34/requirements.toml` |
| `/etc/codex/managed_config.toml` | 1 | `/dev/fd/34/managed_config.toml` |

Every replacement must occur zero times before adaptation. Missing, extra, or
already-patched occurrences reject the input. The output differs only at the
selected byte positions; its deterministic patch report binds the archive,
raw-runtime, adapted-runtime, and code-mode-host SHA-256 values, the four source
counts, and the changed-byte count.

The unsigned output contains exactly `generation.meta`, the adapted `runtime`,
and an unmodified root-level `codex-code-mode-host` beside `runtime`; the first
target declares zero `helpers/<index>` artifacts without changing that optional
generation contract. The root-level placement is required because upstream
Codex resolves this companion beside its own executable, not through `PATH`.
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

The Milestone 2 local layout is:

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
~/.local/lib/codex/core/publications/<id>/             locally built signed publication cache
~/.local/lib/codex/core/release-public-key.pem         bootstrap-only initial trust seed; never update fallback
~/.local/share/codex/core/activation-state            authoritative generation + bounded trust state
~/.local/share/codex/core/activation-journal[.tmp]    crash-recovery transaction state
~/.local/share/codex/core/activation-state.tmp        atomic state publication temporary
~/.local/share/codex/core/core-entrypoint-rollback    one retained prior Core launcher
~/.local/share/codex/core/core-entrypoint-rollback.meta
                                                      rollback binding for that launcher
~/.local/share/codex/core/config/                     process-local managed config directory
~/.local/share/codex/manager/                         Manager-owned mutable state
```

The Manager v1 state root is independent of the Core root:

```text
~/.local/share/codex/manager/
  state-v1                                           selection/config metadata only
  profiles/<PROFILE_ID>/profile.meta                 Manager profile record
  profiles/<PROFILE_ID>/home/                        selected upstream CODEX_HOME
  notifications/config-v1                            notification configuration
```

`state-v1`, `profile.meta`, and `config-v1` are versioned Manager records and
contain no tokens, cookies, OAuth values, private keys, or session bodies.
Manager creates profile directories with mode `0700` and record files with
mode `0600`. It publishes a new profile tree with create-new atomic rename;
selection/config record replacements use a private same-directory temporary,
final-mode synchronization, atomic rename, and parent synchronization. A
profile directory is accepted only when its path components are real
directories rather than symlinks and its `PROFILE_ID` passes the command
grammar. The `home/` child becomes an upstream `CODEX_HOME` only for a child
launch; Manager does not interpret the files written there by upstream Codex.
The default profile is the existing upstream default home and is never copied
into this tree.

The MGR-1 Manager records use exact UTF-8 text formats with a final newline.
`state-v1` contains exactly:

```text
codex-manager-state-v1
last_profile\t<PROFILE_ID>
```

where the profile ID is `default` or an existing custom profile. An absent
`state-v1` means `default`. Each `profiles/<PROFILE_ID>/profile.meta` contains
exactly:

```text
codex-manager-profile-v1
id\t<PROFILE_ID>
```

The derived profile-home path is not duplicated inside metadata. Unknown
records, duplicate records, invalid UTF-8, or a mismatched ID invalidate that
record and never cause a path to be followed. MGR-1 does not write
`notifications/config-v1`; MGR-3 owns that record.

Manager v1 does not persist a session transcript or a second session database.
The future session index is a bounded read-only projection of upstream session
metadata: malformed, oversized, symlinked, or unreadable entries are skipped
or reported as unavailable, and message bodies, auth-derived fields, and
arbitrary path data are never emitted. Cross-profile session copying,
symlink-sharing, and auth-state migration are outside v1.

The authoritative state format is `codex-activation-state-v3`. It owns one
forward `update_key`, one `current` generation with its exact `current_key`, and
at most one `previous` generation with its exact `previous_key`. Ed25519 public
keys in state are the canonical 32 raw key bytes encoded as exactly 64 lowercase
hex digits. The `previous` generation and `previous_key` are present or absent as
one pair. There is no separate `verified` pointer, keyring, discovered-key set,
or unbounded key history.

Only content that has already passed release admission and candidate probes may
become `current`. Ordinary launch reads only `current`; it does not perform
signature verification, consult `update_key` or either generation verifier key,
scan generations, contact the network, invoke OpenSSL, or implicitly fall back
to another generation. The generation directory name is one safe path component
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
exception: its ephemeral public verifier becomes `current_key`, the former
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

`install-online.sh` accepts no arguments. It is only a bootstrap transport
frontend for a fresh target; it is not an update command, alternate trust
authority, package-manager installer, release producer, or legacy handoff. It
requires the existing Termux shell, `$PREFIX/bin/curl`, and
`$PREFIX/bin/openssl`; it never installs or discovers replacements for them.
Its production locators are fixed project HTTPS URLs, not caller-selected
mirrors.

Before trusting any network-selected generation, the online frontend retrieves
the bootstrap public key from the fixed project release locator and requires
the exact repository-pinned SHA-256 of those key bytes. It then retrieves the
canonical stable `update-index-v1` and sibling signature and verifies that
signature with the pinned bootstrap key before parsing the generation identity
or `release_base`. HTTPS transport alone is never release authority. The
verified index must have the exact stable four-record grammar already consumed
by Core, and the release base must be canonical HTTPS ending in the signed
generation identity.

The online frontend retrieves that generation's `release.manifest` and
`release.sig`, verifies the manifest with the same bootstrap key before using
its inventory, and accepts only the existing bounded v4 bootstrap inventory:
`generation.meta`, `core`, `runtime`, `codex-code-mode-host`, optional
`manager`, and exactly one supported two-helper layout. It downloads only
those signed relative paths into a private `$TMPDIR` workspace with the Core
remote-acquisition per-file and total byte ceilings and applies only the
signed regular-file modes needed by the existing bootstrap admission. It does
not make downloaded file digests authoritative; the audited bootstrap/Core
admission re-verifies the signed manifest, descriptor, digests, modes, Core
binding, candidate probes, and activation transaction.

The network frontend obtains the audited local `install.sh` and
`bootstrap/codex-bootstrap` only from one immutable accepted repository commit
chosen by the RALD-6 source, recreates their sibling layout in the private
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

Normal installation and update must not require on-device Rust, Cargo, or
Clang compilation. The no-argument update fallback is a bounded execution of
the prebuilt release-builder routines already contained in Core; it is not a
Core self-recompile or a package-manager installation.

After bootstrap, the Core owns every top-level `codex update` form. `install.sh`
is not an update dispatcher and must not bypass the authenticated local
admission, staging, probe, activation, or rollback path. The local, explicit
remote, and automatic channel forms use the same forward activation
transaction, and rollback remains the explicit swap of the one retained
complete previous generation. A bare `codex update` is never the upstream command: it resolves a
wrapper-owned signed channel and, only when automatic-channel transport is
unavailable, may enter the same official-source local-derived construction used
by `--build-local`. It therefore cannot install an unpatched upstream runtime or
turn transport failure into official publication.

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
manager, an alternate mirror, or a raw package. A transport-level absence or unavailability while resolving the automatic
channel may enter the local-derived construction path below; an index or release
that was received but failed signature, format, policy, digest, mode,
compatibility, probe, or activation validation may not.

The signed index is a pointer to an already-adapted wrapper generation, not an
upstream source authority. The only upstream source authority is the official
versioned OpenAI archive acquired by the release-production `fetch` operation
and consumed by `build` before qualification and signing. In the primary no-argument path, when the wrapper publication is
transport-unavailable, Core may perform the exact official fetch/build/adapt
steps locally, sign the immutable candidate with a fresh ephemeral device-local
key, bind it to the authenticated public baseline, and feed it only to the
dedicated local-derived admission path. That path performs no official
publish/signing action. Core never downloads a raw upstream archive directly
into the active generation and never runs an upstream self-updater.

`codex update --build-local` is an explicit selector owned by Core. It is valid
only by itself. It resolves the same exact official stable metadata and versioned
archive used by the qualified release builder, verifies the metadata-bound
archive digest, builds/adapts in private staging, signs with a fresh ephemeral
Ed25519 key, runs the normal candidate probes, and atomically activates the
result. The presence or absence of an official release private key has no effect
on this route. The same route is used by the allowed automatic-channel transport
fallback.

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
   transport-level automatic-channel absence enters the local build path,
   which resolves and digest-binds one exact official upstream version before
   building);
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
   sign, and probe a complete candidate generation when the local fallback is
   selected;
6. atomically publish the new trust-and-generation state;
7. retain one complete previous generation with its exact verifier key as
   rollback state;
8. report failure without damaging the active trust-and-generation state.

One migration-only coordinated v4 bridge is permitted for an already-installed
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
indexed layout without the exact marker fails closed.

The bridge carries the same qualified browser helper bytes and policy as the
canonical TC-2 layout; it does not temporarily remove browser protection. A
new Core may retain read support for this exact bridge layout so the bridge can
run and can be the one retained rollback generation. Normal generation builds
and publications must continue to use only `browser/open/curl` and
`browser/manual/curl` for these identities and must never select the bridge
layout implicitly. The bridge adds no alternate signing key, bootstrap path,
package-manager action, PATH widening, raw-runtime patch, or second activation
mechanism. Its sole purpose is one signed R10-compatible forward step before a
second ordinary signed update publishes the canonical browser-helper layout.

The signed release format is `codex-release-v3`; v1 and v2 are not retained as
release compatibility paths. In addition to generation identity, monotonic
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
ephemeral verifier in `current_key` as defined above.

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
activation state exists, bootstrap may verify one initial v3 release only when
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
are exactly the files named by the signed inventory.

Inventory URLs are derived only by preserving `/` separators and percent-
encoding every UTF-8 path byte outside the RFC 3986 unreserved set. Every
resulting resource URL is also at most 4,096 ASCII bytes. After signed control
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
300-second transfer timeout, follows redirects only with `--location` and
`--proto-redir =https`, and writes response bytes only to a caller-created
regular file. The manifest limit remains 128 KiB and each signature file is
limited to 1 KiB; each generation-file response is limited to 512 MiB and the
sum of all response bytes is limited to 1 GiB. Curl and Core both enforce the
applicable remaining byte bound. A transport or bound failure is terminal for
that explicit attempt.

Acquisition uses one create-new `.acquire-<pid>-<counter>` directory beneath the
generation root and create-new output files beneath that directory. Core attempts
cleanup after every acquisition outcome, and remote success is impossible until
that cleanup succeeds. A cleanup error is terminal and preserves the authoritative
activation pointers. An uncatchable process kill or a filesystem cleanup failure
may leave a dot-prefixed partial directory; Core never scans, launches, verifies
as a generation, or activates such a path, and later work must not add a retry,
fallback, or registry merely for it. Remote success reports
`activated remote generation <id>`; every failure preserves the authoritative
activation pointers.

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

## Manager v1 definition (post-Core)

This section defines work after the two Core milestones. It does not weaken or
extend the Core completion threshold, and it does not make Manager a
prerequisite for ordinary upstream launch, Core doctor, update, rollback, or
fresh installation. Manager is an optional, separately qualified artifact
behind the existing `codex termux` boundary.

### MGR-0 — process and ownership boundary

Core selects Manager only from the signed, qualified generation and invokes
the artifact with a versioned handoff environment:

```text
CODEX_TERMUX_CORE_API=codex-manager-core-v1
CODEX_TERMUX_CORE_ENTRYPOINT=<validated stable Core entrypoint>
```

MGR-4 repair requests use the additional exact internal handoff values
`CODEX_TERMUX_CORE_REQUEST=codex-manager-repair-v1` and
`CODEX_TERMUX_CORE_OPERATION=plan|apply`. The Manager supplies `plan` with
the Core argv shape `doctor --json` and `apply` with the Core argv shape
`update` and no arguments. Core consumes these values before public dispatch;
they are never forwarded to an upstream runtime or provider process. A
missing, malformed, or mismatched request/argv pair fails closed.

The Manager validates both values before doing work. Its only route back to
Core is an `exec` of that validated entrypoint with one of the explicitly
allowed Core-owned argv shapes: ordinary upstream argv whose first token is
not the exact Core selector `termux`, `doctor`, or `update`; `doctor` with its
Core-owned options; `update` with its Core-owned options; or `update
--rollback`. It cannot address a generation path, activation state,
trust key, resolver, or journal directly. Core remains the final validator of
every requested route. MGR-0 has no callback socket, network protocol, or
second state authority.

Manager receives no credential or session-content payload from Core. It may
inherit ordinary process environment needed for a child launch, but it must
not print or persist that environment. A missing, malformed, or incompatible
handoff fails before any Manager state mutation.

### MGR-1 — profile selection and isolated launch

MGR-1 is the first implementation bundle. It implements only the profile
commands from the public grammar:

```text
codex termux profile list
codex termux profile current
codex termux profile create <PROFILE_ID>
codex termux profile use <PROFILE_ID> [--] [UPSTREAM_ARGS...]
```

`default` is the existing upstream default home and is never copied. A custom
profile uses the derived path
`~/.local/share/codex/manager/profiles/<PROFILE_ID>/home`. `profile create`
creates only that directory and its Manager metadata; it never creates,
copies, parses, or edits `auth.json`, session files, logs, or other upstream
state. Existing legacy profile directories are not imported implicitly.

`profile list` emits `default` followed by valid custom profile IDs, one per
line, in deterministic bytewise order. It ignores symlinked or malformed
entries rather than following them. `profile create <PROFILE_ID>` emits
exactly `created: <PROFILE_ID>` followed by one LF after the create-new
transaction commits; it emits no path, environment, credential, or
upstream-state detail. `profile current` emits exactly two LF-terminated
lines, `current: <TARGET>` followed by `source: <SOURCE>`. `<TARGET>` is
`default`, a valid custom profile ID, or `external`; `<SOURCE>` is `inherited`
when the caller supplied `CODEX_HOME`, or `last-selection` otherwise. It
never reports auth identity, token state, session bodies, paths, or arbitrary
environment values. `profile use` requires an existing profile, atomically
records the selected ID, then `exec`s Core and emits no Manager-owned success
output before Core runs. For `default` it removes `CODEX_HOME` from the child
environment; for a custom profile it sets `CODEX_HOME` to the validated
profile home only in that child. The original upstream argv after the profile
selector is preserved exactly. If selection-state publication fails, Core is
not launched.

MGR-1 does not implement profile deletion, cross-profile session copying,
interactive terminal UI, or profile-auth migration. A missing profile is a
non-mutating validation failure; it is never created as a side effect of
launch.

### MGR-2 — bounded session listing and resume

MGR-2 adds exactly these local, non-interactive forms:

```text
codex termux session list
codex termux session list --all
codex termux session list --profile <PROFILE_ID>
codex termux session resume <SESSION_ID> [--profile <PROFILE_ID>] [--] [UPSTREAM_ARGS...]
```

`--all` and `--profile` are mutually exclusive for `session list`; no other
session-list option is accepted. For both commands, an omitted `--profile`
uses the persisted MGR-1 last-selection target, and does not reinterpret an
inherited arbitrary `CODEX_HOME` as a Manager profile. `default` and `home`
select the existing `$HOME/.codex` home; a custom selector must name a
complete MGR-1 profile. `--all` visits `default` followed by every complete
custom profile in bytewise profile-ID order. An invalid persisted selection or
an incomplete explicit profile is a non-mutating Manager operation failure.

The discovery root for a profile is `<PROFILE_HOME>/sessions`. A missing root
produces an empty successful list. The root and every traversed directory must
be a real non-symlink directory. Discovery traverses at most eight directory
levels and 4,096 directory entries in one command. It considers only regular,
non-symlink files whose final name ends in `.jsonl`; the `SESSION_ID` is the
UTF-8 filename with only that final suffix removed. A session reference is
1--256 ASCII bytes, begins with an alphanumeric byte, permits only
alphanumerics, `.`, `_`, `-`, and `:`, and rejects `.` and `..`. The reference
is opaque: Manager never parses its timestamp or concatenates it into a path.
Entries with invalid references, special types, symlink components, unreadable
files, a size over 64 MiB, or a negative/unavailable mtime are skipped. Manager
opens a candidate only to establish readability and never reads session-file
bytes, parses JSONL, or derives a title, worktree, branch, auth field, or
message field. If the bounded directory-entry limit is exceeded, the command
returns an unavailable operation result rather than emitting a partial list.

`session list` emits no header and exactly one LF-terminated TSV row per
accepted candidate:

```text
<PROFILE_ID>\t<SESSION_ID>\t<UPDATED_UNIX_SECONDS>\n
```

`UPDATED_UNIX_SECONDS` is the nonnegative decimal filesystem mtime in UTC
seconds. Rows sort by newest timestamp first, then profile ID, session
reference, and an internal bytewise path tie-breaker. An empty discovery emits
no stdout and returns success. The output contains no paths, session contents,
working-directory values, or credentials.

`session resume` validates the reference grammar, resolves one selected
profile, performs a fresh bounded discovery of that profile, and requires
exactly one row with the supplied reference. A missing or ambiguous reference
fails without selection or Core launch. The selected profile is then recorded
through the existing MGR-1 atomic selection transaction. Manager invokes the
validated Core entrypoint with exactly `resume`, the discovered opaque
`SESSION_ID`, and the original trailing upstream argv; it sets or removes
`CODEX_HOME` only in that child as MGR-1 does, emits no Manager success output,
and preserves Core's streams, TTY, signals, raw arguments, and exit status.
No session is copied, symlinked, rewritten, migrated, or indexed persistently.
Interactive session UI, cross-profile sharing, and transcript inspection are
outside MGR-2.

### MGR-3 — notification configuration and delivery

MGR-3 adds exactly these user-facing local forms:

```text
codex termux notify show
codex termux notify set [--channel <notification|toast|both>]
    [--hooks <none|all|EVENT[,EVENT...]>]
    [--content-chars <0|1..4096>] [--preserve-newlines <0|1>]
    [--toast-gravity <top|middle|bottom>] [--toast-short <0|1>]
    [--toast-background <empty|#RRGGBB>] [--toast-color <empty|#RRGGBB>]
    [--group <GROUP_ID>]
```

`notify set` accepts options in any order, at most once each, and no trailing
arguments. An option omitted from `set` retains the stored value; when no
record exists, omitted values use these defaults: `channel=notification`,
`hooks=Stop`, `content_chars=0`, `preserve_newlines=1`,
`toast_gravity=top`, `toast_short=0`, empty toast colors, and
`group=codex-turns`. `0` means no user-configured character limit, but delivery
still applies a hard 4,096-byte payload bound. `none` disables all hooks.

The canonical event allowlist and order are:
`SessionStart`, `PreToolUse`, `PermissionRequest`, `PostToolUse`,
`PreCompact`, `PostCompact`, `UserPromptSubmit`, `SubagentStart`,
`SubagentStop`, and `Stop`. Hook lists contain unique canonical names, or
`all`; malformed names, duplicates, empty list members, and invalid values are
usage failures. Event lists are stored and displayed in the canonical event
order above. `GROUP_ID` is a 1--64 byte ASCII identifier beginning with an
alphanumeric byte and containing only alphanumerics, `.`, `_`, and `-`.
Colors are empty or exactly `#` followed by six ASCII hexadecimal digits and
are stored canonically in lowercase.

The Manager record at `notifications/config-v1` is mode `0600` beneath a
mode-`0700` `notifications` directory and contains exactly these final-newline
UTF-8 lines, in order:

```text
codex-manager-notify-v1
channel\t<CHANNEL>
hooks\t<none|all|EVENT[,EVENT...]>
content_chars\t<DECIMAL>
preserve_newlines\t<0|1>
toast_gravity\t<GRAVITY>
toast_short\t<0|1>
toast_background\t<empty|#rrggbb>
toast_color\t<empty|#rrggbb>
group\t<GROUP_ID>
```

`notify show` emits exactly these nine public `key=value` lines for the
effective configuration, in this order: `channel`, `hooks`, `content-chars`,
`preserve-newlines`, `toast-gravity`, `toast-short`, `toast-background`,
`toast-color`, and `group`. It emits no path, source detail, payload,
environment, or credential. An absent record yields the defaults without
creating Manager state. A malformed, symlinked, overlong, incorrectly-modeled,
or conflicting record is an operation failure and is never replaced
implicitly. `notify set`
merges validated values and publishes the complete record with a private
same-directory create-new temporary, file synchronization, atomic replacement,
and parent synchronization. A successful `notify set` emits exactly `saved\n`.
It never writes an upstream profile, Core generation, activation state, or
notification payload.

For an ordinary upstream launch, Core may read this exact bounded Manager
record read-only. Core alone renders the enabled hooks into its own managed
`config.toml`; Manager never writes that Core directory. The generated file is
owned by Core, carries a fixed `codex-termux-notify-v1` marker, contains only
the enabled hook blocks, and is atomically replaced before runtime exec. Core
replaces a missing file or its own marker file only; an unrelated regular file
is preserved and the optional hooks are skipped. Each enabled event is mapped
to `codex termux notify emit <EVENT>`. Missing or invalid Manager notification
state, or a generation without a qualified Manager artifact, disables the
optional hooks and must not make ordinary upstream launch fail.

The internal `notify emit <EVENT>` endpoint reads at most 64 KiB of hook input,
which must be a UTF-8 JSON object when delivery is requested. It considers
only the string field `title` for the notification title and, independently,
the first string body field in this precedence: `content`,
`last_assistant_message`, then `message`; no other field is inspected.
Missing title uses `Codex`; missing body uses the fixed event status strings
`Notify session start`, `Notify tool start`, `Notify permission request`,
`Notify tool finish`, `Notify before compact`, `Notify after compact`,
`Notify prompt submit`, `Notify subagent start`, `Notify subagent stop`, and
`Notify turn completion`, in the canonical event order above. Malformed or
oversized input is a successful no-op. The selected text is normalized for
CRLF/CR, then the configured character limit and final 4,096-byte cap are
applied, and the result is passed only to the selected Termux providers. It
never persists, logs, or prints the input, notification content, paths,
credentials, or session data.
Provider absence, provider failure, malformed hook input, and disabled hooks
are successful no-ops so an upstream turn cannot fail because notification
delivery is unavailable. `notification`, `toast`, and `both` select the
corresponding capability-aware provider attempts; `both` attempts each
independently. Manager emits no success text for the endpoint.

### MGR-4 — repair planning through Core

MGR-4 adds exactly these no-option, non-interactive forms:

```text
codex termux repair plan
codex termux repair apply
```

Any option or trailing argument is a usage failure. The Manager validates the
normal MGR-0 handoff, then `exec`s the validated Core entrypoint without
printing a Manager success line. `repair plan` submits `doctor --json` with
the versioned repair request and `repair apply` submits `update` with no
arguments. The Manager does not submit an action, generation ID, path,
package, URL, rollback selector, or arbitrary Core argument.

Core owns the repair decision. The plan request is read-only: it loads and
qualifies the selected generation through the existing read-only Core path,
does not invoke upstream, does not access the network, and does not mutate
state. It emits exactly these final-newline lines:

```text
codex-core-repair-v1
action=<none|update|unavailable>
reason=<healthy|legacy-generation|core-state-unavailable>
```

The root-level generation layout produces `action=none` and
`reason=healthy`. The bounded legacy `compat/` layout produces
`action=update` and `reason=legacy-generation`, because the next authenticated
generation is the permanent migration. A failure to load or qualify the
current Core state produces `action=unavailable` and
`reason=core-state-unavailable`, with operation status `1`; a plan with
`none` or `update` returns status `0`. The plan contains no path, generation
ID, digest, credential, environment, or upstream output.

For `repair apply`, Core recomputes the plan and ignores any Manager-supplied
action. `none` emits exactly `codex repair: no repair needed` followed by one
LF and returns `0`; `update` invokes the existing no-argument Core update
operation, including its signed stable-channel and transport-fallback rules,
and preserves that operation's output and status; `unavailable` emits the
fixed error `codex repair: plan unavailable` and returns `1`. Apply never
creates a second updater, package-manager action, raw upstream installation,
legacy-state import, bwrap repair, or direct Manager write to Core state.
Rollback remains an explicit Core `update --rollback` operation and is not
silently selected by Manager repair.

### MGR-5 — Manager artifact build and qualification

MGR-5 adds no user-facing `codex termux` command and no Manager persistent
record. A release qualification supplies one separately built executable
Manager artifact; the release builder must snapshot and qualify that exact
private copy before publishing it into a generation. The normal on-device
update path does not compile Rust, Cargo, or Manager.

The normal/native release-builder path executes the bounded Manager artifact
probe before generation publication exactly as defined below. A GitHub-hosted
cross-build has one explicit pre-sign exception: `build --defer-manager-probe`
is accepted only when `--manager` is present. That mode does not execute the
cross-target binary on the build host. Instead it first proves that the private
Manager snapshot is an ELF64 little-endian Android/AArch64 PIE using exactly one
`/system/bin/linker64` interpreter, then emits the unsigned generation with the
exact regular mode-0644 marker `.manager-probe-deferred` containing
`codex-manager-probe-deferred-v1` plus one LF. The default CLI path and the
library API used by Core never select this exception.

A generation carrying `.manager-probe-deferred` is not publishable:
`codex-release-builder publish` must fail closed before release signing. The
hosted producer must execute the exact MGR-5 probe in its Termux-compatible
Android/AArch64 smoke environment and may remove the marker only after the probe
returns the exact accepted result. No deferred candidate may enter official
signing or signed release inventory.

The artifact has one reserved build-time probe form, invoked only by the
release builder and never forwarded through Core's `codex termux` boundary:

```text
codex-manager --artifact-probe
```

The release builder supplies the exact private marker
`CODEX_MANAGER_ARTIFACT_PROBE=1` only in that child. The probe accepts exactly
that one argument and marker pair, requires no `HOME`, `PREFIX`, Core handoff,
profile state, auth state, or network, and emits exactly these
UTF-8 bytes on stdout with one final LF, no stderr, and status `0`:

```text
codex-manager-artifact-v1
core_api=codex-manager-core-v1
```

Any other probe argument, nonzero status, stderr output, or byte mismatch is a
qualification failure. Release-builder executes the probe with an empty
environment, null stdin, a private staging directory as its working
directory, a 512-byte captured-output bound, and a five-second wall-clock
bound. It kills and rejects a probe that exceeds either bound. Probe output is
not persisted or included in the generation.

After a successful probe, release-builder revalidates the source as a regular
owner-executable file within the existing 64 MiB bound, snapshots it through
the existing private bounded path, sets and verifies mode `0755`, binds the
snapshot digest in `manager_artifact_digest`, and includes `manager` in the
signed release inventory. A snapshot/read failure or probe mismatch publishes
no generation and leaves an existing destination unchanged.

Core does not repeat the probe during ordinary launch or update. Its existing
qualified-generation path remains responsible for the signed descriptor,
regular-file, mode, and digest binding; it executes the Manager only after
that admission and preserves the existing Core handoff, streams, TTY,
signals, raw arguments, and exit status. A generation without a Manager
artifact remains valid and reports Manager unavailable.

### MGR-6 — Manager artifact distribution and disposable qualification

MGR-6 adds no public command, Manager record, Core trust source, or on-device
build path. It defines how the MGR-5-qualified optional Manager artifact is
supplied to release production, carried by a signed generation, and proven in
disposable environments before any live cutover. Manager remains optional for
ordinary upstream launch, Core doctor, update, rollback, and fresh install.

The release producer builds `codex-manager` off-device from the exact accepted
rewrite revision with the locked release toolchain and supplies that one
regular executable through the existing bounded release-builder input. The
producer must not discover a Manager from `PATH`, a live installation, a
Manager state root, or an unpinned build output. The signed generation's
`manager_artifact_digest` and inventory are the on-device content authority;
source revision and toolchain provenance are release evidence and never a
second device trust source. The device update path never invokes Cargo, Rust,
or a Manager build.

A Manager-bearing generation must contain the probe-qualified Manager file at
the signed `manager` path with its signed digest and owner-executable mode.
The immutable remote Release asset set must contain that file before the
signed index is advanced. Missing, extra, mismatched, non-regular, or
symlinked Manager content fails the existing signed-generation admission and
must not advance the index. A generation without `manager` remains a valid
Core-only generation and reports Manager unavailable. The automatic local
fallback may produce only such a Core-only generation unless an explicit
already-qualified Manager artifact is supplied through the bounded release
producer; it never builds or discovers one on-device.

MGR-6 disposable qualification uses private temporary roots and separate
fresh-install and legacy-upgrade consumers. For a hosted cross-build carrying the
RALD-3 deferred marker, the Termux-compatible Android/AArch64 smoke must first
execute the exact MGR-5 artifact probe successfully and remove the marker before
any later signing step. It then installs or selects a release-built
Manager-bearing generation through the existing Core admission,
then proves `codex termux help`, one read-only profile query, and one isolated
Manager-to-Core launch with the existing raw-argv, stream, TTY, signal, exit,
and child-only `CODEX_HOME` contracts. The qualification must observe the
Manager probe marker absent at the public handoff, a successful signed
Manager digest binding, and no Manager-unavailable result for a Manager-bearing
generation. It snapshots and compares the resolver, auth, installed
launcher/runtime, profile/session, and package identities before and after;
it must not modify any protected live surface or invoke or repair `bwrap`.

If artifact input, probe, signed inventory, Release asset, remote readback,
or disposable launch qualification fails, the candidate is rejected and the
existing active/previous state remains unchanged. Remote publication and any
live runtime replacement remain separate operational actions requiring an
explicit target and authorization; MGR-6 source acceptance alone authorizes
neither a push nor a live cutover.

### MGR-7 — Remote publication readback and bounded operational qualification

MGR-7 adds no public command, persistent state, trust source, or release
format. It is an operational qualification of one already accepted,
probe-qualified signed generation; it must not rebuild Manager, rewrite the
generation, or infer a target from the live active generation. The candidate
generation ID, local publication directory, GitHub repository/branch, and
whether a device qualification is requested are explicit inputs recorded
before external I/O.

Remote publication, when explicitly authorized for that exact candidate,
uses the existing fixed `humtr/codex`/`main` boundary and ordering: upload the
complete immutable Release asset set, including `manager` when the signed
inventory lists it; publish `update-index-v1.sig`; then publish
`update-index-v1`. A missing or mismatched Manager asset, failed or timed-out
upload, private-key exposure, or index update out of order fails the operation
without advancing the index and without undoing a successful local
activation. No OpenAI repository, live runtime, auth, profile, session, or
Manager state is an alternate publication target.

Readback uses a separate private disposable consumer and bounded HTTPS
transport. It fetches the signed index and signature, verifies the index with
the recovered update authority, acquires the exact signed control files and
every inventory asset, and runs the existing local signed admission. For a
Manager-bearing generation it must observe a regular `manager` asset whose
digest and mode equal the signed inventory; it must also run the actual
release-built Manager probe/handoff path. It never trusts a GitHub API listing,
mutable tag metadata, or an unsigned asset as evidence.

The disposable device qualification uses separate private fresh-install and
legacy-upgrade roots. Each root snapshots protected host identities before
and after, installs or selects only the read-back signed generation, runs
`codex doctor` and `codex termux` read-only/isolated checks, and verifies raw
argv, streams, TTY, signal, exit status, child-only `CODEX_HOME`, ANSI doctor
presentation, and absence of bwrap invocation. Any readback, admission,
Manager handoff, doctor, or protected-surface failure rejects the candidate
and leaves the live installation untouched.

MGR-7 source acceptance authorizes no remote push or live runtime replacement.
Those actions require a separate explicit operational authorization naming the
exact candidate generation, target, and rollback boundary. A failed optional
remote publication never changes a locally accepted generation; credentials,
tokens, and unredacted session content are never recorded as evidence.

### Manager definition gate

Before MGR-1 implementation, the repository must have focused proof for the
exact profile grammar, path containment and symlink rejection, create-new
profile publication, last-selection atomicity, child-only `CODEX_HOME`, raw
argv/stream/signal/exit preservation, and no credential/session-content
inspection. Each later MGR bundle requires its own focused proof and must not
use an unaccepted future command as a hidden implementation dependency.

## 9A. Post-Core Termux compatibility extension

The upstream runtime remains the official `aarch64-unknown-linux-musl` Codex
artifact adapted by this product. Running that Linux-target executable inside
Termux does **not** activate upstream compile-time `target_os = "android"`
branches. Compatibility decisions therefore belong to an explicit Core-owned
Termux boundary rather than to accidental Linux desktop behavior.

This extension does not reopen or weaken the accepted Core completion
invariants. The resolver, signed-generation trust model, wrapper-owned update,
coordinated Core/generation activation and rollback, profile/session/auth
boundaries, sandbox policy, and no-on-device-compiler rule remain unchanged.
It also does not authorize a new raw-runtime byte patch: patch policy
`termux-fd-remap-v1` remains exact. Any additional binary rewrite requires a
separate normative amendment that names its exact bounded substitutions and
qualification evidence.

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
Codex installation under `$CODEX_HOME/packages/standalone`, and no Termux
app-server path may fetch or execute the upstream standalone installer or any
other upstream self-updater. Bare `codex update` and all runtime replacement
remain Core-owned signed-generation operations.

A daemon-backed app-server command, including `remote-control start`, may run
only if it is bound to the currently qualified signed generation and cannot
enter the upstream standalone installation/update loop. If that binding is
not available, the command must fail closed with a stable Termux-specific
unsupported result before creating standalone state or performing network I/O.
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
doctor/user-facing evidence. The bootstrap and repair paths must not install a
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

## 10. Milestones

### Milestone 1 — local Core

Deliver a buildable, test-backed Rust Core with:

- public dispatch and exact upstream passthrough;
- upstream-only `--version` and `-V` behavior;
- environment planning and final process execution;
- FD 33/34 setup and resolver non-mutation tests;
- explicit sandbox capability behavior;
- read-only local doctor composition;
- generation manifest and updater interfaces without live network mutation;
- focused unit, integration, fault, and real-Termux smoke tests.

Milestone 1 does not install or activate the candidate over the currently
working Codex runtime.

### Milestone 2 — delivery and recovery

Deliver:

- prebuilt Android/Termux Core release artifacts;
- minimal fresh-install and explicit legacy-handoff bootstrap;
- signed immutable release manifests and key-rotation policy;
- official upstream artifact acquisition and safe adaptation;
- atomic update, activation, recovery, and rollback;
- offline install/recovery;
- basic launch/update overlap and injected-failure coverage proving launches
  see only complete generations; speculative multi-writer coordination is not
  a release requirement without demonstrated product need;
- isolated fresh-Termux and upgrade-from-legacy qualification;
- a complete candidate suitable for independent product review.

## 11. Acceptance principles

- Passing source tests proves only the tested source behavior.
- A build does not prove installation or activation.
- An active pointer does not prove process behavior.
- A successful local launch does not prove fresh installation, update,
  rollback, offline recovery, or another Termux device.
- Every release claim must name the exact source, artifact digests, generation,
  test set, and observed device/runtime boundary.
- After the core integrity invariants are met, release velocity and a small
  state machine take priority over speculative resilience mechanisms.
- A new defensive branch, retry, fallback, lock, lease, or fencing mechanism
  requires a concrete product failure that is not already handled by complete
  generation construction, atomic activation, or last-known-good rollback.
- Prefer one recovery path over fallback chains. Complexity added only for a
  hypothetical edge case is itself a reliability and security cost.
- Review findings change implementation only after the responsible normative
  contract is updated.

## 12. Change discipline

A separate SDD is intentionally omitted for speed. Its necessary function is
covered by the following rules:

- normative product or architecture changes update this specification first;
- success-threshold changes update `GOAL.md` first;
- current sequencing changes update `WORKBOARD.md` without copying history;
- implementation details that preserve these contracts need no design record;
- a new decision document is introduced only when an irreversible choice has
  multiple viable alternatives that cannot be resolved within one bounded
  specification change.

This policy may be revised when the product demonstrates a real coordination
need. Documentation ceremony alone is not a reason to add another owner.
