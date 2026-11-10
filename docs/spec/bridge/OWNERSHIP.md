# Approved next architecture: independent execution and optional integrations

> Normative annex of [SPEC](../../../SPEC.md); [INDEX](../INDEX.md) is navigation only and [CURRENT](../../../CURRENT.md) is observed status.

### Approved next architecture: independent execution and optional integrations

This heading names the accepted independence design, not a claim that every item
is still unimplemented. CURRENT records source, execution and delivery gates.
The Provider sections describe the existing model-adapter path and its Rust
migration. They do not prescribe Provider-owned planning/context strategy for the
new ChatGPT-operated MCP proposal. Reusable execution/identity/history/stop rules
remain requirements for either path; replacing an adapter policy requires an
explicit contract update and proof before activation.

All Termux execution continues through Core and the
same authenticated installation/update authority. Manager is never an alternative
unsigned runtime launcher. Core may retain a small versioned optional-component
dispatch contract; it must not inspect Manager's preference filenames or interpret
account, window, quota or provider policies to make an execution decision.

`CODEX_TERMUX_CORE_ONLY=1 codex ...` selects the direct path when Manager is
installed but not wanted. It skips Manager preparation, launch policy, optional
TUI and Manager-provided configuration for that invocation. Core still performs
its signed-runtime, environment, sandbox, shared-state and server preparation.
Core consumes and removes the switch before starting upstream; it is not an
upstream setting and does not alter saved Manager preferences.

Manager prepares account/window policy and requests Core execution with the
resolved home and invocation settings. Core owns signed runtime/frontend selection,
required environment/FD preparation and opaque execution-configuration isolation.
A direct Core path must query no Manager and use no Manager fragment or enhanced
frontend, while retaining native storage, locks and server identity protections.
Do not reuse a backend with incompatible effective configuration or rewrite a
live sibling's defaults. Do not create a second execution or update supervisor.

Optional settings must be isolated from Core's required configuration; their
upstream syntax/types and application must be qualified before execution. Manager
configuration discovery must not also project files into an execution home.
Account preference validation and explicit profile projection remain Manager
operations. A saved account selection that Manager cannot fulfill returns blocked
and must not silently become another account. Unavailable optional preparation before
handoff retains the confirmed current execution home; it is not permission for
Manager to ignore a known invalid saved choice. Safe preparation
failure may continue only with the confirmed execution home and proof that no work
was dispatched. Failure after handoff or uncertain submission never triggers a
blind task restart. Explicit user configuration errors remain explicit errors.

Core, Manager, TUI and Provider are separate components of one product. Manager
owns policy and headless commands; TUI owns upstream UI integration and connects
to Manager only for its optional capabilities. Provider selection/settings must
not require Manager. Preserve separate executables/builds where upstream TUI
dependencies require them; do not link the upstream execution engine into Manager
to reduce artifact count. Generic model routing reads the installed provider
capability/configuration authority rather than duplicating Chat families/efforts
in Manager or TUI. Optional capability absence removes only the affected extension;
native UI and basic Core execution remain usable. TUI installation and launch
must not require Manager. Without Manager, its profile/quota/window commands
report that capability unavailable, while native UI and Provider selection work.
This composition requires its own execution proof before release.

The source candidate may expose `codex-chatgpt-provider serve --descriptor PATH
--state PATH [--port PORT]`, `status --state PATH` and command-backed `token
--state PATH`. These are Provider-owned operations, not Core dispatch commands.
Bind only loopback, keep endpoint credentials in private owner-bound state and
verify a fresh keyed readiness challenge before returning the local token.
With no explicit port, a restart reuses its private journal's published loopback
port, retaining generated profile URLs while rotating the local credential. An
occupied prior port fails before starting an app; it never silently selects a
different address or trusts the listener at that port. A malformed service record
is an error, not permission to reset its state.
Never export official-app credentials. Listener readiness is distinct from app
connection, model progress and production qualification. A saved unsupported
feature must fail explicitly before app submission, never silently lose behavior.
These commands do not qualify the remaining host/project/context lifecycle.
The generated command-backed `token` authentication and explicit `prepare` may
start this same Provider executable on demand, using only an already configured
official-app rootfs and the Provider's own persistent app home. Serialize concurrent
starts and return credentials only after authenticated readiness. Never restart a
live lease holder or bypass failed readiness of a recorded live process; preserve
pending lanes. No Chat is submitted by startup/authentication/preparation. `status`
remains read-only. Bound startup time and retain content-free errors in a private
startup log; no alternate launcher, Manager dependency or new supervisor is added.

