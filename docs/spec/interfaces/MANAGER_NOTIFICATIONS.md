# Notification configuration and delivery

> Normative annex of [SPEC](../../../SPEC.md); [INDEX](../INDEX.md) is navigation only and [CURRENT](../../../CURRENT.md) is observed status.

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
