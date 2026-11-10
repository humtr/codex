# Installation-wide upstream resume visibility

> Normative annex of [SPEC](../../../SPEC.md); [INDEX](../INDEX.md) is navigation only and [CURRENT](../../../CURRENT.md) is observed status.

### Installation-wide upstream resume visibility

Core extends the upstream-owned shared
conversation architecture to known external execution homes directly beneath
`$HOME/.codex-profiles/<PROFILE_ID>`. Core must apply the same canonical SQLite
requirement even when those launches lack a Manager-provided
`CODEX_SQLITE_HOME`. Canonical/default, registered Manager, and declared external
execution identities discover the installation's one upstream conversation
store under `$HOME/.codex`; credentials, user/provider configuration, logs and
runtime snapshots remain profile-local. An arbitrary external `CODEX_HOME`
remains isolated rather than becoming an implicit import source. Core validates
the declared topology and never routinely reads SQLite, parses transcripts,
merges sessions, or maintains another discovery index. Empty declared homes
may receive only the exact compatibility links; nonempty legacy roots require
the bounded operator transition first. User argv is unchanged.

Upstream resume filtering is retained unchanged: `cwd` covers the current CWD
across all execution accounts, and `all` covers all CWDs across all execution
accounts, with the existing upstream archive/provider/source semantics. Account
identity never partitions local discovery. New sessions written through any
supported execution identity immediately enter this same store. Discovery does
not merge authentication or let session creation identity select credentials.
The user explicitly retains upstream's CLI/VSCode picker source policy for both
CWD and All. Retained exec and internal guardian records stay in the shared
store and remain addressable through upstream APIs; they are not additional
interactive picker entries.

Any legacy profile-data transition is a separately authorized bounded
operation, not a launch-time side effect or recurring deletion policy.
Preserve canonical conversation identity, the maximal verified history,
authoritative original record types and active writers. Duplicate thread
payloads may merge only when their complete sequences prove a prefix
relationship; divergent histories fail without alteration. No general SQLite
merge or profile-history scanner is introduced.

A bounded online handoff may publish retained authoritative rollouts by atomic same-filesystem move
and atomically exchange a legacy session directory with its exact canonical
compatibility symlink. This preserves the active writer's inode while future
path opens use the shared store. Immediately before handoff, inspect every
upstream writer's open retained rollout inode: multiple active differing copies,
an active nonmaximal copy, or an active excluded rollout reject apply. Existing
private SQLite projections used by pretransition writers must remain untouched
until those writers exit; all new Core launches enforce canonical SQLite.
Per-profile upstream writer coordination locks serialize the handoff; per-thread
lock inodes move into the canonical namespace with their active owners. This
avoids Android-denied hardlinks while retaining the writer FD. Inactive legacy
projections and displaced payload trees may then be removed
within the declared roots. No writer is killed, no auth/config is copied, and
no restoration backup is created. The online exception does not permit a
steady-state migration service or a second thread/schema authority.

### Upstream session authority

The historical `codex termux session list/resume` family is retired. Upstream
owns discovery, filtering, selection and resume through ordinary `codex resume`
or `codex termux profile use <PROFILE_ID> -- resume [UPSTREAM_ARGS...]`.
Manager must not restore the former directory scanner, session index, saved
selection dependency or alternate resume grammar. Retired helper history is
retained in Git and is not a current command contract.

### Active-task account assistance

`codex termux task` presents only currently loaded upstream work, not conversation
history or a second session picker/index. A supplied canonical lowercase thread
UUID restricts the view. `task status [THREAD_UUID]` is noninteractive and reports
only UUID, owner execution home/account label, runtime-bound server PID/start,
native state and force-server identity. No transcript, preview, goal objective,
credential or tool output is printed or persisted. External supported CODEX_HOME
identities are included, not just Manager-registered profiles. All retained Core
server rendezvous records are considered, including old-generation active servers.
This helper adds no permanent task registry, watcher, supervisor or automatic
account switch. Quota rotation is the separate Manager/TUI contract above.
Discovery holds the existing execution directory's shared coordination lock
while classifying its private PID record. A confirmed absent or fully exited
process is ignored read-only before parsing its inactive binding schema or
requiring its home, runtime or socket to remain present. For a current valid
binding, a PID whose kernel executable differs from its recorded runtime is
also retired: PID reuse does not make that unrelated process a server. A running
image of that same runtime which the kernel marks deleted remains an uncertain
live owner; it is not classified as PID reuse. Failure to inspect a still-live
process is not proof of retirement. This does not restore
retired binding formats as executable authority. Initialization in progress,
invalid PID/directory metadata and uncertain liveness still fail discovery.
Retired records are neither deleted nor repaired by Manager. Every live process
still requires the complete current namespace/home/runtime/socket and peer
checks; aliases and substituted bindings remain invalid. The64-server limit
counts live candidates, not accumulated retired records; the existing end-to-end
scan deadline bounds enumeration and native queries together.

