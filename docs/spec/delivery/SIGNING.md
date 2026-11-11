# GitHub-native transport and legacy compatibility

> Normative annex of [SPEC](../../../SPEC.md); [INDEX](../INDEX.md) is navigation only and [CURRENT](../../../CURRENT.md) is observed status.


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
separate states. CURRENT records observed proof; HANDOFF records unfinished acceptance.

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
A new producer names the independent component with `--tui` paired with
`--tui-version`, or `adapt-tui` for standalone artifact adaptation. The previous
`--manager-tui`, `--manager-tui-version` and `adapt-manager-tui` spellings remain
equivalent input aliases. Aliases select the same artifact and validation path;
they do not change Manager or TUI execution lifetimes. Signed helper records retain
the exact `termux-manager-tui-v1` identity required by existing consumers.
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

## Release execution

One maintained release workflow owns construction and publication. main is a
small immutable-source caller, with no second implementation or manually mirrored
producer. The caller selects an accepted exact implementation commit; nested
publication logic comes from that same source. Job-specific permissions separate
read-only construction/smoke from signing/public mutation. Signing secrets appear
only at the signing boundary. Development pushes and qualification dispatches are
source-only by default. A green CI run or scheduled reconciliation is not release
authorization. Before creating even an unsigned public input release, changing
main refs or dispatching publication, the controller requires `release-intent-v1`
from the exact immutable main commit it is evaluating. That bounded JSON record
has schema `codex-release-intent-v1`, the exact `source_sha`, selected
`upstream_version` and boolean `publication: true`. A missing or nonmatching
intent leaves development accepted and publication pending; malformed or
unavailable authority fails closed. The candidate branch cannot supply intent.
All existing exact-run, caller-pin, signing and promotion checks still apply.
A prospective Full input or reference additionally requires its successful exact
[Full qualification](QUALIFICATION.md), selected TUI run and unchanged main before
any public frontend container or reference mutation. Proof transport is bound to
its immutable artifact ID, archive digest, bounded inventory and source runs.
A Core-only lane discards pending optional frontend publication and prospective
optional binding; it preserves issued Full and rollback assets. The signing
reservation separately compares the actual assembled candidate with that proof,
so earlier controller admission cannot authorize different bytes.
A qualification completion relay additionally requires an explicit manual
`advance_release=true` input on the official development branch; default false
prevents a source push from requesting publication. An explicitly
authorized current-version rebuild uses its exact qualified upstream/archive and
normal next sequence, with all ordinary integrity/publication gates intact.

Remove completed one-shot acceptance/deployment code paths, old mutation workflows
and experimental preview construction. Existing immutable migration releases and
required consumer compatibility remain ordinary product support, not executable
historical producer modes. Preserve normal signature/inventory verification,
LKG continuity, candidate site size, complete public readback, disposable consumer
proof and non-forced exact-parent CAS. No public release or installed activation
is implied by a development-plane change.


### Global issue reservation

Read-only qualification computes a prospective sequence from authenticated Core
and Full heads and all existing release/candidate/reservation tags. Signed but
unpublished candidates therefore cannot have their issue number reused. Only a
separately authorized publication job reserves that sequence by atomically
creating an annotated `codex-release-reservation-sN` tag in the existing repository;
it never force-updates an existing ref. The bounded canonical record binds the
exact source, immutable authorization main commit, complete candidate digest,
generation, upstream version, component revisions, composition revision and date.
The source must still have exact release intent and main must remain unchanged
before reservation. A concurrent conflicting claim refuses; an exact repeated
claim returns its original record and requires reconciliation of the original
signing/publication result, without issuing a second signature. Failed or uncertain
signing does not free a sequence for other inputs.

The signer revalidates the reserved candidate and selected component identities
before reading the signing secret. Its optional signed revision metadata must
match the reservation and every applicable manifest digest. Sequence allocation,
component revision allocation and signing are separate operations; development
pushes, Source Acceptance and component/Full qualifications neither reserve tags
nor sign or publish. These source rules apply to future authorized issues and do
not rewrite any already signed Core or Full generation.
