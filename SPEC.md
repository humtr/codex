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

The launcher intercepts only an exact first argument of `update`, `doctor`, or
`termux`. Every other invocation is passed to upstream Codex.

| Command | Owner | Required behavior |
| --- | --- | --- |
| `codex [UPSTREAM_ARGS...]` | Core | execute upstream with original arguments |
| `codex --version`, `codex -V` | upstream | print exactly the upstream version output |
| `codex update [OPTIONS]` | Core | install a verified compatible generation |
| `codex doctor [OPTIONS]` | Core | combine upstream and Termux diagnostics |
| `codex termux [COMMAND]` | Manager boundary | invoke the Manager artifact or report it unavailable |

`codex version` is not introduced. Wrapper/Core/Manager version rows must not
be appended to upstream `--version` or `-V` output.

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

## 6. Artifact and patch qualification

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
  release.manifest                                    signed release/integrity inventory
  release.sig                                         Ed25519 signature over release.manifest
  runtime                                             patched upstream executable
  compat/                                              runtime compatibility assets
  manager                                              optional Manager executable
  helpers/<index>                                      optional helper artifacts
~/.local/lib/codex/core/release-public-key.pem         bootstrap-provisioned trust anchor
~/.local/share/codex/core/activation-state            current + one previous rollback target
~/.local/share/codex/core/activation-journal[.tmp]    crash-recovery transaction state
~/.local/share/codex/core/activation-state.tmp        atomic state publication temporary
~/.local/share/codex/core/config/                     process-local managed config directory
~/.local/share/codex/manager/                         Manager-owned mutable state
```

The activation-state record owns `current` and at most one `previous` rollback
target; there is no separate `verified` pointer. Only content that has already
passed release admission and candidate probes may become `current`, so a second
pointer duplicating `current` is redundant. Ordinary launch reads only
`current`. It does not scan generations or implicitly fall back to another
generation. The generation directory name is a single safe path component and
generation content is complete before it can become `current`.

A generation is complete or absent. Candidate construction occurs outside the
active path. Activation changes one bounded pointer set only after integrity,
runtime, and doctor probes pass.

Activation and rollback must be recoverable after process kill, power loss,
short write, full storage, permission failure, and a stale journal. Recovery
must resolve to one complete old or new generation and must never synthesize a
mixed generation.

Core optimizes for the shortest correct release path rather than speculative
defense layers. Complete-or-absent generations, atomic activation, and recovery
to one complete last-known-good generation are the primary safety invariants.
Do not add a second mechanism for a failure already covered by those invariants.
If a simpler base invariant makes an existing check, retry path, pointer role,
or fallback redundant, remove the redundant mechanism instead of maintaining
both.

One installer/updater transaction is the normal product model. Simultaneous
install or update attempts are not a first-class coordination feature and do
not by themselves justify locks, leases, fencing tokens, or a multi-writer
protocol. If attempts overlap, the required outcome is limited to preserving a
complete generation boundary: one attempt may succeed while another fails or
retries, and recovery may return to the already complete last-known-good
generation. Launch must never observe a mixed or partially constructed
generation.

`previous` is the only rollback pointer and is not permission to build a
fallback ladder. Rollback is an explicit bounded activation-state transition;
ordinary launch never consults `previous` automatically.

## 8. Installation and update

Fresh installation uses a small audited bootstrap because Core cannot install
itself before it exists. The bootstrap may only detect the environment,
retrieve or accept a local immutable release, verify it, stage Core, run a
self-test, and activate the initial generation.

Normal installation and update must not require on-device compilation.

`codex update` must:

1. resolve an immutable signed release manifest;
2. enforce architecture, API, channel, and anti-rollback policy;
3. download into a private staging location or accept an explicit local
   artifact;
4. verify signature, digest, archive safety, and compatibility metadata;
5. build and probe a complete candidate generation;
6. atomically activate it;
7. retain one complete previous generation as rollback state;
8. report failure without damaging the active generation.

Automatic update checks must be bounded and fail open when a verified runtime
already exists. Ordinary `codex` launch must not depend on network success or
silently run a package manager.

The updater must not depend on the same resolver implementation as the patched
upstream runtime without an explicit qualification proving that dependency.
Offline local-artifact installation and recovery are required before release.

For the local/offline release path, the bootstrap provisions one Ed25519 public
trust key at `~/.local/lib/codex/core/release-public-key.pem`. Core does not
search for alternate keys or accept an untrusted key supplied beside a release.
The signed `release.manifest` is strict/versioned and binds generation identity,
monotonic release sequence, supported channel, platform, architecture, Core API,
persistent schema, and a SHA-256 inventory of every load-bearing generation
file. The signature is over the exact manifest bytes. On Termux, Core may use
the already-present `$PREFIX/bin/openssl` for Ed25519 verification and SHA-256;
it must fail clearly if that executable or the pinned public key is unavailable
and must never install a crypto package itself.

## 9. Doctor contract

`codex doctor` is read-only. It runs the raw upstream doctor when supported and
adds Core and Manager sections without recursively invoking the public
launcher.

Human output contains clearly separated upstream, Core, and Manager sections.
`--json` emits one redacted envelope rather than concatenated documents:

```json
{
  "schema_version": 1,
  "upstream": {},
  "termux_core": {},
  "manager": {},
  "summary": {}
}
```

Unsupported upstream or Manager diagnostics are represented explicitly and do
not fabricate success. Diagnostic failure returns nonzero while preserving a
valid machine report when `--json` was requested. Usage errors remain distinct
from health failures and API incompatibility.

Doctor must not expose tokens, OAuth data, cookies, auth-derived private data,
notification content, or unredacted session content. A filesystem snapshot
before and after doctor must be unchanged except for operating-system access
metadata outside product control.

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
- minimal fresh-install bootstrap;
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