#### Explicit device service integration

An owner may explicitly opt an already configured Provider and its existing
OpenAI Secure Tunnel runtime into Termux's runit services. This is device service
configuration, not implicit Core launch behavior or a bundled provisioning flow.
Provider owns the bridge/MCP service; the tunnel runtime transports its MCP
traffic. They remain separate processes. Use the existing `termux-services`
startup/root-recovery mechanism rather than tmux or another daemon manager.

**Startup ownership contract.** Provider configuration selects
`service.startup=standalone|supervised`, defaulting to standalone. The default is
omitted from persisted config serialization used by existing fingerprints.
The generic termux-services
layer owns the shared runit graph and its recovery, not Provider health or task
policy. When supervised service operation is selected, runit owns Provider
startup; preparation/authentication must use the verified supervised endpoint
or report bounded readiness failure, rather than race it with detached startup.
Standalone operation retains its existing lazy-start capability. Supervised
authentication/preparation waits within the existing45-second startup bound for
authenticated readiness and returns provider_supervised_start_timeout if absent;
it creates no detached service. Status remains read-only in both modes. Changing
the setting does not adopt, stop or replace an existing service: maintenance must
settle work, stop the old owner and enable the selected foreground supervisor.
Older Provider binaries reject the new service configuration section. Preserve
the previous configuration with the package/state rollback inputs, and restore
the compatible configuration while stopped before starting that old binary.
Establish the
selected lifecycle authority explicitly without guessing from a process name or
an unresponsive endpoint. Concurrent preparation and supervisor startup must
converge to one authenticated service under the selected authority, preserving
the Journal lease, active Native owners and pending work. No second supervisor
or Provider-specific recovery policy belongs in the generic root monitor.
CURRENT records implementation, qualification and delivery separately; the
serialized maintenance procedure remains applicable to installed older code.

Run Provider's existing `serve` command in the foreground under runit. A separate
opt-in host keeper may observe that service and use `host ensure` only when the
owned host is confirmed absent. Provider still owns Xvnc, the Linux app, private
pipes, profile lease and optional viewer. Do not supervise another Xvnc against
that profile, replace a live/unready owner or retry unknown work. Host acquisition
must use provisioned assets and retained login, without model input. A status
timeout or present-but-unready host remains unknown/pending, not permission to
restart it. Disabling the host keeper preserves lazy direct-Chat operation.

Run the existing configured tunnel runtime/profile in the foreground under its
own runit service, after authenticated local Provider readiness. Reuse its
remote tunnel and credential references; never create another remote tunnel or
expose credentials merely to change supervision. Local readiness does not prove
remote control-plane health. Stop the exact old runtime before starting its
replacement; do not run tmux and runit owners concurrently.

Select an Android-compatible tunnel image explicitly; an executable name/version
probe alone does not establish platform compatibility. Do not change a system
resolver to accommodate an incompatible Linux runtime. Keep service-selected
paths/profile in private common configuration; native execution home/workspace
remain Provider-config authority. Observation may use the admitted immutable
Provider image to avoid Core activation-lock contention, with explicit rebinding
during package/service maintenance. A failed observation preserves the current
transport. MCP stdio uses that explicitly admitted Provider image directly and
verifies the current service endpoint/credential per RPC. Provider PID/start
changes do not restart the tunnel or replay a request. Rebinding the immutable
stdio executable is a separate bounded maintenance action. MCP stdio stdout
remains reserved for protocol messages, separate from diagnostics.

