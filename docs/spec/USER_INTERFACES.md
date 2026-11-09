# User-facing interfaces and optional Manager contracts

**Status:** Normative detail of [SPEC.md](../../SPEC.md).
These are feature contracts and acceptance requirements, not evidence of
deployment. Planned means an optional or requested capability still needs
implementation evidence; it does not create a second executor or bypass
upstream ownership.

## Native /switch behavior

Manager owns execution-profile selection within the upstream conversation UI.

Quota switching settings must present a compact, grouped interface. Each selectable
row occupies one terminal line even at narrow supported widths; explanatory help
must not become wrapped per-row descriptions. Keep all existing policy controls,
account enrollment, custom ranking and session override accessible. Clipping must
preserve the selected action/value and must not change the underlying account ID.
Verify actual rendering and navigation, not only the generated labels.

Quota-aware selection is an optional Manager/TUI capability. Core and the base
edition acquire no quota policy, account registry, polling or Manager dependency.
`codex termux usage [--refresh] [--json]` reads cached registered-account usage;
refresh uses each account's qualified native app-server account/model read methods
only, never a thread or inference request. A separate bounded profiles-v2 view
adds rows, freshness, selection reasons and rotation settings; v1 stays unchanged.
Usage caches are private XDG cache data, not authentication authority. Unknown,
stale, unavailable, API-key and changed account identities cannot be automatic
destinations. Two profiles for the same metered account are one allowance pool.
Windows are labelled by actual duration; reset timestamps do not prove recovery.
Ordinary automatic Codex destinations need both advertised5h and weekly windows;
an absent window stays unknown rather than implying unlimited usage. Manual
selection remains available. Model-specific buckets stay distinct from codex.

`codex termux rotation show|preview MODEL|set JSON|exclude PROFILE SECONDS` and /switch settings share Manager's
policy: off by default, explicitly enrolled account identities, a deterministic
5h/weekly/combined/remaining/custom ordering, reserves, proactive safe-boundary
threshold and cooldown. Custom values are data, never executable commands.
Removing, renaming or replacing an account does not enroll another identity.
The UI first shows cached rows, then bounded asynchronous refresh. Refresh replaces
the existing picker, retaining selection/search and leaving other modals intact;
one cancellation closes the picker. It exposes
refresh, pool membership, enable/disable, ranking and session override without
ordinary-launch network work or a resident refresh service. The separately
enabled window-start maintenance described below has its own Manager lifecycle;
opening the picker or enabling ordinary rotation does not enable that maintenance.

Automatic handoff requires a confirmed terminal UsageLimitExceeded or an opted-in
proactive threshold at an idle primary turn boundary. Transport errors and generic
429s never trigger it. Both the frontend and Manager revalidate the destination,
same model entitlement, source identity and existing writer release. Preserve
thread UUID, CWD, terminal, model/effort, permissions/reviewer and queued input.
No stop, kill, writer-lock removal, transcript copying or original-prompt replay
is an automatic fallback. Continuation needs an unambiguous settled durable
checkpoint; pending tools/questions/reviews or uncertain partial work pause with
an explicit reason. Switching accounts and successfully continuing work are
distinct visible outcomes. Attempts and per-thread handoffs are bounded and
serialized; exhausted destinations cool down and never ping-pong in a retry loop.
The new frontend uses private __profile-resume-v2 with bounded serialized native
session flags to preserve model/effort, configuration profile, permissions/reviewer
and explicit session options. A PID-bound comparison of the native resolved
permissions rejects incompatible destination definitions before a continuation;
account configuration and managed requirements are never copied or weakened.
Retained v1 callers remain supported unchanged.
Quota-failed continuation is one explicitly consented checkpoint-continuation
turn, not resubmission of the original prompt or retry of completed tools.
Proactive idle switches submit no prompt. A durable attempt claim precedes exec;
an uncertain crash pauses instead of replaying. At most3 automatic handoffs per
thread per hour are attempted, with the selected cooldown between attempts.

### Quota-window activation

This is a requested, not yet implemented Manager capability. Its outcome is that
an enabled account's renewed five-hour allowance has an actually started window.
A temporary chat, a literal `hi`, and even an additional inference are mechanisms
to minimize, not required product behavior. Existing usage refresh remains a
read-only account/model operation; it must never silently create a model turn.
Core/Base acquire no quota scheduling, account policy or Manager dependency.

- A separate /switch setting controls window-start maintenance independently of
  automatic account switching. The owner selects account identities; multiple
  profiles of the same account share one operation for each metered bucket/cycle.
  An enabled account is handled at every applicable reset even when it is not
  selected for foreground work or the picker is closed. Replacing an account
  does not transfer enrollment to the replacement identity.
- Discover actual window duration, bucket, account identity and reset data through
  the installed Native account interface. Watch the advertised five-hour window,
  not a fixed five-hour timer started at login or application launch. No advertised
  five-hour window means no fabricated window or probe: distinguish unknown from
  positively established non-applicability. Reading a reset timestamp that is now
  in the past is not evidence that allowance recovered or a new window started.
- Prefer, in order: a window already started by normal account activity; a
  documented and qualified non-inference operation that really starts that same
  allowance window; and only when needed one minimal native model request using
  the target account and metered bucket. Qualification must test whether ordinary
  reads or thread initialization have the desired effect before adding inference.
  No dedicated non-inference window-start operation is currently established.
  Do not invent an API, redeem reset credits or purchase usage for this feature.
- When inference is necessary, use the smallest qualified isolated invocation.
  Native ephemeral execution is a candidate, not a requirement to fork a user
  chat. Keep user conversation history, active workspace, foreground account,
  model/effort and working-thread writer untouched. The maintenance model/effort
  comes from explicit configuration or verified account capabilities; a cheaper
  model is useful only if it advances the intended bucket. Do not route through
  ChatGPT Provider reasoning and assume it starts the Codex allowance window.
  The maintenance invocation has no project tools/MCP or project instructions;
  qualify the actual invocation surface instead of assuming a sandbox name
  provides Android isolation. This does not narrow ordinary development tools.
  Qualify suppression of ordinary completion alerts for the internal maintenance
  request without suppressing the user's foreground/background work notifications.
- Manager owns one optional deadline-driven worker, supervised through the
  existing selected service lifecycle. UI displays settings and observed results;
  neither TUI, Provider nor the generic supervisor owns quota decisions. Reuse
  qualified Native notifications where available and coalesce reset deadlines.
  Admit due accounts with bounded concurrency instead of spawning every account
  at once; an existing foreground request can satisfy the goal first.
  No per-account permanent process, two-second poll, or message loop is required.
  Restart/reconnection catches up due accounts after fresh identity/window reads.
  Android termination, an offline device or unavailable authority cannot be
  described as successful on-time execution; expose the delayed/pending reason.
- Reuse existing account/home binding, private state and operation serialization.
  Claim an account/bucket/cycle before a possible submission; the state is consumed
  by restart recovery and duplicate suppression, not a new conversation ledger.
  A foreground request that already starts the window satisfies the same goal.
  Recheck the server's actual window after any maintenance operation: dispatch
  acceptance, a successful model response, 100% remaining, or a locally predicted
  timestamp alone is not a started-window result. Unavailable window evidence
  remains unverified rather than causing repeated prompts. Definitely unsubmitted
  failures can resume on supported recovery/backoff; ambiguous submissions must
  be reconciled before another possible effect. Generic429 does not trigger
  account rotation or an unbounded retry loop.

