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

The current two-milestone program completes Core. It defines but does not yet
implement Manager product features.

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
package, applies the accepted Termux patch, qualifies it, and publishes a
signed generation; the installed Core obtains and activates only that signed
adapted generation.

| Command | Owner | Required behavior |
| --- | --- | --- |
| `codex [UPSTREAM_ARGS...]` | Core | execute upstream with original arguments |
| `codex --version`, `codex -V` | upstream | print exactly the upstream version output |
| `codex update` | Core | resolve the signed stable wrapper release channel, acquire its already-patched generation, and activate it through the authenticated Core path |
| `codex update --help` | Core | print the wrapper-owned update usage without invoking upstream or changing state |
| `codex update [INVALID_ARGS...]` | Core | reject unsupported updater options without invoking upstream or changing state |
| `codex update --local <DIRECTORY>` | Core | verify, stage, probe, and activate one compatible local generation |
| `codex update --remote <HTTPS_BASE_URL>` | Core | acquire one immutable signed generation and activate it through the local update path |
| `codex update --rollback` | Core | explicitly swap to the one retained complete previous generation |
| `codex doctor [OPTIONS]` | Core | combine upstream and Termux diagnostics |
| `codex termux [COMMAND]` | Manager boundary | invoke the Manager artifact or report it unavailable |

`codex version` is not introduced. Wrapper/Core/Manager version rows must not
be appended to upstream `--version` or `-V` output.

The Core-owned update surface accepts exactly no arguments, `--help`,
`--local <DIRECTORY>`, `--remote <HTTPS_BASE_URL>`, or `--rollback` after
`update`. A malformed invocation beginning with one of those Core selectors is
a Core usage error. The local form and rollback remain fully offline. The
explicit remote form is an immutable signed-generation source. No top-level
`codex update` argument is passed to upstream, and Core never invokes a package
manager or an upstream self-updater. Rollback is an explicit Core operation,
not an ordinary-launch fallback and not a search through generation history.

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

Manager must not directly write Core generations, pointers, manifests, locks,
runtime state, resolver data, or activation journals. A Manager request that
would mutate Core state must use a versioned, runtime-validated Core contract.

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
does not compile, run the release builder, patch a raw upstream executable, or
execute an upstream self-updater; its update path accepts only the signed
adapted generation produced by that pipeline. The builder's `build` operation
accepts the version, archive and digest, generation identity, Core artifact,
creation metadata, and an absent output directory. It performs no discovery,
signing, activation, or live-state mutation and emits only an unsigned
generation source for the `codex-release-v3` signing and delivery path.
`codex-release-v2` remains implementation history and is not retained as a
release compatibility path.

The release builder complete output boundary is durable: final file modes are set before the corresponding final file synchronization, the complete staging tree is synchronized bottom-up, the no-replace output publication is atomic, and the output parent is synchronized before the build reports success. A failed publication leaves no accepted output.

The accepted archive is gzip-compressed POSIX ustar. A per-entry POSIX PAX
header is optional and may contain only `mtime`; all other extended semantics
are rejected. There are at most 32 logical entries, paths are canonical relative
UTF-8 of at most 256 bytes, the compressed archive is at most 256 MiB, each
regular file is at most 384 MiB, and total regular-file payload is at most
512 MiB. Duplicate paths, absolute or dot/dot-dot paths, links, special files,
unknown entry types, malformed headers, and nonzero trailing content are
rejected. Archive ownership and modes are not output authority.

The archive contains exactly these directories and regular files:

```text
bin/
bin/codex
bin/codex-code-mode-host
codex-package.json
codex-path/
codex-path/rg
codex-resources/
codex-resources/bwrap
codex-resources/zsh/
codex-resources/zsh/bin/
codex-resources/zsh/bin/zsh
```

`codex-package.json` must bind layout version `1`, the requested version, target
`aarch64-unknown-linux-musl`, variant `codex`, entrypoint `bin/codex`, resources
directory `codex-resources`, and path directory `codex-path`. `bin/codex` and
`bin/codex-code-mode-host` must be little-endian 64-bit AArch64 ELF files with
no `PT_INTERP`. They are the only selected binaries. The Linux `rg` and `zsh`
artifacts are excluded, and `bwrap` is excluded by the Section 5 sandbox
contract.

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
  runtime                                             patched upstream executable
  codex-code-mode-host                                first-target companion beside runtime
  manager                                              optional Manager executable
  helpers/<index>                                      optional helper artifacts
