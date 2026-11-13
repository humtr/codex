# Core update selection and installed version display

> Normative annex of [SPEC](../../../SPEC.md); [INDEX](../INDEX.md) is navigation only and [CURRENT](../../../CURRENT.md) is observed status.

## Edition selection and offline inventory

`codex update --core` selects the signed Core/Runtime lane; `--full` selects
an authenticated Core/Manager/Enhanced TUI composition. With neither option,
the authenticated installed inventory selects the lane. An explicit immutable
source still must match the requested inventory. Core/Full flags may accompany
manual waiting/details, local/remote selection or local build; conflicting flags,
rollback, force and help combinations fail before effects. Existing Base index,
schema and generation identifiers are compatibility storage, not a third public
edition. Full-to-Core retains the previous Full; selecting a lower public
sequence does not bypass anti-rollback or the explicit rollback contract.

A Core local build consumes qualified Runtime inputs and Core without requiring
a same-version Manager or Enhanced TUI. Full retains its actual frontend and
composition qualification. Explicit Core selection does not update Provider.
Default manual update retains its independent already-selected Provider group.

The missing-release choice is exactly `[1] Build locally`,
`[2] Request remote build`, and `[q] Keep current`. Keep/cancel never starts a
build. Non-TTY input never guesses consent or emits terminal control sequences.
Failure, cancellation, unchanged current and committed activation remain distinct.

Core provides `codex termux version`, `--details`, and `--json` offline without
Manager, TUI, Provider service or network access. It verifies the selected signed
manifest and payloads, reports the authenticated upstream version, Core/Full,
release sequence, generation, Runtime/Core digests, platform, optional component
availability and retained previous generation. A damaged optional file does not
relabel Full as Core. JSON uses `codex-termux-version-v1`. The independently
selected Provider package is verified through the existing package store; service
and authentication readiness are explicitly not queried, and no Core selection
does not prove that an externally managed Provider service is absent.
The offline version result describes *package verification only*; it must not
label a selected signed Provider as compatible with a possibly different
running process. Interactive/native inference uses the actual scoped service
capability proof before request admission. When an update selects a newer
package but leaves an existing listener/profile unchanged, the update result
must preserve that distinction: installed/selected is not activated/ready.
Core-only and ordinary OpenAI flows do not require this optional readiness.

Optional signed `component-input-v1` metadata is bounded to16KiB, binds the exact
manifest and per-component artifacts, and carries positive module/composition
revisions. Legacy, missing or unverifiable optional metadata yields unavailable
revisions, never revisions inferred from global sequence numbers. It is not an
alternative payload admission authority. Only new acquisition/staging copies
sidecars; existing signed generations are not rewritten. `codex --version` and
`codex -V` retain exact upstream behavior without update/optional discovery.

## Authenticated update behavior and presentation

The Core-owned update surface accepts no arguments, `--details` optionally
combined with one of `--wait`/`--no-wait`, either wait option alone, `--help`, `--force`, `--build-local`, `--local <DIRECTORY>`, `--remote <HTTPS_BASE_URL>`, or `--rollback`
after `update`. No top-level `codex rollback` command is introduced. A malformed
invocation beginning with one of those Core selectors is a Core usage error. The no-argument
form is the primary product path: it first tries the signed stable wrapper channel
and offers an explicit build-policy choice only when official freshness proves
a required artifact is missing. Transport failure remains a transport failure. An already-current
signed release remains a no-op; an authenticated rollback hold never triggers a
local build. `--build-local` is explicit and also refuses an active rollback hold. A
verified index with an invalid signature, malformed fields, an incompatible
release, a digest or mode mismatch, a candidate-probe failure, or an activation
failure is never converted into a local-build fallback. The explicit local,
remote, and rollback forms remain secondary diagnostic/recovery paths; local and
rollback remain offline, and the explicit remote form remains an immutable
signed-generation source. No top-level `codex update` argument is passed to
upstream, and Core never invokes a package manager or an upstream self-updater.

Human `codex update` output distinguishes the upstream version from the Termux
edition and public release number.
The ordinary signed-channel path derives both the installed and candidate Codex
versions only from already authenticated generation metadata; it must not execute
an untrusted candidate merely to discover presentation data. Before the candidate
version is authenticated, an interactive update may show only a transient
`Checking for updates...` status.