**Optional component and tunnel distribution target (not yet implemented).**
Core is mandatory. Manager, enhanced TUI, Provider and the patched Termux tunnel
client are independently selectable optional capabilities. Full selects all of
these optional capabilities, including the prebuilt Android tunnel client;
individual selection must also work without installing Full. Remote MCP setup
resolves its tunnel dependency; a Core-only or local-function route has no new
remote-tunnel requirement. Installing binaries does not enable services, log into
an app, acquire credentials or create remote registrations implicitly.

Ordinary installation/update uses qualified prebuilt artifacts produced during
release preparation, with authenticated inventories and independent component
version/selection/rollback. It requires neither Go/clang nor another source
checkout on the user's device. Full may carry exact signed component packages
under the same installation authority; it must not become a second implementation
of their installers or couple independent tunnel updates to Core/Provider rebuilds.
Existing Base/Full releases retain their original inventories and interpretation;
CURRENT distinguishes that installed format from this future composition.

The wrapper repository owns one authoritative tunnel compatibility recipe: the
official source/version resolution, bounded Termux/mobile patches, Android build
and affected-path tests. Reuse it for release builds and an explicitly selected
source-build/recovery path. The latter acquires official source into temporary
storage, checks required Go/clang/build tools and qualifies the result without
requiring ~/prj/tunnel-client or a separate user-managed repository. Resolve latest
stable when requested; a source revision outside the validated recipe must fail
with its actual incompatibility rather than silently use an old build. Build-tool
setup is scoped to the selected build operation, never ordinary Core startup.
The additional tunnel checkout may remain a development experiment, but cannot
be the product's patch/build authority or a runtime dependency. Keep one copy of
each product patch and build rule in this repository. CURRENT records preparation,
source-build availability, signed distribution and activation separately.

Qualify platform execution, existing profile/credential compatibility, actual
MCP request/result delivery and the mobile half-open HTTP/2 regression before
switching the configured service image. Evaluate each local patch against the
selected upstream instead of blindly retaining it. Preserve remote registration,
credentials and unrelated global/shared tunnel consumers. Intentional replacement
stops the exact old service runtime, selects the validated image and verifies the
actual new process and remote delivery; failure restores the matched previous
selection without inference/effect replay. A successful build, version probe or
package install alone is not activation proof. Supervision never performs an
update on its own. CURRENT owns installed versions and the separate maintenance
versus future automatic-dependency integration milestones.

Runit may restart an exited service, but it must not replay native/Chat work,
submit prompts, grant approval, perform updates or change account/model settings.
The device's existing root recovery can recreate the registered supervisors.
It cannot recover a fully killed Termux UID without the next external Boot/login
opportunity. Intentional maintenance must first disable the relevant service;
otherwise desired-up supervision will start it again. Service enablement, dependency
checks, private bounded logs and explicit disable/restore instructions belong in
the future shared minimum-setup scope; automatic installation remains deferred.

`status --thread ID` may additionally inspect that native thread's private lane
journal without connecting to the app or starting a service. Report observed
transport phase, pending native call count, cached public event count, separate
dispatch/user-object/tool-return counters and a bounded static error code; never
export prompt/result text, account identifiers, host IDs or credentials. Distinguish
endpoint liveness from the last journal observation. Running/unknown/unconfirmed
stop state is not completion, and tools-ready means awaiting native execution or
results. If the local endpoint is unavailable, retain the observed lane state and
say its current remote activity is unknown. Inspection never resumes or cancels
work or infers Chat allowance deductions.
An explicit Provider `stop --state PATH` requests graceful shutdown of its verified
local service, including owned app cancellation/teardown, without removing its
journal, app profile or configuration. Use a PID-bound kernel handle and revalidate
the recorded process before signaling; never signal an unverified reused PID or
fall back to a broad process match. Bound acknowledgment time, report an unconfirmed
stop if the owner remains, and never automatically force-terminate the service.
Existing bounded teardown of that service's own child groups is unchanged.
An unavailable kernel signaling capability fails explicitly. Stopping this service
does not stop the native Codex engine or undo completed native tool effects.