Owner means the currently verified kernel writer-lock holder and that process's
execution identity, never the original conversation author, recorded origin,
latest historical author, or a server that merely has a read-only copy loaded.
Discovery must match the current native lock inode and its exclusive kernel lock
to the exact runtime PID/start and peer-verified server. A released writer has
no current owner, even when its old server still reports the thread loaded. After
each handoff subsequent discovery must follow the new holder. Reconnect/stop
revalidate ownership immediately before acting; uncertain ownership fails closed.

Manager may read existing private Core server binding metadata and connect only
to same-UID kernel-peer-verified exact runtime processes. This is read-only
consumption of execution metadata, not generation selection/preparation/activation
ownership. Core gains no task, cancellation or transfer controller. Native
WebSocket JSON-RPC owns discovery/status, cancellation and history. Connections,
messages, server/task counts and scans are bounded; incomplete/ambiguous discovery
never authorizes cancellation, force termination or successful takeover.

`task reconnect UUID` selects the unique live owner's CODEX_HOME and existing
server through upstream's explicit local `--remote unix://SOCKET` TUI route and
resumes the exact UUID. This preserves old-generation server routing instead of
silently launching a competing current-generation writer. It never signs out or
changes a server's authentication. Missing/ambiguous/untrusted ownership fails
without mutating account or conversation state.

`task stop UUID` includes loaded descendants identified by native ancestry in the
same owner server only when that server also currently holds their writer locks,
pauses their active native goals, requests cancellation of the actual active
turns, and observes terminal/inactive state; cancellation submission alone
is not success. The Manager connection never claims to unsubscribe other clients.
A live remaining subscriber may retain the writer; report it distinctly. A
successful stop does not imply another account can write. User-requested attached
background terminal cleanup uses the native thread-scoped endpoint. Detached
processes outside native ownership and already-applied filesystem edits are not
claimed undone.

`task takeover UUID` performs that stop, then requires native writer release before
ordinary same-ID resume through the current inherited account, saved default
(native default when neither is selected), or explicit validated
Manager profile. Existing native writer/coordination files may be opened read-only
and temporarily kernel-probed under native coordination; never created, modified,
unlinked or stolen. A race after the probe is adjudicated by upstream resume.
Bounded timeout reports stopped-but-still-owned or cancellation failure and never
starts a second writer. Resuming does not automatically reactivate a paused goal.
If complete discovery finds no current writer, ordinary takeover requires a free
native writer probe and resumes directly; it must not seek the original author or
stop a read-only cached copy. A force token without a current owner is rejected.

If the owner cannot stop/release, force takeover requires explicit `--force-server
PID:START` matching the freshly verified target owner; the UI instead requests an
explicit whole-server confirmation after showing all known affected loaded UUIDs
or clearly stating scope is unknown when the server is unresponsive. Force acts
on that process through a PID-stable kernel handle, first TERM then bounded KILL.
Unsupported PID-stable signalling fails; no kill-by-name/PID fallback, process-group
kill, unrelated-server termination or lock-file deletion. Server exit and native
writer release must both be confirmed before current-account resume. Other work
on that server may stop; this is never represented as thread-only force termination.
The UI permits cancel/return without changes. Non-TTY interactive forms fail usage;
explicit noninteractive actions retain original TTY/streams/signals/exit at exec.

The interactive helper is the convenience route for a cross-account writer conflict;
bare upstream launch/resume remains independent when Manager is absent. Manager
help and README explain this route rather than intercept upstream argv or add a
Core process supervisor. Qualification must reproduce running-disconnect,
interrupt/disconnect race, same-owner reconnect, goal cancellation, retained writer,
old-runtime owner, profile takeover, stale token, malformed protocol/substituted
records and whole-server force scope in owned roots; native runtime proof is
required beyond simulated RPC.
