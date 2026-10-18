# ChatGPT provider

This optional provider is owned by `rewrite/rust-core`. Core executes verified
Codex on Termux; this directory owns Chat transport, model capabilities,
authentication context and host/conversation lifetime. Manager is not required.
TUI supplies native transcript/model-selection integration. No provider code is
linked into Core or Manager.

## Approved Rust migration

The standalone Rust source candidate (`Cargo.toml`, `src/`) now implements
configuration/catalog, loopback Responses/SSE, owned CDP, original-call tool
returns, a private lane journal and a minimal app JavaScript bridge. It offers
`serve [--descriptor PATH] [--state PATH] [--port PORT]`, `status [--state PATH]`
and command-backed `token [--state PATH]` for qualification. Authentication and
`prepare` start this executable on demand when its configured rootfs is already
provisioned; concurrent starts serialize. Failed readiness of a live process or
writer lease never authorizes restarting it. `status` remains read-only. Readiness verifies
the local process and a fresh keyed challenge; it does not claim model progress.
Never paste the token output into chat or logs.

Run long-lived qualification services from a private immutable candidate snapshot,
not a Cargo target path that a later build can replace. Replacing a running image
invalidates its recorded process identity; neither a live listener nor its writer
lease authorizes bypassing that check or restarting it. `status` is read-only;
`token` can bootstrap a missing service and is not a read-only diagnostic.

`status --thread NATIVE_THREAD_ID [--state PATH]` reads the private lane's last
observation even when its local endpoint is unavailable. It distinguishes app
running, pending native tool results, stop requested/unconfirmed, acknowledged stop
and unknown state. A pending host confirmation/authentication retains the same
request, emits one waiting report and exposes `host_block`; approve/reject in ChatGPT itself. A host
reply resolves only its matching prompt. No automatic approval or new user
submission continues the wait. Content-free SSE data pings keep Codex's parsed-event
idle deadline alive during silent waits; they establish transport liveness, not
actual model progress, and add no Chat or native transcript item. Counts separate
app dispatches, user-role objects and tool returns; they do not measure Chat
allowance. It neither starts a service nor queries/resumes/cancels the app. No prompt, tool output, account identifiers or
credential is included. A journal observation is not current remote progress or
proof that native tools have finished; review canonical native history as well.

`stop [--state PATH]` explicitly requests graceful shutdown of the verified
Provider service using a PID-bound kernel handle. It preserves journal, app profile
and configuration, refuses a mismatched/reused process and reports an unconfirmed
stop without force-killing the service. Native Codex and completed tool effects
are not rolled back. Older kernels without the handle capability fail explicitly.

The candidate optional `view [--state PATH]` uses configured `host.viewer_assets`
(an already provisioned noVNC tree). It reuses the Rust service and private RFB
socket, with a separate temporary viewer capability; no Bun/Python viewer process
is needed. The URL is neither printed nor logged. `view` can start a configured
Provider but sends no Chat message; missing viewer settings refuse before startup.
An absent/broken viewer never prevents ordinary inference. Save host settings
before startup; changing them does not restart a live owner. Mobile keyboard auto
and manual controls, scrolling/drag, Esc and fullscreen assets are retained.
Actual private RFB/version and capability-revocation proof is separate from fresh
login and physical screen/keyboard acceptance.

The Rust candidate also supports explicit `transport.return_path = "mcp"`, with
`transport.connector_name = "Termux Codex"` configurable for the installed Chat
connector. The default remains `local_function`; changing the return path or
connector name requires a service restart at a settled boundary. No automatic
fallback follows an uncertain request. Its `/mcp` HTTP endpoint shares the Provider
process and common native lane/executor/transcript path. Static discovery reveals
no native state; a random per-turn capability gates catalog and tool operations.
Existing stdio tunnels can invoke this same binary's `mcp-stdio --state PATH`;
it verifies the already running owned service and forwards bounded JSON-RPC,
without starting an app or running native tools. Losing/cancelling a tunnel
observer leaves the original native action/result available for exact retry.
Logical call IDs retain exact inputs/results across disconnected HTTP observers.
Tool results complete original MCP invocations, without another app/user message.
An active MCP turn refuses new user input, model/effort or developer-policy changes
until settled. Deferred native tool catalogs update before results wake the model.
Account, project, native approval and cancellation checks remain in force.
Three actual Android command/patch/verification returns pass with one synthetic app
submission; real connector ACK/auth/confirmation is still required for live
admission. Fixtures do not establish actual Chat message-limit debits. A restarted
service never reactivates old pending MCP capabilities or replays native actions;
generation-bound reconciliation remains explicit.