The optional Rust-host viewer reuses that Provider service and its owned private
RFB Unix socket; no Bun/Python daemon or second app process is introduced. An
explicit host.viewer_assets setting selects already provisioned noVNC assets.
Absent/broken viewer assets must not prevent inference. Expose viewer HTTP and
WebSocket only on loopback behind a separate random capability, with no-store and
no-referrer headers and exact local-origin checks on WebSocket/control requests.
Reject traversal, symlinks and unexpected/oversized assets. The viewer capability
never authenticates Responses or general app/CDP operations. An explicit `view`
may start the Provider and open its private URL in Android through Termux; it
neither logs that URL nor submits Chat messages. Preserve mobile keyboard toggle,
manual keyboard, scroll/drag, Esc and fullscreen controls. Viewing is optional for
inference and does not establish successful fresh login or correct physical UI.

The retained Linux qualification desktop exposes compact pinned ChatGPT/browser
icons from the installed official packages. A single tap launches an absent application, minimizes its focused window,
or restores/activates an unfocused or minimized window. Existing-window actions
are immediate, not queued through the half-second controller loop. Match the
private desktop's exact app class and controller ancestry, never a window title
or another desktop/profile. Serialize duplicate launch taps, preserve authentication
and running work, and do not submit Chat or create a conversation by switching
windows. This qualification UI is not a Core dependency or a second Provider
execution path. Superseded desktop/adapter helpers retire with the experiment;
reusable qualification tools remain development dependencies and are not added
to the Rust Provider payload.

The Chat provider source lives under `providers/chatgpt`, outside crates/core,
Manager and TUI. Its approved implementation target is a native Rust adapter
process, usable through upstream configuration without Manager or enhanced TUI.
Rust owns configuration/capabilities, Responses HTTP/SSE and tool mapping,
conversation/context correlation, cancellation/reconnect/retry policy, CDP
communication and owned-host lifetime. The provider may use the signed app's
authenticated context; it does not replace that app's login or inference.
Its failure must not disable native Codex, account commands or window management.
When Electron retains several index-page targets after sign-in, select the sole
main Chat/Work surface through read-only DOM capability inspection. Do not select
by target order, activate/create a chat, or attach inference to an auxiliary
window. Even one initial index page must pass the visible-composer check before
attachment is retained. Window count, creation order and localized button labels
are not readiness evidence. Duplicate target identities or multiple eligible main surfaces refuse;
an unready surface remains a readiness failure. App account and bridge admission
still run after target selection.
Reuse existing upstream provider configuration and bounded local contracts; do
not introduce a speculative general plugin framework. Unify component discovery/
status and lifecycle presentation where appropriate, not credentials, transcript
storage, process ownership or unqualified update mechanisms.

Earlier JS/Bun and Python implementation paths are migration evidence, not
separate product authorities. Keep only the JavaScript required to execute inside the
app renderer for the current app-owned function transport, plus optional viewer
assets. Rust controls and validates that bridge through CDP; no token extraction,
second tool executor, duplicated configuration validator or task/retry/lifetime
policy belongs in the injected script. Define a versioned, bounded bridge contract
and reject incompatible app capabilities before an uncertain submission. This is
not a claim that the entire desktop app is Rust or JavaScript, nor a promise that
changing the provider's language bypasses the host's limits.

The optional Rust Provider package is providers/chatgpt/Cargo.toml with runtime code in src/
and separately inventoried app-side bridge/assets. It must not be a Core, Manager
or TUI dependency. Workspace membership must not make Base acquire/build the
provider or desktop app. Replace product-path Bun/Python dependencies only after
their behavior is qualified in Rust; development/qualification scripts may remain
Python or shell. Use one selected adapter per launch, not automatic dual-runtime
execution or fallback after uncertain dispatch. Old signed generations remain
explicit rollback targets. Preserve installation identities, login/profile data,
remote conversation associations, live jobs and current configuration overrides
through bounded migration; source integration alone never activates a replacement.