Qualification requires a controlled comparison of existing activity, read-only
queries, thread-only initialization and a minimal native request; select the
least costly path that demonstrably starts the same account's actual window.
Use injected clock/failure tests for duplicate profiles, simultaneous activity,
restart and unknown outcomes, then actual account/window and Android process/
history observations. Repeated reset cycles need recurring on-device proof;
synthetic timestamps or a single `hi` response cannot qualify the whole feature.
The evidence baseline is the [official rate-limit interface](https://learn.chatgpt.com/docs/app-server#6-rate-limits-chatgpt)
and [current plan limits](https://learn.chatgpt.com/docs/pricing#what-are-the-usage-limits-for-my-plan).
Neither source establishes a guarantee that a short message always rearms a
five-hour window. A CLI option advertised as ephemeral does not itself establish an actual
account allowance-window start; metering and event semantics require real
product-path qualification.

### Clipboard and image-input behavior

Preserve upstream /copy, its available copy-picker targets, Ctrl+O, selection
copy and /config copy/paste preferences. Upstream owns the logical content and
composer; a Termux clipboard adapter is justified only by an actually missing
platform operation. A clipboard feature must not move conversation UI into Core
or make ordinary upstream launch depend on Manager.

Preserve the distinction between the /copy picker (whole response, available
code/quote/status targets and upstream transcript selection), Ctrl+O's native
last-response action, a Termux extra-key macro and Android terminal-selection
copy. A button sending Ctrl+O does not invoke /copy's picker. A bounded picker
preview must never bound the copied source. Restore a missing native picker at
its actual dispatch/source owner rather than replacing it with unconditional
last-answer copying or a private command.

Capacity is measured in UTF-8 bytes at the effective upstream transport,
not as a fixed character quota. The native terminal and Android receiver must
support the upstream-specified full OSC52 raw limit after Base64 framing,
without a smaller encoded buffer, character-based truncation or a confusing
KiB/decimal-byte conversion. The upstream cap and terminal receiver capacity
are separately owned limits; neither implies a fixed count of Korean characters.

An unconfirmed clipboard-write outcome must remain distinct from
payload capacity. Upstream maps a terminal-only Requested outcome to Unconfirmed
because OSC52 write success carries no Android acknowledgement. Growing the
receiver buffer or implementing image reads alone cannot change that fact.
Prefer the existing source-only Termux terminal-control clipboard commands for
the necessary Android boundary, after source and protocol review. A qualified
Codex consumer maps only its matching successful clipboard-write-text response
(correct schema/operation/result and confirmed:true) to the existing confirmed
outcome. Android confirmation means its clipboard write succeeded; receiving
the actual pasted content remains a separate acceptance observation. Invalid,
failed, timed-out or unknown responses must not become successful copies.
Retain ordinary upstream OSC52 behavior when no confirmed backend is available;
report that path as unconfirmed, not as a repaired confirmed path. An unknown
write outcome must not cause a blind delayed second write over a newer clipboard.

The Termux side owns terminal ancestry/peer checks, foreground Android access,
OSC parsing and clipboard effects.
Private input admission identifies the app's files/cache directories by their
kernel device/inode identity, including Android's bind-mounted path aliases;
literal canonical-path prefixes must not reject the same private directory.
External files and symlinks escaping the private roots remain rejected.
The TUI owns source/format selection, asynchronous operation ownership and
feedback. Reuse the existing worker and
native outcomes, including cancellation before delivery, busy state and stale
completion handling. No extra daemon, global clipboard polling, X11 dependency,
Core clipboard UI or duplicate text-copy path is justified by this feature.
Validate actual required operations at capability discovery rather than an
exact historical aggregate operations string: the clipboard extension must not
disable existing origin/validate/focus consumers. Preserve schema, peer identity,
live-origin and foreground checks; capability advertisement is not authorization.

Qualify the actual composition. An unmodified upstream Base runtime can use the
expanded terminal OSC52 receiver but still lacks the new confirmed-write/image
consumer. The maintained TUI consumer must work without a Manager dependency;
neither adding Manager nor activating the APK alone supplies that consumer.
Report supported capacity, acknowledged text copy and image input separately
for the installed pair rather than silently declare every composition fixed.

Copy the selected logical source/Markdown according to the upstream copy format,
not a rendered terminal viewport. Soft wrapping introduces no extra spaces or
newlines; intentional paragraphs, code indentation, Unicode and selected code
blocks remain intact. Collapsed display does not truncate copied source. Verify
Base and enhanced TUI, ordinary Termux and supported native window paths, native clipboard/
OSC52 routing, failure/cancel behavior and actual Android clipboard readback or
paste. A successful helper exit or emitted terminal sequence alone does not
establish that Android received the full content. Manual qualification prompts
retain the existing automatic clipboard attempt, exact-text readback and separate
one-step fallback; that workflow is distinct from /copy product acceptance.

Required boundary proof includes99,999/100,000/100,001 raw-byte OSC52 requests,
Base64/framing and fragmented BEL/ST input, malformed UTF-8/Base64, ASCII and
mixed Unicode, and the Termux candidate's separate102,400-byte boundary. Keep
upstream rejection beyond its OSC52 limit; do not impose it on a different native
backend. Verify actual /copy whole-response and code-block choices, Ctrl+O,
terminal selection and configured extra keys. Inject write failure and invalid/
missing confirmation: no false success or clipboard truncation is acceptable.
Source tests, complete APK build, Codex consumer build, installed pair and actual
Android clipboard/paste are distinct gates. Preserve existing installed APK,
package/signing identity, origin/focus behavior and rollback; installing an APK
can terminate the hosting session and needs its own controlled activation step.

Image paste requires review/qualification of the existing app-side candidate
and implementation of its missing Codex consumer. Reuse upstream
image composition, attachment and model submission; inspect the actual deployed
target/clipboard backend, Android clipboard MIME/content-URI access, foreground
permissions and any needed Termux companion. Text clipboard support or a build
on an Android host does not prove image clipboard access. Prefer a minimal
platform adapter over a second attachment pipeline or required Linux desktop.
Do not assume rebuilding Full alone supplies Android clipboard images.

There is already an unqualified app-side candidate in humtr/termux:
the clipboard-read-image, clipboard-write-text and clipboard-release-image
operations of termux-terminal-v1 over its existing authenticated AM socket.
It provides image content-URI snapshots and confirmed
text writes; Codex consumer integration remains missing. Audit/reuse that source
instead of inventing another bridge. A clipboard-read-image snapshot is not yet
a durable Native attachment: the TUI must validate and import it into the existing
image-input lifetime before releasing the Termux token, including failure/cancel
cleanup. Review claimed transfer time/size bounds, real URI permissions, foreground
and process ancestry on the Android path; a source declaration is not proof.
An elapsed-time check after a blocking URI open/read does not establish a bounded
operation. Qualify cancellation and cleanup at the provider I/O boundary, not
only the immediate-return fixture or a longer caller timeout.

Direct clipboard-image paste, image-file attachment and Android share/picker
input have separate acceptance results. Qualify image bytes reaching the existing
composer/native request, text-plus-image, multiple images, model capabilities,
cancel/retry and supported resume, without changing the selected model/effort or
silently losing/resizing content. Keep image data needed by an active/restorable
thread; cleanup follows the actual persisted-image consumer, not a temporary-file
name. Clearly report unsupported clipboard paths and available file/share paths;
one must not be counted as proof of the other. Missing platform support does not
hold the urgent restoration release after the supported text-copy path passes.

The native frontend source has two owners: pristine upstream files receive only
the maintained integration patch; standalone TUI implementations live as
ordinary source under frontend/tui/src. The single maintained source
assembler supplies the patch and owned payloads to the common Rust producer's
single staged merge. Owned input reads, path validation and collisions are
resolved before checkout mutation; an owned source never overwrites an upstream
file. Qualification binds both inputs, the assembler and tests
to the exact successful source. Final build inputs and artifact proof still
include the complete assembled TUI diff; no source is downloaded or modified at
device launch. This source organization does not change public behavior or relax
the upstream-Core/dependency, executable, lint or native behavior gates.
Common Native source preparation belongs to release-builder and is consumed by
both runtime and frontend builds. The enhanced frontend requests
`prepare-source --frontend provided` so the common implementation is applied
once and its own TUI supplies presentation. The assembler consumes that exact
preparation's declared source paths and preserves them during three-way merging.
The upstream start function delegates resolved-catalog binding to the owned
frontend adapter. Native owns provider-transition admission, persistence and
observer continuity; App sends user intent and renders the resulting settings.
Provider/profile configuration composition does not acquire execution or server
lifecycle authority. Existing permission/reviewer inheritance, local profile
precedence, installed-route validation and model admission remain unchanged.

- The added public slash names are `/switch`, for Manager execution-home
  selection, and `/provider`, for the independent TUI's provider/model chooser.
  `/profile` is not an alias. Upstream `--profile`/`-p` configuration
  layers, `/resume`, `/agents`, `/permissions` and every existing slash name/alias
  retain upstream meaning. `/provider` first selects a provider, then its model
  and reasoning level through the same Native admission as `/model`. Opening or
  cancelling its picker cannot change the active route or submit inference.
  Before applying the native patch, the source build must inspect the pristine
  pinned upstream command inventory, including serialize/to_string aliases, and
  refuse a `/switch` or `/provider` collision or an unrecognized naming
  representation. After patching, every upstream variant/name/alias must remain
  identical; Switch and Provider must be the exact added variants/names.
  This gate is mandatory in the real hosted
  frontend build. A future collision requires an explicit contract decision;
  silently replacing an upstream command or choosing another name is forbidden.
- Upstream owns history, authentication, thread ancestry and kernel writer locks;
  Manager owns execution-profile selection and task policy. Core gains no UI,
  transcript parser, second session index or account-switch controller.
- Same-terminal transition must explicitly detach the old native client, preserve
  or stop work according to the selected action, confirm writer release before
  same-thread account takeover, and re-enter Core with the selected child home.
  Selecting a profile must never silently stop work, remove locks, share auth
  between agents or create additional Android terminals.
- The profile-selection slice makes arrow selection followed by Enter directly
  switch the current idle conversation to the selected registered profile. There
  is no intermediate history/agents/new-conversation menu. Selecting the current
  profile closes the picker without re-entry; Esc/Ctrl-C likewise cancels.
  Re-entry requires an idle primary session with no queued input, pending steers
  or running local commands, unsent composer input or pending native tool calls.
  A failed profile/Core revalidation leaves the picker and current chat usable. After ordinary native client cleanup and terminal
  restoration, the frontend execs Core with fixed private Manager argv
  `termux __profile-resume-v1 PROFILE_ID THREAD_UUID`. This exact command validates
  the registered destination and canonical UUID, holds the existing Manager
  registry shared lock against rename/delete during handoff, waits at most3 seconds
  for the existing kernel coordination/writer locks to become free, then execs ordinary
  Core resume in the selected home and same terminal/CWD. It never interrupts,
  pauses goals, cleans background terminals, signals an owner or removes locks.
  Before cleanup, the frontend verifies the current native rollout exists as a
  regular file. For an unmaterialized blank conversation it requests upstream
  thread/read(includeTurns=true), which persists before history hydration; only
  positive rollout materialization permits re-entry. The sole tolerated request
  failure is the qualified backend's exact -32601 list_turns-not-supported response
  after persistence; transport, decoding and other server failures refuse re-entry. No synthetic turn, transcript write,
  archive/unarchive or metadata rewrite is permitted. Metadata lookup or failure
  to materialize within3 seconds leaves the current picker/chat alive. Existing
  stored history uses metadata-only lookup and is never loaded by this check.
  Re-entry sets the child working directory to the current App workspace, including
  an upstream /cd change, rather than inheriting the original process directory.
  Another connection or newly acquired writer causes bounded refusal; native
  resume still owns the final atomic writer acquisition. A profile disappearing
  after frontend cleanup likewise causes bounded refusal. Saved default, auth,
  conversation history and unrelated jobs remain unchanged. Automated transition
  proof uses disposable owned accounts.
- The independent idle-profile admission preparation adds explicit source return
  after frontend cleanup: on an operational re-entry refusal, the private
  __profile-resume-v1 Manager path may offer Enter to return to the original
  execution home and q/other input/EOF to exit only when all three standard streams
  are terminals and a valid explicit inherited CODEX_HOME identifies that source.
  Manager binds the source directory's open physical identity before attempting
  the destination, then revalidates identity and private ownership/mode immediately
  before ordinary same-thread Core resume. No saved-default lookup or destination
  substitution determines the return account. Missing, unsafe or replaced source
  refuses; noninteractive/invalid-grammar invocations retain prior status/output.
  Successful switching never prompts. Return preserves current CWD, UUID, TTY,
  original Core exit behavior and existing writer authority; it never cancels work,
  forces takeover, removes a lock or recreates a profile. Input is bounded to128
  bytes, only an actual empty newline confirms return, and EOF never confirms.
  Exit/cancel has status130.
- The generation frontend reproduces the accepted Termux built-in permission
  menu/shortcut behavior: Ask and Approve for me use the no-sandbox profile,
  Full Access remains explicit, unsupported Read Only is omitted, and current
  markers/descriptions follow actual settings. This frontend adaptation is enabled
  only for the explicit absolute CODEX_PROFILE_CORE bridge; ordinary
  unselected upstream UI and custom profile definitions retain their semantics.
  Core still rejects explicit unsupported sandbox argv before runtime entry.
  Actual menu selections/shortcuts and native settings require owned proof.
- The native frontend reads profile data through exact no-argument
  `codex termux __profile-snapshot-v1`. Manager alone resolves registered profiles,
  inherited/current selection and saved default. Output is one bounded JSON v1
  record with schema, profiles (default first, at most256), current (registered ID
  or null for an external inherited home), current_source and saved_default.
  It contains no paths, credentials or conversation data and performs no writes.
  Existing public profile text output and inherited-home precedence stay intact.
- Native display reads this endpoint using an explicitly provided
  absolute `CODEX_PROFILE_CORE` bridge. The native client validates
  regular executable ownership, uses fixed argv without a shell, limits stdout to
  64KiB and elapsed time to3 seconds, and rejects malformed/unknown protocol data.
  Failure is bounded in the current UI; cancellation returns to the current chat.
- Integrated native views read observed current writers through exact no-argument
  `codex termux __task-snapshot-v1`. Its one JSON record uses schema
  codex-manager-tasks-v1 and tasks sorted by canonical thread UUID, at most1024.
  Each record contains id, owner_profile (registered ID or null for a qualified
  external execution home), state (active/idle/systemError/notLoaded), and
  server_token (the qualified current server's PID:start identity). Existing
  server qualification and kernel writer-lock discovery alone determine ownership;
  a creator, history author, selected profile or loaded read-only copy is not an
  owner. Output is bounded to256KiB and has no paths, credentials or transcripts;
  this command performs no writes and never stops or starts work.
- Native task reads use the same qualified explicit Core bridge, fixed argv,
  256KiB stdout and12-second total bound (existing Manager scan deadline10 seconds).
  Execution-profile context is distinct from a confirmed writer's profile.
  A null owner_profile means a confirmed external writer; missing task data or a
  failed query is unconfirmed/unavailable, never evidence of an unlocked thread.
  Observations are transient; later actions must revalidate actual owner identity.
  Native history/agent navigation reuses upstream lists, ancestry and UI lifecycle
  without a second history index or new persistent state. Cancellation/error must
  leave the current chat usable.
- Before the integrated frontend prepares a backend for resume/fork, the private
  `codex termux __task-connection-v1 THREAD_UUID` is owned by Core and uses the
  shared runtime-binding Native process/socket/writer observation implementation.
  Manager task views consume that same observation; their profile labels, Native
  RPC presentation and explicit task actions remain optional. This discovery and
  canonical execution-home handoff must work when Manager is unavailable. They
  observe Native ownership and never authorize or transfer a writer. Its bounded JSON record has schema
  `codex-manager-task-connection-v1` (retained for the already built frontend
  consumer) and connection=null only when the writer is
  confirmed free; otherwise connection contains execution_home and socket for
  the qualified owner. This local connection metadata is not a public task list,
  credential, new writer registry or execution capability. Unsafe, ambiguous or
  unresponsive ownership fails rather than preparing a competing writer. A
  kernel-verified unrelated server/read-only copy is not a connection prerequisite
  for that selected task; unfiltered inventory keeps its complete observation scope.
  The frontend connects that owner without applying catalog startup selection to
  it. New/ownerless work retains the resolved configuration coordinator; a generic
  connection operation never replaces an already selected backend. The owner's
  execution home governs attachment. If it differs from the invocation context,
  reuse the qualified execution-profile/context handoff while preserving the
  selected thread, explicit invocation intent and unsent input. Do not change a
  global default, substitute credentials or connect with another home's local
  configuration. An unqualified context rebind remains an explicit incomplete
  boundary, not a permanent requirement that the user manually choose profiles.
  For a canonical resume/fork identity known before UI startup, the private
  `codex termux __task-reentry-v1 THREAD_UUID -- UPSTREAM_ARGV...` uses that same
  Core-owned verified connection and qualified execution context. It reexecutes the same
  qualified Core in that owner's home with the original opaque upstream arguments,
  before any editable startup draft. It starts no separate writer and changes no
  default, permission or inference selection. The frontend removes only Core's
  verified injected rendezvous argument, not user arguments. Picker/alias flows
  must also preserve already collected drafts when their later identity requires
  a context rebind; canonical startup proof alone does not qualify those flows.
  Loaded Native resume can ignore construction overrides while another client
  observes the thread; this protects its original writer. An explicit local TUI
  inference selection records model, provider and effort independently; unspecified
  fields retain the Native selection. It uses the existing thread/settings/update admission
  and rereads effective Native settings when resume has not applied that selection.
  Apply only requested model/effort on the same admitted provider; defaults and
  permissions are not another selection. Unsupported or rejected admission keeps
  the original task/input and reports the actual cause. Do not rebuild/stop a live
  writer to force startup flags, or treat UI configuration as Native acceptance.
  Attachment to a verified live Native work has no new local OpenAI onboarding
  precondition. Native retains credential, model and policy enforcement for that
  work; TUI login controls remain available. Configured new work keeps its selected
  provider's onboarding policy. No credentials or account defaults are replaced.
  Native still admits attachment/writes and detects races after this observation.
  This is an optional integrated-frontend contract; Core's ordinary independent
  runtime launch does not acquire a Manager dependency.
### Independent terminal return (Manager)

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

### Bare interactive launch update discovery

The exact zero-argument bare `codex` launch may perform update discovery only
when stdin, stdout, and stderr are all terminals. Any non-empty argument vector,
including `update`, `doctor`, `termux`, `exec`, `--version`, `-V`, and
all other upstream commands/options, bypasses this discovery path completely.
A bare launch with any non-terminal standard stream also bypasses it completely.
Those bypasses perform no startup-update network request, prompt, or advisory
state write.

Discovery is advisory and never becomes update authority. It authenticates only
the normal signed stable index pair under the already installed official
`update_key` and compares the signed generation identity with the active
generation. It does not fetch release payloads, stage a candidate, probe a
candidate, change activation state, or execute an untrusted candidate. The
startup fetch is separately bounded to at most two seconds per signed-index
resource. Discovery failures, malformed advisory cache state, clock failure, or
unavailable transport fail open to the existing active upstream runtime without
a user-visible error; the explicit `codex update` command retains its existing
fail-closed update semantics.

A successful startup discovery is cached for six hours. Transport or discovery
failure suppresses another startup network attempt for thirty minutes. When a
different signed generation is available, Core may show one transient all-TTY
prompt:

```text
Codex update available. Update now? [y/N] 5s
```

The prompt names no candidate version before authenticated generation metadata
is available through the normal update path. On Linux/Android TTYs, Core enters
a bounded non-canonical, no-echo prompt mode only for this advisory: a single
`y` or `Y` key selects update immediately without Enter, while `n`/`N`, Enter,
or any other single input byte keeps the current runtime immediately. Five
seconds with no input also keeps the current runtime. Core restores the exact
prior terminal mode before update execution or upstream launch, consumes the
prompt byte so it cannot leak into upstream Codex input, and clears the
countdown line in place on every normal exit. A keep/timeout decision snoozes
that exact signed generation identity for six hours; a different signed
generation is not suppressed by the old snooze. An effective rollback hold or
rollback-Core guard suppresses the startup prompt entirely; explicit
`codex update` / `--force` remains the recovery authority.

Selecting update re-enters the ordinary no-argument `codex update` path from
the beginning, including fresh signed-index authentication and all normal
signature, digest, mode, anti-rollback, candidate-probe, atomic-activation, LKG,
and rollback protections. On success, bare launch then loads and executes the
newly active upstream runtime. If that explicit-in-response-to-the-prompt update
fails, its ordinary concise failure remains visible but bare launch still
continues with the unchanged active runtime. Startup advisory cache/snooze state
is Core-owned convenience state only and is never trusted as release,
activation, rollback, or signing authority.

Rollback is an explicit Core operation, not an ordinary-launch fallback and not
a search through generation history. It selects the one retained previous complete
generation, including explicit reactivation of a newer previous generation.
After an authenticated public rollback to a lower release sequence commits, Core
atomically records a separate bounded exact-format `update-hold` file containing
only the authenticated generation identity and signed release sequence that were
current before rollback. Activation-state v3 is not extended. The hold file must
be a regular file, must reject malformed/oversized/symlink state, and must use
temporary-create, file sync, atomic rename, and parent-directory sync semantics.
A failed rollback must not create or change the hold. Explicit selection of an
equal or newer previous sequence creates no hold for the older generation and
clears obsolete hold/guard controls only after the existing activation transaction
commits; an existing hold for that selected sequence remains authenticated while
its rollback guard is removed. It never weakens qualification of that retained target.

The first rollback from a hold-aware Core to an older Core that does not advertise
exact `codex-update-hold-v1` capability must not lose hold enforcement. Core may
therefore retain the rolled-back-from signed Core launcher as a bounded rollback
control-plane guard while the runtime, Manager, helpers, and authoritative current
pointer roll back to the retained previous signed generation. The guard is a
separate exact-format Core-owned state record; it must bind the rollback target
generation, the held generation, the held signed release sequence, and the held
generation's signed Core digest. The stable launcher is accepted under this
exception only when the authoritative pointer pair matches the guard, the held
previous generation verifies under its retained verifier key, and the launcher
digest exactly equals that held signed Core digest. No unsigned or arbitrary Core
path is authorized. A target Core that advertises `codex-update-hold-v1` uses the
normal full Core rollback instead and does not retain the guard.

Core writes the guard intent durably before the pointer swap but treats it as an
effective hold only after the authoritative pointer becomes the exact guarded
rollback pair. This closes the commit-to-hold crash window without treating a
failed rollback as held. After the committed rollback, Core writes the normal
`update-hold`; a hold-aware target then removes the temporary guard, while a
legacy target retains it until a force retry or greater-sequence activation
restores a normal Core/generation pairing. Malformed, mismatched, stale, or
unverifiable guard state fails closed whenever it is needed to justify a launcher
mismatch.

Ordinary update applies signature, digest, and release-sequence validation before
the hold. After rollback, an otherwise eligible candidate that does not exceed the
held signed release sequence must not stage, probe, or activate; the hold therefore
prevents both the exact bad sequence and any intermediate sequence from displacing
the retained evidence. A differently named same-sequence repack cannot bypass it.
A greater release sequence is eligible normally. After a greater-sequence
activation commits, Core removes the now-obsolete hold and guard while still
inside the activation writer boundary; a failed activation must not clear them.
`codex update --force` requires a valid authenticated hold and may retry exactly
that held sequence for that invocation. It bypasses only the hold comparison; a
same-held-sequence force retry retains the normal hold while removing any now-stale
rollback-Core guard after the normal Core/generation pairing is restored. Force
never enters the transport-unavailable local-build fallback and cannot target an
unheld or greater sequence. Neither path weakens signature, digest, anti-rollback,
candidate-probe, or atomic-activation rules.

Internal release IDs, component digests, API versions, and schema versions are
still mandatory for update, diagnosis, and rollback. They may appear only on a
clearly Termux-specific status or redacted machine-diagnostic surface.
## Manager contracts

This section defines the optional Manager. It does not weaken or
extend the Core completion threshold, and it does not make Manager a
prerequisite for ordinary upstream launch, Core doctor, update, rollback, or
fresh installation. Manager is an optional, separately qualified artifact
behind the existing `codex termux` boundary.

### Process and ownership boundary

Core selects Manager only from the signed, qualified generation and invokes
the artifact with a versioned handoff environment:

```text
CODEX_TERMUX_CORE_API=codex-manager-core-v1
CODEX_TERMUX_CORE_ENTRYPOINT=<validated stable Core entrypoint>
```

The retired `CODEX_TERMUX_CORE_REQUEST` and
`CODEX_TERMUX_CORE_OPERATION` environment values have no dispatch meaning.
Core does not parse or consume a repair request, even if those retired values
are inherited. Ordinary doctor/update/rollback/upstream argv retain their
normal behavior. There is no replacement repair request or hidden recovery API.

Manager validates the versioned API and Core entrypoint before doing work.
Its return path to Core is an `exec` of that entrypoint with ordinary upstream
argv whose first token is not the exact selector `termux`, `doctor`, or
`update`. Recovery uses the public Core commands directly. Manager cannot
address a generation path, activation state,
trust key, resolver, or journal directly. Core remains the final validator of
every requested route. There is no callback socket, network protocol, or
second state authority.

Manager receives no credential or session-content payload from Core. It may
inherit ordinary process environment needed for a child launch, but it must
not print or persist that environment. A missing, malformed, or incompatible
handoff fails before any Manager state mutation.

### Profile selection and isolated launch

Profile commands use this public grammar:

```text
codex termux profile list
codex termux profile current
codex termux profile create <PROFILE_ID>
codex termux profile delete <PROFILE_ID>
codex termux profile rename <PROFILE_ID> <NEW_PROFILE_ID>
codex termux profile default [PROFILE_ID]
codex termux profile use <PROFILE_ID> [--] [UPSTREAM_ARGS...]
```

`default` is the existing upstream default identity home. A custom profile uses
the derived path `~/.local/share/codex/manager/profiles/<PROFILE_ID>/home`.
Profiles are execution identities, not conversation owners: authentication,
user configuration, MCP/plugin credentials, model/provider caches, logs, and
shell/runtime snapshots remain profile-local, while local conversation state is
user-global beneath the canonical shared-state root `$HOME/.codex`.

`profile create` creates only the private profile directory, an empty private
account home and Manager metadata. It never creates canonical conversation
directories or copies/parses credentials or transcript bytes. Registration is
valid before the first launch; list/use validate the private home and exact
metadata rather than requiring prepared shared links. Core alone prepares and
validates execution topology before upstream runs, including direct supported
CODEX_HOME launches without Manager. Existing complete homes remain supported.
The compatibility topology links these profile-home names to their exact
canonical siblings beneath `$HOME/.codex`: `sessions`, `archived_sessions`,
`session_index.jsonl`, `thread-writer-locks`, `rollout-migrations`, `memories`,
`memories_v2`, `history.jsonl`, `installation_id`, and
`tui-thread-reference-capabilities`. Directory targets are real private
directories beneath the shared root; file links may initially be dangling only
to the exact canonical sibling so upstream can create the target atomically.
Core accepts missing entries or empty real directories for preparation and
rejects substituted, relative, chained or otherwise unexpected shared-state
links and nonempty conflicting entries rather than repairing/importing them.
Manager's accepted dotted profile IDs are also recognized by Core; declared
external homes retain their already-supported underscore/hyphen identifiers.
Existing legacy profile directories are not imported implicitly. Legacy
consolidation is a separate, one-time bounded compatibility migration with a
verified restorable backup and conflict checks. For each legacy user profile,
the migration selects at most the five most-recent distinct conversations by
last activity. Selection must be deterministic; if the legacy state does not
provide enough trustworthy information to establish that order, migration for
that profile fails closed rather than guessing. For each selected conversation,
the migration may inspect legacy state only as needed to normalize and carry the
dependency closure required for faithful current-upstream resume/history
behavior. Proven-identical copies of one conversation/thread identity may be
deduplicated; divergent state for the same identity is a conflict and must not
be overwritten automatically. Whole-SQLite-file replacement, blind `INSERT OR
REPLACE`, and persistent generic legacy-merger behavior are prohibited.
Non-selected legacy conversations remain unchanged in the verified backup or
source profile and are outside canonical import scope.

The bounded migration implementation must remain outside steady-state Manager/Core
session semantics. Its planning phase may read legacy SQLite read-only only to select
the bounded recoverable set, preserve explicit names, and reject unsupported
thread-scoped dependencies. It must not merge or write SQLite. The migration payload is
the selected authoritative rollout JSONL, verified by thread identity and content hash.
Upstream `thread/list` owns metadata read-repair, `thread/resume` owns paginated-history
materialization, and `thread/name/set` owns explicit-name restoration. Rebuildable
`thread_history_1.sqlite` projections and the dedicated diagnostic `logs_2.sqlite` store
are not migration payloads. A selected canonical rollout that is only a legacy-profile
symlink must be normalized to an identical regular canonical rollout before legacy
profiles can be retired.

Before upstream activation, an apply journal may roll back only rollout files or alias
normalization created by that apply. Once upstream activation begins and upstream may
have mutated canonical SQLite, partial payload rollback is prohibited; rollback becomes
restoration of the separately verified full canonical backup. SCS-6 therefore requires
writer quiescence plus a restorable full backup before live apply/finalize.

The steady-state architecture is deliberately thinner than that migration
exception. Manager profiles are execution identities; conversation persistence
and thread/session schema semantics belong to the official upstream runtime.
Manager's normal responsibility is profile registration/selection, bounded
profile-local environment/configuration and `exec` through the validated stable
Core entrypoint. Core owns physical shared execution-path preparation; upstream
owns the conversation store. Once
migration is complete, ordinary Manager operation must not routinely parse
SQLite or transcript contents, infer conversation ownership, or maintain a
parallel thread/session store or index. Introducing such behavior is an
architecture-regression signal unless an explicit, narrowly bounded
upstream-compatibility exception is added to this specification with executable
proof that the upstream-supported surfaces are insufficient.

`profile list` emits `default` followed by valid custom profile IDs, one per
line, in deterministic bytewise order. It ignores malformed profile records and
validates private profile registration rather than treating arbitrary
symlinks as profiles. `profile create <PROFILE_ID>` emits exactly
`created: <PROFILE_ID>` followed by one LF after the create-new transaction
commits; it emits no path, environment, credential, or upstream-state detail.
`profile current` emits exactly two LF-terminated lines, `current: <TARGET>`
followed by `source: <SOURCE>`. `<TARGET>` is `default`, a valid custom profile
ID, or `external`; `<SOURCE>` is `inherited` when the caller supplied a nonempty
UTF-8 `CODEX_HOME`, `saved` for a valid explicit preference, or `default` otherwise.
Empty and non-UTF-8 values provide no inherited selection; a saved preference
still applies, with native upstream default-home interpretation when it is absent.
Current reports the account home a
fresh ordinary launch would use, not an earlier child selection. Known external
homes are reported as `external`, without treating them as Manager registrations. It never reports auth identity,
token state, session bodies, paths, or arbitrary environment values.

`profile use` requires an existing profile, validates its private registration,
then `exec`s Core without selection-history writes and emits no
Manager-owned success output before Core runs. For `default` it explicitly sets
`CODEX_HOME` to `$HOME/.codex`, bypassing any persistent custom default; for a custom profile it sets `CODEX_HOME` to the validated
profile home. Manager removes an inherited `CODEX_SQLITE_HOME` override instead
of producing a parallel shared-store selection signal. Core derives the shared
root from the supported execution home and supplies a system `requirements.toml`
requirement that fixes `sqlite_home` to the same
canonical shared root and requires `local_thread_store_compression=false` and
`background_paginated_rollout_migration=false` where the selected
upstream execution topology still relies on incompatible profile-local
rollout maintenance locks. The requirements
file is Core-owned, is removed or replaced only through the Core marker
contract, and prevents a profile `config.toml` from silently splitting SQLite
or enabling those incompatible background writers. The original upstream argv
after the profile selector is preserved exactly. If registration validation fails,
Core is not launched. If execution topology is unsafe or conflicting, Core rejects
before upstream execution. A selection changes neither later ordinary launches nor
the caller's environment. Core uses the same native-equivalent home interpretation
for shared preparation and server startup, preserving the supplied environment.

The profile lifecycle contract below applies. Interactive selection is the
existing launch/TUI convenience, not an auth-migration or conversation-copy feature.
Cross-profile session copying is deliberately absent
because conversations are shared objects rather than profile-owned objects. A
missing/invalid registration is a non-mutating Manager validation failure. Core
prepares only the declared missing/empty compatibility entries during launch;
conflicting nonempty state is never migrated or repaired as a launch side effect.

### Explicit profile lifecycle and launch default

The profile commands extend execution-account convenience; they never own or
remove installation-wide conversation history. `profile default` prints exactly
`default: <TARGET>\n`. With a target it validates an existing registration (or
native `default`), atomically saves the explicit preference and prints the same
line. This is not selection history: `profile use` never writes it. Preference
format is exactly `codex-manager-default-profile-v1\nprofile\t<TARGET>\n`,
private mode0600 under Manager's private state root. Missing preference means
native default. Invalid/missing-target preferences never follow an untrusted path:
explicit default/current queries and lifecycle mutations fail; Manager's ordinary
launch preparation returns blocked instead of silently choosing native default.

A nonempty UTF-8 inherited CODEX_HOME always wins. Otherwise Manager resolves its
saved preference through the bounded `__prepare-launch-v1`/`__execute-launch-v1`
contract. Core never reads the preference filename or profile metadata. Manager
validates registration, sets CODEX_HOME for this child and execs stable Core with
original argv/TTY/streams/signals/exit behavior. `__profile-launch` remains an
internal Manager endpoint, not Core's preference-discovery mechanism. Unavailable
optional preparation leaves normal Core launch usable in its unchanged confirmed
home; there is no post-handoff retry. Core-only/update/doctor/termux and unsupported
daemon routes keep their defined behavior. No selector may be smuggled through
an internal endpoint. Bare startup discovery
still runs at most once, after selection. `profile current` adds source `saved`
when the explicit preference selects a fresh launch; inherited source remains
inherited. The retired state-v1 record stays ignored and unchanged.

`profile delete ID` deletes that validated custom profile's local login/config/
cache files and registration; `default` is never deletable. Exact output is
`deleted: ID\n`. Shared canonical conversation files and compatibility-link
targets are preserved, and deletion never follows any symlink inside the account.
The command itself is the explicit request; no automatic deletion or cleanup of
other accounts is introduced. `profile rename OLD NEW` changes only the registered
ID/path, preserving local file bytes, inodes and modes without copying auth or
session payload; exact output is `renamed: OLD -> NEW\n`. Destination collisions,
unsafe/malformed registration and reserved/default names fail without replacement.
A profile selected by the explicit default preference cannot be deleted or renamed:
first choose another default. No two-record default/rename transaction is needed.

Profile creation, preference writes and lifecycle mutations serialize with an
exclusive kernel lock on the existing Manager profiles directory. Manager owns
that registry coordination; Core execution does not lock or parse the registry.
For every existing owned upstream execution home, Core opens the actual directory
and holds a shared kernel lock on its inode across native runtime and shared-server
execution using FD36. The lease primitive knows no account ID, Manager path or
registration format. At the lease boundary, aliases/relative paths bind the same
actual inode; upstream still receives the original CODEX_HOME. This does not
broaden the existing shared-layout or server-path admission rules. A missing home
is left to the existing execution preparation/upstream rules, and a foreign-owned
home does not become a Manager-managed account through this optional lease.

The existing declared Manager-home path contract still requires a real private
owned home and safe ancestors before the generic lease is acquired; missing,
replaced or unsafe declared homes fail, never become fresh isolated identities.
The compatibility topology/SQLite binding for the three already-supported shared
home layouts remains Core-owned so direct launches still see canonical history
without Manager. This is a retained data-integrity contract, not account selection
or a reason for Core to parse Manager metadata. No new binding registry, state
marker, user-data migration or second conversation store is introduced.

The home lease closes the mutation race: lifecycle requires its exclusive lock;
launch takes the shared lock and revalidates the opened inode against the selected
path before use. A registry-only mutation for another account cannot block an
otherwise safe existing-home launch. Core provides this execution lease only;
it does not implement deletion, rename or default policy. Manager
requires the exclusive home lease before any rename/delete and additionally
rejects existing old-generation live Core server bindings for that home. No user
server is stopped automatically. For pre-lease runtime processes, bounded same-UID
process metadata identifies CODEX_HOME/cwd/open account handles; only the execution
home field may be retained transiently, never unrelated environment values,
credentials, argv or session content. Unreadable/ambiguous relevant processes
reject mutation rather than claiming idle. Default selection may change while
work runs; existing work retains its account and authentication.

An execution-home lease proves directory use, not conversation writer ownership;
it never authorizes takeover, writer-lock removal or stopping another turn.

Core's generation-retention lease must occupy a descriptor above the reserved
33–36 range. Installing an execution-home lease under descriptor pressure must
not release generation protection before the final kernel exec.

New registration metadata is exactly `codex-manager-profile-v2\n`, without an ID
already owned by the directory name. Existing exact private v1 metadata remains
valid; rename first atomically replaces v1 by v2 while OLD is still valid, then
uses create-new directory rename and synchronizes the parent. Either observable
registration remains usable after interruption. No duplicated ID or journal is
introduced. Delete first atomically moves the validated account into one private
hidden `.delete-*` sibling under the held locks, synchronizes the directory, then
removes only that tree without following links. Before that move, a non-following
preflight rejects more than 65,536 entries or a directory depth above 128; an
unreadable tree fails before unregistering the account. Interrupted cleanup may leave
hidden private residue; it is not a profile, never used for launch, and a failure
is reported without pretending deletion of shared state or other profiles.
Version rollback never reverses explicit profile lifecycle actions. Managers
predating this extension ignore the preference and do not list v2 registrations;
account bytes and Core direct CODEX_HOME execution remain independent of metadata.

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

### Notification configuration and delivery

#### Notification preferences and Android delivery ownership

Preserve upstream /config notification preferences, TUI event filtering/
conditions and OSC9/BEL delivery wherever supplied by the selected runtime.
Core-only operation retains official notification controls. Optional Manager's
Android notification/toast/window-return path is a distinct capability. Its
absence is a verified break in that path, not a sufficient diagnosis of every
missing terminal or Android notification.

The optional Manager integrates with upstream notify/hooks rather than owning
a separate upstream event policy. Its Android adapter consumes selected native
completion and hook events and delivers through the configured Termux channel.
Do not describe it as an entirely independent event engine or assume an
upstream settings UI alone provides every Android delivery responsibility.

Before changing ownership, trace /config saved/effective settings, real event
production, focus filtering, selected backend, Manager hook/adapter availability,
Termux API/permissions and physical Android delivery. Compare Base and the
qualified enhanced frontend with the same event/preferences. Produce an explicit
adopt/retain/remove matrix for completed turns, approvals, structured questions,
other currently selected hooks, channel/content preferences, notification IDs,
window return, global/profile/session setting scope and account/profile changes.
Include a detached/headless backend, CLI, Direct and Provider work; replacing a
native backend hook with a TUI-only event must not lose alerts when no TUI exists.

Prefer upstream settings/UI, event semantics and delivery wherever they actually
cover the required behavior. Keep only demonstrated Android/Termux or profile/
window adaptation and uncovered required events in the optional device/Manager
boundary. Whole or partial retirement is determined by that comparison; Core
does not acquire notification policy. A settings migration must define one owner
for each equivalent preference/event/delivery effect, preserve existing user
choices and current/rollback compatibility, and remove replaced producers,
consumers, commands and tests together where no actual caller remains. Do not
layer another notification system beside upstream merely to hide a failure.

Acceptance includes actual /config navigation/save/restart, on/off and focus
conditions, completion/approval/question event coverage, foreground/background
Android notification/toast delivery, headless and CLI/Direct/Provider work,
selected-window return without starting a second workload, distinct conversations,
internal maintenance/memory exclusion and exactly one intended delivery per
event/channel, failure reporting and rollback. A native event, OSC/BEL write or
Termux command exit is not physical notification proof. The boundary comparison
is required before choosing the migration; current commands/contracts below
remain operative until a qualified replacement is delivered.

#### Manager notification commands and state

Notification commands use these user-facing forms:

```text
codex termux notify show
codex termux notify test
codex termux notify set [--channel <notification|toast|both>]
    [--hooks <none|all|EVENT[,EVENT...]>]
    [--content-chars <0|1..4096>] [--preserve-newlines <0|1>]
    [--toast-gravity <top|middle|bottom>] [--toast-short <0|1>]
    [--toast-background <empty|#RRGGBB>] [--toast-color <empty|#RRGGBB>]
    [--group <GROUP_ID>] [--focus <window|termux>]
```

`notify set` accepts options in any order, at most once each, and no trailing
arguments. An option omitted from `set` retains the stored value; when no
record exists, omitted values use these defaults: `channel=notification`,
`hooks=Stop`, `content_chars=0`, `preserve_newlines=1`,
`toast_gravity=top`, `toast_short=0`, empty toast colors, and
`group=codex-turns`. `0` means no user-configured character limit, but delivery
still applies a hard 4,096-byte payload bound. `none` disables all hooks.

The canonical event allowlist and order are:
`SessionStart`, `PreToolUse`, `PermissionRequest`, `PostToolUse`,
`PreCompact`, `PostCompact`, `UserPromptSubmit`, `SubagentStart`,
`SubagentStop`, `UserInputRequest`, and `Stop`. `UserInputRequest` is a Manager
notification selector, rendered as upstream `PreToolUse` with an anchored
matcher for only `request_user_input` and `request_user_input_async` (including
their `functions.` namespace spelling). It alerts before a structured question
or follow-up input request, using the actual question text from the recognized
question tool's `tool_input.questions` rather than a second generic prompt.
Question tools without usable question text do not emit an empty reminder.
It is not `UserPromptSubmit`, which observes the
user submitting input. No invented upstream event or transcript watcher is used.
Tool-execution approvals use the separate `PermissionRequest` selector;
selecting `UserInputRequest` alone does not subscribe to approval events.
If broad `PreToolUse` is also selected, it already covers these tools and the
narrow selector adds no duplicate handler. Hook lists contain unique canonical
names, or
`all`; malformed names, duplicates, empty list members, and invalid values are
usage failures. Event lists are stored and displayed in the canonical event
order above. `GROUP_ID` is a 1--64 byte ASCII identifier beginning with an
alphanumeric byte and containing only alphanumerics, `.`, `_`, and `-`.
Colors are empty or exactly `#` followed by six ASCII hexadecimal digits and
are stored canonically in lowercase.

The Manager record at `notifications/config-v1` is mode `0600` beneath a
mode-`0700` `notifications` directory and contains exactly these final-newline
UTF-8 lines, in order:

```text
codex-manager-notify-v1
channel\t<CHANNEL>
hooks\t<none|all|EVENT[,EVENT...]>
content_chars\t<DECIMAL>
preserve_newlines\t<0|1>
toast_gravity\t<GRAVITY>
toast_short\t<0|1>
toast_background\t<empty|#rrggbb>
toast_color\t<empty|#rrggbb>
group\t<GROUP_ID>
```

`notify show` emits the nine configuration `key=value` lines in this order:
`channel`, `hooks`, `content-chars`, `preserve-newlines`, `toast-gravity`,
`toast-short`, `toast-background`, `toast-color`, and `group`, followed by
`focus=window|termux` from the separate focus record. It emits no path, source detail, payload,
environment, or credential. An absent record yields the defaults without
creating Manager state. A malformed, symlinked, overlong, incorrectly-modeled,
or conflicting record is an operation failure and is never replaced
implicitly. `notify set`
merges validated values and publishes the complete record with a private
same-directory create-new temporary, file synchronization, atomic replacement,
and parent synchronization. A successful `notify set` emits exactly `saved\n`.
It never writes an upstream profile, Core generation, activation state, or
notification payload.

For an ordinary upstream launch, the admitted Manager reads this exact bounded
record and validates its path, mode, bound, version and complete shape. Manager
renders the selected native callbacks and hooks through the bounded private
configuration preparation endpoint above. Core neither reads the record nor
recognizes notification events. No persistent projection record/schema is
introduced, and Manager never writes Core's system configuration. Core combines
its execution defaults with the returned fragment in the immutable per-launch
snapshot before runtime exec. The shared base directory contains Core defaults
only; a live server's snapshot is not rewritten. The Core-owned base configuration
retains its `codex-termux-notify-v1` marker. A missing/owned-marker base file may be
prepared; conflicting foreign state is preserved and rejected before execution.
This Core-owned configuration conflict is distinct from invalid optional
Manager state. Each enabled event is mapped
to `codex termux notify emit <EVENT>`. Selected `Stop` uses upstream's native
`notify` argv callback, with no generated `hooks.Stop` command. Upstream excludes
internal memory consolidation from that callback; memory generation and use
remain enabled according to operator configuration. Other selected events retain
their command hooks, including the user-input matcher. Operator configuration
retains ordinary upstream precedence. Missing or invalid Manager notification
state, or a generation without a qualified Manager artifact, disables the
optional hooks and must not make ordinary upstream launch fail.

The internal `notify emit <EVENT>` endpoint reads at most 64 KiB of hook input,
which must be a UTF-8 JSON object when delivery is requested. It considers
only the string field `title` for the notification title and, independently,
the first string body field in this precedence: `content`,
`last_assistant_message`, then `message`. For the native `Stop` callback only,
exactly one final argv JSON argument replaces stdin and must have nonduplicate
`type = "agent-turn-complete"`; its same 64 KiB limit, successful no-op behavior
and presentation policy apply. Native `last-assistant-message` is an alias for
`last_assistant_message`; native `thread-id` is an alias for `session_id`.
Duplicate aliases follow the existing duplicate-field rules. A canonical,
nonduplicate top-level `session_id` is used only for the separately selected
focus action described
in Section 3. Other metadata, including commands, account/profile identifiers
and working directories, does not become presentation content or shell input.
For non-native hook input, the exact allowlisted `tool_name` values
`request_user_input`, `request_user_input_async` and their `functions.` spellings
allow presentation of `tool_input.questions`. Sync questions use their `question`
string and async questions use their `title` string; nonempty strings are joined
in question order. Headers, answers, options and other tool arguments are never
forwarded. Duplicate selected fields or malformed question input are no-ops.
Recognized question text takes precedence over generic top-level body fields.
A notification with a valid session ID uses `--id codex:<GROUP_ID>:<session_id>`;
question and later completion callbacks update one notification for that
conversation instead of adding a generic reminder alongside its body. Separate
conversations remain separate, updates still alert, and consuming a notification
allows a later event to create a fresh one. No notification deduplication state
is stored. Missing or duplicate/invalid session identity never coalesces unrelated
notifications. The manual delivery test remains an independent notification.
Native input-message arrays are ignored. The callback uses upstream's argv
transport directly, without logging or persisting its JSON payload.
Missing title uses `Codex`; missing body uses the fixed event status strings
`Notify session start`, `Notify tool start`, `Notify permission request`,
`Notify tool finish`, `Notify before compact`, `Notify after compact`,
`Notify prompt submit`, `Notify subagent start`, `Notify subagent stop`,
`Codex needs your input`, and `Notify turn completion`, in the canonical event
order above. Malformed or
oversized input is a successful no-op. The selected text is normalized for
CRLF/CR, then the configured character limit and final 4,096-byte cap are
applied. Notification title and body always fold all whitespace runs to one
space and trim surrounding whitespace, producing single-line provider arguments
so leading blank lines cannot hide content in the collapsed notification. Android
may still ellipsize text according to available width. `preserve_newlines` keeps
its existing meaning for toast delivery; the notification rule applies regardless
of that setting. The result is passed only to the selected Termux providers. It
never persists, logs, or prints the input, notification content, paths,
credentials, or session data.
Provider absence, provider failure, malformed hook input, and disabled hooks
are successful no-ops so an upstream turn cannot fail because notification
delivery is unavailable. `notification`, `toast`, and `both` select the
corresponding capability-aware provider attempts; `both` attempts each
independently. Manager emits no success text for the endpoint.

Default window focus uses the admitted Manager's `__terminal-focus-v1 UUID`
action for a canonical session identity. It resolves the live registered workload
and revalidates its native terminal or stock dedicated-window route as defined in
Section3; it never starts/resumes Codex or creates a replacement workload.
AI and tmux are not needed. A missing or invalid session identity uses the
Activity-only action. A valid identity with a stale/unavailable exact target must
not guess another workload or construct a window to simulate successful return.

Explicit termux focus and `notify test` use the Activity-only action: bring the
Termux Activity forward with reorder-to-front/single-top, preserving its selected
terminal. This invokes the absolute sibling am compatibility entrypoint (canonical
Termux path for non-UTF-8 entrypoints). The outer timeout is20 seconds and the
compatibility socket request budget is15 seconds; do not add a shorter wrapper
or retry a service request. Only that compatibility entrypoint may fall back to
cmd activity after an unsuccessful idempotent Activity return. Arguments are
fixed/shell-quoted and output silenced; no hook payload/auth/CWD is passed.

An Activity acknowledgment is not exact-window selection proof. Stock APKs lack
the native existing-handle API and use one registered dedicated terminal as
described in Section3; a manual rename/closing-window race may create an inert
empty terminal. Native-capable APKs must select the qualified existing handle
without creating another terminal. Socket availability and read-only commands do
not prove background Activity authorization; Android display-over-other-apps
permission and notification permission remain user-granted capabilities. With no
terminal, Termux owns its normal initial-terminal behavior. Physical one-tap
return and unchanged workload/window identities require separate device proof.
Existing notifications are not rewritten by a source or setting change.

Each provider attempt has a five-second bound; timeout terminates its owned
process group, including API-helper descendants, and reaps the direct provider
child. Generated command hooks
allow fifteen seconds for two attempts and Core/Manager handoff overhead.
`notify test` is an explicit operator delivery test, accepts no arguments, uses
the effective selected channels and presentation settings, and sends only fixed
non-sensitive test text. It works even when automatic hooks are disabled and
never writes settings or upstream state. Output contains one line per selected
channel, in notification/toast order: `<channel>=ok|unavailable|failed|timeout`.
`ok` means the provider exited successfully, not independent proof of Android
notification visibility or permission. Any selected-channel failure returns 1
with a fixed error; all successful attempts return 0. Provider stdout/stderr and
arbitrary input are never forwarded. Hook delivery retains its silent best-effort
contract regardless of these outcomes.

The Manager launch path does not create or manage tmux sessions; its
historical implementation and qualification fixtures are development notes
only, not product build inputs. Upstream /title composes terminal display
items and /rename changes the conversation name. Neither Manager nor an
external AI wrapper is another title composer or conversation-identity
authority. Independent window launches do not share workloads or migrate
live work to a different terminal. Caller color preferences remain
per-launch.

### Core recovery authority

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