These paths have deterministic TCP/app-wire fixtures and actual Android Codex
native-command/transcript proof. Rust directly boots/stops a fresh official app
through private Chromium pipes, PRoot and Xvnc; no Bun/Python host is required for
that path. Set `host.rootfs` to an already provisioned rootfs. App home persists
under provider state (`desktop/home`) with an exclusive lease; temporary display
resources are owned and cleaned separately. No existing profile is copied or
modified. `--descriptor` explicitly connects a qualification relay instead.
Normal restart reuses the journal's loopback port, rotates its local credential
and refuses an occupied port before starting an app. Startup errors append to the
private state `startup.log`; no Chat message is sent by authentication/startup.
The operating adapter is still JS/Bun/Python. Rust transport has real GPT-6/native
command, patch, validation and final-transcript proof through the existing app's
explicit private relay. A separate bounded device proof now also starts the authenticated app directly
from Rust, verifies the same account and executes native commands/patch/validation
without a Bun/Python host. It restores the original profile inode and controller
afterward; this does not activate a permanent operating migration.
The retained qualification desktop pins ChatGPT/browser icons: tap to launch an
absent app, minimize its focused window or restore an unfocused/minimized window.
Existing windows switch directly; duplicate startup taps coalesce. These icons
and their Python/tint2 helpers belong to the retiring qualification desktop, not
the Rust Provider payload or Core. Icons come directly from the installed official
ChatGPT/Chromium packages; they are not redrawn or bundled here. A live panel
reload preserves the app/login.
Electron's auxiliary index pages are excluded by read-only main-surface probing;
multiple real main surfaces still refuse instead of selecting by target order.
Actual project create/reuse/readback and project-bound GPT-6 native tools now pass.
Android native manual compaction commits a readable checkpoint and continues the
same thread; app replies in that separate compaction proof are synthetic. A
separate Android fixture preserves foreign opaque input and its original public
history while a readable user-context handoff admits the next native turn, without
an extra user turn or inference at injection. Actual native API re-entry in both
directions also passes with a real GPT-6 response/summary (two app model starts,
four observed user-role objects, allowance debits unknown). The source provider
responses were synthetic; actual TUI transition admission remains unfinished.
Permanent operating migration, fresh login/provisioning and reboot
remain unfinished. The direct real AI repository development trial is qualified
within its bounded scope, including a Provider defect repair and same-thread resume. No production fallback silently selects the Rust candidate. The
repository-wide work order is in [CURRENT.md](../../CURRENT.md#product-completion-scope-and-work-order).

| Responsibility | Target implementation |
| --- | --- |
| Configuration, effective values and model capabilities | Rust; one validator and generated catalog/profile outputs |
| Responses admission, streaming, tools, deltas and compaction | Rust; preserve upstream history/approval/execution ownership |
| App connection, host lifetime, cancellation and reconciliation | Rust; bounded CDP and process ownership |
| App-local function exposure, events and result delivery | Minimal JS executed in the app renderer; typed/versioned wire boundary to Rust |
| Optional remote viewer | Browser JS/TS/CSS assets; not a mandatory inference path |
| Build/research/qualification helpers | Rust, Python or shell as appropriate; not product runtime dependencies |

The candidate `models` command reads `catalog.models` from the same user-owned
configuration as settings. Initial routes live in `default-models.toml`; they are
bootstrap defaults, not a fixed Rust model list. Configuration can add a model or
change host-model/thinking-effort mapping without rebuilding. Defaults include
GPT-6 and Pro plus a hidden Instant compatibility alias; GPT-5.6 is excluded.
Catalog metadata does not establish entitlement or runtime qualification.

The source binary supports `config show`, `config effective`, `config validate`,
`config set FIELD VALUE [FIELD VALUE ...]`, `config unset FIELD ...` and
`config edit` (`VISUAL`/`EDITOR`). Structured catalog entries are edited in TOML.
`catalog` emits upstream-native model JSON from the same configuration. Each
`catalog.models` entry may contain `[catalog.models.context]` overrides for
`window_tokens` and `auto_compact_token_limit`; effective output reports their
origin. Explicit upstream invocation overrides still win. `prepare --home PATH`
authenticates or starts the candidate endpoint and generates immutable
catalog snapshots plus one shared command-auth profile. Its per-home link preserves
existing user profiles/config/auth. No bearer is written into the profile. Manager
is not required. Saving settings does not hot-edit an already bound backend;
prepare again for the next startup. Data-only host route changes are read on the
next Provider request without app restart.

`checkpoint --thread ID --from-model ID --model ID --effort EFFORT` reads a complete
idle public `thread/read` snapshot from stdin and emits one ordinary upstream
external-context user message. It neither starts the app nor reads credentials,
performs inference or writes native history. TUI persists the message with the
existing upstream API; it remains readable by the source model on failure. Only
an explicit message in the same thread or native-declared fork/parent lineage can supply public context for preceding foreign
opaque input. Reasoning/internal metadata are excluded. Current policy/settings
and subsequent input remain intact; historical tools are quoted, never replayed.
The generated catalog omits a compaction compatibility hash because this Provider
does not share OpenAI's opaque format. Token/window compaction remains enabled.

A definitely rejected single gateway call can receive a structured error on its
original host call ID. Its entire batch validates before any native effect; no
user message is added. Up to three correction continuations may run within the
same Responses request. Each is a real additional model start, not a free retry.
Lost delivery is observed rather than resent, and changed account/host, compaction
or ambiguous host calls never use this path. Native malformed function arguments
remain native errors. Actual Android discovery/correction fixtures pass without
Chat inference; this does not establish Chat allowance debits or real-model
correction quality across tasks.

Actual native automatic-review fixtures also use independent Provider lanes for
primary and reviewer requests, preserve the native JSON decision schema, execute
allowed owned commands and prevent denied commands. These use synthetic replies,
not qualified real-Chat judgment. Native guardian_review metadata keeps reviewers
outside the configured ChatGPT project, preserving the ordinary primary project
and preventing implicit project instructions/files from entering the reviewer.
Only upstream-supplied reviewer context is sent; global account personalization
is not changed. Upstream prefix-rule matching misses quoted
executable paths; the fixture uses a plain owned program path. Provider never
rewrites command arguments or takes over native approval policy.

Public text adapts MemoryCite tags and box containers to terminal Markdown without
changing already streamed text. Literal inline/fenced code, unknown markup and
exact tool arguments/results stay intact. No extra model request is made for
formatting. A native blocking-question fixture also returns its actual answer on
the original app call without resubmitting the user request; physical question UI
and real Chat reviewer judgment remain separate qualification.

Set related fields together, for example:

```sh
codex-chatgpt-provider config set history.mode project history.project_name "Codex"
```

Use optional `history.project_id` to disambiguate an existing exact project.
Only owned, private, unshared projects are admitted; their existing memory scope
is preserved. Shared destinations fail explicitly. Changed destinations
affect new lanes; existing conversations stay in place. An uncertain create never
blindly retries. Project grouping does not reduce stored history or establish
allowance savings.

These are candidate source commands, not installed public commands. Effective
output reports file/default origins and `runtime_application=not_queried`.
The source Rust server reloads it at request admission; the existing installed
JS adapter does not consume this file. New files/locks are private;
invalid edits preserve the old file, cooperating writers serialize and stale
editor snapshots refuse to overwrite a newer edit. Unsetting an explicit token
threshold delegates to upstream defaults; it does not disable all compaction.

Current code demonstrates an app JS/React connection point; it does not establish
the language of every part of the desktop app. Moving the adapter to Rust must not
move credentials out of the app, duplicate tool execution, add model requests or
promise a different Chat allowance. Rust owns policy; the injected bridge only
implements the app operations/events needed by the qualified transport.

Port existing deterministic boundary tests and real-Core drivers before removing
each legacy runtime responsibility. Keep the working installed provider unchanged
until candidate admission and a separate bounded activation. Preserve data paths,
account/workspace/conversation identity, context overrides and pending-call state;
do not restart live work or guess whether an uncertain submission completed.
After the Rust product path passes equivalent acceptance, retire superseded
runtime JS/Python and Bun requirements. Retained signed generations supply explicit
rollback, not a second automatically selected implementation. Keep provenance and
licenses for reused or translated code. No whole historical Core/TUI/release
snapshot is merged from the experiment.

The active runtime was selectively ported from `experiment/web-termux` at
`ab41733`; the user's existing uncommitted1m catalog/profile assertions were
included here and remain untouched in that checkout. Historical experiment Core,
Manager, TUI, release workflows, probes and account data were not imported.
[Product contract](../../SPEC.md) and [current work](../../CURRENT.md) own the
remaining configuration, project placement, context and release requirements.

The local-function transport returns native tool outputs on the original Chat
call. Native Codex owns approval, actual tool execution, writer locks and history.
Independent Rust MCP transport also has actual Android executor/HTTP return proof;
actual Chat connector admission remains unfinished, and it is not an
automatically admitted live fallback. User submissions, tool returns, model starts
and Chat allowance deductions are different quantities.

Warm requests reuse their owned prefix. A successful summary instruction remains
transient unless native history commits its checkpoint. Identical summary text in
the previous prefix is not a new checkpoint acknowledgment; actual history
replacement is required before retiring a lane. Bounded content-free
observations record append/replay, changed instructions/tools and host-acknowledged
conversation counts; they contain no prompt text. Responses Lite tool declarations are excluded from the accepted conversation
prefix, so their removal/reinsertion during native compaction does not rewrite
history. Local text and V2 `ocx1:` summaries stay transient until acknowledged by
canonical native history replacement; summary-only policy is then removed.
The1m native context setting is
the user's override, not a measured host capacity. Cross-provider opaque history
handoff remains unqualified; do not silently drop or relabel it.

## Source and installed runtime

The existing experimental immutable installer is preserved for development and
rollback verification. It is not the signed production delivery mechanism.
Its runtime file allowlist excludes all tests, probes and research. Dependencies
must match their pinned source/hash checks; effective dependency source/licenses
travel with the built payload. Existing app login, endpoint binding and user data
paths remain unchanged during source integration. Importing source does not
activate it, restart the app or replace the installed provider.

Do not change the operating runtime while a job is live. Stable admission still
requires authenticated optional-payload packaging, exact runtime/device proof,
installation/update/rollback and the release gates in SPEC. Ordinary Base/Full
startup or update must not download or log into the desktop app.

## Verification

From the repository root:

```sh
BUN=/absolute/path/to/qualified/bun scripts/check.sh provider
```

This gate runs Rust formatting, tests, lint and release build, then the existing
JavaScript/Python suites in disposable roots; it performs no real Chat submissions.
Missing Bun is an unfinished legacy regression gate, not a skipped success. Normal
Core checks do not require the optional provider runtime. Actual Android binary and
app-bridge execution gates are still needed before Rust runtime admission.
A live trial requires explaining its purpose, scope and expected submissions;
automated tests never sample Pro or replay an uncertain submission.
After no-MCP completion, the real development trial runs directly in `~/prj/ai`,
with its AGENTS/contracts and existing Git changes preserved. No temporary
worktree/project copy is used. The ChatGPT-backed Codex must do the actual
analysis, modifications, tests and failure repair; a supervisor-made patch or
synthetic app response is not development qualification.
The supervising agent runs the trial and independently reviews its diff and tests;
it does not merely leave instructions for a future user-run trial. Start only
after the Rust no-MCP path is complete, rather than substituting the old adapter.


Configuration remains generated upstream `--profile chatgpt` fallback and TUI
`/model` routing. The Rust candidate generates settings/catalog/profile outputs, and TUI source
consumes the new contract. Hosted TUI qualification at310f0f7 passed. Operating
activation and actual Android TUI composition remain unfinished. Configured
project mode has actual create/reuse and native tool execution/membership proof,
with account-scoped durable uncertain-creation records. On-demand authentication
has actual Android fresh-profile startup/restart proof, without inference or
existing-profile access. Account/host binding and settled cold-lane reconstruction
have app-wire fixture proof. Real authenticated inference has explicit relay proof;
authenticated Rust boot/inference now have independent actual proof; permanent
cutover/fresh login and full lifecycle admission remain separate gates. Host API changes can still require binary/bridge changes.

See [NOTICE](NOTICE.md) for dependency provenance and licenses.
