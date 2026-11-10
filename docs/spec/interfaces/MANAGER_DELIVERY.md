# Core recovery authority

> Normative annex of [SPEC](../../../SPEC.md); [INDEX](../INDEX.md) is navigation only and [CURRENT](../../../CURRENT.md) is observed status.


`codex termux repair`, including the former `plan` and `apply` forms, is a
non-mutating Manager usage failure (status 2). It does not launch Core, inspect
or modify generations, access the network, or produce a repair plan. Help does
not advertise it. Existing Manager notification/profile state is unchanged.

The former facade offered only generation-layout classification and an optional
call to Core update. It has no distinct recovery outcome. Core retains ordinary
`codex doctor`, signed `codex update`, activation recovery, update-failure
retention and explicit `codex update --rollback`; Manager owns no recovery
planner, updater, fallback, automatic rollback or hidden request dispatch.
Retirement requires no persistent-state migration.

### Manager artifact build and qualification

Artifact qualification adds no user-facing `codex termux` command and no Manager persistent
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
hosted producer must execute the exact Manager probe in its Termux-compatible
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

Release-builder validates a regular owner-executable source within the64-MiB
bound, snapshots it privately and sets/verifies mode0755. It probes that exact
snapshot (or uses the declared pre-sign cross-build exception), binds its digest
in manager_artifact_digest and includes that same Manager in the signed inventory.
A source-side probe alone cannot qualify different subsequently copied bytes.
A snapshot/read failure or probe mismatch publishes
no generation and leaves an existing destination unchanged.

Core does not repeat the artifact probe during ordinary launch or signed-bundle
acquisition. Explicit local-derived construction uses its producer's bounded
probe for any carried-forward Manager, under the same construction rules. The
existing
qualified-generation path remains responsible for the signed descriptor,
regular-file, mode, and digest binding; it executes the Manager only after
that admission and preserves the existing Core handoff, streams, TTY,
signals, raw arguments, and exit status. A generation without a Manager
artifact remains valid and reports Manager unavailable.

### Manager distribution and disposable qualification

Distribution adds no public command, Manager record, Core trust source, or device
build path. The qualified optional Manager artifact is
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
Core-only generation and reports Manager unavailable. Core's local-derived
construction may carry forward the currently authenticated,
qualified Manager and same-version admitted frontend under the preservation rules
above. It never builds or discovers an unrelated Manager on-device. An optional
component missing from an installed generation does not become a new artifact;
a frontend-bearing generation cannot silently lose its required paired frontend.

Disposable qualification uses private temporary roots and separate
fresh-install and legacy-upgrade consumers. For a hosted cross-build carrying the
deferred probe marker, the Termux-compatible Android/AArch64 smoke must first
execute the exact Manager artifact probe successfully and remove the marker before
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
explicit target and authorization; source acceptance alone authorizes
neither a push nor a live cutover.

### Public readback and device qualification

Readback adds no public command, persistent state, trust source or release format. It is an operational qualification of one already accepted,
probe-qualified signed generation; it must not rebuild Manager, rewrite the
generation, or infer a target from the live active generation. The candidate
generation ID, local publication directory, GitHub repository/branch, and
whether a device qualification is requested are explicit inputs recorded
before external I/O.

Remote publication, when explicitly authorized for that exact candidate,
uses the existing fixed `humtr/codex`/`main` boundary and ordering: upload the
complete immutable Release asset set, including the signed Manager when declared;
complete public readback and disposable consumer proof; then commit each index
and its signature together through the non-forced exact-parent CAS promotion.
Never publish an index/signature as separate mutable ref updates. A missing or
mismatched asset, failed/timed-out upload, private-key exposure or incomplete
consumer proof leaves public stable unchanged. Publication does not activate the
operating device or roll back any separately accepted local activation. No OpenAI
repository, live runtime, auth, profile, session, or
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

Source acceptance alone authorizes no remote push or live runtime replacement.
Those actions require a separate explicit operational authorization naming the
exact candidate generation, target, and rollback boundary. A failed optional
remote publication never changes a locally accepted generation; credentials,
tokens, and unredacted session content are never recorded as evidence.

### Manager definition gate

Profile behavior requires meaningful proof for exact grammar, path containment,
symlink rejection, create-new publication, effective account reporting and raw
process preservation. Later features need their own affected-path proof.
