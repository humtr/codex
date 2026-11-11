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
conversation UUID. Construction is allowed once per admitted launch/display request. Raw arguments,
CWD and caller environment travel in a bounded, private, single-consumption
local descriptor, never through notification actions; the descriptor is 0600,
at most 64 KiB, removed before workload execution and never logged. It may
temporarily contain caller authentication environment, while permanent
terminal bindings never do. A new terminal retains its own identity
variables. Return validates the live launch root and foreground binding
before selecting the registered name with an inert executable. Exact focus
never starts or resumes Codex; the separate verified headless display admission
below can create one observation/resume terminal for an existing thread. Stock API cannot atomically select an existing
window: manual rename or a closing-window race can create an inert empty
terminal. This is not equivalent to native existing-handle guarantees;
failure must not retry construction or change routes automatically.

A private process-bound terminal record contains only the process start
identity, native foreground FD identity and terminal route. It is mode 0600
under the private Manager terminal directory, bounded and checked against
live same-UID processes; it contains no conversation content, credentials
or launch arguments. Dead records are pruned lazily. A notification resolves
exactly one live foreground UUID. Missing, ambiguous, closed or recycled
targets refuse exact focus. Headless display must independently revalidate
Native ownership before construction. /new, /resume, /switch, profile changes and /title
cannot replace the originating terminal. Visible titles remain presentation.

Manager owns `notify set --focus window|termux`. Default is window;
`termux` selects Activity-only return. The private notifications/focus-v1
contains `window\n` or `termux\n`. The removed `tmux` setting
is invalid in both stored records and command-line input. Absent state uses
window; malformed/insecure state uses Activity-only return. For a canonical hook
session_id in window mode, Manager uses installed Core's safely quoted
private `__terminal-show-v1 UUID` action; no AI executable is needed.
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

Except for Core-owned `codex termux version`, `codex termux` is a Manager
boundary and is never passed to upstream. Manager
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

Core update selection and presentation, including the offline Core-owned
`codex termux version`, are defined in [UPDATE_DISPLAY](UPDATE_DISPLAY.md).

## Exact headless Native display

The existing terminal-directory lock serializes only durable descriptor admission
and the launch-to-consumed-to-route transitions. Android dispatch, route waiting,
focus and workload execution occur after releasing it. Concurrent click/automatic
requests may report an original launch as pending only after checking its exact
thread, program and display arguments, including execution-home device/inode.
Unresolved lock contention cannot claim that an original launch exists. Duplicate
starters cannot remove another starter's route; cleanup remains bound to the
descriptor actually consumed and the route's publishing PID/start identity.

Notification click and optional automatic display use the same Manager admission.
They validate the full thread UUID, original execution home and its device/inode,
qualified Native owner/socket/Writer, and any live terminal binding. Caller labels
or a matching visible title are not ownership evidence. A valid existing terminal
is selected; stale or ambiguous identity cannot select another account/thread.
An absent binding permits one existing Manager window construction after owner
validation. Live work attaches to its verified server; confirmed writer-free work
uses upstream exact-UUID resume, which fails for a missing thread and cannot
silently create a fresh conversation. Unknown ownership refuses construction.
Neither path sends a user prompt, starts inference or transfers a live Writer.
Managed display enters Core's qualified local `resume <UUID>` frontend route in
that exact execution home. The frontend revalidates the current Native Owner and
subscribes without embedded fallback when one exists. An explicit arbitrary
remote CLI route is not a substitute: it bypasses the qualified foreground-FD
producer needed for exact repeated-click selection. Owner uncertainty still
fails before attachment; only verified Writer absence permits saved-thread resume.
Before construction and again before reentry, Manager requires Core's exact
`__terminal-capabilities-v1 -- resume <UUID>` native-identity response. A missing
qualified foreground frontend fails without a launch descriptor or a new terminal.
An already valid binding remains focusable without constructing another frontend.

The private launch directory lock and deterministic per-thread display token
serialize notification/automatic races. Existing live routes are revalidated;
unconsumed or consumed pending requests block duplicate construction. Uncertain headless construction keeps its original pending descriptor even after
the requesting process departs; that departure does not prove Android rejected
the launch. A delayed service consumes only the original once. One-shot
launch descriptors are privately owned and consumed before execution. A closed
proven-dead terminal may be constructed again through the same admission; an
unresponsive or uncertain live route is preserved with a bounded error.

`notify set --auto-display on|off` controls optional admitted Bridge/AppHost turn
start display. It defaults off, is independent of notification delivery, and is
stored in private `notifications/auto-display-v1`. Disabled display does not query
owners or construct a terminal. An enabled automatic request uses the same verified existing-terminal focus
path as a manual click and never changes work acceptance/results on display failure.
A failed manual click may show a bounded non-sensitive error toast. Android
background/lock-screen restrictions remain authoritative; service acknowledgment
and a route receipt do not prove Activity visibility. Notification click remains
the user-initiated display path when automatic foregrounding is denied.
