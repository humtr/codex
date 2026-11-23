# Runtime, release, and Manager delivery contracts

**Status:** Normative detail of [SPEC.md](../../SPEC.md).
This document defines release-independent artifact, installation, update,
rollback, device compatibility and qualification behavior. Exact upstream
release identities and prior patch/admission reports are evidence outside
this contract, not a condition requiring a new specification on every upgrade.
**Implementation status is not implied by a normative subheading:** the
[architecture map](../ARCHITECTURE.md) names what the current binary implements,
while [CURRENT](../../CURRENT.md) records its separate CI, device,
published-asset and operating proofs.

> **Quick navigation (Stage 1 active vs Stage 2 conditional):**
> - **Active Stage 1 delivery & runtime contracts:** [State & generation model](#7-state-and-generation-model) · [Stage 1 `install.sh`](#stage-1-standalone-installation-installsh) · [Stage 1 `codex update`](#stage-1-standalone-update-lifecycle-codex-update) · [Artifact retention](#bounded-installed-artifact-retention) · [Doctor contract](#9-doctor-contract) · [Termux compatibility](#9a-termux-compatibility-extension) · [Release execution](#release-execution)
> - **Deferred Stage 2 conditional contracts:** [Base qualification & lanes](#base-qualification) · [Signed / source-built qualification](#6-artifact-and-patch-qualification) · [Stage 2 lightweight signed releases](#stage-2-lightweight-signed-releases-and-bootstrap-trust-slice-1) · Roadmap in [ROADMAP.md](../ROADMAP.md).

## Base qualification

The active standalone path uses the Stage 1 local builder (`codex-release-builder`)
and account-free CI qualification harness (`wrapper-verify.yml` / `qualify_wrapper.py`).
Per the Stage 2 zero-coupling redesign (Slices 1–6), legacy coupled `Base`/`Full`
editions, CDN `update-index` / `asset-map.json`, `codex-runtime-binding`, and
`frontend/tui` source forks are **retired** (preserved only in Git /
[docs/history/LEGACY_RUST_CORE.md](../history/LEGACY_RUST_CORE.md)).

- **Wrapper independence:** `crates/wrapper` depends only on `crates/release-builder`
  and `libc`, with zero compile-time or runtime coupling to Stage 2 components
  (`codex-ext`, `@TermuxShell`, `@TermuxCodex`, `chatgpt-provider`, `@TermuxChatGPT`,
  `tunnel-client`).
- **Local upstream adaptation:** Ordinary `codex update --upstream` resolves official
  OpenAI stable releases directly, verifies OpenAI SHA-256 digests, and adapts the
  official archive locally with built-in SHA-256 and zero `openssl-tool`, signing-key,
  or CI schedule prerequisite.
- **Per-component independent CI & signed GitHub Releases:** Each Stage 2 component
  lives in its own independent `[workspace]` with path-triggered CI and publishes
  independently under the shared lightweight Ed25519 `SHA256SUMS.sig` contract
  ([Stage 2 lightweight signed releases](#stage-2-lightweight-signed-releases-and-bootstrap-trust-slice-1)).
  No Stage 2 component failure can block or veto standalone Wrapper qualification or
  release publication.

## 6. Artifact and patch qualification

Upstream freshness, request acceptance, build/port qualification, signed public
publication, and device activation remain strictly separate states.

- **Official upstream input:** The sole upstream input for the supported target is
  the exact versioned OpenAI asset
  `https://releases.openai.com/codex/releases/<version>/codex-package-aarch64-unknown-linux-musl.tar.gz`,
  where `<version>` is an explicitly selected stable `MAJOR.MINOR.PATCH` value
  verified by SHA-256. Mutable `latest` aliases, mirrors, and upstream self-updaters
  are never artifact authority.
- **Local adaptation without signing keys:** Official upstream archive adaptation
  produces trusted local derivatives from verified OpenAI inputs and the installed
  Wrapper (`codex-release-builder build`) using adaptive rule-based `termux-fd-remap`
  and `termux-uds-fd35` verification per binary in `bin/*`. It requires no local
  signing key, `gh` login, or network publication authority.
- **Self-built signed publication:** Signing authority belongs solely to explicit
  `workflow_dispatch` publication of self-built bundles (`wrapper`, `codex-ext`,
  `termux-shell-mcp`, `termux-codex-mcp`, `chatgpt-provider`, `termux-chatgpt-mcp`,
  `tunnel-client`) using `.github/scripts/release_sign.py` plus fail-closed
  publishers (`.github/scripts/publish_wrapper_release.py` and
  `.github/scripts/publish_stage2_release.py`), which enforce component identity
  matching, strict monotonic `sequence`, peeled Git tag verification, and mandatory
  post-upload signed readback against the pinned repository public key.

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
~/.codex/packages/standalone/current                   symlink -> releases/<current> synchronized with activation-state
~/.codex/packages/standalone/releases/<id>/            immutable complete generation (`<version>-<6hex>`)
  codex                                               relative symlink -> bin/codex (upstream package-root entrypoint alias)
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
~/.codex/packages/standalone/releases/.acquire-*/      private incomplete remote acquisition staging; never activatable
~/.codex/packages/standalone/releases/.candidate-*/    private candidate build staging; never activatable
~/.local/share/codex/wrapper/                          state root (directory flock: LOCK_SH for healthy read, LOCK_EX for writes/recovery)
  activation-state                                     authoritative generation state (`codex-activation-state`)
  activation-state.tmp                                 atomic state publication temporary
  activation-journal[.tmp]                             crash-recovery transaction state (`codex-activation-journal`)
  .local-update-*/                                     private offline/upstream archive staging workspace
  config/config.toml                                   Wrapper-owned FD 34 compatibility config (`# codex-termux-wrapper`)
  config/snapshots/<id>/                               Stage 2 immutable per-launch effective system settings
  resolv.conf                                          Wrapper-owned FD 33 DNS resolver file
  update-check.cache                                   15-minute official release metadata cache for interactive bare `codex` startup prompt
  downloads/<sha256>.part                              private bounded untrusted/verified payload cache (1 GiB total cap)
  publications/                                        optional staged/pruned local publication artifacts
  wrapper.toml                                         Stage 2 optional module & update mode config
  wrapper-entrypoint-rollback                          Stage 2 retained prior Wrapper launcher (when signed bundles enabled)
  wrapper-entrypoint-rollback.meta                     Stage 2 rollback binding (`codex-wrapper-entrypoint-rollback`)
  {servers,catalogs}/<id>/                             Stage 2 bound native server record/socket/config snapshot
~/.local/share/codex/manager/                          Stage 2 Manager-owned mutable state (absent in Stage 1)
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
prerequisite for executing a valid current generation. The generation directory name is one safe path component (normally
`<upstream_version>-<6hex>` derived from the built runtime SHA-256, with an
incrementing suffix only on same-build collision) and generation content is
complete before it can become `current`. Before ordinary launch consumes that
generation, Wrapper must verify the selected generation directory and every
selected asset parent directory is a real directory, and every selected
runtime, Manager, helper, and descriptor file is a regular non-symlink file
(allowing the sibling `codex -> bin/codex` internal relative symlink). Ordinary
launch must not follow a symlink from the generation root to content outside
that generation.

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

Normal execution, `wrapper status`, `doctor`, updater recovery and rollback
must evaluate a published activation journal even when the authoritative
`activation-state` file is missing (initial activation `before=None`). Stage 1
checks that `standalone/current` is missing or is a symlink under the activation
lock before both activation and rollback. It durably publishes the journal,
prepares the state file, and publishes and syncs the link before committing
the state rename. The state remains authoritative for recovery. Before its
rename, recovery restores the previous projection (or removes the uncommitted
initial link). After the rename, recovery completes the new projection and
syncs the state directory before clearing the journal. A regular file or
directory at the link path is preserved and reported as an error.
New journals record the initiating operation so recovery can distinguish
an update that reselects the previous generation from a rollback.

Any unmatched pointer, simultaneous published and temporary journals, or
orphan state temporary without journal remains fail-closed. Healthy readers
use a shared (`LOCK_SH`) directory lock. Pending journal or temporary state
artifacts, and repairs to a missing or stale link, require the exclusive
(`LOCK_EX`) writer lock and a fresh state read.
Repeated recovery preserves the same authoritative pointer. When an installer
interrupt recovery or startup check qualifies a candidate activation, `install.sh`
retains ownership of the executable/updater receipts while delegating active
generation compatibility, digest, requested version, and candidate probe checks
to Wrapper's authoritative `codex wrapper status --verify` path.

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
install.sh --prefix <DIR> --recover
```

- When `--binary`, `--build`, and `--release` are omitted and the installer runs
  from a regular script file (`[ -f "$0" ]`), it looks beside the script for an
  unpacked bundle (`bin/codex` or `codex` with `SHA256SUMS`) and then for
  `target/release/codex` in a checkout. If those sources are absent, or the
  script runs from stdin via `curl ... | sh`, it fetches the GitHub Release
  assets `codex-wrapper-aarch64-linux-android.tar.gz` and `.sha256` from tag
  `wrapper` of `humtr/codex` over HTTPS (`curl`) + checksum verification (`sha256sum` or
  `openssl` fallback). (Stage 1 verifies `.sha256` / `SHA256SUMS` digests only
  without an Ed25519 signature; Phase 2.2 adds Ed25519 signature verification of
  the Wrapper bundle and `install.sh` self-update path.)
- On Android AArch64, `install.sh` runs a non-mutating `/system/bin/linker64`
  `update --help` sanity probe on the staged binary before atomically publishing it to
  `$PREFIX/bin/codex`.
- A single exclusive installer writer stores a synced owner identity plus
  a phased SHA-256 receipt for the prior/new executable and the original
  `activation-state`, and a verified same-directory previous executable before
  renaming the candidate into place. The standalone executable commit and the
  updater's activation-state commit are distinct, not one atomic rename.
  Installer owns the binary and install receipt; the Wrapper updater owns the
  generation pointer and its separate activation journal. Neither may guess
  the other's commit status or replay an uncertain updater.
- Unless `--skip-upstream` was requested:
  - When the existing active generation passes `codex wrapper status --verify
    --json` (valid descriptor, compatibility with `(wrapper_api_identity,
    persistent_schema_identity, patch_policy_id)`, verified runtime/helper/companion
    SHA-256 digests, and candidate probe), the installer retains it without
    calling the updater. An explicit `--version` adds `--upstream-version` to
    this qualification. Retaining a compatible generation preserves the
    `previous` upstream rollback slot and allows offline Wrapper upgrades.
  - When the active generation is absent or fails qualification (including a
    requested version mismatch), `install.sh` launches `codex update --upstream`
    and verifies the resulting activation via `codex wrapper status --verify
    --wrapper-sha256 <candidate_sha>` (and `--upstream-version` if requested).
- A caught bootstrap failure restores the previous executable only when the
  activation state has not changed; otherwise the executable, backup and receipt
  are preserved and installation returns failure. An interrupted install may
  resume after proving that the old writer is gone and any recorded updater and
  its process group cannot mutate state (treating zombie/dead `Z`/`X` updater
  PIDs with matching start ticks as quiescent, not PID reuse). Partial ownership
  or contradictory byte/activation state always fails closed. `install.sh --recover`
  performs reconciliation without launching a new updater, and ordinary installs
  recover only the provably safe states before continuing. During post-updater
  recovery (`updating`/`updated`), `install.sh` continues requiring `--wrapper-sha256`
  on `wrapper status --verify` so an interrupted updater is never confused with a
  pre-existing generation. Activation-journal recovery is separate and is not
  bypassed by the installer.

### Stage 1 standalone update lifecycle (`codex update`)

After installation, Wrapper exclusively owns `codex update`:

- `codex update` / `codex update --upstream [--version <MAJOR.MINOR.PATCH>] [--force-rebuild]`:
  resolves official `openai/codex` release metadata (`api.github.com/repos/openai/codex/releases/latest`
  cross-checked against `https://releases.openai.com/codex/releases/<version>/release.json`,
  falling back on GitHub API `403`/`429` rate limits to the canonical
  `https://github.com/openai/codex/releases/latest` HTTPS redirect target
  verified against `https://releases.openai.com/codex/releases/<version>/release.json`) with a
  bounded control timeout (30 s), downloads `codex-package-aarch64-unknown-linux-musl.tar.gz`
  into private `$state_root/.local-update-*` / `downloads/` via `$PREFIX/bin/curl`
  (15 s connect, 300 s idle transfer timeout, up to `512 MiB` compressed archive,
  `1 GiB` download cache cap), displays live downloaded/total bytes when known,
  verifies the official SHA-256 using Wrapper/release-builder's internal FIPS 180-4
  SHA-256 implementation (no external `openssl-tool` runtime prerequisite in Stage 1),
  prepares the generation catalog root and acquires a shared `UpdateLease` BEFORE
  staging even on an uninitialized HOME, unpacks and adaptively patches `bin/*`
  (`termux-fd-remap`, requiring both `/etc/resolv.conf` and `/etc/codex/config.toml`
  on main `bin/codex`, plus `termux-uds-fd35` failing closed if `"codex-daemon-"`
  is present without a recognized format pattern) in `.candidate-*` (emitting
  transient per-section patching progress on interactive TTYs), records companion
  binary SHA-256 digests (`companion\tbin/<name>\t<sha256>`) in `generation.meta`
  while remaining backward-compatible when loading earlier generations without
  `companion` lines, verifies actual on-disk SHA-256 digests of copied runtime,
  helper, and declared companion binaries against `generation.meta`, probes the exact
  `bin/codex --version` and `features list` under Termux FDs in a fresh account-free
  temporary home (clearing `CODEX_SHARED_HOME`/`CODEX_SQLITE_HOME`, bounded time and
  output; no personal-device allowlist or account test), and atomically activates
  (`codex-activation-state`, `LOCAL_ZERO` verifier slots, and
  `~/.codex/packages/standalone/current`). When the currently active generation
  already matches the target `(upstream_package_version, source_artifact_digest)`
  and is compatible with `(wrapper_api_identity, persistent_schema_identity,
  patch_policy_id)`, `codex update` reports already current without redownloading
  or evicting `previous`. The explicit `--force-rebuild` flag downloads the archive
  again, verifies it, and builds and activates a new generation. A damaged
  current generation does not block forward recovery via `codex update` or `--path`.
- `codex update --path <FILE_OR_DIR>`: adapts an offline official `.tar.gz` package archive
  or validates/activates a prepared `package-root` generation directory (`generation.meta`),
  verifying actual on-disk SHA-256 digests of runtime, declared browser helpers, and any
  declared companion binaries before probing. Raw standalone ELF files without
  `codex-package.json` or `generation.meta` are rejected.
- `codex update --rollback`: recovers any incomplete journal under the state-root
  lock, verifies the retained `previous` generation's descriptor, digests,
  executable bits, and isolated runtime probes, and swaps `current` and
  `previous` through the activation transaction. A damaged current runtime
  does not prevent rollback. A pre-commit rejection preserves the authoritative
  state bytes. A direct rollback retry that recovers a published rollback
  finishes that transaction without swapping again. If recovery instead
  finishes a published update, rollback operates on the recovered pair. For
  an older journal without an operation whose committed pair could represent
  either command, direct rollback recovers the selection and reports ambiguity.
  Inspect the selection with `codex wrapper status` before choosing a new rollback.
  An error syncing an
  already-published state returns exit status `3` and keeps its journal for
  recovery. Failure to clean up the journal after a durable commit is reported
  as a warning. If the outcome of an interrupted command is uncertain, inspect
  `codex wrapper status` before issuing a new rollback. Once recovery has
  cleared the journal, a new rollback operates on the current retained pair.
- `codex update --check [--version <MAJOR.MINOR.PATCH>] [--json]`: uses the same
  official resolver and bounded HTTPS transport in an owned disposable temporary
  directory. Latest-release discovery is cross-checked against the exact-version
  official descriptor. Output binds the version, canonical archive URL/SHA-256,
  exact descriptor URL/SHA-256 and current Wrapper SHA-256. It reports `current`
  when `(upstream_package_version, source_artifact_digest)` matches and the active
  generation is compatible with `(wrapper_api_identity, persistent_schema_identity,
  patch_policy_id)`, `older` when the requested version is lower, and `available`
  otherwise — without activation, retention, artifact download, journal recovery,
  or installed-state writes.
- Removed/Stage 2 flags (`--signed`, `--remote`, `--local`, `--build-local`) fail
  explicitly in Stage 1 with exit status `2` and zero state mutation. No compiler toolchain,
  signing key, or upstream self-updater is ever invoked in Stage 1.
- Under `activation_lock`, `load_activated_generation` reconciles a missing or
  stale `$HOME/.codex/packages/standalone/current` symlink to `activation-state`.
  It preserves a non-symlink entry and reports the conflict. Successful
  updates also run [managed daemon synchronization](#managed-daemon-synchronization).
- On an interactive bare `codex` launch (zero arguments, `stdin`/`stdout`/`stderr`
  all terminals, and `CODEX_DISABLE_UPDATE_PROMPT` unset), Wrapper checks
  official stable release metadata using `~/.local/share/codex/wrapper/update-check.cache`
  (15-minute TTL) and a bounded fail-open HTTPS probe (3 s connect / 5 s total
  timeout across discovery requests). A set `CODEX_TERMUX_UPDATE_VERSION`
  also bypasses startup discovery. When a strictly newer upstream version is
  available, Wrapper prompts
  `Codex <latest> is available (current: <current>). Update now? [y/N]: ` on stderr.
  A single `y` or `Y` confirms immediately. `n`, `N`, or Enter declines
  immediately. The prompt restores the terminal mode before running the updater
  or opening the active Codex. The updater checks the displayed release's
  official descriptor again. Declining or a failed update defers the same offer
  for the remaining cache lifetime. Launch resolves the active generation after
  the prompt.
- Interactive update progress, including managed daemon stop/start phases,
  updates one terminal row in place using atomic single-write updates (keeping
  the cursor at the end of the row without full-line erase flicker, and clearing
  only trailing cells when a line shortens or the terminal resizes). Frames fit
  within the terminal width and are cleared before final results or warnings are
  printed. Non-interactive streams emit neither transient progress nor terminal
  control sequences.
- `codex doctor`
  combines its upstream and
  Wrapper checks; it is not an upstream `state_*.db` or rollout inventory
  repair API. A healthy Wrapper diagnosis does not imply that every upstream
  check reports no warnings.

Browser helpers occupy the fixed package-root paths `browser/open/curl` (`termux-browser-open`)
and `browser/manual/curl` (`termux-browser-manual`), with `helper_count = 2`. No indexed
`helpers/0` / `helpers/1` compatibility bridge is retained.

### Managed daemon synchronization

After a successful Wrapper update, including an already-current result,
`--path`, or `--rollback`, Wrapper synchronizes daemon package selection to
the active generation. `update --check` performs read-only release inspection.

- **Home selection.** Wrapper checks the invocation's `$CODEX_HOME`, the
  default `$HOME/.codex`, and existing
  `$HOME/.local/share/codex/manager/profiles/*/home`. It checks ownership and
  real directories, skips symlinked profile paths and homes under
  `standalone`, and deduplicates homes by device and inode. A home is eligible
  when it has an entry at `packages/app-server-daemon/current` or a
  `daemon.pid` or legacy `app-server.pid` record in `app-server-daemon/`.
- **Package selection.** Successful synchronization selects the same active
  executable via `packages/app-server-daemon/current ->
  $HOME/.codex/packages/standalone/current`. This keeps the profile's logical
  launch path stable across generation changes. Wrapper disables automatic
  upstream package updates and preserves ordinary files and directories at
  the link path. Dedicated per-home packages stay on disk. Wrapper rollback
  uses the separately retained `previous` generation.
- **Process lifecycle.** Each running daemon has its own PID and control socket.
  If the executable and version already match and the daemon uses the current
  PID path, Wrapper preserves the PID. A stopped home or a stale PID record
  receives link alignment and stays stopped. Wrapper asks upstream to stop a
  verified running daemon when its executable or version differs or its legacy
  PID path needs migration. It switches a legacy PID path once it confirms
  the old process is gone and starts a replacement only after upstream reports
  `stopped`. The replacement retains the daemon's launch environment,
  including its SQLite binding, with the active Wrapper's Termux preparation.
  Wrapper checks its executable identity and reported version after startup.
- **Failure handling and reporting.** Wrapper serializes its updates through
  daemon synchronization and uses upstream's home operation lock for
  package-link changes. Upstream owns each stop and start, so another upstream
  command can run between them. Unsafe paths and unverified live processes are
  preserved. If a daemon requiring restart is an ancestor of the running
  `codex update` process, Wrapper aligns its non-legacy package link without
  terminating its own parent daemon and defers that restart with a warning so
  other homes finish and the caller receives the full result. Normal completion
  prints the active generation and a hierarchical per-profile daemon summary
  under the shared runtime distinguishing restarted (with version transition),
  link-aligned, already up to date, deferred, and failed homes using short
  profile names (`default`, manager profile names, or custom `$CODEX_HOME`
  paths). Each failure or deferred restart is reported as a warning on stderr
  identifying the profile, full home path, and cause; activation remains
  committed, exit status stays `0`, and synchronization proceeds for other
  homes.

### Stage 2 lightweight signed releases and bootstrap trust (Slice 1)

Self-built artifacts distributed by this repository (`codex` wrapper via
`install.sh`, and independent Stage 2 components such as `codex-ext`,
`termux-shell-mcp`, `termux-codex-mcp`, `termux-chatgpt-mcp`, `chatgpt-provider`,
and `tunnel-client`) share one lightweight Ed25519 verification contract over
standard GitHub Release assets — without reintroducing legacy CDN indices
(`update-index` / `update-index-base`), `asset-map.json`, `Base`/`Full`
whole-generation bundling, or remote CI signing of official OpenAI upstream
tarballs (which remain authenticated directly by OpenAI metadata + SHA-256 +
local ELF adaptation with zero `openssl-tool` or signing-key requirement):

- **Uniform signed GitHub Release format:** each component release publishes
  its archive(s), `SHA256SUMS`, and `SHA256SUMS.sig` (a raw Ed25519 signature
  over the exact `SHA256SUMS` bytes). `SHA256SUMS` carries bounded metadata
  comment lines at the top (`# component=<name>`, `# sequence=<N>`,
  `# commit=<40-hex>`) followed by standard `<64-hex>  <filename>` entries.
- **Three-step verification before execution or swap:**
  1. Verify `SHA256SUMS.sig` against the pinned Ed25519 verifier key
     (`release-public-key.pem`, bound by pinned public-key SHA-256) using
     `openssl pkeyutl -verify -pubin -rawin`.
  2. Enforce component-scoped monotonic `sequence` anti-rollback against the
     last installed signed sequence for that component (preventing replay of an
     older signed release without ever blocking or interacting with local
     upstream `codex update --upstream` / `--rollback` generations).
  3. Verify the downloaded archive's exact SHA-256 against the signed
     `SHA256SUMS` entry before unpacking or swapping executables.
- **Trust root & key rotation:** one dedicated Stage 2 Ed25519 keypair signs
  repository releases (private key held exclusively in authorized CI release
  secrets, never on device and decoupled from historical `0.167.2` keys).
  When key rotation is required, a signed `release-public-key.pem.sig`
  authorizes the replacement public key under the previously trusted key before
  verifying `SHA256SUMS.sig` under the new key.
- **Installer pre-execution verification (`install.sh`):** downloaded Wrapper
  tarballs verify `SHA256SUMS.sig` (and `sequence` / `SHA256SUMS`) before
  extracting or replacing `$PREFIX/bin/codex`. Explicit local installs
  (`--binary`, `--build`, local unpacked bundle) remain offline-usable without
  requiring remote signature metadata.
- **Component isolation:** each independent component owns its own GitHub
  Release tag, sequence counter, and installation path; signature/update
  failures in any optional Stage 2 component never affect bare `codex`,
  `codex doctor`, or `codex update --upstream` / `--rollback`.

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
parsing:
- A Wrapper runtime-preparation failure (e.g. missing resolver, config dir, or
  invalid generation state) marks `termux_wrapper.status = "unhealthy"` (with a
  bounded `"reason"` field when applicable) and `upstream.status = "not_run"`
  without spawning the upstream binary.
- An executed upstream probe tracks a bounded wall-clock deadline covering both
  stdout collection and child process wait (`SIGTERM` -> short wait -> `SIGKILL`
  + reap on timeout or > 64 KiB output), reporting `upstream.status` as
  `"healthy"`, `"unhealthy"` (nonzero exit), `"timed_out"`, or
  `"output_exceeded"` cleanly in both human output and `--json` while preserving
  redacted diagnostic capture.
Upstream doctor output is capped at 64 KiB, has terminal control sequences and credential-like
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
notification content, or unredacted session content. A filesystem snapshot of
`$HOME`, `$CODEX_HOME`, `$CODEX_SHARED_HOME`, and `$PREFIX` before and after
`codex doctor` must be unchanged except for operating-system access metadata
outside product control (when `$HOME/.local/share/codex/wrapper/config/config.toml`
is absent or drifted, `doctor` does not write to the state root; it supplies FD 34
from an ephemeral read-only temporary directory cleaned before exit and reports the
config drift in the Termux wrapper section, and validates `CODEX_SHARED_HOME`
read-only without creating symlinks or mutating logs). Non-PTY execution also
captures bounded `stderr` diagnostics instead of discarding them.

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
A fresh home can receive the non-clobbering link to
`$HOME/.codex/packages/standalone/current` on a TUI invocation eligible to use a
managed daemon or an explicit start route, as specified in [SPEC.md](../../SPEC.md).
Upstream owns the decision to start a daemon. Wrapper owns
[post-update package synchronization](#managed-daemon-synchronization).

#### Optional shared app-server and catalog (Stage 2, deferred)

The following contracts apply when the optional Manager modules are attached.

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
Wrapper's Stage 1 default package preparation creates the per-home link to
`standalone/current`. Dedicated package installs and the daemon updater remain
owned by upstream.
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

### Stage 1 GitHub Release source and uploaded-byte binding

The standalone Wrapper publisher runs only after the exact source SHA's CI
packaging and smoke qualification. It must verify the real Git tag's peeled
commit (including annotated tags), its verified source commit, and each local
asset's actual SHA-256 before mutation. Existing lightweight tags may be moved
only by explicitly authorized publication, never to conceal a failed update;
a mismatched annotated tag is preserved and requires manual reconciliation.
Tag readback must succeed before upload and after all asset readbacks. Upload
or tag API failures cannot be suppressed or retried blindly.

Each upload is followed by GitHub API metadata and a fresh download of the exact
assets; local and downloaded SHA-256, byte size, archive checksum and Release
notes source/archive SHA must all match. `target_commitish` is a separate
Release metadata field, not proof that the Git tag moved; its mismatch is
reported independently rather than accepted as a source/asset mismatch. GitHub
Actions publishing is serialized per tag, with no cancellation of an in-flight
publisher. The verifier runs with mock GitHub responses during ordinary CI and
must never publish an actual Release as part of pull-request verification.


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
