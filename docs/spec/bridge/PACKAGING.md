# Optional Provider packaging and selection

> Normative annex of [SPEC](../../../SPEC.md); [INDEX](../INDEX.md) is navigation only and [CURRENT](../../../CURRENT.md) is observed status.


In the deployed format Base excludes all optional components and Full composes
qualified Manager/frontend; Provider is a separately selected package. The future
selectable-component/Full target is defined under device service integration.
Provider remains opt-in outside Full and must not require signed desktop app/rootfs download
or login on ordinary update/startup. A stable provider needs a declared versioned
payload/dependency inventory, authenticated delivery and tested activation/rollback
before replacing its experimental installer. Preserve app authentication and user
state; importing provider source does not authorize redistributing third-party
desktop software. Choose the bounded packaging extension from its real reachable
payload; do not assume the current eight-file inventory already supports it.
The optional-component packaging contract selects an independent Provider payload, outside Base/Full generation
inventories. Core exposes `codex provider install chatgpt --local DIRECTORY` or
`codex provider install chatgpt --remote HTTPS_DIRECTORY_URL`,
`codex provider install chatgpt` / `codex provider update chatgpt` (explicit stable
optional-component discovery),
`codex provider status chatgpt`, `codex provider rollback chatgpt`, and
`codex provider chatgpt ARGS...`. These management routes need neither Manager
nor TUI. Ordinary Core execution never consults or starts an absent Provider.
Explicit Provider installation may enroll a payload; default manual update may
acquire only a previously selected Provider. Both consumers use the same verified
acquisition/selection implementation. Ordinary launches do not enroll or update it.
Remote installation uses a canonical HTTPS directory URL with a trailing slash.
Remote asset names flatten the manifest’s fixed `licenses/NAME` paths to
`licenses--NAME`; all other names stay unchanged. This permits immutable GitHub
Release delivery without a separate mirror or changes to Base/Full public heads.
Core downloads the bounded manifest/signature first, authenticates them against
its admitted authority before fetching payload files, enforces signed file and
aggregate bounds and reuses the same staging/selection checks as local install.
Controls have finite transfer deadlines; payload downloads fail after 120 seconds
without new bytes and continue while bytes are arriving, subject to signed size ceilings.
Failed or uncertain transfers are never retried implicitly and cannot select a
payload. Stable discovery reads the independent `provider-index-v1` and its
detached signature from the owned main branch. The bounded ordered stable index
binds provider/generation/sequence, immutable release directory and manifest digest;
Core authenticates it with the current admitted update authority and requires
all bindings to match the downloaded package. Base/Full heads remain independent.
Only the invocation-owned private acquisition is removed on exit; an
interrupted acquisition remains unselected for later owned-artifact cleanup.

New Provider payloads contain the Android/AArch64 executable, dependency lock,
NOTICE, two retained third-party licenses and an opaque source-input inventory.
New packages use a six-file signed inventory; Core continues to authenticate
immutable eight-file packages from earlier releases for existing selection and
rollback. New six-file packages require this updated Core verifier; existing
Core versions that accept only eight files must be upgraded first. Neither
signed layout permits undeclared assets. The ordered manifest
binds provider identity, generation, monotonically increasing release sequence,
base source commit, source-input digest, target/API/dependency contract and exact
file names/hashes/sizes/modes. Detached Ed25519 authentication uses Core's admitted
update authority, never a public key supplied by the candidate itself. Payloads
contain no Linux app/rootfs, login, profiles or model/user configuration. External
PRoot/Xvnc/explicit app rootfs are needed only for managed app work; viewer assets
are optional. Direct Chat-first native work retains its no-app boundary.

Install verifies the complete immutable inventory before and after private
staging, probes the executable without inference, then atomically selects it.
Selection retains current/previous identities, their admitted verifier keys and
a high-water release sequence. Rollback revalidates the admitted previous payload
and never lowers that high-water sequence. Equal identity/content is observable
without replacement; changed bytes under an existing identity are rejected.
Unknown/malformed state fails before mutation. Install/rollback serialize selection
writes through the existing Core activation lock and change only the atomic
selection record and absent immutable payload directories. Running services and
pinned profiles retain their original immutable generation; install/rollback never
stop services, replay work, rewrite profiles, delete older generations or overwrite
user data. New ordinary launches use the newly selected generation. Service restart
or explicit profile preparation is a separate user/configured lifecycle decision.

The shared store lease is retained for installed older Core consumers whose writers
still use that directory exclusively. New selection writers take the same shared
lease under the Core activation lock, so old running readers do not block them,
while an in-progress old exclusive writer is still excluded. This compatibility
exists while installed old Cores remain supported; it introduces no new lock/state
file. Launch revalidates signature/inventory under that shared lease before exec.
The Provider keeps inherited non-stdio descriptors, including the store lease, in
its own process
but marks them close-on-exec before any runtime or child startup. Only descriptors
explicitly exported by a child launch cross that boundary. Native backends, app
children and detached service children must not retain an incidental parent lease;
a live Provider keeps its immutable executable and does not block new selection.
Stopping an idle Provider must not require terminating an independent Native
backend solely to release such an inherited lock. This adds no environment/FD ABI
and does not change existing Core dispatch, payload verification or work authority.

Package selection and generated native-profile preparation are distinct. Existing
profiles/catalogs keep their immutable executable binding until explicit
`codex provider chatgpt prepare`; install/rollback report selection only. Retain
generations referenced by existing profiles and running work through verified retirement.
This preserves the current direct-executable auth/checkpoint contract and does not
silently replace the installed JS adapter or any user-selected model route.
A default manual update advances the selected Provider independently of a Core
rollback hold. Provider rollback retains its previous/high-water contract; a later
explicit `codex update` checks its forward channel again. No background Provider
update or service/profile restart follows from that selection.
An independently signed Provider payload's release sequence, its Core-selected
package identity, its legacy `current` pointer, the PID/start/executable of the
actual responding service and a profile's base URL/token source are separate
facts. The client must authorize the actual listener and check its advertised
Native major/capabilities before effectful inference. Signed package selection
never attests a live process; an active service/Native thread is not replaced
on package installation, and legacy profiles keep their previous bindings
until explicit user-authorized preparation. Core-only and ordinary OpenAI
execution must remain independent of Provider availability and compatibility.
Authenticated remote delivery/promotion and actual-device activation require their
separate signed-release and device proofs/authorization; local fixture signing does not establish either.
Once the Core interface work is admitted, Manager/provider-only changes must leave
base inputs unchanged. Upstream/backend changes still require the applicable
frontend and execution-protocol compatibility proof.