Once the supported Rust Provider path has individually qualified every
replaced consumer, retire duplicate source adapters, experimental installer/
profile writer and desktop/viewer launch policy with their unused callers.
The authenticated optional Provider package is the supported installation
interface; preserve a qualified renderer bridge, viewer assets, explicit
transport choices, Native configuration/approval/history and reusable
qualification tools. Preserve user changes, licenses, original experiment
proof, installed experiment consumers and immutable current/rollback inputs.
Source retirement never silently replaces a running app, profiles or service:
live migration remains a separate authorized, observable transition.

Native automatic reviewers have their own Provider lanes and remain outside the
configured ChatGPT project. Recognize upstream guardian_review thread metadata;
send only that native reviewer's supplied context, never the primary app lane's
conversation or implicit project instructions/files/memory. This transport
isolation does not determine the native approval policy or reviewer decision.
An existing project-bound lane cannot be relabeled as an isolated reviewer.

New independent MCP trials use the repository's `Termux Bridge` connector display
name, with display/connector identity configurable rather than hardcoded branding.
Historical `Codex Native2` references remain provenance only. Renaming never changes
an outstanding native tool call/result identity. Reuse authorized existing admin
and computer-use access when exercising this path; actual connector ACK remains
an independent gate from discovery and the local-function transport.