~/.local/lib/codex/core/generations/.acquire-*/        private incomplete remote source; never activatable
~/.local/lib/codex/core/release-public-key.pem         bootstrap-only initial trust seed; never update fallback
~/.local/share/codex/core/activation-state            authoritative generation + bounded trust state
~/.local/share/codex/core/activation-journal[.tmp]    crash-recovery transaction state
~/.local/share/codex/core/activation-state.tmp        atomic state publication temporary
~/.local/share/codex/core/config/                     process-local managed config directory
~/.local/share/codex/manager/                         Manager-owned mutable state
```

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
active path. Forward activation publishes one complete new state: the candidate
becomes `(current, current_key)`, the old current pair becomes
`(previous, previous_key)`, and `update_key` becomes the candidate release key.
For a non-rotating release the old and new update keys are equal. For a key
rotation they differ.

The authoritative journal format is `codex-activation-journal-v3`. Its before
and after records contain the entire bounded trust-and-generation state, not only
generation identities. Activation, rollback, and recovery therefore treat
`update_key`, both generation identities, and both generation verifier keys as
one transaction. After process kill, power loss, short write, full storage,
permission failure, or a stale journal, recovery must resolve to exactly one
complete old or new trust-and-generation state and must never synthesize a mixed
state.

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

The release bundle ships one human-facing `install.sh` delivery frontend. It
accepts exactly the two bootstrap forms below and forwards their original
arguments, without compiling, downloading, signing, discovering releases, or
implementing a second installation protocol:

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

Normal installation and update must not require on-device compilation.

After bootstrap, the Core owns every top-level `codex update` form. `install.sh`
is not an update dispatcher and must not bypass the authenticated local
admission, staging, probe, activation, or rollback path. The local, explicit
remote, and automatic channel forms use the same forward activation
transaction, and rollback remains the explicit swap of the one retained
complete previous generation. A bare `codex update` is never the upstream
command: it resolves a wrapper-owned signed channel and therefore cannot
install an unpatched upstream runtime.

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

`codex update` must:

1. recover the authoritative v3 state and resolve one immutable signed wrapper
   release against its `update_key` (automatic update first verifies the signed
   channel index, while explicit remote update starts from its supplied base);
2. enforce architecture, API, channel, and the existing monotonic
   release-sequence anti-rollback policy;
3. download into a private staging location or accept an explicit local
   artifact;
4. verify the required current-authority and candidate-key signatures, exact
   digest/mode inventory, archive safety where applicable, and compatibility
   metadata;
5. build and probe a complete candidate generation;
6. atomically publish the new trust-and-generation state;
7. retain one complete previous generation with its exact verifier key as
   rollback state;
8. report failure without damaging the active trust-and-generation state.

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
update, `release_public_key` must equal the recovered authoritative `update_key`;
`release.sig` is then the only release signature and no
`release-authority.sig` is accepted. For a rotation, `release_public_key` differs
from `update_key`; `release-authority.sig` is then mandatory and must verify over
the same exact manifest bytes with the current `update_key` before Core treats
the candidate key as trusted, after which `release.sig` must verify with the
candidate key. A rotation is rejected if either proof is missing or invalid.
Core never accepts an adjacent key file, alternate-key search, network key
lookup, CA/PKI chain, key server, or unbounded keyring as authority.

Before forward admission, Core recovers the v3 state. Installed-generation
verification requires the signed manifest `release_public_key` to equal the
selected state verifier key before `release.sig` is verified with that key. Core
applies that rule to the installed current generation with `current_key` and uses
that signed current release for the existing release-sequence anti-rollback
comparison. After staging and probing the candidate, successful activation sets
`update_key` and `current_key` to the candidate release key, sets `current` to the
candidate generation, and moves the former `(current, current_key)` pair to
`(previous, previous_key)` in the same atomic transaction.

`codex update --rollback` must recover any pending activation transaction,
require the one retained `(previous, previous_key)` pair, verify exactly that
previous generation with `previous_key`, and atomically swap the current and
previous generation/verifier pairs. It deliberately does not apply forward
release-sequence anti-rollback policy and deliberately leaves `update_key`
unchanged. Missing, malformed, mismatched, or unverifiable rollback state fails
without changing authoritative state; rollback never scans generations,
searches keys, restores a rotated-away key to forward authority, or constructs a
fallback ladder.

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
identity encoded as canonical UTF-8 URL-path bytes. Core never follows a
redirect or tries another URL.

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
again through the same local v3 admission, and feeds the existing staging,
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
300-second transfer timeout, and writes response bytes only to a caller-created
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

## 9. Doctor contract

`codex doctor` is read-only. It runs the raw upstream doctor when supported and
adds a Termux Core/Manager diagnosis without recursively invoking the public
launcher. Human output includes the bounded upstream doctor output itself, not
only its exit-status projection.

Human output contains clearly separated `Upstream Codex doctor`, `Termux
doctor`, Manager, and summary sections. `--json` emits one redacted envelope
rather than concatenated documents:

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
cleanup, and ANSI SGR markup retain the upstream layout. Core normalizes
carriage-return/erase controls and preserves only safe SGR sequences before
composition. Non-TTY output and explicit `NO_COLOR` remain plain. The Termux
doctor section follows the upstream doctor presentation: a `Codex Termux
Wrapper Doctor` status header,
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