When both stdout and stderr are attached to terminals, Core may use exactly one
transient stderr progress line. It replaces that line in place with bounded
carriage-return/erase-line terminal control and flushes each phase immediately.
The transient line is always cleared before a permanent success or failure
result. The ordinary signed-channel phases are concise user-facing states such as
`Checking for updates...`, `Downloading signed Termux release...`,
`Verifying release signature and contents...`, `Checking candidate runtime...`,
and `Activating Codex <VERSION>...`. If either stream is not a terminal, Core
emits no transient progress line and no cursor-control bytes.

For signed releases that publish the optional authenticated
`compat/download-size-v1` control resource, Core may enrich only the
`Downloading signed Termux release...` transient state with actual downloaded
bytes and the authenticated total payload size. The sidecar is not part of the
installed generation inventory and therefore does not change release-manifest
format or legacy-consumer compatibility. It is signed by the release key and
binds itself to the exact signed `release.manifest` SHA-256; its ordered file
list and file count must match the signed manifest inventory one-for-one, and
its declared total must equal the checked sum of those file sizes. If the
sidecar body is unavailable, Core falls back to the existing phase-only
presentation. Once a sidecar body is fetched, an unavailable or invalid
signature, manifest-digest mismatch, malformed/duplicate/missing file entry, or
actual GET byte count that differs from the authenticated file size fails the
update closed. Progress is derived only from bytes actually written by the GET
transfer; Core does not issue per-file HEAD requests. Non-TTY output contains
neither progress counters nor terminal-control bytes.

After both versions are authenticated, a genuinely version-changing signed
channel activation writes the permanent stdout header
`Updating Codex <OLD> -> <NEW>...`. A same-upstream-version corrective
generation instead writes `Updating the Termux release for Codex <VERSION>...`.
Successful signed activation then writes exactly
`Verified and activated the signed Termux release.` followed by
`Codex <NEW> is now active.`. Exact-current success writes
`Codex <VERSION> is already up to date.` and performs no staging, probe, or
state mutation. Human operational update failures omit redundant command-name
prefixes such as `codex update:`; after clearing any transient line they emit
one concise sentence ending in a period. Success and failure results use
plain text with no emoji decoration. Usage errors retain the canonical usage
surface and are not decorated as operational failures.
One concise ordinary signed-update detail line names Core or Full and its signed
release number, including the previous number when changing releases. Full also
shows its exact Core release number when authenticated display metadata exists.
Default update output shows the selected Termux edition/Core and each selected
optional package once. A Provider result explicitly describes package installation;
it does not imply service activation. Unchanged output omits immutable generation
IDs, repeated component rows and publication timestamps. `codex update --details`
retains publication dates, generation identifiers and component rows, and may be
combined with `--wait` or `--no-wait`. Other update forms keep their existing
argument scope. The latest upstream version is shown when it differs from the
installed version, changed in this invocation, or details were requested; equality
alone produces no duplicate upstream row. A short
changelog is shown only for a component whose authenticated executable inputs
change in this update; unchanged components retain their concise package identity
without repeating notes or full-note links. A Full repack alone does not count
as a Core, Manager or TUI change when that component's signed bytes and modes
are unchanged. Upstream notes are shown when a newer official version is
available or this invocation changed the installed upstream version. Exact-current
updates repeat neither component nor upstream changelogs. Installed and latest/available versions are distinct;
a default manual `codex update` updates an already selected Provider as a
separate dependency group after the Core attempt and before any missing-upstream
build choice or wait. Each group admits against its own authenticated current
state: a Core candidate/transport/hold failure retains its failing result while a
ready Provider may still advance under the valid installed Core authority. An
invalid installed Core authority cannot authorize Provider effects. Provider
operations reauthenticate the installed Core envelope/inventory with its admitted
current key under the activation lock, including Core without an enhanced TUI;
ordinary-launch assumptions about immutable files are insufficient at this update
authority boundary. A Provider failure likewise does not suppress Core freshness,
keep/local/remote choice or publication waiting. The updater retains both group
results and returns failure for an incomplete update. Manual waiting returns an
explicit completion/continue decision to the updater; it does not replace the
process before group errors are reported. An explicit continue/current-client
transition reports an earlier module failure before launching the admitted client;
that client's later exit belongs to the requested client run.
An absent/unselected Provider causes no
installation, store creation or Provider request. Startup discovery and explicit
Core source/force/local-build/rollback commands retain their bounded Core scope.
A Provider group failure returns failure with its actual cause; an independently
activated Core remains active and is reported as such. Missing Core artifacts do
not prevent a ready independent Provider update. When a Full
release declares its authenticated Core (legacy Base), Core uses that Core release's own
publication time and notes. Full package time must not replace the Core time.
Without a Core identity, Core is explicitly labelled as part of the Full package. Manager and
enhanced TUI identify the Full package that supplies them, including its package
publication time, rather than inventing independent module release dates.
Use full date/time in KST (UTC+09:00) when the source supplies a timestamp.
Date-only historical metadata retains its actual UTC date and precision; never
invent midnight, use installation time or infer publication from file timestamps.

