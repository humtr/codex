# Independent terminal return (Manager)

> Normative annex of [SPEC](../../../SPEC.md); [INDEX](../INDEX.md) is navigation only and [CURRENT](../../../CURRENT.md) is observed status.


Codex Manager owns profile selection, launch convenience, notifications and
foreground conversation binding. Core remains independently usable. The
Core-to-Manager handoff strips inherited LD_PRELOAD and LD_LIBRARY_PATH as
ordinary upstream launch does. Caller preferences and workload authentication
remain intact. Manager routing must not reinterpret upstream argv or make
ordinary execution depend on Android connectivity. AI owns its separate
provider conveniences. No additional daemon, watcher, PTY manager or
conversation execution authority is introduced.

The private `__terminal-launch-command-v1 off -- PROGRAM ARGV...` adapter routes
an explicitly selected same-UID local executable through the same one-shot
window path, without interpreting its authentication or profile semantics.
`__terminal-launch-capabilities-v1` reports exactly
`codex-terminal-launch-v1\ncontainer=off\n`. Unsupported containers,
including `hidden` and `status`, fail as usage errors before starting work.
AI passes its existing authentication supervisor intact. The Manager build
does not include tmux session creation, attach, pane scanning or pane
selection. Historical code and tests are retained only under
`docs/development/manager-tmux/`, outside active Cargo sources and tests.

`codex termux launch [--profile PROFILE_ID] [-- UPSTREAM_ARGS...]` presents
the existing profile inventory when no profile is explicitly selected.
Up/down and Enter select; Esc/Ctrl-C cancel without work. An inherited external
CODEX_HOME remains a distinct current-home choice. Explicit profiles use
the existing profile contract. CWD, raw upstream argv and caller color
preferences (including absence) are preserved. Ordinary interactive Codex
launches use optional window routing without showing a profile picker.
Noninteractive and maintenance commands retain ordinary execution behavior.
`codex termux attach`, `codex termux tmux` and `--tmux=...` are unavailable
in this build; they do not start or attach a session.

For native Android origin/validate/focus support, run in the invoking
terminal and return to that exact live terminal. For confirmed unsupported
native API and working AM socket, create one uniquely named dedicated
terminal per launch and run directly there. If AM support is missing, failed
or inconclusive, ordinary execution remains usable and exact return is
reported unavailable, without fabricating a successful return.

The native extension uses the existing same-UID AM socket and app session
table. Origin is resolved from actual socket peer ancestry. Validation and
focus require an exact live handle, process ID and kernel start identity.
Closed, recycled or ambiguous references fail without creating a window.
Transport acknowledgment alone is not proof of terminal selection; visible
names and titles are not native addresses.

The stock-APK route uses RunCommandService shell_name and no-shell-with-name.
Its immutable random launch name is separate from visible title and
conversation UUID. Construction is allowed once at launch. Raw arguments,
CWD and caller environment travel in a bounded, private, single-consumption
local descriptor, never through notification actions; the descriptor is 0600,
at most 64 KiB, removed before workload execution and never logged. It may
temporarily contain caller authentication environment, while permanent
terminal bindings never do. A new terminal retains its own identity
variables. Return validates the live launch root and foreground binding
before selecting the registered name with an inert executable; it never
starts or resumes Codex. Stock API cannot atomically select an existing
window: manual rename or a closing-window race can create an inert empty
terminal. This is not equivalent to native existing-handle guarantees;
failure must not retry construction or change routes automatically.

A private process-bound terminal record contains only the process start
identity, native foreground FD identity and terminal route. It is mode 0600
under the private Manager terminal directory, bounded and checked against
live same-UID processes; it contains no conversation content, credentials
or launch arguments. Dead records are pruned lazily. A notification resolves
exactly one live foreground UUID. Missing, ambiguous, closed or recycled
targets refuse return. /new, /resume, /switch, profile changes and /title
cannot replace the originating terminal. Visible titles remain presentation.

Manager owns `notify set --focus window|termux`. Default is window;
`termux` selects Activity-only return. The private notifications/focus-v1
contains `window\n` or `termux\n`. The removed `tmux` setting
is invalid in both stored records and command-line input. Absent state uses
window; malformed/insecure state uses Activity-only return. For a canonical hook
session_id in window mode, Manager uses installed Core's safely quoted
private `__terminal-focus-v1 UUID` action; no AI executable is needed.
Malformed or ambiguous IDs preserve the ordinary notification and cannot
become shell input. Activity-only mode brings Termux forward without
selecting a terminal.

#### Optional terminal launch capability

The read-only private command `codex termux __terminal-capabilities-v1 -- ARGV...`
queries this generation's actual launch capability. Core supplies only a qualified
frontend availability bit and its existing supported-argv decision to the same-
generation Manager; Manager emits exactly `codex-terminal-capabilities-v1` and
`identity=native` or `identity=title`. No authentication, profile/session scan,
configuration mutation or runtime launch occurs. AI never parses a generation
manifest or adds an independent qualification authority. Missing Manager/query or
unsupported argv uses the historical title-ID fallback. For `identity=native`,
AI leaves the user's complete upstream terminal_title configuration unchanged;
Manager's existing descriptor owns foreground UUID independent of `/title`.

Codex launch and notification identity come from Manager. AI must not override
native terminal_title configuration when native foreground identity is supported,
or introduce a competing Codex profile/window authority.


#### Native terminal identity bridge

