# 8. Installation and update

> Normative annex of [SPEC](../../../SPEC.md); [INDEX](../INDEX.md) is navigation only and [CURRENT](../../../CURRENT.md) is observed status.

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
