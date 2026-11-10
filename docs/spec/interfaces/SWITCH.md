# Native /switch behavior

> Normative annex of [SPEC](../../../SPEC.md); [INDEX](../INDEX.md) is navigation only and [CURRENT](../../../CURRENT.md) is observed status.

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