The optional qualified native frontend maintains one same-UID private temporary
file descriptor, created exclusively
with mode0600 and close-on-exec and immediately unlinked. Its former basename
is `.codex-terminal-thread-v1-PID-NONCE`.
Its entire content is the canonical full UUID of the displayed thread, or empty
when no thread is displayed. It updates this slot before rendering a changed
foreground thread, independently of title configuration. Native identity remains
in the open unlinked file, automatically reclaimed on process exit; there is no
new persistent session index. It calls the private
exact command `codex termux __terminal-bind-v1 FD` once to register this descriptor.
Its bounded registration wait runs outside the rendering thread, with at most one
attempt in flight. Foreground slot updates and ordinary TUI input never wait for
the optional Manager to reply.
This registration applies to native frontends in directly bound terminal PTYs;
there is no Manager tmux socket, pane or process-tree discovery.
Core forwards only that exact three-argument shape to its same-generation Manager.
FD grammar and caller/TTY authority remain Manager responsibilities. Ordinary
Core dispatch and installed signed generations gain no terminal controller.
Manager verifies its direct parent is a same-UID installed native frontend and its
private unlinked foreground descriptor. Registration publishes one bounded private
process-binding record with stable PID/start, FD inode, TTY and qualified Android
route. Foreground UUID stays in the live descriptor, never a persistent conversation
index. Selection rechecks process, FD, TTY and window identity.
A frontend re-exec may replace its own process binding: the prior private
record must have the same schema, PID/start, TTY and Android route, and the new
unlinked descriptor must qualify against the current native image. Replacement is
atomic; mismatched, foreign or malformed records remain untouched. This preserves
return across `/switch` without changing the original Android terminal.
This command cannot rename a native Android terminal or start a workload.

AI consumes this binding without rewriting it, reads the current full UUID from
that qualified native process's descriptor, and rechecks it at the selection and
Android dispatch boundaries. A thread change remains visible even if Manager
registration later becomes unavailable; AI must not cache stale UUIDs.
With a binding present, malformed, stale or mismatching native-process/descriptor
identity refuses focus; it never falls back to the title. Unbridged installed
frontends retain the previous title qualification
until the native producer is admitted. The bridge is independent of upstream
`/title` selections and OSC display text. Core's ordinary launch and the current
original Android terminal registration remain unchanged. Native source/build
proof precedes admission; consumer-only or seeded fixtures do not establish
automatic thread-switch integration.

`codex termux` is a Manager boundary and is never passed to upstream. Manager
v1 has profiles, notifications, and a bounded active-task assistance family.

```text
codex termux
codex termux help
codex termux launch [--profile PROFILE_ID] [-- UPSTREAM_ARGS...]
codex termux profile list
codex termux profile current
codex termux profile create <PROFILE_ID>
codex termux profile delete <PROFILE_ID>
codex termux profile rename <PROFILE_ID> <NEW_PROFILE_ID>
codex termux profile default [PROFILE_ID]
codex termux profile use <PROFILE_ID> [--] [UPSTREAM_ARGS...]
codex termux task [THREAD_UUID]
codex termux task status [THREAD_UUID]
codex termux task reconnect <THREAD_UUID>
codex termux task stop <THREAD_UUID>
codex termux task takeover <THREAD_UUID> [--profile PROFILE_ID] [--force-server PID:START]
codex termux notify show
codex termux notify set [NOTIFY_OPTIONS...]
codex termux notify test
```

`codex termux` with no command is equivalent to `codex termux help`.
Session browsing and resume belong to upstream Codex; Manager owns profiles,
active-writer assistance and notifications. An unavailable Manager reports a
bounded unavailable result; it never forwards an unknown `termux` command.
Manager does not provide `codex termux install`, `codex termux update`, or a
second doctor/version authority. Installation, update, rollback, and top-level
doctor remain Core commands.

`PROFILE_ID` is one ASCII path-safe identifier of 1--64 bytes beginning with
an alphanumeric character and containing only alphanumerics, `.`, `_`, or
`-`. `default`, `home`, `termux`, `.`, and `..` are reserved aliases or
rejected names. SCS deliberately provides no Manager-owned `SESSION_ID` grammar. Upstream
`codex resume` is the canonical session picker/resume surface and already owns picker
mode, `--all`, `--last`, names, and UUIDs. To use that upstream surface with a custom
Manager execution identity, invoke `codex termux profile use <PROFILE_ID> -- resume
[UPSTREAM_ARGS...]`; Manager forwards those upstream arguments byte-for-byte and does
not inspect conversation storage.
Arguments after `--` are forwarded byte-for-byte to the Core entrypoint. A
Manager child launch preserves standard streams, TTY, signals, process exit
status, and raw argument bytes at the final Core execution boundary.

The generated upstream hook command may call the bounded internal Manager
endpoint `codex termux notify emit <EVENT>`. The native completion callback
may append exactly one JSON argument for `Stop`. It is not a configuration or
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
One concise ordinary signed-update detail line names Base or Full and its signed
release number, including the previous number when changing releases. Full also
shows its exact base release number when authenticated display metadata exists.
Default update output shows the selected Termux edition/base and each selected
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
current key under the activation lock, including Base without an enhanced TUI;
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
release declares its authenticated Base, Core uses that Base release's own
publication time and notes. Full package time must not replace the Base time.
Without a Base identity, Core is explicitly labelled as part of the Full package. Manager and
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