The Rust MCP return path is explicitly selected at service startup through
transport.return_path=mcp and a configurable transport.connector_name (default
Termux Bridge). Local functions remain the default; never switch transports after
uncertain dispatch. The same Provider service exposes a bounded streamable-HTTP
/mcp endpoint; it is not a second executor. Static discovery discloses no native
state. Discovery descriptions state capabilities, not instructions overriding the
host's approval or retry policy. native_catalog reads the selected native tool
catalog; native_tool dispatches native tools.
Exact-call validation and replay protection remain enforced by the Provider.
Upstream Codex owns tool execution, approvals and canonical history; Termux Core
only prepares and launches its verified runtime. The same binary's optional
mcp-stdio command verifies an already running owned Provider for each RPC and
forwards bounded multiplexed JSON-RPC for an existing tunnel. Each request uses
the same verified service snapshot for its process, port and credential; later
requests discover replacement generations without retaining stale endpoints.
An unavailable service produces a request error while stdio stays open, including
when the service is absent at bridge startup. No mutation is automatically replayed;
it never starts an app/service, repeats delivery or executes tools. EOF/request
cancellation detaches only that observer. A random per-app-turn capability authorizes native_catalog/native_tool;
each logical call ID binds exact tool/input/report, independently of JSON-RPC IDs.
Native execution/results still pass through the common Responses/lane journal.
Persist invocation intent and native results before publication/delivery. Peer
loss detaches its waiter, not the native action; an exact retry receives the same
result. Changed IDs/arguments, stopped turns and unresolved owner changes refuse.
Return native results to the retained original MCP invocation, never submit them
as a new Chat user/tool message. A retained app turn cannot silently change model,
effort or developer policy. Effective tool definitions belong to the current
request: explicit top-level declarations own the complete current set. In
Responses Lite, input declarations are chronological incremental updates or
one rebuilt catalog for a nonincremental window. Validate each batch independently
and apply later definitions by tool key; history is not one simultaneous catalog.
Each individual catalog must remain internally unambiguous. Discovery results
can extend that set, while explicit current declarations retain precedence;
pending calls retain their original tool identity and result binding. Historical
catalogs cannot re-advertise tools absent from a current catalog.
new user input waits for a settled boundary. Account/project/stop/transcript rules
are identical to the local-function path. Host confirmation/authentication must
surface as blocked state, never be bypassed. Rust wire fixtures alone do not admit
this route for live use; actual connector ACK is required before activation.
Pending host authentication/confirmation keeps the original request attached and
emits one public waiting report; status records the blocking reason and observed
submission counters. Poll the same invocation until the host resolves it or the
user cancels. A correlated host response clears only its own confirmation. Without
explicit user delegation, never click approval automatically. Bounded explicit
user delegation permits selecting the app's real Allow control for that authorized
operation; it never permits synthesizing consent or bypassing the host's controls.
For delegated app permission, `approve --thread ID` first uses the signed app's
own approval action without opening a window. Prefer its offered "for this
conversation" action; only when that scope is unavailable choose one-time Allow.
A successful conversation approval must never be followed by one-time approval.
Only a known capability rejection before any approval intent permits the visible
UI fallback. `approve-ui --thread ID` explicitly exercises that fallback: open the
bound private viewer/Chat and actually select/click the same preferred scope.
Both paths require the current bound account, an active operator lane and exactly
one unresolved confirmation for its configured connector. Use only server-offered
actions; the app constructs its own reply. Scope refers to this Chat and connector,
never account-wide/persistent permission or a successor Chat. Native approvals are
unchanged. Authentication, forms, passkeys and ambiguous targets remain pending.
Wait for cold UI rendering through bounded pre-intent readiness observations;
never queue a delayed approval merely because navigation started.
Fence approval intent before sending/clicking. An uncertain result never permits
another path or scope; reconcile its actual reply before proceeding. Opening a
window or returning HTTP success does not prove acceptance/native completion.
Optional viewer assets and a verified private RFB socket are needed for automated
visible fallback. Operator-owned lazy hosts register that viewer only after the
host actually starts. An external owned relay may explicitly configure
`host.viewer_socket`; it must pass the same private socket/identity checks.
Never submit another Chat user message merely to continue. Record
observed counters before reporting a host block or stream failure.
Loopback SSE keepalive uses a content-free data `ping` event: upstream's parsed
SSE idle deadline ignores comment-only heartbeats. This proves transport
liveness only, not model or native work progress. It adds no transcript item,
Chat submission or retry; ordinary observer cancellation still reaches the host.
An explicit host permission/authentication reply may use the signed app's separate
tool-role continuation. Admit only a response correlated to an observed pending
host action in this lane, including a target explicitly offered in that action's
server metadata which addresses the invoking message instead of the confirmation
message. Unrelated ancestors, ambiguous matches and a second response to the same
action refuse. A failed stream takes precedence over a historical host wait.
The permission stream's `done` does not retire its retained MCP capability.
Native round admission accepts the original dispatch plus exactly the observed
host replies; it never normalizes away actual request counts. Late callbacks from
a prior permission stream cannot terminate the current continuation.
Never fabricate a reply instead of the signed app's real consent control. Count
those actual host requests/reply objects separately from native tool returns and
user-role objects. They are not automatic retries or proof of allowance debits.
A retained renderer bridge is bound to the exact Provider bridge-source revision.
A different revision refuses admission without replacing hooks or pending work;
explicit settled qualification cleanup or a newly owned host establishes the new
revision. A new binary must never silently keep executing old renderer hooks.
`reveal --thread ID` selects only that native lane's actual saved app conversation,
using the existing owned service and current account binding. It neither starts a
service/viewer, submits Chat input nor approves tools. Missing/mismatched lanes or
accounts refuse; never pick a chat by title, sidebar order or a guessed remote ID.
Last-known host blocks do not make an unknown/stopped lane a live attached wait.
A confirmed host authentication/permission wait pauses the model-work deadline;
transport cancellation still requests and verifies stop. Human response time alone
must not expire a correctly attached request and force another Chat submission.
The provisioned Linux UI includes Korean-capable CJK fonts; typography is not an
account-language or personalization change.

Experiment integration targets rewrite/rust-core as a repository, not crates/core
as a code destination. Port exact reviewed provider/runtime sources, dependencies,
licenses and affected regression drivers onto current authority. Never replace
current Core, Manager, UI, release workflows or contracts with experiment snapshots.
Preserve source history and user changes; compare intent and current equivalents
before replaying a commit. Separate runtime modules from qualification/research
material. Do not ship historical probes, GUI trials or user app/login data as
runtime dependencies merely because they share an experiment directory.

