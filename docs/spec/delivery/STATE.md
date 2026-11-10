# 7. State and generation model

> Normative annex of [SPEC](../../../SPEC.md); [INDEX](../INDEX.md) is navigation only and [CURRENT](../../../CURRENT.md) is observed status.


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