Official upstream metadata already acquired by manual update supplies its
published_at and release body. For the exact admitted Core generation and the
latest signed Provider index, bounded explicit-update requests may read the
corresponding public GitHub release tag's published_at/body. These are informational
publication records, not signature/admission authority or executable instructions.
They cannot select a version or weaken signature, sequence, rollback or source
checks. Unselected Provider modules cause no Provider query. Publication failures
show unavailable information without failing an otherwise valid signed update;
ordinary launch, offline operations and background services make no such requests.
An explicit Provider stable update authenticates its current index first. If its
generation, manifest digest and sequence match the admitted selection, verify the
installed payload and return unchanged before fetching package controls, release
notes or payloads. A matching label alone is insufficient; changed or corrupt
input must not become an unchanged result. Default manual update additionally
shows the authenticated selected Provider's publication identity/date when
unchanged, without its changelog; that optional publication read is not
package acquisition or selection authority. Explicit Provider update retains its
unchanged fast path without publication requests. The Core release display no
longer performs a separate informational Provider index lookup: the Provider update
operation owns its one index admission, payload reuse/acquisition and result.
A changed stable Provider release authenticates its new envelope before reusing
payloads. A retained complete generation is admitted directly through the same
installer if its signed envelope, inventory and sequence match the new index;
corrupt or conflicting retained generations fail without overwriting them.
Otherwise an entry with the same signed path, digest, byte count and mode in the
verified current selection may be copied into private acquisition through the
same target-bound file copier as Core. The copier rejects aliases/special files,
checks actual ownership/size/mode and rehashes the private copy; it never hardlinks
installed files. Changed entries or rejected reuse candidates are downloaded and
verified against the new signed inventory. The final installed/current payload
checks and artifact probe remain required. Corrupt admitted selection is still
an error, not an implicit repair or unchanged success. Explicit --remote
installation/reinstallation retains acquisition of every entry; local installation
retains its existing path. All remote Provider payloads use Core's existing private,
digest-keyed download cache. Interrupted bytes remain untrusted and may resume only
on the next explicit request after the new signed envelope is admitted. A complete
cached entry is rehashed before staging; invalid complete bytes or rejected ranges
are discarded with an error, without an automatic restart. Provider controls have
finite transfer deadlines, and payload downloads use Core's progress-aware idle
bound for cache misses and resumes. Terminal progress identifies the Provider file
and authenticated byte total; redirects, signatures, sequence floors and final installation admission keep
their existing authority. No new cache format, public switch or release format is
added.
Within one explicit command, optional release information is acquired once per
admitted candidate generation and is reused during CI publication waiting. A
changed candidate receives a fresh view; this command-local snapshot is never
update or rollback authority. Custom/private release origins do not trigger public GitHub tag or Provider
publication queries; they retain their explicitly supplied signed display metadata.

New release producers publish brief component sections from release/notes.md
(at most16KiB of UTF-8 source text);
Core, Manager, enhanced TUI and Provider sections describe their actual package
scope. Display at most three brief entries per component and link the complete
original release notes. Upstream uses the official release body's public entries.
Missing notes remain unavailable, not a fabricated claim of no changes.

The historical signed release date is the publisher-declared UTC date of
signing/preparing that immutable release.
It is carried as optional `display` fields in the existing signed
`component-input-v1`, bound to the exact manifest digest/generation. Older sidecars
without display fields remain valid. Missing/unavailable/invalid optional display
information is reported as unavailable; it cannot select a release, change
activation eligibility, trigger local building, or fail an otherwise valid update.
Display acquisition is bounded and only on explicit remote update, including its
exact-current result; it adds no ordinary-launch query, cache authority or daemon.
Local/offline lifecycle does not acquire remote display data. Raw generation IDs,
digests and API/schema identities remain diagnostic details.
