# Clipboard and image-input behavior

> Normative module of [docs/spec/USER_INTERFACES.md](../USER_INTERFACES.md). Check [CURRENT](../../../CURRENT.md) for implementation/evidence.

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
Installed Provider definitions supply only fields absent from the enabled
upstream configuration layers. Explicit file and invocation values retain their
normal precedence during provider selection; a valid partial provider table must
not discard its missing installed defaults. Invalid upstream configuration remains
an error. Re-entry preserves file-owned credentials in their files and combines
only missing defaults with original session flags
before emitting CLI tables. The installed command used for Provider control
remains distinct from an explicitly overridden inference endpoint.

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
  work; TUI login controls remain available. When a validated independent Provider
  is installed, the Core-owned local TUI can present `/provider` before OpenAI
  sign-in is repaired. A Native account/read reply explicitly classified as an
  authentication failure remains an unavailable account observation with a
  visible error, never a fabricated authenticated account. An unclassified
  legacy error is not guessed from its text.
  Connection, model-catalog and managed-policy failures remain blocking. New
  inference retains the selected provider's actual authentication requirement;
  opening the picker changes no route. No credentials or account defaults are
  replaced.
  Native still admits attachment/writes and detects races after this observation.
  This is an optional integrated-frontend contract; Core's ordinary independent
  runtime launch does not acquire a Manager dependency.