The runtime/tests/license port and standalone Rust implementation are already
on rewrite/rust-core. Remaining lifecycle/package/activation gates and preservation
of unique experiment work still precede closure; source existence is not release
admission. Current implementation and proof status belong in CURRENT. Port useful
remaining qualification drivers alongside the affected Rust implementation, preserving their
provenance and independent inputs. Review branch-specific and uncommitted work
before the final integration/closure; retain unique evidence and attribute reused
or translated code. Do not overwrite current product history with a wholesale
experiment merge, delete unreviewed user work, or leave two active provider source
authorities. Rust admission, product acceptance and preservation of unique work
precede experiment closure. CURRENT.md owns the ordered migration work.

First qualify no-Manager execution and configuration isolation, then generic
Manager/frontend/provider composition. Before stable provider admission, establish
cross-provider readable-history transfer, fresh/config-profile inference, native
tools/reviewer, interruption/resume and observed native transcript behavior against
the exact release inputs. Report live Chat submissions and explain trial budgets
before execution; Pro is excluded from automatic qualification. Experimental
source integration is not stable publication or device activation.

The actual development acceptance uses the real `~/prj/ai` checkout directly
after the evaluated path can execute development. The existing Rust no-MCP path
has its bounded actual trial recorded in CURRENT; do not schedule that completed
trial as if it had never run or count it as proof of the proposed workflow. New
control paths require their own development acceptance. This user-approved
source-development trial does not use a temporary worktree, clone or project copy. Inspect that
repository's AGENTS/contracts and Git baseline, preserve unique uncommitted and
untracked source, and let the ChatGPT-backed native Codex perform a real sequence
of analysis, implementation, tests and failure repair. The supervising agent must
not substitute its own implementation for the evaluated model. Preserve unrelated
user work and operating authentication/data/runtime.
The qualification controller runs the bounded sequence without per-turn
supervisory model polling or quality intervention. After the terminal notice,
the supervisor independently reviews actual changes and applicable validation;
recording a future task or accepting the evaluated model's own success report is
insufficient. Earlier C/C2 reviews retain their actual assisted/unassisted scope;
their per-turn supervision does not become a future requirement.
Qualification observation has an explicit no-activity budget as well as the total
deadline. Rewriting a local snapshot or retaining an active-stream flag cannot
reset that budget or establish progress/completion. The absolute wall deadline
still applies across Android suspension; report wall and monotonic elapsed time
separately rather than assuming they are identical. After a known failure, retain
completed artifacts and bound further waiting. A fresh authoritative server
terminal observation may clear a stale local active-stream flag after Stop;
local abort alone cannot. Reject late starts on cancelled owned lanes before
changing their active state. Preserve the original Stop promise without resending
it; keep uncertain source state until independently reconciled.
A persisted settled response with its committed Completed event and no pending
calls can satisfy a later host Stop without recreating its disappeared app lane.
If that same round remains present during shutdown, its fresh owned snapshot
must show no pending input, stream, approval or error before release. The committed
Completed receipt also permits release after the receipt-only Stop path; absence
of that receipt preserves the original server Stop requirement.
An idle label alone, an offered tool, or an unfinished round cannot. Native Stop
still independently accounts for active turns and jobs before acknowledging the
whole work or tree.
Verify complete immutable-input hashes before work and final acceptance. During
local protocol observation, cached verification may reuse unchanged file identity,
size, modification time and change time; any metadata change triggers a new hash.
The original manifest is pinned and cannot be replaced during that run. Never
rehash a large unchanged CLI binary on every status/approval observation.
Native transcript, exact
effects, interruption/reconnection/latest-state continuation and meaningful test
results qualify the path; synthetic replies alone do not. A live budget and
separate submission/result/retry counters remain required; automated Pro is
excluded. Git protection is not permission to reset or discard existing changes.
