# Codex Termux Rewrite Specification

Status: current product contract
Repository: `humtr/codex`
Active implementation branch: `rewrite/rust-core`

Document interpretation, reviewed2026-10-18: Core, Manager, TUI and Provider are
the component names. TUI is an optional upstream-based frontend, not Manager's
private screen. Existing `frontend/manager-tui`, artifact identities, flags,
workflow names and wire markers retain their compatibility spelling until an
explicit migration; a terminology change alone changes no executable behavior.
CURRENT.md owns the current installed/public baseline and the ordered completion
plan. Sections labelled approved target describe authorized work still to be
implemented; deployed dispatch/default/configuration contracts remain in effect
until their coordinated replacement passes qualification. Historical release
versions below describe retained compatibility, not the latest installed release.

## Generation-bound TUI admission (2026-10-10)

The0.163 adaptation is termux-fd-remap-v5, bound to official package SHA256
9d8ecf2d19d3e184877983902d6c85de48b3888dcb72c382a8a0c975845927fb and raw CLI
8a96f66d059d653e68bfd5f3c79337e6f1d7d170d80bf705eb7d0249b34c0ece.
It retains FD33/34 remaps and exact protected FD35/permission-menu adaptation,
including no Read Only choice and distinct Ask/Auto-review/Full access policies.
The adapted native local TUI resume forwards the current permission configuration
rather than restoring an obsolete Linux sandbox; remote requests, preserve-existing
reads and saved approval/reviewer choices retain upstream semantics. This is a
Termux frontend compatibility correction, not a change to the upstream engine.
Existing v2/v3/v4 retained-generation admission remains valid. Passing construction
alone does not qualify Android tools, reviewer, cold resume or side conversations.

Qualified upgrade: official0.163.0 source
83086188ae0614bd882818ff1aa23f3194962012, with the same Rust1.95.0 source
build toolchain and unchanged external dependency bindings. Public stable53 passes
the exact artifact, native, permission/lifecycle and signed publication gates;
live device activation remains separate. Core-bound /daemon views retain diagnostics
and Core update guidance; upstream
package update/restart actions are disabled and their dispatched mutation events
refuse before any handoff. Remote/unselected targets keep upstream semantics.
The stable52/0.161
contract and retained-generation compatibility remain valid throughout the upgrade.

The signed generation owns the maintained `/switch` frontend. Existing preview
clients retain their executable until exit; no preview construction or parallel
update authority is maintained.

- The official adapted `runtime` remains the sole server/execution backend.
  Qualified local frontend launches forward selected upstream `--profile` and
  invocation settings through its per-thread config APIs, preserving daemon
  defaults and other threads. Explicit remote/executor/`--no-daemon` and
  noninteractive routes retain the official runtime path. Optional frontend
  absence never makes Core unavailable.
  An opt-in installed Chat model catalog augments `/model`; its marked private
  config owns command-backed, nonsecret routing references. Changes across the
  native/Chat provider boundary re-enter Core with the same home, durable thread, permissions/reviewer and
  terminal. Active work, pending input/tools or side conversations refuse the
  transition. Selected config profiles survive; no Chat slug is persisted into
  an ordinary default. `--profile chatgpt` remains an upstream config fallback,
  distinct from Manager login accounts. Model/effort metadata is not an entitlement
  proof; deferred families and unexecuted Pro choices are labelled accordingly.
  Model/effort changes within the selected Chat provider use upstream session
  updates, preserving the frontend and compatible owned remote conversation.
  An exact no-op submits nothing; a pending native call retains its original route.
  Reasoning changes target the active collaboration mode, including Plan, through
  its upstream session event; they do not persist account defaults or change the
  inactive mode's reasoning preference.
  Core's existing private config directory may contain `profiles/NAME.config.toml`.
  Manager launch preparation projects missing upstream filenames into an owned execution home with
  atomic no-replace symlinks; existing local overrides win. Shared profile sources
  must be private owned regular files and the registry a private owned real
  directory. No account auth/base config is merged or replaced. Core does not
  interpret the registry. Previously registered files remain usable without
  Manager; absent Manager skips new convenience registration, while native
  account-local upstream profiles continue to work. Missing or unavailable
  optional registration never prevents ordinary Core launch.
  An optional source-built TUI is the third signed helper at `helpers/2`,
  identified by `termux-manager-tui-v1:UPSTREAM_VERSION`. The first two helpers
  retain the accepted browser identities. Its version must exactly match the
  generation's upstream version and its generation must declare Manager. Local
  canonical layout uses browser/open/curl, browser/manual/curl and helpers/2.
  Public stable retains the R10 compatibility floor through helpers/0, helpers/1
  and helpers/2 plus its exact legacy marker/activation-doctor signal. Its digest
  is owned by the existing generation descriptor and signed release inventory;
  no parallel pointer,
  installation root, stable-core backup or preview-specific updater exists.
  Release flattening, Pages reconstruction, LKG retention and HTTPS readback use
  that exact signed inventory through one publication validator. Historical seven
  and extended eight files remain distinct exact sets; missing or undeclared
  assets refuse transport, and failed reconstruction removes only its own new
  disposable output. An occupied output is preserved.
- Maintained frontend source/build tools live in frontend/manager-tui. The active
  source qualification pairs official0.163.0 with exact upstream commit
  83086188ae0614bd882818ff1aa23f3194962012 and Rust1.95.0. Local workspace lock
  normalization changes only local0.0.0 versions/references to0.163.0; all external
  bindings remain unchanged. Both the retained161 and candidate163 builds leave
  upstream Core source unchanged. Its Clippy check alone may temporarily prepend
  recursion_limit=256 to the exact pristine upstream Core crate root, because
  Rust1.95 Clippy exceeds the default128 query depth in unchanged plugin handling.
  The check must verify the annotated body remains exact and restore the original
  bytes even on failure; every other Core modification refuses qualification.
  On ARM64 musl, lint may also temporarily allow unnecessary_cast solely on
  pristine upstream uses_wsl_drvfs, preserving its portable filesystem-type cast.
  Verify and restore that exact function's original bytes even on failure.
  This validation resource allowance never enters the native patch or artifact.
  Historical0.160.0 preview assets remain protected
  until their clients exit and are not a second source/build authority.
- Actual source build qualifies the exact official upstream source/version,
  pristine slash inventory, external dependency lock, bounded native patch,
  frontend FD remaps and actual TUI behavior before publication. The builder
  refuses a frontend version mismatch. Manager remains optional. An unqualified
  newer version cannot silently publish without `/switch`; retain the prior
  admitted generation until the new pair is qualified.
- The producer build interface accepts `--manager-tui RAW_FRONTEND` only paired
  with `--manager-tui-version UPSTREAM_VERSION`. Both are absent for historical
  or Core-only builds. A declared frontend requires exact backend-version
  equality and one of these exact browser-helper layouts. The builder privately
  snapshots a regular executable static AArch64 ELF within its existing512MiB file bound,
  applies only the shared FD33/34 remap policy (counts2,1,1,1;54 changed bytes),
  and publishes the adapted executable as helper2 with its actual digest.
  Source/lock/command/version/native qualification belongs to the real producer
  gate; a caller-supplied version or ELF check alone never closes qualification.
- Public production pins a successful native qualification run, its exact rewrite
  source and upstream version. Before construction, it authenticates that run's
  repository/workflow/push source and successful completion through GitHub, checks
  the producer's maintained native inputs against that qualified source, and
  verifies the downloaded executable digest, source/version/lock audit and actual
  nonzero native proof with identical formatted/linted patches. The163 gate also
  requires executed upstream tests for retained162 worktrees, pinning, copy,
  scrolling, hyperlinks, startup defaults, provider capabilities/compaction,
  CodeMode settlement/search, Retry-After and patch line endings. Each topic
  needs an actual PASS. CodeMode tests use the exact authenticated official
  package host used in production, not a separately built V8 runtime.
  Artifact existence
  or a failed/partial run is insufficient. The qualified executable and minimal
  proof are retained as an exact-digest GitHub Release source-input asset, never
  overwritten after publication. The producer checks the pinned archive digest
  before extraction and still verifies the successful run through GitHub. This
  input has no runtime pointer or installation authority and does not depend on
  expiring Actions artifacts. The builder retains sole ownership of
  ELF/FD snapshot adaptation. An ordinary newer backend without this qualified
  pair is ineligible for public production and leaves LKG unchanged; historical
  explicitly bounded acceptance fixtures retain their historical construction.
  An explicitly authorized manual rebuild may publish the authenticated current
  qualified backend at the next sequence. Full public candidates require the paired
  frontend; the independent base lane does not. Scheduled production requires a strictly newer qualified backend;
  unsupported versions leave LKG unchanged. Failed staging cannot replace any
  published assets or promote stable.
  The migration bridge remains available as immutable GitHub Release assets.
  Historical Pages remains within its existing1GiB bound. New shared base/add-on
  payloads use immutable Release-backed transport; see the publication contract. Offline
  intervening strict-Core clients materialize the exact nine bridge Release files
  in a disposable TMPDIR root, restore numbered helper names/executable modes,
  and use the existing authenticated `codex update --local` path once before
  ordinary stable updates. No trust key, runtime pointer, downloader entrypoint
  or service is added. The disposable download is cleaned on success or failure.
  Two brief stable promotions alone are insufficient for clients that miss the
  preparatory release; a transient Pages bridge URL is not durable migration.
- Ordinary eligible interactive Core launch selects this frontend only when its
  signed helper and same-generation Manager are usable. Core supplies its existing
  qualified local-server socket address and exact Core entrypoint. A bounded,
  noninteractive query of that exact signed helper may advertise
  `codex-termux-resolved-startup-v1`. The hidden frontend-only
  `--termux-backend-capabilities` query returns that exact line before configuration,
  terminal initialization or server connection; it performs no work or inference.
  Only an exact successful response within two seconds enables deferred startup.
  Core then passes `CODEX_TERMUX_BACKEND_STARTUP=resolved-v1` and the planned native
  socket address without creating a server. The frontend loads upstream
  configuration and requests exactly its resolved catalog or the native/default
  server before connecting. Older/unknown helpers keep the original eager startup.
  The Core-owned local target bypasses the TUI's upstream daemon auto-start and
  package/feature recovery paths, including before the resolved server exists.
  Only Core's coordinator may materialize that target; a missing socket is not
  permission to enter an upstream installation or embedded-execution path.
  Upstream's read-only shared-feature compatibility check still runs against the
  resolved server before connection admission; an incompatibility is explicit,
  without an installer, restart prompt or embedded fallback.
  Absent Manager, Core-only and explicit remote/embedded routes retain their paths.
  No capability file, persistent probe cache or generation-format change is added.
  Unsupported explicit upstream modes/options retain the official runtime path
  and original argv.
  Core owns no picker, account transition, title composition or terminal controller.
  Missing or changed optional frontend/Manager on an installed generation disables
  that optional UI while leaving independent Core usable. Candidate admission and
  publication require every declared asset and digest; an incomplete candidate is
  rejected. The frontend is never executed from an unqualified asset.
- Local-derived update from a frontend-bearing generation retains its admitted
  frontend only for the exact same backend version and a usable same-generation
  Manager/frontend. Its authenticated baseline and ordinary ephemeral local signing
  bind that preservation. An unpaired newer backend or unavailable declared asset
  refuses local construction and retains the current generation; it never silently
  publishes a frontend-free replacement. Official paired updates remain available.
- Install, update, activation and rollback use the existing complete generation
  transaction and signed inventory, including the helper. Failure retains the
  current complete generation. Rollback selects the matching assets of its target;
  a historical pre-TUI generation retains its historical upstream-only behavior.
  Existing clients keep their executable and jobs. The old preview can be removed
  only after its remaining native clients have exited, without terminating them.
- Qualified launch, upstream command probes and doctor capture derive browser
  aliases only from the generation's declared complete browser-helper pair.
  Historical signed generations declaring neither helper remain executable in
  the base qualified Termux environment, including under a hold-aware rollback
  launcher. They do not acquire invented helpers or helpers from another
  generation. A partial pair still refuses environment construction, and missing
  or changed declared assets still fail ordinary inventory qualification. The
  current builder and public producer continue to declare the complete pair.
- Explicit rollback selects the authenticated retained previous generation in
  either direction. If it reactivates an equal or higher signed release sequence,
  it installs that generation's matching Core/assets and clears obsolete hold and
  guard state only after activation commits, through ordinary hold finalization.
  Reactivating the held sequence retains its normal authenticated hold and removes
  the rollback guard, just as an explicit force retry does. A public rollback hold and legacy
  Core guard apply to movement to a lower sequence; a local-derived current still
  creates neither. Failed selection preserves the prior launcher, pointers and
  controls. This explicit retained selection does not relax ordinary channel
  update, signature, inventory or force-retry rules.
- `/switch`, permissions, current-writer/source-return guards and title-independent
  identity retain their accepted native/Manager contracts below. Manager and AI
  recognize only the exact qualified generation frontend path for binding/focus;
  titles and fixed native-window identifiers remain separate from conversation ID.

Admission requires real public-path install/update/rollback/fault proof; device
observations are reported separately from automated qualification.

### Independent unsigned base qualification

The reusable producer also has an explicit `base_only` qualification mode. It
constructs only the qualified upstream runtime, Core, Code Mode host and browser
helpers through the existing release-builder. Manager and TUI are absent;
their compilation, downloads and qualification do not gate this mode. Complete
base inventory, digest/mode checks and actual Android/AArch64 Core/runtime smoke
remain required. A base result is not an integrated product qualification.

This mode is read-only with respect to publication. It refuses `publish=true`
before artifact acquisition; signing, Release/Pages staging, stable promotion and
device activation are never reachable. Its unsigned CI artifacts have distinct
base-qualification identities and cannot replace an integrated candidate. It
adds no update channel, installed pointer, runtime mode or public CLI command.
The main publication path still requires the complete qualified backend/frontend
pair and retains the current generation on an unsupported update.

Base qualification may run from an exact rewrite/rust-core push or manual dispatch
with read-only repository permissions. Source must equal the event's immutable
commit. `rebuild_current` selects the version and source archive digest from the
authenticated stable descriptor independently of TUI pins; otherwise the
ordinary official upstream discovery is used. An unsupported upstream artifact
still fails existing builder qualification, rather than weakening patch admission.
Skipped integrated construction distinguishes a non-newer release from missing
frontend qualification or an archive mismatch; it must not report all three as
already up to date. Independent base qualification and feature-preserving public
promotion are different guarantees; section4.6 defines their final boundary.

## Native /switch behavior

Manager owns execution-profile selection within the upstream conversation UI.

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
ordinary-launch network work or a new background service.

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

The native frontend source has two owners: pristine upstream files receive only
the maintained integration patch; standalone TUI implementations live as
ordinary source under frontend/manager-tui/src. The single maintained source
assembler preflights the patch and refuses to overwrite any upstream file with
an owned source file. Qualification binds both inputs, the assembler and tests
to the exact successful source. Final build inputs and artifact proof still
include the complete assembled TUI diff; no source is downloaded or modified at
device launch. This source organization does not change public behavior or relax
the upstream-Core/dependency, executable, lint or native behavior gates.
The upstream start function delegates resolved-catalog binding to the owned
frontend adapter. The App provider-transition hook owns active-work guards,
thread persistence and same-terminal re-entry; provider/profile routing and
configuration composition live in the owned model module without App or server
lifecycle access. Existing permission/reviewer inheritance, local profile
precedence, installed-route validation and model admission remain unchanged.

- The sole added public slash name is `/switch`, for Manager execution-home
  selection; `/profile` is not an alias. Upstream `--profile`/`-p` configuration
  layers, `/resume`, `/agents`, `/permissions` and every existing slash name/alias
  retain upstream meaning. Provider/model selection is a separate capability.
  Before applying the native patch, the source build must inspect the pristine
  pinned upstream command inventory, including serialize/to_string aliases, and
  refuse a `/switch` collision or an unrecognized naming representation. After
  patching, every upstream variant/name/alias must remain identical and Switch
  must be the sole added variant/name. This gate is mandatory in the real hosted
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

## 1. Product definition

The product provides one public `codex` entrypoint that runs the official
upstream Codex CLI correctly on supported Termux environments.

It consists of two strictly separated layers in one repository and release
system:

1. **Core** — a minimal native Rust compatibility, execution, installation,
   update, diagnosis, activation, and rollback layer.
2. **Manager** — a separately implemented convenience layer reached through
   `codex termux` for profiles, sessions, notifications, and related Termux UX.

Manager remains optional for ordinary launch. Its separately built artifact must
qualify before entering a signed generation with matching Core and runtime.

This is a clean rewrite. Legacy source is historical evidence, not an
implementation dependency or migration base.

## 2. Product priorities

In descending order:

1. preserve a working, recoverable upstream Codex execution path;
2. never damage system resolver, auth, profile, session, or installed runtime
   state;
3. preserve upstream process behavior at the execution boundary;
4. make installation, update, activation, and rollback crash-safe;
5. keep Core small, fast to build, and fast to execute;
6. add Manager conveniences without giving Manager ownership of Core state;
7. optimize implementation velocity without weakening acceptance gates.

## 3. Public command contract

The launcher classifies only an exact first argument of `update`, `doctor`, or
`termux`. Every invocation with one or more arguments that is not one of those
Core/Manager routes is passed to upstream Codex unchanged after the existing
Termux launch planning. The exact zero-argument bare `codex` invocation has one
narrow Core preflight exception: only when stdin, stdout, and stderr are all
terminals may Core perform the bounded signed-stable update discovery described
below before launching upstream. `update` is a Core-owned safety boundary: the
installed wrapper must never execute the upstream distribution updater, because
that updater can install an unadapted runtime on Termux. The wrapper release
pipeline obtains the official upstream package, applies the accepted Termux
patch, qualifies it, binds the matching Core artifact, and publishes a signed Core-plus-generation bundle; the
installed Core obtains and activates only that signed adapted bundle.

| Command | Owner | Required behavior |
| --- | --- | --- |
| `codex` | Core then upstream | on an all-TTY zero-argument launch, perform only the bounded signed-stable advisory check/prompt below, then execute the active upstream runtime; otherwise execute upstream immediately |
| `codex [UPSTREAM_ARGS...]` | Core | for every non-empty upstream argument vector, execute upstream with original arguments without startup update discovery or prompting |
| `codex --version`, `codex -V` | upstream | print exactly the upstream version output |
| `codex update` | Core | resolve the signed stable wrapper release channel; only transport-level channel unavailability may fall back to an official-source local-derived build signed by a fresh ephemeral device-local key, activated while preserving the official `update_key`, and never published |
| `codex update --help` | Core | print the wrapper-owned update usage without invoking upstream or changing state |
| `codex update --force` | Core | retry exactly the authenticated held public release sequence for this invocation while bypassing only the local rollback-hold comparison; a valid matching hold is required and signature, digest, release-sequence, candidate-probe, and activation checks remain unchanged |
| `codex update --build-local` | Core | resolve the exact official upstream stable metadata/archive, build and probe one local-derived generation, bind it to the authenticated public baseline with a fresh ephemeral device-local Ed25519 signer, activate it without changing `update_key`, and never invoke official signing or publication |
| `codex update [INVALID_ARGS...]` | Core | reject unsupported updater options or combined selectors without invoking upstream or changing state |
| `codex update --local <DIRECTORY>` | Core | verify, stage, probe, and activate one compatible signed official/operator bundle through ordinary public-authority admission; it is not a local-derived import surface |
| `codex update --remote <HTTPS_BASE_URL>` | Core | acquire one immutable signed Core-plus-generation bundle and activate it through the local coordinated path |
| `codex update --rollback` | Core | atomically activate the retained previous complete signed generation; record a public update hold only when rolling back from an authenticated public generation, while rollback from a local-derived current creates no public hold or rollback guard |
| `codex doctor [OPTIONS]` | Core | combine upstream and Termux diagnostics |
| `codex doctor --color` | Core | explicitly request colored human diagnostics on a TTY, including when an outer Termux wrapper supplied `NO_COLOR` |
| `codex termux [COMMAND]` | Manager boundary | invoke the Manager artifact or report it unavailable |

`codex version` is not introduced. Wrapper/Core/Manager version rows must not
be appended to upstream `--version` or `-V` output.

### Independent terminal return and optional tmux

Codex Manager owns Codex profile selection, launch convenience, notifications and
foreground conversation binding. Core remains independently usable.
The Core-to-Manager handoff removes inherited LD_PRELOAD and LD_LIBRARY_PATH,
as ordinary upstream launch does, so the optional route cannot reintroduce an
Android linker failure. Caller preferences and workload authentication remain intact.
Manager routing must not reinterpret upstream argv or make ordinary execution
depend on Android connectivity. AI consumes these contracts and owns its separate
provider conveniences. The private __terminal-launch-command-v1 MODE -- PROGRAM
ARGV... adapter routes an explicitly selected same-UID local executable through
the same window/container path without interpreting provider authentication or
profile semantics. A read-only __terminal-launch-capabilities-v1 query advertises
this exact command grammar; older Managers retain their previous client path.
AI passes its existing authentication supervisor intact, rather than replacing
its recovery behavior with a second implementation. No new daemon, watcher or PTY management service is added.

`codex termux launch [--profile PROFILE_ID] [--tmux=off|hidden|status]
[-- UPSTREAM_ARGS...]` presents the existing profile inventory when no profile is
explicitly selected. Up/down and Enter select; Esc/Ctrl-C cancel without work.
An inherited external CODEX_HOME remains a distinct current-home choice. Explicit
profiles use the existing profile contract. CWD, raw upstream argv and caller
color preferences including absence are preserved. Default is tmux off.
`codex termux tmux ...` remains a shorthand for the hidden tmux launch.
A hidden/status container request inside an existing tmux client is rejected
before profile selection, state publication or work. No nested tmux session,
new tmux window/pane, implicit detach or replacement workload is created. Use
the existing container or a separate Android terminal for an independent launch.
AI follows the same refusal even when an older Manager lacks the adapter.
Ordinary interactive Codex launches use the same optional window routing without
showing a profile picker. Noninteractive and maintenance commands retain their
ordinary execution behavior.

Window capability and execution container are independent:

| Android capability | tmux off (default) | tmux enabled |
| --- | --- | --- |
| Native origin/validate/focus API | Run in invoking terminal; return to it | Attach an independent tmux session there; return selects its pane and that terminal |
| Confirmed unsupported native API, working AM socket | Create one uniquely named dedicated terminal per launch; run directly there | Create one uniquely named dedicated terminal per launch; attach tmux there |
| Missing, failed or inconclusive AM capability | Keep ordinary execution usable; report exact return unavailable | Keep tmux usable; report exact return unavailable |

The native extension uses the existing same-UID AM socket and app session table.
Origin is resolved from the real socket peer ancestry. Validate/focus require the
exact live handle/PID/kernel-start association. Closed, recycled or ambiguous
references fail without creating a window. A transport acknowledgment alone is
not semantic selection success. Names, titles and tmux IDs are not native addresses.

The stock-APK route uses RunCommandService shell_name and no-shell-with-name. Its
immutable random launch name is separate from the visible title and conversation
UUID. Initial construction is allowed once at launch; workload arguments and
selected environment travel through a private, single-consumption local descriptor,
not through notification actions. The same single-consumption descriptor carries
raw argv and the caller environment into tmux, preventing old server environment
from changing account selection or authentication. Opaque launch descriptors are
limited to 64 KiB, private 0600, removed before execution and never logged; they
may temporarily contain caller-supplied authentication environment. Permanent
terminal bindings contain no such values. Native terminal identity variables are
kept from the new terminal/container, not copied from another launch. Return revalidates the live launch root and
foreground binding before selecting that name with an inert executable. It never
contains a Codex start/resume/replay command. Stock API has no atomic focus-existing:
manual rename or a target closing between validation and service selection can
create an inert empty terminal instead of returning to the workload. This stock limitation must not be
reported as equivalent to the native existing-handle guarantee. Failure must not
retry construction or switch routes automatically.

The private process-bound record contains only process start identity, native
foreground FD identity, terminal reference and optional tmux pane/socket identity.
It is mode 0600 under the private Manager terminal directory, bounded and checked
against actual live same-UID processes. It contains no conversation content,
credentials or launch arguments. Dead records are pruned lazily. A notification
resolves exactly one live foreground UUID; missing, ambiguous, closed or recycled
targets refuse return. /new, /resume, /switch, profile changes and /title cannot
replace the originating terminal. Visible titles remain presentation. Detached
tmux work may be reattached through normal tmux commands.
`codex termux attach SESSION` explicitly reattaches an unattached Manager-created
session and binds return to the new connection terminal without restarting work;
already attached or foreign sessions are rejected. Changing execution mode
applies to a new safe launch, not migration of a running bare PTY.

Manager owns `notify set --focus window|termux`; default is window. `tmux` is a
legacy alias for window and existing `tmux\n` focus records remain accepted.
The private notifications/focus-v1 contains `window\n` or `termux\n`.
Absent state uses window; malformed/insecure state uses termux. A canonical hook
session_id in window mode selects installed Core's safely quoted private Manager
`__terminal-focus-v1 UUID` action. No AI executable is needed. Malformed or
ambiguous IDs preserve the ordinary notification and cannot become shell input.
Explicit termux mode brings the app forward without selecting a terminal.

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

AI may use the same native origin reference for its optional tmux containers.
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
This registration applies to bare and tmux frontends alike; tmux environment
variables cannot gate native foreground identity or notification return.
Core forwards only that exact three-argument shape to its same-generation Manager.
FD grammar and caller/pane authority remain Manager responsibilities. Ordinary
Core dispatch and installed signed generations gain no terminal controller.
Manager verifies its direct parent is a same-UID installed native frontend and its
private unlinked foreground descriptor. Bare launches publish one bounded private
process-binding record, with stable PID/start, FD inode and TTY plus the qualified
Android route. In tmux, the same-UID socket and exact pane must contain that frontend
directly or through the installed AI PTY supervisor. The existing pane-local
@codex_terminal_binding_v1 holds native/root PID/start plus descriptor; the managed
session holds its separately validated Android route. Foreground UUID always stays
in the live descriptor, never a persistent conversation index. Selection rechecks
process, FD, pane and window identity.
A bare frontend re-exec may replace its own process binding: the prior private
record must have the same schema, PID/start, TTY and Android route, and the new
unlinked descriptor must qualify against the current native image. Replacement is
atomic; mismatched, foreign or malformed records remain untouched. This preserves
return across `/switch` without changing the original Android terminal.
This command cannot rename a native Android terminal or start a workload.

AI consumes this binding without rewriting it, reads the current full UUID from
that qualified native process's descriptor, and rechecks it at the selection and
Android dispatch boundaries. A thread change remains visible even if Manager
registration later becomes unavailable; stale UUIDs cannot remain cached in tmux.
With a binding present, malformed, stale or mismatching native/root/descriptor
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
codex termux launch [--profile PROFILE_ID] [--tmux=off|hidden|status] [-- UPSTREAM_ARGS...]
codex termux tmux [--profile PROFILE_ID] [-- UPSTREAM_ARGS...]
codex termux attach SESSION
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

The Core-owned update surface accepts exactly no arguments, `--help`, `--force`,
`--local <DIRECTORY>`, `--remote <HTTPS_BASE_URL>`, or `--rollback` after
`update`. No top-level `codex rollback` command is introduced. A malformed
invocation beginning with one of those Core selectors is a Core usage error. The no-argument
form is the primary product path: it first tries the signed stable wrapper channel
and may then run the local release-production path defined in Section 8 when the
channel is unavailable at the transport boundary, is already current, or its
otherwise valid candidate is suppressed only by the local rollback hold. A
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
The release date is the publisher-declared UTC date of signing/preparing that
immutable release, never the device installation time or a filesystem timestamp.
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

## 4. Architecture and ownership

### 4.1 Core

Core exclusively owns:

- public command dispatch;
- official upstream artifact qualification;
- Termux runtime adaptation and launch environment;
- FD 33 and FD 34 preparation;
- final upstream process execution;
- sandbox capability enforcement;
- immutable generation construction and integrity metadata;
- update, activation, last-known-good selection, and rollback;
- read-only Core diagnosis;
- composition of upstream and Termux doctor results;
- the typed boundary through which Manager requests Core operations.

Core must remain usable when Node.js, Manager, network access, update services,
or optional Termux APIs are unavailable.

### 4.2 Manager

Manager owns:

- `codex termux` command UX;
- profile selection and presentation;
- notification configuration and delivery;
- Manager-local state and UI;
- profile execution handoff to the validated Core entrypoint.

Manager's profile UX and declared profile state are standalone Codex product
capabilities, assessed through the public `codex termux` path. Their completeness
is determined by how they support the official upstream runtime and user tasks,
not by features in a separate convenience product.
Conversation persistence, discovery, selection and resume belong to upstream,
as established by the shared-conversation contract below; Manager does not own
a second session index or picker.

The Manager executable is built separately from Core and is an optional signed
generation asset. Core does not compile it, discover it from `PATH`, or probe
it during ordinary launch. Release-builder owns the bounded artifact probe;
Core consumes only the signed generation and its digest-bound Manager path.

Manager must not directly write Core generations, pointers, manifests, locks,
runtime state, resolver data, or activation journals. A Manager request that
would mutate Core state must use a versioned, runtime-validated Core contract.
The Manager may create and update only its declared state and an explicitly
created profile-home directory, plus the no-replace shared configuration-profile
links in a validated selected upstream home described below. It must not copy,
parse, summarize, or rewrite
authentication files, cookies, OAuth material, arbitrary upstream state, or
session transcript content. Profile execution changes `CODEX_HOME` only in the
child environment and returns through the stable Core entrypoint; it never
changes the caller's environment or persistent Core state. Manager commands
that need an update, rollback, doctor, or other Core operation must request the
corresponding Core-owned command through that entrypoint and must not
reimplement the operation.

The Manager artifact is optional and independently qualified. Its absence,
incompatibility, or failure must not make ordinary upstream launch, Core
doctor, Core update, or Core rollback unavailable. Manager v1 has no external network,
package-manager, OpenSSL, bwrap, resolver, or generation-discovery authority.
Profile registration creates only Manager metadata and a private new account
home. Selection affects only the child invocation; execution readiness and
shared conversation-path preparation belong to Core before upstream launch.
Optional shared configuration-profile projection belongs to Manager preparation.

An installed generation's unavailable Manager is reported at the Manager
boundary and by doctor; it does not invalidate the independently usable Core
runtime. Ordinary dispatch checks the declared Manager path for a regular,
owner-executable file without probing it or requiring OpenSSL. Installed-release
verification still authenticates the signed inventory, descriptor and all
Core/runtime assets; a failed optional Manager file check is excluded from that
verification result and local-build carry forward, without disabling Core update
or rollback. This exception applies only to already-installed generations.
Fresh candidate admission, staging and activation
retain complete Manager verification whenever the candidate declares one. No
generation metadata, signed inventory or user state is rewritten to express absence.

Manager is a separate Rust executable. No Manager runtime or dependency may
become a prerequisite for ordinary upstream launch, Core doctor, update, or rollback.

### 4.3 Shared contracts

Core and Manager may share only explicit versioned data contracts. Compile-time
types are insufficient; every external or cross-layer payload is validated at
runtime. Unknown incompatible schema versions fail without mutation.

### 4.4 Product completeness against upstream

The completeness boundary is the public `codex` and `codex termux` product.
Core supplies the supported official runtime, Termux adaptation, process and
shared-server execution, generation lifecycle and diagnosis. It must preserve
accepted upstream command/configuration behavior and account isolation; optional
convenience failures must not become prerequisites for ordinary execution.
Candidate publication and activation still require the complete signed inventory
and qualification; optionality does not permit admitting an invalid candidate.

Manager supplies standalone user-facing configuration and selection conveniences
above that boundary. A useful convenience must produce a distinct user result,
not another implementation of a Core command. Profile selection must clearly
distinguish the identity supplied to the upstream child from saved selection
history; history alone is not evidence of the identity used by a later ordinary
launch. Authentication, conversation persistence and thread lifecycle remain
upstream-owned. Session conveniences, if selected in a later contract, must use
upstream-supported discovery/resume rather than a second transcript index.

Notification policy, upstream hook rendering and Termux delivery belong to Manager.
Core combines its execution defaults with a bounded configuration fragment from
the admitted Manager; it does not parse Manager notification records or event names.
Delivery failure must
not fail an upstream turn. Public-path proof must establish the intended event
and delivery behavior; a configuration record or generated command alone is not
proof of the user's completed task. Legacy functionality is evaluated against
these user results and ownership boundaries, not as an automatic feature backlog.

### 4.5 Responsibility and compatibility admission criteria

Ownership is determined by the invariant and public execution path, not by
which component currently contains the code. Apply these criteria in order:

1. **Upstream first.** Authentication, conversation schemas, storage engines,
   discovery/resume, writer ownership, thread/turn processing and native server
   behavior remain upstream-owned. Use the supported runtime's native commands,
   configuration and protocol before adding local machinery.
2. **Core execution necessity.** Core owns the minimal adaptation needed to run
   a qualified upstream runtime correctly on Termux, including direct launches
   without Manager. Binding the selected account home, compatible runtime,
   resolver/configuration descriptors and safe socket namespace is an execution
   invariant. It may require Core preparation even when Manager selected the
   account. Manager must not become the only route that establishes it.
3. **Policy versus application.** Account selection, notification preferences
   and their presentation are Manager conveniences. A selected product policy
   may be applied by Core when every supported launch must satisfy it. That does
   not make the policy a Termux limitation, or transfer upstream state ownership.
   Installation-wide conversation sharing is the already-selected product
   policy below; it is not required for arbitrary isolated account execution.
4. **Concrete insufficiency.** Retain an adaptation only with evidence tied to
   the exact supported upstream revision: the native surface attempted, the
   observable limitation, the smallest remedy and the public path it serves.
   Historical audits and current web documentation alone do not establish a
   limitation of the installed or next candidate runtime. Revalidate affected
   assumptions during qualification before deleting or expanding an adaptation.
5. **One preparation owner.** Give each physical preparation/mutation one
   authoritative implementation. Manager owns profile registration and its
   private metadata; Core owns shared execution-path preparation. Collapse
   duplicate preparation when aligning the current profile contract. Independent
   validation of a cross-layer payload remains necessary; identical operations
   or policy drift are not justified by that validation requirement.
6. **Minimal lifecycle authority.** Core may bind and launch upstream's server
   to preserve signed-runtime authority and Termux execution compatibility.
   Native protocol, threads and writers remain upstream-owned. Custom reuse or
   retirement is admissible only for a demonstrated runtime-binding or generation
   retention invariant that the selected native surface cannot establish.
   Do not add a parallel supervisor, session registry or storage engine.
7. **Distinct convenience result.** Manager functionality needs an observable
   user benefit beyond renaming Core/upstream operations. Recovery authority
   remains Core-owned; a repair facade with no distinct recovery outcome is a
   deletion candidate, not a reason to add a second recovery implementation.
8. **Public-path proof.** KEEP necessary behavior, COLLAPSE duplicate execution
   paths, DELETE superseded machinery. For each disposition verify the real
   public entrypoint, including Manager-unavailable execution, protected account
   state, original argv/process behavior and signed update/rollback integrity.
   A synthetic fixture proves routing, not native upstream compatibility.

In particular, account homes are valid upstream execution identities. A long
Unix socket path is a communication-path constraint, not a reason to relocate
authentication/configuration or share conversations. Shared conversation links
and SQLite configuration implement the separate selected sharing policy; the
canonical files, schemas and locks continue to be operated by upstream.

These criteria govern bounded alignment slices. Each specific command/state
contract is amended before its implementation changes. Profile preparation and
effective identity are defined by profile contracts; notifications belong to
Manager and recovery belongs to Core. These criteria authorize neither data migration nor
an installed-runtime change.

### 4.6 Final component boundary and Core independence

The minimum product is the qualified upstream runtime plus Core's Termux
compatibility and authenticated generation lifecycle. Manager, the maintained
TUI and any Chat provider are optional components, not successive mandatory
launch stages. Manager can operate without the enhanced TUI; the provider exposes
an upstream configuration route without requiring its model picker.

| Component | Owns | Must not own |
| --- | --- | --- |
| Upstream | Authentication, inference, tools, reviewer/approval engine, conversation schemas, thread/turn state, compaction, resume and writer locks | Our generation trust or Android window identity |
| Core and release-builder | Qualified executable/environment/FD/socket binding, unsupported-sandbox enforcement, generic execution-home lease, one physical shared-layout binding, signed inventory, activation/retention/update/rollback, authenticated release identification and base diagnosis | Account selection/registry, notification policy, model/provider catalog interpretation, Chat transport or TUI interaction |
| Manager | Account/config-profile convenience policy, task discovery/handoff UX, notification hooks/delivery, native window registration/return, optional tmux and title forwarding | Runtime pointer/trust mutation, another updater/executor, conversation storage/schema or model inference |
| Maintained frontend | Narrow upstream UI integration for /switch, supported permissions, /model catalog/routing and title callbacks; upstream renders canonical transcripts | Provider login/transport/lifetime, generation mutation or a second backend supervisor |
| Optional provider | Provider catalog/capabilities, auth context, request/tool-return transport, retry and owned-host lifetime | Core runtime selection, canonical conversation DB/writer ownership or native tool execution |

Core retains two deliberate bindings beyond Android syscall adaptation. The one
physical shared-layout/SQLite preparation preserves the accepted sharing policy
for the three supported homes, including a first direct launch without Manager;
arbitrary homes stay isolated. The one signed-server coordinator enforces the
runtime/home/environment identity, safe endpoint, current-client retention and
opaque configuration-byte binding. Native server protocol/turns remain upstream
owned. The qualified native model/list API cannot replace a server's startup
catalog, so frontend resolution precedes that bounded Core request. Neither
binding authorizes account selection or model-schema interpretation in Core.

Release identification follows that existing lifecycle boundary. The producer
owns signed edition/base/date provenance; Core's updater validates and renders
it alongside its update result. This path requires no Manager executable, RPC,
configuration or source dependency. A base installation can identify its release
without Manager. A full installation with unavailable optional Manager still
identifies as full from its authenticated manifest, not from executable presence.
Display metadata is informational and never selects components or relaxes admission.

### Approved next architecture: independent execution and optional integrations

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
operations. A saved account selection that cannot be fulfilled must not silently
become another account; implementing this changes the existing optional-default
fallback contract and requires coordinated tests/documentation. Safe preparation
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

The Chat provider source lives under `providers/chatgpt`, outside crates/core,
Manager and TUI. Its approved implementation target is a native Rust adapter
process, usable through upstream configuration without Manager or enhanced TUI.
Rust owns configuration/capabilities, Responses HTTP/SSE and tool mapping,
conversation/context correlation, cancellation/reconnect/retry policy, CDP
communication and owned-host lifetime. The provider may use the signed app's
authenticated context; it does not replace that app's login or inference.
Its failure must not disable native Codex, account commands or window management.
Reuse existing upstream provider configuration and bounded local contracts; do
not introduce a speculative general plugin framework. Unify component discovery/
status and lifecycle presentation where appropriate, not credentials, transcript
storage, process ownership or unqualified update mechanisms.

The existing JS/Bun and Python runtime is a migration reference, not the target
production architecture. Keep only the JavaScript required to execute inside the
app renderer for the current app-owned function transport, plus optional viewer
assets. Rust controls and validates that bridge through CDP; no token extraction,
second tool executor, duplicated configuration validator or task/retry/lifetime
policy belongs in the injected script. Define a versioned, bounded bridge contract
and reject incompatible app capabilities before an uncertain submission. This is
not a claim that the entire desktop app is Rust or JavaScript, nor a promise that
changing the provider's language bypasses the host's limits.

The planned Rust package is providers/chatgpt/Cargo.toml with runtime code in src/
and separately inventoried app-side bridge/assets. It must not be a Core, Manager
or TUI dependency. Workspace membership must not make Base acquire/build the
provider or desktop app. Replace product-path Bun/Python dependencies only after
their behavior is qualified in Rust; development/qualification scripts may remain
Python or shell. Use one selected adapter per launch, not automatic dual-runtime
execution or fallback after uncertain dispatch. Old signed generations remain
explicit rollback targets. Preserve installation identities, login/profile data,
remote conversation associations, live jobs and current configuration overrides
through bounded migration; source integration alone never activates a replacement.

Native automatic reviewers have their own Provider lanes and remain outside the
configured ChatGPT project. Recognize upstream guardian_review thread metadata;
send only that native reviewer's supplied context, never the primary app lane's
conversation or implicit project instructions/files/memory. This transport
isolation does not determine the native approval policy or reviewer decision.
An existing project-bound lane cannot be relabeled as an isolated reviewer.

New independent MCP trials use the repository's `Termux Codex` connector display
name, with display/connector identity configurable rather than hardcoded branding.
Historical `Codex Native2` references remain provenance only. Renaming never changes
an outstanding native tool call/result identity. Reuse authorized existing admin
and computer-use access when exercising this path; actual connector ACK remains
an independent gate from discovery and the local-function transport.

The Rust MCP return path is explicitly selected at service startup through
transport.return_path=mcp and a configurable transport.connector_name (default
Termux Codex). Local functions remain the default; never switch transports after
uncertain dispatch. The same Provider service exposes a bounded streamable-HTTP
/mcp endpoint; it is not a second executor. Static discovery discloses no native
state. The same binary's optional mcp-stdio command verifies an already running
owned Provider and forwards bounded multiplexed JSON-RPC for an existing tunnel;
it never starts an app/service, repeats delivery or executes tools. EOF/request
cancellation detaches only that observer. A random per-app-turn capability authorizes native_catalog/native_tool;
each logical call ID binds exact tool/input/report, independently of JSON-RPC IDs.
Native execution/results still pass through the common Responses/lane journal.
Persist invocation intent and native results before publication/delivery. Peer
loss detaches its waiter, not the native action; an exact retry receives the same
result. Changed IDs/arguments, stopped turns and unresolved owner changes refuse.
Return native results to the retained original MCP invocation, never submit them
as a new Chat user/tool message. A retained app turn cannot silently change model,
effort or developer policy. Catalog discovery may extend declared native tools;
new user input waits for a settled boundary. Account/project/stop/transcript rules
are identical to the local-function path. Host confirmation/authentication must
surface as blocked state, never be bypassed. Rust wire fixtures alone do not admit
this route for live use; actual connector ACK is required before activation.

Experiment integration targets rewrite/rust-core as a repository, not crates/core
as a code destination. Port exact reviewed provider/runtime sources, dependencies,
licenses and affected regression drivers onto current authority. Never replace
current Core, Manager, UI, release workflows or contracts with experiment snapshots.
Preserve source history and user changes; compare intent and current equivalents
before replaying a commit. Separate runtime modules from qualification/research
material. Do not ship historical probes, GUI trials or user app/login data as
runtime dependencies merely because they share an experiment directory.

The initial runtime/tests/license port is already on rewrite/rust-core; Rust
conversion and experiment integration are not complete. Port the remaining useful
qualification drivers alongside the affected Rust implementation, preserving their
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

After completing the no-MCP transport, the sustained development trial uses the
real `~/prj/ai` checkout directly. This explicit user-approved source-development
trial does not use a temporary worktree, clone or project copy. Inspect that
repository's AGENTS/contracts and Git baseline, preserve unique uncommitted and
untracked source, and let the ChatGPT-backed native Codex perform a real sequence
of analysis, implementation, tests and failure repair. The supervising agent must
not substitute its own implementation for the evaluated model. Preserve unrelated
user work and operating authentication/data/runtime.
The supervisor launches and follows the trial, independently reviews its actual
source changes and runs the applicable validation; recording a future task or
accepting the evaluated model's own success report is insufficient.
Native transcript, exact
effects, interruption/reconnection/latest-state continuation and meaningful test
results qualify the path; synthetic replies alone do not. A live budget and
separate submission/result/retry counters remain required; automated Pro is
excluded. Git protection is not permission to reset or discard existing changes.


Base excludes all optional components. Full composes qualified Manager/frontend;
the provider remains opt-in and must not require signed desktop app/rootfs download
or login on ordinary update/startup. A stable provider needs a declared versioned
payload/dependency inventory, authenticated delivery and tested activation/rollback
before replacing its experimental installer. Preserve app authentication and user
state; importing provider source does not authorize redistributing third-party
desktop software. Choose the bounded packaging extension from its real reachable
payload; do not assume the current eight-file inventory already supports it.
Once the Core interface work is admitted, Manager/provider-only changes must leave
base inputs unchanged. Upstream/backend changes still require the applicable
frontend and execution-protocol compatibility proof.

### Approved provider configuration, context and remote project behavior

Quota switching settings must present a compact, grouped interface. Each selectable
row occupies one terminal line even at narrow supported widths; explanatory help
must not become wrapped per-row descriptions. Keep all existing policy controls,
account enrollment, custom ranking and session override accessible. Clipping must
preserve the selected action/value and must not change the underlying account ID.
Verify actual rendering and navigation, not only the generated labels.

The proposed single user-owned provider configuration is
`$XDG_CONFIG_HOME/codex/providers/chatgpt.toml`, defaulting to
`~/.config/codex/providers/chatgpt.toml`. The path/schema becomes public only with
its implementation. Provider owns validation, effective settings and rendering
to native upstream configuration/catalog; generated profiles, models.json and
immutable installation modules are outputs, not additional editable authorities.
Explicit invocation overrides win, then per-model settings in this file, then
file-wide settings and declared defaults. Preserve ordinary upstream overrides
and show their effective origin rather than silently rewriting user configuration.
Existing --profile chatgpt remains a generated upstream fallback. Provider's
candidate `prepare --home PATH` projects a native catalog and command-auth profile
for an explicitly selected existing Codex home. One private shared profile is a
generated output of Provider configuration; per-home links never override a user
file or different/dangling link. Manager may project the same shared registry to
new managed homes, but is not needed for Provider preparation. Native defaults
and unrelated config/auth remain unchanged. Generated catalog snapshots are
immutable; settings edits apply to projections at the next explicit preparation,
and active backends retain their bound catalog. A model route reload in the
running Provider does not require regenerating metadata when its public contract
is unchanged. Native Codex
model defaults/account configuration are not modified to configure this provider.

Expose show/effective, validate, edit and atomic field-setting operations through
the provider's existing invocation surface; optionally present the same operations
in TUI and Manager. Do not introduce another permanent launcher, require Manager
for file-based configuration or make a TUI implementation a second validator.
Validate before atomic commit; failed edits preserve the old configuration.
Show invalid configuration and unapplied changes clearly. No settings read/edit
may submit Chat or create a project. Unknown keys/types and inconsistent limits
must be diagnosed, not silently ignored. Avoid secret values in status output.

The candidate `host.rootfs` field selects an explicitly provisioned official-app
Linux rootfs. Rust owns a private persistent app home beneath provider state, an
exclusive profile lease, a temporary X server and Chromium FD3/FD4 transport.
Resolve `proot` and `Xvnc` from the Termux prefix, with fixed isolated arguments;
never interpret a configured shell command or bind the user's host home into the
guest. No implicit app/rootfs installation or profile copying occurs. Preserve
app home/authentication on shutdown; remove only owned temporary resources.
A lost host with unsettled lanes is not permission to replay their requests.
Bind each lane to the app's currently authenticated user/account/workspace and
owned host generation using identifier-only app state, never a token dump. Check
that identity before warm submissions/results and again after a completed app
round, before publishing executable calls, committing a summary or declaring
completion. Account/host changes during inference must not authorize later native
effects. An account mismatch refuses before sending canonical history or results;
restoring the original account reconciles the existing round without resubmission.
A settled lane may be rebuilt from current
native canonical input at a new native user turn after its app context disappears;
retain the old remote chat. A model/effort-only change is not a new user turn.
Pending/uncertain rounds and executable cached calls must not be replayed into a
replacement host. Report that reconciliation is required and preserve their state.
A cached settled final response can still be returned locally without inference.

A definitely rejected, single local gateway call may receive a structured tool
error on its original host call ID, allowing the model to correct an unadvertised
name or malformed gateway envelope. Validate its entire batch before publishing
any executable call; rejected batches have no partial native effects. This is a
tool-result continuation, never a new user message or a resend of the original
request. Permit at most three such correction continuations per Responses request,
record each rejection before returning it, and expose the rejection in the public
transcript. Additional model starts remain real usage and must be counted.
Uncertain delivery, identity/host/protocol failures, ambiguous multiple host calls,
compaction controls and replayed call IDs never enter this correction path.
Native function argument validation stays native; ordinary malformed arguments
are not rewritten by Provider. Exhaustion returns a specific terminal failure.

Public model text may translate known Chat presentation components for native
terminal Markdown: remove MemoryCite component tags and render box containers as
plain text with an optional title. Preserve their useful contents, unknown markup,
literal inline/fenced code and exact tool arguments/results. Hold incomplete known
tags while streaming so an emitted prefix never needs rewriting; retain malformed
or unfinished literal text at final settlement. This projection never requests
another model turn, exports private reasoning or changes native execution policy.

The private relay descriptor remains an explicit qualification connection option,
not a fallback or required Bun/Python production dependency.
An explicit owned bridge disposal may restore only its own installed hooks after
all its lanes have been released. Refuse disposal while any lane remains; never
remove another owner's hooks/state or close the shared app as qualification cleanup.

The Rust configuration tool remains a candidate developer surface until product
admission. Its effective output distinguishes file values from defaults and
reports runtime application as not queried; saving a file does not establish
that an installed adapter consumes it. The source Rust server reloads this one
file at request admission. Models output distinguishes implemented candidate
transport from production qualification. A field
transaction may set several dependent values together, validated once before
commit. Unsetting a field restores its declared default. Concurrent tool edits
serialize; an external editor changed since opening must cause a conflict rather
than silently overwrite newer content. No-op changes preserve the existing file.
An editor failure or invalid buffer preserves the original. Use private bounded
files, reject symlink/nonregular destinations, and publish without hard links.
The project fields are history.mode, history.project_name,
history.project_id and history.create_if_missing; account/workspace-scoped bindings are
added only with qualified host discovery. Retained history remains the migration
default; choosing project mode requires a name and defaults creation to enabled.
Neither saving that choice nor listing models proves a host operation is available.

The model catalog is validated data in the same provider configuration, under
catalog.models, rather than a Rust list of model families or effort mappings.
Each entry declares its public ID/display name and effort-to-host-model route,
with an optional host thinking-effort value. Supply initial defaults as a data
resource; users can add or correct mappings through configuration without a
binary rebuild. The binary owns protocol/schema validation and lifecycle safety,
not the list of models. Listing a configured route proves neither account
entitlement nor successful execution. The initial Rust runtime remains unavailable
until actual transport acceptance. Exclude GPT-5.6 per the latest user decision;
retain GPT-6, manual-only Pro use and the hidden legacy Instant alias. A host API
or tool-return protocol change can still require a new binary/bridge version.
Use the selected model's effective catalog record at an idle boundary; changing
catalog data must not mutate an active lane or replay a pending request.

Editable context controls distinguish native context_window_tokens (also projected
to the appropriate upstream catalog fields), automatic-compaction trigger/headroom,
and encoded/decoded transport byte limits. Support model-specific values. The
user's current1,000,000-token override is preserved as an explicit override; it is
not a verified hosted Chat context capacity or proof that no compaction is needed.
Do not treat the user's 1m override as a measured maximum or impose an arbitrary
32k minimum. Validate positive upstream-compatible integer values separately from
the hard encoded/decoded transport ceilings. An absent token threshold delegates
to upstream defaults; it is not an automatic-compaction disable switch.
No proven transport/model limit is bypassed by inflating a catalog number. Native
compaction caused by model/hash changes is separate from token thresholds; a
disabled token-trigger setting must not promise to suppress every compaction.
Changing a startup catalog creates/reuses only the matching qualified execution
binding; do not hot-edit an immutable package or an active sibling's metadata.

The1m override was a workaround for the user's observed reply/compaction/new-Chat
summary-transfer cycle on successive prompts. Investigate and reduce that cycle,
not just token-trigger frequency. Distinguish compaction reason, checkpoint
acceptance, lane retirement, actual Chat creation and context retransmission.
A checkpoint is acknowledged only after canonical history replacement, not merely
because identical summary text already appears in the old prefix. An acknowledged
checkpoint must be applied once; warm requests must reuse valid
owned conversation/context and send only needed deltas. Do not suppress necessary
compaction, discard history, replay a submitted request or promise that stored
host history increases the model's real context capacity. Deterministic regressions
must cover repeated checkpoints, unchanged prefixes, compaction failure, restart
and model/effort changes before budgeted live qualification. Report message
submissions separately from tool returns and model starts.
Recognize compaction from upstream request metadata and its explicit trailing
`compaction_trigger`, rather than guessing from user prose. Text-summary requests
return ordinary summary text; remote V2 returns one Provider-issued `compaction`
item containing an `ocx1:` public-summary envelope. This envelope is readable
Provider context, not encrypted OpenAI reasoning or a way to decode foreign opaque
checkpoints. Never execute tools in a compaction round. A transient summary prompt
and reply do not become the accepted native prefix. Keep the pre-summary canonical
history until upstream replaces it with the exact offered checkpoint. Retire that
settled remote lane once after replacement, preserving its chat and pending-call
integrity; an unchanged old prefix containing identical summary text is not an ACK.

Cross-provider entry may use a readable public-history handoff at an idle primary
thread. TUI reads the complete upstream public transcript and asks the installed
Provider to encode it; upstream alone persists the resulting contextual user
message through `thread/inject_items`. This is not a new user turn or inference,
another history store, a developer instruction, or an opaque checkpoint. Keep
the original native items and thread ID. The message uses upstream's ordinary
external-context wrapper and remains readable by the previous provider if entry
fails. Exclude reasoning and internal metadata; reject unsupported public items,
incomplete pagination, active work and mismatched thread identity. Imported tool
events are quoted historical observations, never calls to execute again.

Only an explicit, validated handoff in the same thread or inherited through
upstream-declared fork/parent lineage can supply the readable replacement for
preceding foreign opaque input. Lineage metadata correlates quoted history; it
does not authenticate an app account or grant permissions. A child keeps its own
native ID and Provider lane, including after cold resume and nested forks.
Unrelated threads without lineage still refuse a mismatched handoff.
Preserve current system/developer
policy and contextual user settings; retain all later input. Never claim to decode
the old opaque state. The Provider's opaque compaction format is not compatible
with upstream OpenAI compaction: do not advertise a fabricated common compaction
hash. Hash absence does not disable token/window compaction. Reverse entry must
first obtain an upstream-owned readable replacement while the current Provider
still owns inference; failure or interruption keeps that Provider selected.

First admission may wait for the same owned app target and authentication to
finish loading. Bound this read-only wait; never create a target, change an
account, restart a live owner or infer merely to test readiness. Distinguish a
missing startup target from multiple matching app targets; ambiguity refuses
immediately. A local listener readiness proof is not model readiness.

Public message phase is not determined by the app's channel name alone. A
`final` channel message with `finished_partial_completion`, even with
`end_turn=true`, remains commentary before its tool call. Stream its text once;
an unfinished message may promote from commentary to a final answer only after
`finished_successfully` plus `end_turn=true`. Preserve one item ID and text,
never duplicate prose or rewrite already closed output. Emit terminal Provider
failures as `response.failed` with a structured response error that native Codex
actually reads; a top-level `error` event alone is not sufficient. Retain only
static content-free failure codes in the private lane record.

The provider should accept a ChatGPT project destination, resolve it in the
currently authenticated Chat account/workspace and create it when absent, as
configured. This is a remote ChatGPT project, not the local CWD, a Manager login
profile or an OpenAI API project. Fields are `history.mode=project`,
`history.project_name`, `history.create_if_missing` and optional `history.project_id`
for an exact existing destination. The configured destination is the default for
new provider-owned conversations. Existing conversations are not moved/deleted
implicitly. A project/retention change applies to new lanes at a safe boundary;
model/effort changes should reuse an existing owned conversation when supported.

Host account/workspace identity plus exact project ID binds remote placement.
Reuse an unambiguous owned/readable matching project; duplicate names, missing
permissions and inaccessible IDs require a useful error, not another account or
silent root-level chat. Serialize owned creation and reconcile uncertain results
before retrying. A definite server rejection is distinct from uncertain delivery;
only the latter blocks another creation until reconciled. Use the app's omitted
defaults instead of supplying nullable memory/sharing options. Verify project
membership before sending the first user request;
if creation succeeded but placement failed, report the partial state and retain
the known project for reconciliation. Do not create one project per tool result.
Retire/replace a local-function lane only when its pending tools/results are
settled. Scope any durable remote-ID mappings to native thread/lane ownership;
these are lifecycle bindings, never a second canonical transcript database.

Projects may add shared instructions/files/memory and therefore affect model
behavior. Do not silently change global personalization, shared-project access,
MCP permissions or the native reviewer policy. Verify tool declarations and
instruction priority with project context, including guardian child isolation.
Temporary chat and retained project history are alternative modes; do not claim
their combination is available before the signed host proves it. Project grouping
reduces sidebar clutter, not message count, stored history size or allowance use.
Retain history by default. Any later archive/delete/temporary retention option must
be explicit, scoped and independently qualified; never mass-delete existing chats.

Official product documentation supports projects and shared project context, but
does not prove this signed-app adapter can create or select one using its local
function return transport. Read-only host capability/operation discovery precedes
implementation; then prove project create/reuse/readback without model sampling.
Verify the declared destination before starting inference and read back actual
conversation membership after the budgeted native tool round trip. A new remote
conversation may not exist on the server until its first admitted model request;
metadata-only preparation must not be reported as persisted conversation placement. Use the signed app's own auth and
supported app behavior; do not extract tokens or invent unsupported endpoints.
If the current app cannot express the operation, report the exact boundary and
qualify an owned UI fallback before any ambiguous submission; do not silently
abandon the configured project. References reviewed2026-10-18:
[projects](https://learn.chatgpt.com/docs/projects?surface=app) and
[project selection](https://learn.chatgpt.com/docs/reference/slash-commands).

Settings and project acceptance cover fresh/config-profile launch, /model choice,
same-provider model/effort change, cold resume, account/workspace change, concurrent
lanes, duplicate names, deleted/inaccessible projects, uncertain creation and
configuration migration/reinstall/rollback. Inspect actual ChatGPT project
membership and native TUI reports/tool effects, not just HTTP success or counters.
Count setup, creation requests, user-message objects, tool returns, model starts,
retries and compaction separately; actual Chat allowance debits remain unknown
without suitable evidence. Project operations/settings must not spend inference.

### Automated upstream preparation and base/full publication

Upstream handling must resolve one exact official version/archive/source tuple,
prepare its Termux compatibility inputs, and drive separate base and full release
lanes automatically. A scheduled discovery controller resolves a newly published
stable upstream version, exact source and official archive digest, updates the
single input authority without force, and explicitly starts its read-only proofs.
Qualified base publication must not wait for Manager/enhanced-TUI porting or proof.
Unsupported transformations stop with actionable evidence and keep the last known
good head; source edits are never invented by a publishing script. Failed optional
frontend work cannot block source/Core qualification or basic publication. Device
activation follows its existing user/update policy, not a forced running-session
replacement. Base contains Core plus the adapted upstream default UI;
full adds admitted Manager and enhanced TUI. The opt-in Chat provider remains
independent. Both lanes require actual Android execution, affected behavior proof,
signing, immutable assets, complete public readback, disposable update/rollback
proof and non-forced promotion. Source preparation or a green host build alone
cannot authorize publication. An unsupported or ambiguous transformation stops
that candidate with actionable evidence and preserves the accepted installation.

Version/source/toolchain and qualified frontend references have one machine-
readable authority. Preparation must not guess binary offsets or accept an
unqualified version by removing its guard. Prefer repeatable source/semantic
transformations; any generated artifact policy remains bound to exact inputs and
Android proof. Historic policies needed by retained generations remain supported.

Qualification build/test jobs remain read-only. An isolated completion relay may
only enqueue the accepted-input workflow with its run ID; it has no content-write
permission or signing key. The receiver waits for that run to finish, then checks
owned exact-source qualification independently. Completion delivery is not proof.
Native trigger metadata and the exact isolated relay are excluded from recipe
comparison only while build/test code consumes no event/dispatch input; global
settings and all actual build/test steps remain bound. Native builds start through
the accepted-input controller, reusing a matching build already in progress.

Provider-only source changes under providers/chatgpt use the independent optional
provider gate and do not rebuild unchanged Core/Manager or make Bun a Core
prerequisite. During Rust migration, qualify the Rust package and minimum app-side
bridge together, retaining legacy tests as differential evidence until equivalent
coverage passes. Missing Rust/device/bridge gates are unfinished, not a reason to
fall back to legacy admission. After cutover, the provider gate must not require
Bun/Python merely to execute the production adapter. Scope Cargo lock/toolchain/
shared-build changes by their actual consumers; do not exclude changed shared
inputs from Core qualification. Classification still requires a successful accepted predecessor;
otherwise the conservative full Core gate runs. Changes to Core/shared build inputs
or mixed unknown paths retain full qualification. Provider qualification is separate
from Core acceptance and must be required before future signed provider admission.

Base and full have independent signed public heads. The existing update-index-v1
remains the full head for compatibility; the base head is update-index-base-v1.
The default update head follows the authenticated public baseline of the selected
generation, including a local-derived generation; an explicit update URL remains
an operator override. Full users must never be silently downgraded when only base qualifies. Both public
heads and retained last-known-good assets stay available in their immutable
publication origins; current releases do not replace the legacy Pages site.
Promotion coordinates both heads with non-forced compare-and-swap and increasing
release sequence; a stale concurrent run re-evaluates rather than overwriting
another lane. No device activation or user-profile/provider mutation is implied
by public publication. Qualification-only workflows remain non-publishing.

codex-runtime-binding is dependency-free shared **source**, statically compiled
into its callers, for the existing bounded execution-record codec. Core depends
on release-builder and this primitive, not on Manager, frontend or provider.
It creates no installed shared library, service, registry or optional-component
runtime prerequisite. Core configuration queries remain bounded and optional;
failure skips conveniences while preserving independent execution.

Independence has distinct observable levels:

- **Build and qualification:** Core/base construction and Android execution do not
  compile, acquire or qualify Manager, enhanced TUI or provider. Unsupported native
  runtime revisions still require their own Termux qualification.
- **Installed execution and lifecycle:** a valid Core-only generation supports
  ordinary upstream execution and Core doctor/update/rollback. Unavailable optional
  assets in an already-installed generation do not disable those operations.
  Doctor may separately report missing authentication or conveniences. Fresh
  candidate admission still rejects every missing/changed declared asset.
- **Component reuse:** the builder accepts an independently admitted Manager;
  local reconstruction already carries forward its bytes after the Core-API
  probe. A compatible unchanged Manager need not be rebuilt to change Core.
  The integrated CI build may still compile both; that is not a Core dependency.
- **Feature-preserving public updates:** stable remains one signed combination.
  Preserving /switch and the enhanced /model/permission UI requires its frontend
  to pass the qualified backend protocol/behavior gate. Exact backend-version
  admission remains until a broader pair is actually proven. Failure retains the
  accepted generation; base qualification alone never silently removes features.
  There is no automatic downgrade from full to base; each edition follows its
  own signed public head.

The retained TUI patch contains only upstream call sites/types, supported-permission
integration, startup compatibility and actual test access; standalone policy lives
under frontend/manager-tui/src. Upstream lacks a qualified general extension API
for these UI hooks. Moving them into Core or a separate launcher would not remove
revision maintenance. The official-runtime binary adaptation remains exact-digest
and version-bound for FD/socket/config paths and no-sandbox permission choices;
the source-built frontend's FD remap is separately qualified. Never remove either
gate merely to admit a newer upstream version.

Responsibility realignment is complete when these ownership and independent
execution/lifecycle guarantees pass. It does not claim that future upstream
compatibility, public integration/promotion, same-provider in-place selection or
provider feature acceptance is already complete. Those changes belong to their
respective components and require affected-path proof.

## 5. Termux runtime contract

The first supported release target is `aarch64-linux-android` on a supported
Termux installation. Release users must receive a prebuilt Core and must not be
required to install Rust, Clang, or Cargo.

Core must derive runtime paths from the actual environment and must not embed a
single app data path as product authority.

Before final upstream execution Core must:

- construct the qualified runtime environment without leaking package-manager
  or preload variables;
- preserve stdin, stdout, stderr, TTY behavior, signals, and upstream exit
  status;
- open the selected resolver source read-only and make it available on FD 33;
- make the process-local managed configuration directory available on FD 34;
- expose the qualified temporary directory read-only on FD 35 for the bounded UDS adaptation;
- ensure those descriptors survive the final `exec` boundary;
- use the selected official runtime and compatibility tool paths;
- report unsupported Linux sandbox requests clearly.

Core must never create, rewrite, chmod, repair, or delete
`$PREFIX/etc/resolv.conf`. Resolver diagnosis is read-only.

Linux namespace/bwrap sandboxing is not a Termux capability of this product.
Core must not claim that `read-only` or `workspace-write` Linux sandbox modes
are enforced. Ordinary supported launch uses the explicitly selected upstream
no-sandbox policy; unsupported sandbox requests fail clearly rather than
silently weakening the request.

Core must preserve the accepted upstream argv byte-for-byte, including an empty
bare-launch vector. It must not synthesize `-c`, `--enable`, `--disable`, or
`--search`. The Termux no-sandbox default is supplied through the existing
Core-owned system `config.toml` on FD 34 as
`sandbox_mode = "danger-full-access"`, which is also visible to a qualified
shared background server. User-supplied upstream options remain unchanged;
explicit unsupported sandbox requests still fail before runtime I/O. Hiding a
shared-server warning or disabling the server does not satisfy this contract.

The same Core-owned system configuration supplies the Termux default
`thread_unload_delay_secs = 0`. Upstream unloads a thread only after its last
subscriber disconnects and the thread is inactive; a running turn remains
protected by native lifecycle checks. Closing an idle TUI must therefore release
its native writer without the upstream default 60-second cache delay, allowing
immediate same-ID resume through another account's server. Core never removes
or steals a writer lock, copies a transcript, or overrides explicit higher-layer
user configuration or CLI arguments. Authentication remains per account.
This native setting is read when a shared server starts. Already-running servers
retain their previous delay until normal restart; generation activation starts
the new runtime's server without terminating other active clients or work.

Shared-server reuse must refresh its Core-owned FD-34 configuration files
under the existing namespace coordinator lock before the new client executes.
Publish changed files atomically in the same opened configuration directory;
preserve the server PID, directory inode, credentials and running threads.
Subsequent native config reads and new/resumed thread configuration must see
current Manager hook selection. Do not restart active writers or add CLI
overrides to make notification changes visible.

#### Optional Manager configuration preparation

Optional launch policy is queried only from the admitted Manager through
`__prepare-launch-v1 CAPABILITY -- ORIGINAL_ARGS`. CAPABILITY is `local-frontend`
only when Core can offer that qualified interactive frontend; otherwise `runtime`.
The bounded, no-work query returns exactly `codex-manager-launch-v1\n` followed by
`current\n`, `handoff\n` or `blocked\n`. Manager alone reads account preferences
and decides whether its convenience launcher is needed. Core reads no Manager
preference filename or window recursion marker for that decision. `blocked` stops
without executing another account; unavailable optional preparation falls back
only before handoff, with the unchanged current execution home and arguments.
`handoff` replaces Core with the same admitted Manager's `__execute-launch-v1`
using the original capability/arguments and existing authenticated Core handoff.
There is no post-handoff retry or alternate-account fallback. Manager's explicit
launcher revalidates its selected account before requesting Core execution.

The explicit `__prepare-runtime-v1 CONFIG_DIRECTORY` operation performs best-effort
shared profile projection and returns the configuration envelope below. Core calls
this operation before execution. `__runtime-config-v1` remains a read-only lookup:
it returns the same envelope without creating profile links or changing account
files. Both require the admitted Core handoff and share the same bounds. Neither
starts jobs or performs inference. A malformed saved account preference is an
execution error; it must not fall back to another account. An absent preference
continues to use the native default.

Before final runtime execution, Core may call the admitted generation's Manager
with `__prepare-runtime-v1 CONFIG_DIRECTORY`. This private endpoint requires the
existing Core handoff, never starts a runtime or performs inference/network work,
and receives no upstream workload arguments. It returns UTF-8 bytes beginning with
`codex-manager-runtime-config-v1\n`, followed by the Manager-owned upstream TOML
fragment. The entire response is limited to 8 KiB (inside the existing 16-KiB
shared-server configuration bound) and the call to two seconds;
stdin and stderr are not inherited. Core checks the envelope, byte bound, UTF-8
and absence of NUL, and composes the fragment after its execution defaults into an immutable private
per-launch snapshot supplied through FD34. The shared base directory contains
Core defaults only. Equal complete settings reuse a snapshot; different settings
never rewrite one another. Upstream still owns TOML parsing and configuration
precedence. No account config.toml or credential is rewritten.

Before dispatch, validate optional combined settings using the exact admitted
upstream executable with an inert configuration-loading command, empty private
HOME/CODEX_HOME and working directory, no account credentials, bounded time and
no inference. Cache success only for that immutable snapshot and runtime identity.
If combined syntax/types fail, validate Core defaults and continue on those
defaults only if they pass. Never retry a workload or silently change accounts.

Manager reads its own notification policy and renders the same existing native
completion callback and hook matchers. Missing configuration enables no automatic
hooks; malformed configuration does not become default notification enrollment.
Its system-layer fragment also supplies `approval_policy = "on-request"` and
`approvals_reviewer = "auto_review"` as convenience defaults. Upstream precedence
keeps account/project/config-profile/CLI choices authoritative, including explicit
user review or full access. Core neither selects this reviewer nor interprets
these fields; without Manager, ordinary upstream approval defaults remain usable.
The explicit preparation owns optional shared `*.config.toml` projection from the
private CONFIG_DIRECTORY/profiles registry into the selected upstream home.
Existing local files and dangling links take precedence, and projection does not
copy authentication or conversation data. Projection remains best effort.

An absent Manager causes no call. A failed, incompatible, oversized or timed-out
response is discarded with a concise warning, leaving the Core execution defaults
usable; it must not replay the workload or run an alternate executor. The legacy
`# codex-termux-notify-v1` file prefix remains solely as the ownership marker for
safe replacement of existing Core system configuration. Shared-server snapshots
consume the same complete immutable bytes without a second policy renderer. Managed-home
leases and shared conversation preparation remain Core-owned in this slice.

### Termux permission selection

For a qualified Termux runtime, the upstream TUI permission picker and its
permission shortcuts must preserve the supported no-sandbox execution policy.
`Ask for approval` selects `danger-full-access`, `on-request` and the user
reviewer. `Approve for me` selects `danger-full-access`, `on-request` and
`auto_review`. `Full Access` selects `danger-full-access` and `never`. Changing
reviewer must not select Linux workspace sandboxing or restart a shared server.
Qualified local frontend resume must explicitly carry the effective supported
Termux permission selection even when it came from Core's system defaults, so
saved Linux sandbox settings cannot override that selection. Preserve saved
approval policy and reviewer unless the user supplied a current replacement.
Fork and side conversations inherit the resulting actual parent selection.
Reject an effective explicitly restricted/custom policy before a new local
thread/turn request; never silently reinterpret it as unrestricted execution.
The Termux picker and shortcuts exclude the unsupported Read Only choice;
explicit CLI sandbox requests remain Core usage failures. User-supplied
named/custom profiles retain upstream semantics. This adaptation
must not impose a profile allowlist that silently falls back from an explicit
restricted request to unrestricted execution.
The UI must describe the actual no-sandbox permissions; auto review evaluates
only approval requests and does not imply that every unrestricted command is
reviewed. Read-only and explicitly requested workspace sandbox policies remain
unsupported; a compatibility adaptation must not reinterpret such explicit
requests as full access. Core does not own approval decisions or add synthetic
CLI overrides. Manager is not required for this capability.

A release-specific UI adaptation requires exact official source and artifact
qualification, bounded byte changes, actual TUI selection and effective native
thread-setting proof before release. A default-setting test or a direct API
request alone does not close the picker requirement. Existing installed
runtimes remain unchanged until signed publication and ordinary activation are
separately accepted.

Core never invokes, installs, downloads, or repairs `bwrap`. An explicit Linux
sandbox-policy rejection is a Core usage/policy failure with process status 2
and must occur before resolver/configuration descriptor setup, upstream
execution, generation construction, bootstrap publication, or activation-state
mutation. A failure of a development runner's own sandbox is infrastructure
evidence only and is not a product-path result.

## 6. Artifact and patch qualification

The sole upstream input for the first supported target is the exact versioned
OpenAI asset
`https://releases.openai.com/codex/releases/<version>/codex-package-aarch64-unknown-linux-musl.tar.gz`,
where `<version>` is an explicitly selected stable `MAJOR.MINOR.PATCH` value.
Release production supplies that version, a local regular-file copy of the
archive, and its exact lowercase SHA-256. Mutable `latest` or channel names,
discovery, mirrors, and source fallbacks are not artifact authority.

Acquisition and adaptation are release-production work. The wrapper release
pipeline retrieves the exact official package, supplies its pinned digest to
the real non-installed workspace executable named `codex-release-builder`,
and signs the resulting adapted generation for delivery. The installed Core
does not compile itself, install a Rust toolchain, or execute an upstream
self-updater. It contains the same bounded release-production routines needed
by the no-argument local fallback; those routines may fetch the official
archive, adapt the raw runtime, qualify the result, and sign it with the
already-authorized update key. The fallback never accepts a raw archive as an
activation candidate and never bypasses signed local admission. The builder's
`fetch` operation
accepts one explicit stable `MAJOR.MINOR.PATCH` version, constructs only the
canonical official archive URL above, and downloads that archive with bounded
HTTPS transport into one absent local regular-file output. It prints the exact
lowercase SHA-256 for the subsequent build invocation. `fetch` performs no
channel discovery, mirror selection, source fallback, signing, generation
activation, or live-state mutation. The builder's `build` operation accepts the
version, archive and digest, generation identity, Core artifact, creation
metadata, and an optional regular executable Manager artifact through
`--manager <ABSOLUTE_FILE>`, plus an absent output directory. When supplied,
the Manager is copied into the generation root and bound by
`manager_artifact_digest`; when omitted, the descriptor uses `-` and no
Manager file is emitted. It performs no discovery, signing, activation, or
live-state mutation and emits an unsigned Core-bearing generation source for
the `codex-release-v4` signing and delivery path. Core's local fallback carries
forward the currently authenticated Manager artifact, when one is present,
so a qualified Manager is not silently lost on an upstream update.
`codex-release-v2` remains implementation history and is not retained as a
release compatibility path.

Release production then exposes one non-installed `publish` operation with the
exact form:

```text
codex-release-builder publish --generation <ABSOLUTE_DIRECTORY> \
  --release-sequence <POSITIVE_DECIMAL> \
  --release-base <HTTPS_BASE_URL> \
  --private-key <ABSOLUTE_FILE> \
  --openssl <ABSOLUTE_EXECUTABLE> \
  --output <ABSENT_ABSOLUTE_DIRECTORY>
```

`publish` accepts the current generation layout: a real directory containing
`generation.meta`, the executable Core artifact `core`, `runtime`, and the
root-level `codex-code-mode-host`, with an optional root-level `manager`; all
present entries are regular non-symlink files. A v3 source without `core` is
accepted only for legacy Manager-less publication compatibility. The descriptor must
be a qualified `codex-local-generation-v2` with the supported Android/AArch64,
Core API, persistent-schema, and root-companion bindings. The generation
identity must be a safe URL path component and the supplied canonical HTTPS
release base must end in that encoded identity. The release sequence is a
positive decimal and the base follows the same bounded canonical URL grammar
as `codex update --remote`.

The operation derives one raw Ed25519 public key from the supplied private PEM
using the explicit OpenSSL executable. It emits the non-rotating v3 form only:
the derived key is written as `release_public_key`, `release.sig` signs the
exact `release.manifest` with that key, and no `release-authority.sig` is
created. Therefore an operator producing an ordinary public/non-rotating signed release
with this command must supply the current Core `update_key` private key; this
operation does not implement key rotation. Core's local-derived path is a
different in-process authority boundary: it generates a fresh ephemeral key and
never reads an official release private key. The `publish` command's supplied
private key is read only for derivation/signing, is never copied into the output,
and is never written to repository or device state.

The complete publication output is:

```text
<output>/update-index-v1
<output>/update-index-v1.sig
<output>/releases/<generation_id>/release.manifest
<output>/releases/<generation_id>/release.sig
<output>/releases/<generation_id>/generation.meta
<output>/releases/<generation_id>/core
<output>/releases/<generation_id>/runtime
<output>/releases/<generation_id>/codex-code-mode-host
<output>/releases/<generation_id>/manager                  optional
```

The index contains exactly the four records and final newline defined below,
with the supplied generation identity and release base, and its sibling
`update-index-v1.sig` signs those exact index bytes with the same key. The
publication manifest containing `core` uses `codex-release-v4`; v4 keeps the
v3 control fields and inventory grammar but requires the executable `core`
asset. A v3 publication without `core` remains readable for already installed
legacy generations, but it is not a valid new Manager-bearing update. The
release manifest contains a lexicographically sorted inventory, lowercase
SHA-256 digest, and four-octal-digit regular-file mode for each present
generation file: `generation.meta`, `core`, `runtime`, and
`codex-code-mode-host`, plus `manager` when the optional Manager artifact was
supplied. The `core` digest must equal the generation descriptor's
`core_artifact_digest`; a Manager entry requires `core`. The local
`releases/<id>` tree is the directory to map to the URL represented by
`release_base`; `publish` performs no network upload and does not assume that
the URL is hosted by OpenAI.

Official release production is not a Core runtime behavior. `codex update`,
`--force`, `--rollback`, `--local`, and `--remote` never build or sign an
official release, never read an official release private key, never invoke `gh`,
and never stage, deploy, or promote public stable. The presence of
`CODEX_TERMUX_UPDATE_PRIVATE_KEY`, the former device-local maintainer key path, or
an authenticated `$PREFIX/bin/gh` account is inert to Core update semantics.
Signed-channel exact-current and rollback-held results therefore remain consumer
outcomes; they do not trigger ambient upstream discovery or release production.

Official source production remains an explicit, non-installed tooling boundary.
The repository's `codex-release-builder` `fetch`, `build`, and `publish` commands
may be orchestrated by an authorized producer: `fetch` selects and verifies the
exact official upstream metadata/archive, `build` performs the required Termux
adaptation and qualification, and `publish` constructs the signed immutable
release/index publication tree using an explicitly supplied signing authority.
Those commands do not make `codex update` a producer and do not themselves grant
network publication authority. GitHub-hosted scheduling, secret-backed signing,
and Release/Pages promotion require the complete publication gate.

The maintained release workflow runs from one exact accepted source selected by
the main publication caller. Candidate construction and Android/AArch64 smoke
are read-only and have no signing secret or public mutation authority. Signing,
immutable Release/Pages staging, public consumer proof and stable CAS are separate
jobs on the same admitted payload. Normal scheduling is six-hour; manual dispatch
is available for non-publishing qualification or explicitly authorized publication.

All candidate/component transformations and corrective bindings must complete
before the unsigned transfer archive is created. Package the final admitted
candidate once immediately before artifact upload; never transfer an earlier
archive of a directory that was subsequently changed. The archive must preserve
the admitted file inventory, bytes and modes, including the deferred Manager
marker until native smoke removes it. Smoke packages only its final qualified
candidate, and signing/public readback must consume that same qualified payload.
This invariant applies equally to ordinary, Core-only and Manager-only releases;
a successful directory check cannot qualify stale exported bytes.

For any authorized official producer, an explicit version selector must be one
stable `MAJOR.MINOR.PATCH` value; otherwise the bounded official
`https://releases.openai.com/codex/channels/latest` metadata selects the
`rust-v<version>` tag and digest for the exact
`codex-package-aarch64-unknown-linux-musl.tar.gz` asset. The metadata is a
version/digest selector, not an activation authority. The producer must download
the exact versioned archive and compare its digest to the metadata-bound digest
before adaptation. Missing, malformed, non-stable, mismatched, or unavailable
metadata fails closed; no mirror, package manager, mutable raw runtime, or
upstream self-updater is accepted.

By contrast, `codex update --build-local` and a transport-unavailable bare-update
fallback never read that key path, `CODEX_TERMUX_UPDATE_PRIVATE_KEY`, or any
repository release secret. They generate one fresh Ed25519 key in private
owner-only temporary storage, derive its public verifier, sign only the local
immutable candidate, delete the private key before activation can succeed, and
remove the complete private staging tree on success or ordinary failure. They
never invoke `gh`, create a publication store, dispatch a workflow, or advance a
public stable pointer. The local-derived generation uses the authenticated public
baseline release sequence rather than allocating a new public sequence. Its
signed generation provenance is exactly the semicolon-delimited record
`codex-local-derived-v1;upstream_version=<VERSION>;archive_sha256=<SHA256>;public_generation=<ENCODED_ID>;public_sequence=<POSITIVE_DECIMAL>`; the signed
generation descriptor independently binds the exact upstream version, source
digest, Termux patch report, Core digest, and compatibility identities.

The local-derived special admission is reachable only from this in-process
official-source construction path. `codex update --local` and `--remote` retain
ordinary public-authority admission and cannot turn an arbitrary self-signed
directory into local-derived state. An active authenticated rollback hold blocks
local-derived construction so the single bounded `previous` slot cannot evict
the public generation that the hold protects. Temporary archive/build material
is private, bounded, and removed before success is reported. Local-derived work
never invokes, installs, selects, or repairs `bwrap`.

Core runtime has no official publication path and never invokes
`$PREFIX/bin/gh`. Any later authorized official producer must keep signing and
publication authority outside Core; GitHub authentication alone never authorizes
publication.

The six-hour schedule authorizes publication only when independently authenticated
official upstream stable is strictly newer and its frontend/backend pair is
qualified. Equality is a no-op; older/malformed/unavailable or unqualified inputs
cannot publish. Manual publication requires its explicit authorization input.
Scheduled and authorized manual releases share qualification, production-key
signing, immutable Release staging, complete HTTPS readback,
disposable update/runtime/doctor/no-op proof and non-forced exact-parent CAS.

When such a producer is authorized to publish, its complete official candidate
targets the fixed wrapper repository `humtr/codex` and the selected stable
publication ref. A release whose
complete signed file inventory is flat may use
immutable GitHub Release assets under a tag equal to the validated generation
identity; the signed index's `release_base` is then the matching
`https://github.com/humtr/codex/releases/download/<generation_id>/` asset base.
GitHub Release asset names are not authority to rename signed relative paths: if
any authenticated release file contains a `/` path component, the automated
Release-asset publisher must fail closed before creating a release or advancing
the index.

New nested-path publication uses immutable GitHub Release assets behind the fixed
`https://codex-releases.humtr.workers.dev/<generation_id>/` logical file origin
for `local-relay-hosted-*` and `local-base-relay-hosted-*` generations. Historical
signed Pages URLs and existing assets remain unchanged. The publication validator
chooses canonical origin by namespace. New publication never redeploys the legacy
Pages site, raises its size guard, or evicts a retained public generation.

Base owns Core, adapted upstream runtime/default UI, code-mode host and the two
browser helpers. Full pins one exact authenticated base and adds Manager/enhanced
TUI plus its own descriptor and signatures. Each full result retains one complete
signed v4 inventory and one atomic current/previous activation lifecycle. There
is no device overlay, second updater, mutable latest-base dependency or component
selection policy in Core. Manager/TUI-only changes must not advance the base head
or rebuild unchanged base executables; full-combination Android/runtime proof is
still required. Core/shared input changes invalidate base reuse.
If new base publication succeeds while full integration fails, the new base stays
stable and users retain the previous qualified full combination. This serving
fallback never becomes the automatic repair target: repair/rebuild of add-ons
always targets the latest authenticated successful base. Each build pins that
base immutably. Automatic publishers are serialized; promotion rechecks its exact
main snapshot, including the base head. A superseded snapshot cannot promote:
the next qualification resolves the latest successful base afresh, rather than
modifying the in-flight composition or automatically publishing against an older
base. A failed job calls the owner rather than entering a blind rebuild loop.
Base is never rolled back to satisfy optional UI.
Final unexpected automation failures produce one deduplicated user attention
notification identifying failed stage/logs, retained base/full and latest repair
target; expected negative qualification cases are not failures. The notification
path cannot execute repair, weaken proof or modify installed state.

An exact-input component sidecar is signed with the existing production key and
binds generation, manifest digest, full source SHA and the base build-input
fingerprint. The fingerprint covers Core/builder/binding sources, relevant build/
lock dependencies, upstream/archive and Core toolchain, not Manager/TUI policy.
The producer verifies both the sidecar and base file inventory before reuse;
version equality or a successful unowned workflow alone never permits reuse.

The full Release contains only its additional/replaced files and a bounded
`asset-map-v1.json` pointing common files to that exact base Release. The transport
map is not trust authority; the complete signed inventory authenticates every
logical file. The redirect adapter accepts only GET/HEAD, canonical bounded local
generations, fixed humtr/codex Release assets and exact admitted paths. It rejects
encoded/unsafe paths, queries, missing/foreign maps and cycles. It never proxies
payloads, chooses stable, handles credentials or controls device state. It needs
no secret/binding/storage or per-generation deployment. Administrative deployment
uses existing account credentials locally, never grants CI another credential.
Verify deployed source identity and public negative/positive requests before use.

Release CI stages immutable assets, verifies exact composition and complete public
HTTPS readback through the logical origin, runs disposable Android fresh install/
update/no-op/rollback proof, then non-forced stable promotion. Old clients retain
their ordinary HTTPS fetch and signature validation; they may download common
files again. Core's generic acquisition may instead copy a hash/size/mode-verified
owned retained file into private staging without hardlinks, then apply unchanged
complete-candidate admission. Unavailable/corrupt files fall back to authenticated
download. This optimization never interprets account, UI or rotation policy.

Both current lane heads, rollback/compatibility targets and every base referenced
by a retained full release must be preserved. Deleting an unreferenced retired
release requires explicit retention scope and a reference check; no date-only
cleanup, published-asset overwrite or silent deletion is authorized. Immutable
Release delivery avoids the Pages aggregate-image cap, not all hosting limits.

Until the stable compatibility floor is newer than R10, the public stable target
must itself use the exact R10 browser-helper bridge layout even when its Core is
newer **and** its signed descriptor must carry the bounded legacy activation
doctor signal `upstream_doctor=unsupported` with exact
`creation_metadata=r10-browser-helper-bridge-v1`. Both parts are required for a
retained sequence-7 R10 client to consume the final stable generation directly:
the historical R10 activation gate otherwise treats user credential/provider
doctor health as candidate integrity and rejects a credential-free candidate
before activation. The signal does not weaken public doctor semantics in a
corrected Core: only that exact bridge marker plus `unsupported` combination is
remapped on the public doctor route to real upstream diagnostic execution, while
activation-time candidate integrity remains the signed version/runtime probe.
The official producer must therefore retain this signal on every public stable
candidate while the R10 compatibility floor remains supported; dropping it is a
backward-compatibility failure, not a canonicalization cleanup.
The admitted TUI may extend this exact numbered bridge with helpers/2,
helper_count=3 and its same-version termux-manager-tui-v1 identity/digest. Manager
is mandatory for that extension; no other additional helper is admitted. Historical
two-helper releases retain their exact behavior. Stable publication, staging,
Pages retention and public readback require either exact legacy7-file inventory or
exact extended8-file inventory, preserving the legacy activation-doctor signal.
The retained R10 client must directly consume the extended signed target through
its real public update/rollback path before any stable promotion. An ordinary
nested-browser frontend is valid local construction, not a stable floor replacement.
A newer canonical local generation may continue to use `browser/open/curl` and
`browser/manual/curl`; canonical nested paths are not an R10-readable public
stable target.

For any later authorized official stable promotion, the small
`update-index-v1` and `update-index-v1.sig` files are promoted together in one
Git tree and one commit on the selected stable publication branch only after the
selected release transport, complete readback, and disposable public-update smoke
are verified. Core runtime does not perform this promotion. The branch update is non-forced
and is based on the exact previously verified head, so a concurrent publisher
cannot be overwritten. Historical explicit publication procedures may retain
their previously accepted ordering, but automatic promotion must not expose a
new index with an old signature or vice versa. A failed, timed-out, incomplete,
path-renaming, Pages-deployment, readback, or pre-commit promotion never advances
the automatic stable pointer. An indeterminate final ref result is reported as
indeterminate rather than assuming either state. The optional best-effort flat
Release-asset publisher remains separate, uses bounded child waits, never uploads
the private key, and does not weaken this automatic promotion boundary. The
account credential and the release `update_key` private key are separate
authorities; account authentication alone cannot authorize a release for Core.

All source files are snapshotted into private staging, revalidated as regular
files, and copied without following symlinks. Final modes are applied before
file synchronization; the complete index and release tree are synchronized
bottom-up and the absent output directory is atomically published with
`RENAME_NOREPLACE`, followed by output-parent synchronization. A failure
leaves no accepted output, preserves an existing destination, and does not
alter the source generation or any live Core state. The official OpenAI
versioned archive remains the only upstream source authority; this publication
tree is wrapper-owned distribution content.

The release builder complete output boundary is durable: final file modes are set before the corresponding final file synchronization, the complete staging tree is synchronized bottom-up, the no-replace output publication is atomic, and the output parent is synchronized before the build reports success. A failed publication leaves no accepted output.

The accepted archive is gzip-compressed POSIX ustar. A per-entry POSIX PAX
header is optional and may contain only `mtime`; all other extended semantics
are rejected. There are at most 256 logical entries, paths are canonical relative
UTF-8 of at most 256 bytes, the compressed archive is at most 256 MiB, each
regular file is at most 384 MiB, and total regular-file payload is at most
512 MiB. Duplicate paths, absolute or dot/dot-dot paths, links, special files,
unknown entry types, malformed headers, and nonzero trailing content are
rejected. Archive ownership and modes are not output authority.

The official archive is admitted by its **layout-version semantic contract**,
not by a version-specific exhaustive resource inventory. Every tar entry must
still use one canonical relative UTF-8 path, be a regular file or directory
(except the already bounded PAX mtime header), be unique, and remain inside the
entry-count, per-file, and total-payload ceilings. Symlinks, hardlinks, special
files, path traversal, non-canonical paths, malformed headers, duplicate paths,
and unsupported PAX metadata fail closed. Unselected archive files are streamed
and discarded; they are never materialized in the candidate generation.

`codex-package.json` is parsed semantically as one bounded JSON object. It must
bind layout version `1`, the requested version, target
`aarch64-unknown-linux-musl`, variant `codex`, entrypoint `bin/codex`, resources
directory `codex-resources`, and path directory `codex-path`. Object member
ordering and insignificant JSON whitespace are not authority. Unknown extension
members may be ignored only when they are valid bounded JSON values and do not
duplicate a member name; changing any required semantic field or the layout
version fails closed.

The archive must contain exactly one regular `bin/codex`, exactly one regular
`bin/codex-code-mode-host`, and exactly one regular `codex-package.json`.
Those two binaries are the only selected upstream executables and must be
little-endian 64-bit static AArch64 ELF files with no `PT_INTERP`. Other bounded
regular files/directories, including additions or removals below the declared
resource/path directories and future non-selected package resources, do not
change the Termux generation and therefore do not by themselves require a
wrapper release-builder change. Linux-side helpers such as `rg`, `zsh`,
`bwrap`, voice resources, or later package resources remain excluded unless a
separate normative contract explicitly selects them.

Patch policy `termux-fd-remap-v1` changes only `bin/codex` through these
equal-length substitutions:

| Source bytes | Required count | Replacement bytes |
| --- | ---: | --- |
| `/etc/resolv.conf` | 2 | `/proc/self/fd/33` |
| `/etc/codex/config.toml` | 1 | `/dev/fd/34/config.toml` |
| `/etc/codex/requirements.toml` | 1 | `/dev/fd/34/requirements.toml` |
| `/etc/codex/managed_config.toml` | 1 | `/dev/fd/34/managed_config.toml` |

Every replacement must occur zero times before adaptation. Missing, extra, or
already-patched occurrences reject the input. The output differs only at the
selected byte positions; its deterministic patch report binds the archive,
raw-runtime, adapted-runtime, and code-mode-host SHA-256 values, the four source
counts, and the changed-byte count.

New upstream 0.160.0 builds use patch policy `termux-fd-remap-v2`. It retains the four exact
substitutions above and additionally qualifies the protected Unix socket root
for upstream 0.160.0 only. The official raw runtime SHA-256 must be
`50b06603bdcdac39b714f5c3e68583c002b8ad8779ebfdaaf4932ff016b379c0`.
The selected AArch64 `shared_daemon_socket_directory` canonicalize argument is
redirected from its inline `/tmp` C string to `/proc/self/fd/35`; its UID format
retains the UID and omits only the `codex-daemon-` prefix to fit the Linux socket
limit. Constants occupy verified zero padding inside the existing read-only
ELF load segment. Exact original instruction bytes, offsets and format bytes
must match before the bounded rewrite; all other instructions, `/tmp` uses,
UID/0700 owner checks and physical-path hashing remain unchanged. Qualification
rejects artifact/version drift or already adapted input. Core supplies a
read-only directory FD 35 for the qualified Termux temporary root before the
server starts; the canonical physical socket path must fit 107 bytes. The patch
report includes the exact UDS policy and total changed-byte count. Historical
v1 generations remain valid rollback artifacts. Other upstream versions require
fresh source/artifact qualification before this server path is accepted.

New 0.160.0 builds for the permission-picker contract use
`termux-fd-remap-v3`. This retains all v2 path/UDS changes and the same exact raw
runtime digest. The additional `termux-permission-picker-0-160-0-v1` policy
adapts only the approval UI's default preset/profile selection and descriptions:
no-sandbox with independent approval policy/reviewer. Linux sandbox profile
resolution and explicit requests are not redefined. Original bytes and offsets
are verified before any rewrite; unexpected or previously adapted input fails
closed. The patch report additionally binds the permission policy and exact
changed-byte count. Historical v1/v2 signed generations remain rollback-valid.
Actual native picker, shortcut, no-daemon and shared-server setting readback
qualify this policy; changes to the upstream version require fresh qualification.

Upstream 0.161.0 qualification is the selected current compatibility extension.
Its official archive SHA-256 is
`3c02e2ae34be0d06e62557e98fc5c0a783bec5a2fed406fe00e565803bf84ee8`,
raw runtime SHA-256 is
`0129f94f4f1197bd75f6c4265889061f386ed302bbc50c6e57e3e0660dfb28a7`,
and official source tag commit is
`979011409de0a60b52f179721948e65531d26144`. A new exact-artifact
`termux-fd-remap-v4` record must retain the four common FD remaps and qualify
only that runtime's UDS and built-in permission UI with their own exact source
instruction/string/padding checks and original byte ranges: 22 non-overlapping
blocks change exactly 432 bytes; the unchanged common FD policy changes 54 bytes,
so the complete v4 report must record `changed_bytes=486`. Official debug symbols
are bound to the stripped runtime by `.gnu_debuglink` CRC32 `96904dc1`.
Its identities are `termux-uds-0-161-0-v1` and
`termux-permission-picker-0-161-0-v1`; behavior keeps the same FD35 owner-safe
physical socket/107-byte bound and independent no-sandbox approval/reviewer
contract above. It must reject raw-artifact/offset/instruction drift or already
adapted input before mutation. Existing 0.160.0 v2/v3 and older v1 generations
remain readable, publishable and rollback-valid. The legacy v1 policy applies
to versions through 0.160.0, retaining its already published pre-UDS bundles.
Each newly built qualified version must bind its matching policy, raw digest,
identities and count, and v4 also binds the official archive digest. Unknown
newer versions still require fresh qualification. No Linux helper, excluded resource, browser login,
new session index or experimental source frontend is selected by this extension.
Source/byte qualification, actual native menus/settings and owned launch/server
proof must pass before this version can reach hosted signing or stable promotion.

The unsigned output contains exactly `generation.meta`, the adapted `runtime`,
and an unmodified root-level `codex-code-mode-host` beside `runtime`; the first
target declares zero `helpers/<index>` artifacts without changing that optional
generation contract. The root-level placement is required because upstream
Codex resolves this companion beside its own executable, not through `PATH`.
Every output is create-new in private staging and the absent destination is
published complete-or-absent. Failure never publishes a partial generation,
changes an existing destination, signs or activates content, or writes outside
the selected output parent.

The release builder accepts only a regular executable Core artifact of at most
64 MiB. It must enforce this bound before and during its private snapshot, so a
source that grows after initial inspection still cannot produce an unbounded
copy. The resulting descriptor binds the exact snapshot digest.

An upstream runtime is accepted only when all declared inputs and outputs are
bound in a generation manifest:

- upstream package identity and version;
- immutable source artifact digest;
- expected platform and architecture;
- exact patch-policy identifier and patch report;
- resulting runtime and helper digests;
- Core and optional Manager artifact digests;
- Core API and persistent schema compatibility;
- qualification result and creation metadata.

Binary adaptation must verify expected source occurrences, reject already
patched or unexpected layouts, and compare the result with the declared patch
policy. Upstream layout drift fails before activation.

Archive extraction must reject absolute paths, traversal, escaping symlinks,
special files, duplicate conflicting entries, and writes outside staging.

## 7. State and generation model

Code/artifact generations and mutable user state are separate.

The Milestone 2 local layout is:

```text
$PREFIX/bin/codex                                      stable public entrypoint
~/.local/lib/codex/core/generations/<id>/             immutable complete generation
  generation.meta                                     versioned local descriptor
  release.manifest                                    signed release/integrity inventory + release key
  release.sig                                         candidate-key Ed25519 signature over exact manifest
  release-authority.sig                               rotation-only current-key signature over exact manifest
  core                                                authenticated Core launcher for v4 bundles
  runtime                                             patched upstream executable
  codex-code-mode-host                                first-target companion beside runtime
  manager                                              optional Manager executable
  helpers/<index>                                      optional helper artifacts
~/.local/lib/codex/core/generations/.acquire-*/        private incomplete remote source; never activatable
~/.local/lib/codex/core/publications/<id>/             locally built signed publication cache
~/.local/lib/codex/core/release-public-key.pem         bootstrap-only initial trust seed; never update fallback
~/.local/share/codex/core/activation-state            authoritative generation + bounded trust state
~/.local/share/codex/core/activation-journal[.tmp]    crash-recovery transaction state
~/.local/share/codex/core/activation-state.tmp        atomic state publication temporary
~/.local/share/codex/core/core-entrypoint-rollback    one retained prior Core launcher
~/.local/share/codex/core/core-entrypoint-rollback.meta
                                                      rollback binding for that launcher
~/.local/share/codex/core/config/                     process-local managed config directory
~/.local/share/codex/manager/                         Manager-owned mutable state
```

The Manager v1 state root is independent of the Core root:

```text
~/.local/share/codex/manager/
  profiles/<PROFILE_ID>/profile.meta                 Manager profile record
  default-profile-v1                                 explicit default account preference
  profiles/<PROFILE_ID>/home/                        selected upstream CODEX_HOME
  notifications/config-v1                            notification configuration
```

`profile.meta`, `default-profile-v1` and `config-v1` are versioned Manager records and
contain no tokens, cookies, OAuth values, private keys, or session bodies.
Manager creates profile directories with mode `0700` and record files with
mode `0600`. It publishes a new profile tree with create-new atomic rename;
configuration record replacements use a private same-directory temporary,
final-mode synchronization, atomic rename, and parent synchronization. A
profile directory is accepted only when its path components are real
directories rather than symlinks and its `PROFILE_ID` passes the command
grammar. The `home/` child becomes an upstream `CODEX_HOME` only for a child
launch; Manager does not interpret the files written there by upstream Codex.
The default profile is the existing upstream default home and is never copied
into this tree.

Manager does not persist or consume an implicit last-selected execution account.
Only explicit `profile default` owns the new default-profile-v1 preference. Existing
`state-v1` selection-history records are retired: they are ignored and left
unchanged, including malformed or symlinked remnants. They never select an
account or block execution. No migration or automatic deletion is introduced.

Manager profile records use exact UTF-8 text formats with a final newline.
New `profiles/<PROFILE_ID>/profile.meta` contains exactly:

```text
codex-manager-profile-v2
```

The existing exact private v1 form remains readable:

```text
codex-manager-profile-v1
id\t<PROFILE_ID>
```

The derived profile-home path is not duplicated inside metadata. Unknown
records, duplicate records, invalid UTF-8, or a mismatched ID invalidate that
record and never cause a path to be followed. Notification commands alone write
`notifications/config-v1`.

Manager does not persist a session transcript, a second session database or a
parallel discovery index. Upstream owns discovery/resume and conversation state;
the declared compatibility links above do not transfer that ownership. Auth-state
migration is outside ordinary Manager operation.

The authoritative state format is `codex-activation-state-v3`. It owns one
forward `update_key`, one `current` generation with its exact `current_key`, and
at most one `previous` generation with its exact `previous_key`. Ed25519 public
keys in state are the canonical 32 raw key bytes encoded as exactly 64 lowercase
hex digits. The `previous` generation and `previous_key` are present or absent as
one pair. There is no separate `verified` pointer, keyring, discovered-key set,
or unbounded key history.

Only content that has already passed release admission and candidate probes may
become `current`. Ordinary launch reads only `current`; it does not perform
signature verification, consult `update_key` or either generation verifier key,
scan generations, contact the network, invoke OpenSSL, or implicitly fall back
to another generation. The generation directory name is one safe path component
and generation content is complete before it can become `current`. Before
ordinary launch consumes that generation, Core must verify the selected
generation directory and every selected asset parent directory is a real
directory, and every selected runtime, Manager, helper, and descriptor file is
a regular non-symlink file. Ordinary launch must not follow a symlink from the
generation root to content outside that generation.

For durability, complete means that every regular file has its final bytes and final mode written and synchronized, every generation directory is synchronized after its children in bottom-up order, the complete candidate directory is atomically renamed into the generation root, and the generation root is synchronized after that rename. A failure before the candidate rename leaves no activatable generation. A failure after the rename but before generation-root synchronization may retain that exact complete candidate without changing authoritative state; a retry may reuse it only after signed installed-generation verification and repeating the required tree and root synchronization. A differing, incomplete, or unverifiable existing directory is a conflict and is never activated.

A generation is complete or absent. Candidate construction occurs outside the
active path. Ordinary public forward activation publishes one complete new
state: the candidate becomes `(current, current_key)`, the old current pair
becomes `(previous, previous_key)`, and `update_key` becomes the candidate
release key. For a non-rotating public release the old and new update keys are
equal; for a key rotation they differ. Local-derived activation is the sole
exception: its ephemeral public verifier becomes `current_key`, the former
current pair becomes the retained previous pair, and the existing official
`update_key` is copied byte-for-byte into the new state rather than being
rotated or replaced.

The authoritative journal format is `codex-activation-journal-v3`. Its before
and after records contain the entire bounded trust-and-generation state, not only
generation identities. Activation, rollback, and recovery therefore treat
`update_key`, both generation identities, and both generation verifier keys as
one transaction. After process kill, power loss, short write, full storage,
permission failure, or a stale journal, recovery must resolve to exactly one
complete old or new trust-and-generation state and must never synthesize a mixed
state.

A v4 coordinated update also retains one regular executable
`core-entrypoint-rollback` and its exact `codex-core-entrypoint-rollback-v1`
binding for the launcher active before the forward transition. This cache is
not a trust source: its digest and associated generation are verified before
rollback, and it is atomically replaced only while the activation lock is
held. Existing v3 generations may lack this cache; a v4 forward transition
must create it before changing the stable entrypoint.

Coordinated activation stages and verifies the complete signed generation and
its `core` asset first, snapshots the current stable launcher into the bounded
rollback cache, atomically replaces `$PREFIX/bin/codex` with the candidate
Core, synchronizes its parent, and only then commits the existing generation
pointer transaction. If the pointer commit fails, the old launcher is restored
before the failure is reported. A process kill between launcher replacement and
pointer commit leaves the new Core running against the old complete generation,
which is backward-compatible within the accepted Core API/schema boundary; the
next update may retry without accepting a mixed generation. Rollback swaps the
retained launcher cache and generation pointer as one locked operation and
preserves the exact one-pair boundary.

Core optimizes for the shortest correct release path rather than speculative
defense layers. Complete-or-absent generations, one atomic state transaction,
and recovery to one complete last-known-good state are the primary safety
invariants. Do not add a second trust updater, key-file transaction, fallback
key search, or recovery mechanism for a failure already covered by those
invariants. If a simpler base invariant makes an existing check, retry path,
pointer role, or fallback redundant, remove the redundant mechanism instead of
maintaining both.

One installer/updater transaction is the normal product model. Simultaneous
install or update attempts are not a first-class coordination feature and do
not by themselves justify locks, leases, fencing tokens, or a multi-writer
protocol. If attempts overlap, the required outcome is limited to preserving a
complete state boundary: one attempt may succeed while another fails or retries,
and recovery may return to the already complete last-known-good state. Because
the activation journal is a single pathname, Core serializes activation and
explicit recovery writers with one exclusive kernel-held lock on the state-root
directory. The lock is coordination only: it is not authoritative state, is
not read by ordinary launch, introduces no persistent lock record, and is
released by the kernel when the owning process exits. A contending writer
fails or retries before touching journal/state files. Launch must never observe
a mixed or partially constructed generation.

`previous` is the only rollback pointer and is not permission to build a
fallback ladder. Rollback is an explicit bounded activation-state transition;
ordinary launch never consults `previous` automatically. Rollback swaps only the
`(current, current_key)` and `(previous, previous_key)` pairs. It never rolls
back `update_key`, so a key removed from forward-update authority by an accepted
rotation cannot regain that authority merely because the user rolls back the
runtime generation.

## 8. Installation and update

The release bundle keeps one audited local `install.sh` delivery frontend and
adds one separate network-acquisition frontend, `install-online.sh`.
`install.sh` remains local-only and accepts exactly the two bootstrap forms
below, forwarding their original arguments without compiling, downloading,
signing, discovering releases, or implementing a second installation protocol:

```text
install.sh <CORE_ARTIFACT> <SIGNED_RELEASE_DIR> <BOOTSTRAP_PUBLIC_KEY>
install.sh upgrade-legacy <CORE_ARTIFACT> <SIGNED_RELEASE_DIR> <BOOTSTRAP_PUBLIC_KEY> <EXPECTED_LEGACY_ENTRYPOINT_SHA256>
```

`install.sh` must locate a sibling `bootstrap/codex-bootstrap` that is a
regular non-symlink executable and then `exec` it with the exact original
argv. A missing, symlinked, or non-executable sibling fails before any target
mutation. The frontend owns no trust, generation, entrypoint, or recovery
state; all validation and writes remain behind the audited bootstrap boundary.
The fresh form keeps the existing no-clobber rule, while the
`upgrade-legacy` form is the sole explicit legacy entrypoint handoff.

`install-online.sh [--variant base|full]` defaults to full and selects only the
fixed project head for that lane. It is only a bootstrap transport
frontend for a fresh target; it is not an update command, alternate trust
authority, package-manager installer, release producer, or legacy handoff. It
requires the existing Termux shell, `$PREFIX/bin/curl`, and
`$PREFIX/bin/openssl`; it never installs or discovers replacements for them.
Its production locators are fixed project HTTPS URLs, not caller-selected
mirrors.

Before trusting any network-selected generation, the online frontend retrieves
the bootstrap public key from the fixed project release locator and requires
the exact repository-pinned SHA-256 of those key bytes. It then retrieves the
canonical stable `update-index-v1` (full) or `update-index-base-v1` (base)
and sibling signature and verifies that
signature with the pinned bootstrap key before parsing the generation identity
or `release_base`. HTTPS transport alone is never release authority. The
verified index must have the exact stable four-record grammar already consumed
by Core, and the release base must be canonical HTTPS ending in the signed
generation identity.

The online frontend retrieves that generation's `release.manifest` and
`release.sig`, verifies the manifest with the same bootstrap key before using
its inventory, and accepts only the existing bounded v4 bootstrap inventory:
`generation.meta`, `core`, `runtime`, `codex-code-mode-host`, optional
`manager`, exactly one supported two-helper layout, and an admitted optional
`helpers/2` enhanced TUI in the full lane. Base requires its signed `local-base-`
identity and exact six-file inventory; full requires Manager and seven or eight
files. Selecting a lane cannot cross into another signed lane. It downloads only
those signed relative paths into a private `$TMPDIR` workspace with the Core
remote-acquisition per-file and total byte ceilings and applies only the
signed regular-file modes needed by the existing bootstrap admission. It does
not make downloaded file digests authoritative; the audited bootstrap/Core
admission re-verifies the signed manifest, descriptor, digests, modes, Core
binding, candidate probes, and activation transaction.

The network frontend obtains the audited local `install.sh` and
`bootstrap/codex-bootstrap` only from one immutable accepted repository commit
chosen by the immutable installer, recreates their sibling layout in the private
workspace, and invokes local `install.sh <CORE_ARTIFACT>
<SIGNED_RELEASE_DIR> <BOOTSTRAP_PUBLIC_KEY>`. It never writes
`$PREFIX/bin/codex`, the bootstrap trust pin, a generation, activation state,
or recovery state directly. All persistent writes remain owned by the existing
bootstrap/Core boundary. Failure removes only the private acquisition
workspace.

Fresh installation uses a small audited bootstrap because Core cannot install
itself before it exists. The bootstrap may only detect the environment,
retrieve or accept a local immutable release, verify it, stage Core, run a
self-test, and activate the initial generation.

The bootstrap command surface has exactly two local-only forms:

```text
codex-bootstrap <CORE_ARTIFACT> <SIGNED_RELEASE_DIR> <BOOTSTRAP_PUBLIC_KEY>
codex-bootstrap upgrade-legacy <CORE_ARTIFACT> <SIGNED_RELEASE_DIR> <BOOTSTRAP_PUBLIC_KEY> <EXPECTED_LEGACY_ENTRYPOINT_SHA256>
```

The first form is fresh bootstrap and never replaces a differing existing
`$PREFIX/bin/codex`. The second form is the sole supported legacy handoff. It is
not a `codex update` mode, Manager operation, package-manager action, or legacy
state migration. `EXPECTED_LEGACY_ENTRYPOINT_SHA256` is exactly 64 lowercase
hexadecimal digits. It identifies the one entrypoint the user has explicitly
authorized bootstrap to replace; it is not release authority, is not persisted,
and does not alter `codex-release-v3`. Candidate trust continues to come only
from the bootstrap public key and the exact signed release.

Fresh bootstrap uses the same authenticated Core publication boundary for its stable entrypoint: after self-test, Core creates a private same-directory temporary, writes the exact authenticated bytes, sets and verifies mode `0755`, synchronizes the file after its final mode, atomically publishes without replacing a differing existing target, and synchronizes `$PREFIX/bin` before invoking the stable entrypoint. If publication is interrupted after rename or its parent synchronization fails, activation has not been invoked; a same-Core retry must revalidate the target and re-establish parent durability before activation.

Bootstrap trust-seed publication is also owned by the authenticated Core. The shell bootstrap may snapshot and validate the supplied key, but it must not directly publish the persistent pin. Core writes the exact key bytes to a private temporary in the pin directory, sets final mode `0644` before synchronizing the file, verifies the parsed key, atomically publishes without replacing a differing existing pin, and synchronizes the pin parent before continuing. An existing matching pin is revalidated, repaired to mode `0644` when necessary, and re-synchronized; a differing, symlink, or special-file pin fails without replacement. A failure after rename but before parent synchronization leaves no activation state and is retryable only after the same-key verification and parent synchronization complete.

Before any bootstrap snapshot is consumed, each bounded input is checked and
copied through a bounded path: the bootstrap public-key PEM is at most 16 KiB,
the authenticated Core artifact is at most 64 MiB, `release.manifest` is at
most 128 KiB, `release.sig` is at most 1 KiB, and `generation.meta` is at most
64 KiB. A bound failure occurs before trust-seed, entrypoint, generation, or
activation publication. Core uses the same bounds for direct authenticated
inputs and for persistent descriptor loading.

Bootstrap classifies the target after resolving any recoverable activation
transaction:

- a **fresh target** has no authoritative v3 state or transaction residue and
  has no `$PREFIX/bin/codex`;
- a **same-Core bootstrap retry** has no authoritative v3 state or transaction
  residue and has one regular non-symlink entrypoint whose digest equals the
  authenticated Core artifact digest;
- a **legacy handoff target** has no authoritative v3 state or transaction
  residue and has one regular non-symlink entrypoint whose digest equals the
  explicit expected legacy digest and differs from the authenticated Core
  artifact digest;
- a **prepared legacy handoff** has that same legacy entrypoint and one complete
  recovered initial v3 state whose `update_key` and `current_key` equal the
  candidate release key, whose `current` equals the candidate generation, and
  whose `previous` pair is absent, and whose installed current generation
  verifies with `current_key`; and
- a **completed legacy handoff retry** has that exact initial state and an
  entrypoint whose digest equals the authenticated Core artifact digest.

On a prepared or completed handoff retry, the recovered v3 keys and the signed
installed generation are the authority. The supplied bootstrap key and release
must match that state exactly, but bootstrap must not use them to reinitialize,
reconstruct, replace, or weaken the authoritative state.

Any symlink or non-regular entrypoint, digest mismatch, incompatible bootstrap
key, other v3 state, mismatched prepared generation/key, retained previous pair,
or other transaction residue is a conflicting target and fails without
changing it. Bootstrap must not classify every non-Core file as legacy, scan for
another launcher, execute the legacy entrypoint, infer a legacy version from its
output, or inspect/import a legacy internal schema. It may inspect only the
entrypoint type, mode, and bytes needed to bind the explicit digest.

Legacy handoff is activation-first and entrypoint-last. Before modifying Core
state or the public entrypoint, bootstrap snapshots and verifies the same Core,
key, manifest, signature, descriptor, and Core-artifact binding required for
fresh bootstrap and completes the authenticated Core self-test. It then uses the
existing initial v3 admission, complete-generation staging, candidate probes,
installed-generation verification, and activation transaction to establish the
candidate as `current` with no `previous` pair. Only after that complete state is
recoverable may bootstrap create a same-directory private Core entrypoint
temporary, set and verify mode `0755` and the authenticated Core digest,
revalidate the legacy entrypoint against the explicit expected digest, atomically
replace `$PREFIX/bin/codex`, and durably synchronize its parent directory. That
completed parent-directory durability boundary is the handoff commit.

A completed handoff retry is successful only after it revalidates the authenticated Core target and synchronizes the entrypoint parent directory; an already-Core target does not bypass that synchronization. If the synchronization fails, bootstrap reports failure and remains retryable without changing authoritative state.

If interruption or failure occurs before the entrypoint commit, the legacy
entrypoint remains the public executable. A complete prepared initial Core state
may remain and is resumable only with the same authenticated release, bootstrap
key, Core artifact, and expected legacy digest. If interruption occurs after the
entrypoint commit, the new Core sees the already complete initial v3 state. A
completed handoff retry is idempotent. This ordering is the recovery invariant;
legacy handoff adds no second journal, backup launcher, trust source, generation
type, fallback path, or persistent-schema field.

Successful legacy handoff is one-way. It does not retain or automatically run
the old entrypoint, and ordinary launch never falls back to it. The initial v3
state has no `previous` pair, so `codex update --rollback` fails clearly until a
later successful Core update establishes one previous Core generation. Rollback
then remains exclusively a swap between signed Core generations and never
restores legacy code.

Legacy handoff may create or change only the stable Core entrypoint and the Core
roots declared in Section 7, plus private temporary files needed for that
operation. It must leave the resolver, auth, profile, session, Manager, package,
and all other non-Core state untouched. Except for the explicitly replaced
entrypoint, legacy-owned user/state files remain in place and are neither read as
migration input nor copied into Core state. Qualification compares protected-
state identities before and after the handoff rather than adopting a legacy
schema.

Normal installation and update must not require on-device Rust, Cargo, or
Clang compilation. The no-argument update fallback is a bounded execution of
the prebuilt release-builder routines already contained in Core; it is not a
Core self-recompile or a package-manager installation.

After bootstrap, the Core owns every top-level `codex update` form. `install.sh`
is not an update dispatcher and must not bypass the authenticated local
admission, staging, probe, activation, or rollback path. The local, explicit
remote, and automatic channel forms use the same forward activation
transaction, and rollback remains the explicit swap of the one retained
complete previous generation. A bare `codex update` is never the upstream command: it resolves a
wrapper-owned signed channel and, only when automatic-channel transport is
unavailable, may enter the same official-source local-derived construction used
by `--build-local`. It therefore cannot install an unpatched upstream runtime or
turn transport failure into official publication.

The automatic stable channel is represented by a bounded signed index. Its
default control URL is
`https://raw.githubusercontent.com/humtr/codex/main/update-index-v1`; a
deployment or test may supply the same canonical HTTPS URL through
`CODEX_TERMUX_UPDATE_INDEX_URL`. The URL is only a location hint: the index
bytes and its sibling `<URL>.sig` are verified with the recovered v3
`update_key` before Core trusts any field. The exact index format is:

```text
codex-update-index-v1
channel\tstable
generation_id\t<ID>
release_base\thttps://<host>/<path>/<ID>/
```

It has exactly these four records and a final newline. The generation identity
is a safe path component, the release base is the canonical immutable
generation base already defined below, and its final path component must match
the signed identity. After index signature verification, Core invokes the
existing signed remote-generation acquisition path. Index transport failure,
signature failure, malformed discovery, or release qualification failure is a
hard failure for that attempt; there is no fallback to upstream, a package
manager, an alternate mirror, or a raw package. A transport-level absence or unavailability while resolving the automatic
channel may enter the local-derived construction path below; an index or release
that was received but failed signature, format, policy, digest, mode,
compatibility, probe, or activation validation may not.

The signed index is a pointer to an already-adapted wrapper generation, not an
upstream source authority. The only upstream source authority is the official
versioned OpenAI archive acquired by the release-production `fetch` operation
and consumed by `build` before qualification and signing. In the primary no-argument path, when the wrapper publication is
transport-unavailable, Core may perform the exact official fetch/build/adapt
steps locally, sign the immutable candidate with a fresh ephemeral device-local
key, bind it to the authenticated public baseline, and feed it only to the
dedicated local-derived admission path. That path performs no official
publish/signing action. Core never downloads a raw upstream archive directly
into the active generation and never runs an upstream self-updater.

`codex update --build-local` is an explicit selector owned by Core. It is valid
only by itself. It resolves the same exact official stable metadata and versioned
archive used by the qualified release builder, verifies the metadata-bound
archive digest, builds/adapts in private staging, signs with a fresh ephemeral
Ed25519 key, runs the normal candidate probes, and atomically activates the
result. The presence or absence of an official release private key has no effect
on this route. The same route is used by the allowed automatic-channel transport
fallback.

A local-derived candidate records the authenticated public baseline generation
and sequence in its signed provenance and uses that same sequence in its signed
release manifest. This is not a new public sequence and grants no public signing
authority. Public anti-rollback after local-derived activation therefore compares
against the recorded baseline: a later authenticated public release must advance
that public sequence to supersede local-derived current. The dedicated
local-derived admission may install a different local generation at the baseline
sequence only because it is reached directly from the in-process qualified
official-source build; ordinary `--local`, `--remote`, and signed-channel
admission keep the equal-sequence/different-release rejection rule.

`codex update` must:

1. recover the authoritative v3 state and resolve one immutable signed wrapper
   release against its `update_key` (automatic update first verifies the signed
   channel index, while explicit remote update starts from its supplied base;
   transport-level automatic-channel absence enters the local build path,
   which resolves and digest-binds one exact official upstream version before
   building);
2. enforce architecture, API, channel, and the release-sequence anti-rollback
   policy: a lower sequence is rejected; an equal sequence is a no-op success
   only when the authenticated candidate generation identity and signed release
   manifest exactly equal the installed current release, while an equal
   sequence with any different authenticated identity or manifest is rejected;
   only a greater sequence may enter candidate staging, probing, and activation;
3. download into a private staging location or accept an explicit local
   artifact;
4. verify the required current-authority and candidate-key signatures, exact
   digest/mode inventory, archive safety where applicable, and compatibility
   metadata;
5. resolve the exact official upstream version and package digest, then build,
   sign, and probe a complete candidate generation when the local fallback is
   selected;
6. atomically publish the new trust-and-generation state;
7. retain one complete previous generation with its exact verifier key as
   rollback state;
8. report failure without damaging the active trust-and-generation state.

One migration-only coordinated v4 bridge is permitted for an already-installed
R10 Core whose signed release parser predates the TC-2 browser-helper paths. The
bridge is not a new trust or release format: it uses the recovered v3
`update_key`, the normal monotonic release sequence, the normal v4 Core binding,
and the exact signed digest/mode inventory. Its generation descriptor must bind
`creation_metadata` exactly `r10-browser-helper-bridge-v1`, `helper_count`
exactly `2`, and the two existing helper identities in order:
`termux-browser-open-v1` then `termux-browser-manual-v1`. Only for that exact
marker and helper contract are those identities stored and loaded from the
R10-readable signed paths `helpers/0` and `helpers/1`. The release inventory
must sign those exact two paths, digests, and modes. Missing or extra helpers,
identity/order mismatch, a marker/layout mismatch, or any attempt to use the
indexed layout without the exact marker fails closed.

The bridge carries the same qualified browser helper bytes and policy as the
canonical TC-2 layout; it does not temporarily remove browser protection. A
new Core may retain read support for this exact bridge layout so the bridge can
run and can be the one retained rollback generation. Normal generation builds
and publications must continue to use only `browser/open/curl` and
`browser/manual/curl` for these identities and must never select the bridge
layout implicitly. The bridge adds no alternate signing key, bootstrap path,
package-manager action, PATH widening, raw-runtime patch, or second activation
mechanism. Its sole purpose is one signed R10-compatible forward step before a
second ordinary signed update publishes the canonical browser-helper layout.

The signed release format is `codex-release-v3`; v1 and v2 are not retained as
release compatibility paths. In addition to generation identity, monotonic
release sequence, supported channel, platform, architecture, Core API,
persistent schema, and the exact SHA-256 plus regular-file permission-mode
inventory of every load-bearing generation file, the manifest binds exactly one
`release_public_key`. That value is exactly 64 lowercase hexadecimal digits
encoding the 32 raw bytes of the candidate Ed25519 public key. Each canonical
inventory record continues to contain path, lowercase SHA-256, and four octal
permission digits. Special permission bits are rejected; every file must be
owner-readable and runtime/Manager/helper files must be owner-executable.

`release.sig` is always an Ed25519 signature by the manifest's
`release_public_key` over the exact manifest bytes. For an ordinary non-rotating
public update, `release_public_key` must equal the recovered authoritative
`update_key`; `release.sig` is then the only release signature and no
`release-authority.sig` is accepted. For a public key rotation,
`release_public_key` differs from `update_key`; `release-authority.sig` is then
mandatory and must verify over the same exact manifest bytes with the current
`update_key` before Core treats the candidate key as trusted, after which
`release.sig` must verify with the candidate key. A rotation is rejected if
either proof is missing or invalid. The dedicated local-derived path is not a
rotation: its manifest key is the fresh ephemeral local verifier, its signed
provenance must match the independently authenticated public baseline, and
activation stores that verifier only as `current_key` while preserving
`update_key`. No generic local/remote candidate receives this exception. Core
never accepts an adjacent key file, alternate-key search, network key lookup,
CA/PKI chain, key server, or unbounded keyring as authority.

Before forward admission, Core recovers the v3 state. Installed-generation
verification requires the signed manifest `release_public_key` to equal the
selected state verifier key before `release.sig` is verified with that key. Core
applies that rule to the installed current generation with `current_key` and uses
that signed current release for the release-sequence anti-rollback comparison.
An authenticated candidate below that sequence is rejected. An equal-sequence
candidate is returned as an already-current no-op only when its generation
identity and signed release manifest exactly equal the installed current
release; any other equal-sequence candidate is rejected before staging or
candidate execution. After staging and probing a greater-sequence public candidate, successful
public activation sets `update_key` and `current_key` to the candidate release
key, sets `current` to the candidate generation, and moves the former
`(current, current_key)` pair to `(previous, previous_key)` in the same atomic
transaction. When current is local-derived, its signed release sequence is the
integrity-bound public baseline sequence, so this comparison is still a public
anti-rollback comparison rather than a local sequence allocation. Dedicated
local-derived activation instead preserves `update_key` and stores only its
ephemeral verifier in `current_key` as defined above.

`codex update --rollback` must recover any pending activation transaction,
require the one retained `(previous, previous_key)` pair, verify exactly that
previous generation with `previous_key`, and atomically swap the current and
previous generation/verifier pairs. It deliberately does not apply forward
release-sequence anti-rollback policy and deliberately leaves `update_key`
unchanged. When the generation rolled back from is authenticated local-derived,
rollback creates neither a public `update-hold` nor a rollback Core guard. When
the generation rolled back from is an ordinary authenticated public generation,
the accepted hold/guard rules are unchanged even if the retained previous
generation is local-derived. `--force` continues to require and target only that
authenticated public hold. Missing, malformed, mismatched, or unverifiable
rollback state fails without changing authoritative state; rollback never scans
generations, searches keys, restores a rotated-away key to forward authority, or
constructs a fallback ladder.

Initial bootstrap, whether fresh or an explicit legacy handoff, owns the only
permitted use of `~/.local/lib/codex/core/release-public-key.pem`. Before any v3
activation state exists, bootstrap may verify one initial v3 release only when
the manifest `release_public_key` exactly equals that pinned key and
`release.sig` verifies with it; successful initial activation initializes
`update_key`, `current_key`, and `current` from that release with no previous
pair. Once v3 state has been established, Core update and recovery never treat
the bootstrap key file as a fallback or reconstruction source. Absence or
corruption of authoritative trust state after initialization fails closed rather
than re-authorizing an old bootstrap key. The two bootstrap forms share this one
initial trust rule and create no second bootstrap or update authority.

Automatic update checks must be bounded and fail open when a verified runtime
already exists. Ordinary `codex` launch must not depend on network success,
OpenSSL availability, key rotation, or silently run a package manager.

The updater must not depend on the same resolver implementation as the patched
upstream runtime without an explicit qualification proving that dependency.
Offline local-artifact installation and recovery are required before release.
On Termux, update and explicit rollback may use the already-present
`$PREFIX/bin/openssl` for Ed25519 verification and SHA-256; they must fail
clearly if it is unavailable and must never install a crypto package themselves.

`codex update --remote <HTTPS_BASE_URL>` adds only acquisition in front of the
same local admission/staging/probe/activation path. Core first recovers the
v3 state, so the authoritative `update_key` is known before any candidate trust
transition. The base is at most 4,096 ASCII bytes, begins exactly with
`https://`, ends in `/`, and contains no credentials, query, fragment,
whitespace/control byte, or backslash. It names one generation directory: after
signature admission, its final path component must equal the manifest generation
identity encoded as canonical UTF-8 URL-path bytes. Core may follow only HTTPS
redirects required by the selected release transport and never tries another
URL or permits a non-HTTPS redirect.

The remote control resources are `<base>release.manifest` and
`<base>release.sig`, plus `<base>release-authority.sig` exactly when the parsed
manifest key differs from the recovered `update_key`. Core may parse the bounded
manifest to select that fixed signature set, but the candidate key is not trusted
by parsing. For a non-rotation it verifies `release.sig` with `update_key`. For a
rotation it verifies `release-authority.sig` with `update_key` first and only then
verifies `release.sig` with the manifest candidate key. No generation content is
acquired before this control admission succeeds. The remaining remote resources
are exactly the files named by the signed inventory.

Inventory URLs are derived only by preserving `/` separators and percent-
encoding every UTF-8 path byte outside the RFC 3986 unreserved set. Every
resulting resource URL is also at most 4,096 ASCII bytes. After signed control
admission, Core reconstructs the exact inventory, verifies the assembled bundle
again through the same local v3/v4 admission, and feeds the existing staging,
probe, activation, and recovery path. This file-addressed release transport is
not an archive and does not weaken the archive-safety requirements for later
upstream-artifact work.

Remote acquisition creates directories with owner-only access, creates response
files owner-readable, and after each content digest succeeds applies the exact
signed permission mode before whole-bundle verification. Local admission checks
the same mode inventory before and after staging. Mode reconstruction therefore
does not depend on HTTP metadata, curl defaults, or a blanket executable bit.

Remote transport is exactly the existing `$PREFIX/bin/curl`, independent of the
patched upstream runtime and compatibility resolver. Before network I/O, Core
requires curl, `$PREFIX/bin/openssl`, and a valid recovered v3 trust state; Core
remote update never falls back to the bootstrap key file. The curl child loads no
user config, inherits no environment/proxy settings, permits HTTPS only, uses
the Termux certificate file/directory, applies a 15-second connect timeout and a
300-second transfer timeout, follows redirects only with `--location` and
`--proto-redir =https`, and writes response bytes only to a caller-created
regular file. The manifest limit remains 128 KiB and each signature file is
limited to 1 KiB; each generation-file response is limited to 512 MiB and the
sum of all response bytes is limited to 1 GiB. Curl and Core both enforce the
applicable remaining byte bound. A transport or bound failure is terminal for
that explicit attempt.

Acquisition uses one create-new `.acquire-<pid>-<counter>` directory beneath the
generation root and create-new output files beneath that directory. Core attempts
cleanup after every acquisition outcome, and remote success is impossible until
that cleanup succeeds. A cleanup error is terminal and preserves the authoritative
activation pointers. An uncatchable process kill or a filesystem cleanup failure
may leave a dot-prefixed partial directory; Core never scans, launches, verifies
as a generation, or activates such a path. Bounded installation maintenance may
remove an abandoned partial directory under the retention contract below;
it must not add a retry, fallback, or registry merely for it. Remote success reports
`activated remote generation <id>`; every failure preserves the authoritative
activation pointers.

### Bounded installed-artifact retention

Core performs best-effort local artifact maintenance before ordinary execution
and after an explicit update attempt. Failure or contention in maintenance must
not change upstream argv, streams, exit status or block an otherwise valid
launch. The one authoritative activation state and its existing recovery journal
own all retained pointer roles; malformed or unrecovered state disables pruning.
Keep the current generation, the one previous rollback generation, any generation
bound by an effective rollback guard/local-derived public baseline, at most one
complete higher-sequence staged candidate for the existing crash/retry reuse
contract (highest sequence, deterministic generation-ID tie-break), and complete
generations still used by live local executables or open generation files.
All other Core-owned installed generations/publication caches are disposable.
Abandoned acquisition, candidate and local-build staging directories are also
disposable. Conversation activity is not inferred from executable timestamps.

Use directory flock leases rather than another registry: production update
preparation/activation and launch selection hold a shared lease on the existing
generation catalog parent; pruning requires its exclusive nonblocking lease plus the
existing activation-state writer lock. The launch lease survives until exec's
close-on-exec boundary. In-flight updates therefore protect their whole staging
work without PID-name guesses or deletion of another updater's source.
Inspect local process executable/open-file references before deleting retired
generations or staging; preserve active clients and their companion/helper trees. Inspect
only live process groups: a confirmed zombie with no surviving sibling threads
holds no executable/open-file references and must not disable maintenance.
Incomplete visibility of a live process still disables pruning. Where a
non-dumpable process changes its proc-directory owner to root, use its native
status UID fields to identify a local process; directory ownership alone must
not hide an unreadable local process from this check. If a process
reference read fails during exit, re-read its native process status: only a
confirmed single-thread zombie/dead process or a vanished process may be skipped.
The kernel exiting flag alone never proves that handles are released. A
single-thread process with that flag may be observed until the same confirmation,
using a total scan waiting budget of at most ten milliseconds; an unreadable,
still-live, multi-thread or unconfirmed process disables pruning. Inspect
only owned real directories, never follow substituted parents or deletion-root
symlinks, and never descend into profile/auth/session or resolver state.
Revalidate authoritative state while locked. Removing stale files is idempotent
and does not manufacture an activation success or invalidate rollback.

Retire only an obsolete Core-owned shared server whose private owner record,
socket peer and executable bind to its recorded signed runtime, and whose open
FDs show only its listener socket and no conversation/writer-lock handle.
Coordinate with its existing namespace lock. Request upstream's graceful-only
SIGHUP shutdown; never force an active client or turn to stop. Live process/file
references protect its generation until shutdown finishes. Dead obsolete server
records/config snapshots may then be removed. Unknown ownership or incomplete
process visibility keeps the affected files and never authorizes termination.
A missing process executable path alone does not prove the entire process group
has exited. Removing a dead server record/config snapshot uses the same native
exit confirmation as generation retention, preserving records for surviving
sibling threads or unknown visibility.

The 2026-10-02 user-authorized cleanup additionally permits removing inactive
legacy profile SQLite projections after all retained conversation payloads are
verified in the canonical upstream store. Preserve every conversation with
activity in the preceding seven days and all active writers. Older redundant
installation artifacts and expired conversation records require no backup.
The three obsolete `.codex-scs6-backup-*` / failed-activation copies may be
removed after the same payload/prefix and inactive-handle checks; live account
auth/configuration stays untouched. A stale backup index refresh does not count
as new conversation work when its actual transcript activity has expired.
This is bounded operator cleanup, not routine Core SQL/transcript ownership.

Automatic channel discovery uses the same curl, certificate, timeout, and
owner-only temporary rules. It fetches only the bounded index and its sibling
signature, verifies the exact index bytes with the recovered `update_key`, and
then delegates to the explicit signed remote-generation path. The index is not
an alternate trust source and never authorizes a raw upstream package.

Candidate activation integrity is narrower than public diagnostic health. After
signed descriptor/manifest/index admission, exact signature/digest/mode checks,
qualified Termux environment construction, release-sequence/anti-rollback
checks, and staging, Core must execute the exact candidate upstream
`--version` probe in that qualified Termux environment. A failed version probe,
qualification failure, admission failure, or transaction failure rejects the
candidate before the new launcher/pointer pair is committed. Activation must
not require the full upstream `codex doctor` command to exit zero: credential,
provider, and external-network health belong to the diagnostic surface, not to
release-runtime integrity.

For the bounded backward-compatible transition from the public sequence-10 R10
bridge Core, and only for exact `creation_metadata =
"r10-browser-helper-bridge-v1"`, the signed descriptor may declare
`upstream_doctor = unsupported` as a legacy activation-capability signal. The
legacy Core may use that signal only to omit its historical activation-time
upstream-doctor health gate. The corrected Core must map that exact marked
transition back to supported public-doctor behavior and actually execute the
upstream doctor. The release builder must reject this compatibility signal for
any other creation metadata. It is never a fabricated healthy result and never
weakens signature, inventory, mode, version-probe, anti-rollback, atomic
activation, LKG, rollback, or CAS requirements.

## 9. Doctor contract

`codex doctor` is read-only. It runs the raw upstream doctor when supported and
adds a Termux Core/Manager diagnosis without recursively invoking the public
launcher. Human output begins with the bounded, sanitized upstream doctor
output itself, preserving the upstream doctor's own header, layout, and safe
ANSI SGR sequences; Core does not prepend an `[Upstream Codex doctor]` wrapper
heading or a duplicate synthetic status line. `--json` emits one redacted
envelope rather than concatenated documents:

```json
{
  "schema_version": 2,
  "upstream": {"status": "healthy", "output": "..."},
  "termux_core": {
    "status": "healthy",
    "generation_id": "...",
    "layout": "root-code-mode-host-v2",
    "runtime": {"status": "healthy"},
    "code_mode_host": {"status": "healthy"},
    "sandbox": {"status": "unsupported", "reason": "bwrap is not used"}
  },
  "manager": {},
  "summary": {}
}
```

When human output is connected to a TTY and `NO_COLOR` is absent, Core gives the
upstream doctor a bounded pseudo-terminal so its own headings, progress
cleanup, and ANSI SGR markup retain the upstream layout. `--color` is the
explicit exception for an interactive caller whose outer Termux/AI wrapper
injected `NO_COLOR`; Core removes that variable only from the upstream child
and enables the same bounded PTY path. Non-TTY output remains plain, and
explicit `NO_COLOR` remains plain unless `--color` was requested. Core
normalizes carriage-return/erase controls and preserves only safe SGR sequences
before composition. If the upstream doctor is unsupported or produces no
output, Core emits only a concise status diagnostic for that missing upstream
portion before the Termux section.
The Termux doctor section follows the legacy wrapper presentation: a `Codex
Termux Wrapper Doctor` status header,
`Runtime`, `Support`, `Wrapper`, `State`, and `Store` groups, colored health
rows when enabled, and a bounded summary. It reports the selected generation,
root-level code-mode companion or migration state, and the explicit reason that
Linux bwrap sandboxing is not used.

Unsupported upstream or Manager diagnostics are represented explicitly and do
not fabricate success. Diagnostic failure returns nonzero while preserving a
valid machine report when `--json` was requested. After valid doctor argument
parsing, an upstream probe/setup failure is represented as a redacted
`unhealthy` upstream status with empty output rather than an error string, and
the command still returns its nonzero health-failure status. Upstream doctor
output is capped at 64 KiB, has terminal control sequences and credential-like
values redacted before composition, and is emitted as a JSON string in the
envelope. Usage errors remain distinct from health failures and API
incompatibility.

A credential-free disposable release proof therefore does not require doctor
exit zero. It may accept only the documented success or health-failure exit
class when the bounded JSON envelope is valid, Termux Core/runtime/code-mode
health is `healthy` for the exact activated generation, and the upstream
section proves that the real upstream diagnostic ran by reporting
`healthy` or `unhealthy` rather than `unsupported`. The exit code must agree
with the composed summary. Missing user credentials or external provider
reachability may make the upstream diagnostic unhealthy, but must not be
reclassified as candidate runtime-integrity failure.

Doctor must not expose tokens, OAuth data, cookies, auth-derived private data,
notification content, or unredacted session content. A filesystem snapshot
before and after doctor must be unchanged except for operating-system access
metadata outside product control.

Generation descriptors written by the current release builder use
`codex-local-generation-v2` and the root-level companion layout above. Core
continues to read an already-installed `codex-local-generation-v1` generation
with `compat/codex-code-mode-host` only as a bounded migration input; it never
produces that layout, and it never treats an unlisted root-level symlink as the
companion. The next authenticated generation is the required permanent repair
for such an old layout.

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
`background_paginated_rollout_migration=false` while upstream 0.160.0 keeps
rollout maintenance locks mixed into profile-local `.tmp`. The requirements
file is Core-owned, is removed or replaced only through the Core marker
contract, and prevents a profile `config.toml` from silently splitting SQLite
or enabling those incompatible background writers. The original upstream argv
after the profile selector is preserved exactly. If registration validation fails,
Core is not launched. If execution topology is unsafe or conflicting, Core rejects
before upstream execution. A selection changes neither later ordinary launches nor
the caller's environment. Core uses the same native-equivalent home interpretation
for shared preparation and server startup, preserving the supplied environment.

The profile lifecycle contract below applies;
interactive terminal UI and profile-auth migration remain absent. Cross-profile session copying is deliberately absent
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
explicit default/current queries and lifecycle mutations fail; optional ordinary
launch falls back to the native default without changing the preference.

A nonempty UTF-8 inherited CODEX_HOME always wins. Otherwise the installed Core
may delegate ordinary upstream argv to the admitted optional Manager through the
internal `__profile-launch` endpoint only when default-profile-v1 is present.
Core neither parses nor writes that preference or profile metadata. Manager
validates the preference and registration, sets CODEX_HOME explicitly for this
child, and execs the stable Core with exact original argv, TTY, streams, signals
and exit status. A missing/unavailable Manager leaves normal native launch usable.
Core's update/doctor/termux and unsupported-daemon routes bypass this delegation;
no selector may be smuggled through the internal endpoint. Bare startup discovery
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

The 2026-10-02 corrective contract extends the existing upstream-owned shared
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

For this user-authorized transition only, the historical SCS-6 five-per-profile
selection and backup prerequisite are replaced by all conversations with actual
activity during the seven days preceding the bounded apply. The user explicitly
authorizes deleting confirmed older conversations and declines a recovery
backup. Read-only legacy SQLite activity and authoritative rollout timestamps
select the set; IDs, retained transcript records and retained history entries
are preserved. Copies sharing a thread ID may collapse only when their complete
per-record-type payload sequences prove a prefix relation; divergent histories
reject apply. The maximal history is authoritative. Canonical files are regular
and deduplicated by UUID; upstream APIs own metadata repair, resume/history
materialization, explicit names and deletion of excluded canonical projections.
No SQLite merge or whole-file replacement is permitted.

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
No permanent task registry, watcher, supervisor or automatic account switch is added.
A private, structurally valid binding whose recorded process is confirmed absent
or fully exited is ignored read-only before requiring its execution home, runtime
or socket to remain present. Retired homes, historical home aliases and removed
old generation files do not make an exited server a current owner. Such records
are not deleted or repaired by Manager. A live or uncertain process still requires
all existing home/runtime/socket and peer checks; this is no permission to trust
aliases, ignore substituted metadata, or skip an unverified live writer.

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

Notification commands use these user-facing forms:

```text
codex termux notify show
codex termux notify test
codex termux notify set [--channel <notification|toast|both>]
    [--hooks <none|all|EVENT[,EVENT...]>]
    [--content-chars <0|1..4096>] [--preserve-newlines <0|1>]
    [--toast-gravity <top|middle|bottom>] [--toast-short <0|1>]
    [--toast-background <empty|#RRGGBB>] [--toast-color <empty|#RRGGBB>]
    [--group <GROUP_ID>]
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
`focus=termux|tmux` from the separate focus record. It emits no path, source detail, payload,
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
its execution defaults with the returned fragment and atomically replaces its
own `config.toml` before runtime exec. The generated file retains the fixed
`codex-termux-notify-v1` ownership marker. Core replaces a missing file or its own marker
file only; conflicting foreign state is preserved and rejected before execution.
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

Every notification includes an Activity action which brings the existing Termux
Activity to the foreground using Android reorder-to-front and single-top flags.
In default termux focus it preserves the currently selected terminal and requests
no terminal session, Codex process or resume invocation. `notify test` uses the
same Activity-only route. Explicit tmux focus additionally follows the Optional
AI tmux notification focus contract in Section 3, including its authorized new
native-terminal attach. This Activity action adds no click state or watcher.

The Activity action invokes only the absolute sibling `am` compatibility entrypoint,
which owns transport selection. A non-UTF-8 entrypoint uses the canonical Termux path.
The outer action allows twenty seconds; the installed compatibility entrypoint allows
fifteen seconds for one socket request, exceeding the server's ten-second socket budgets.
Manager must not wrap a shorter timeout around it or call the socket again after failure.
Only the compatibility entrypoint may fall back to `cmd activity` after an unsuccessful
idempotent Activity return. Service requests are never replayed. Paths are shell-quoted;
only a fixed Activity component and flags are passed. Output is silenced and no hook
payload, profile, CWD, auth or session ID is included. It never launches the terminal
execution service. Socket existence and successful read-only commands do not prove
Activity authorization: the APK requires Android display-over-other-apps permission for
background starts. This user-granted capability and actual start results are separate gates.
Socket availability does not imply an existing-terminal selection API: installed Termux
8629e63 restores its current selection. Exact bare-terminal selection remains unavailable;
it must never be simulated by creating a terminal or resuming Codex. The optional AI
focus helper owns its separately opted-in existing-tmux attachment path and gives the one
service response fifteen seconds independently of its two-second identity search budget.
Existing notifications are not rewritten; this contract applies
to newly delivered notifications.

The installed Termux Activity has no accepted intent for selecting a particular
existing terminal by Codex UUID. Activity-only focus therefore returns to Termux's
current terminal; exact originating-window selection is not claimed for that route.
When no terminal
exists, Termux owns its ordinary initial-terminal behavior. Android foreground
restrictions remain provider behavior; device acceptance proves the real
Activity-only action and unchanged running terminal/Codex process identities.
Physical notification tap is distinguished from programmatic action execution.

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

The optional external AI tmux launcher owns presentation only within a session
marked as AI-managed. Hidden mode hides that session's native bottom status row;
status mode shows one native status row and enables session-local mouse support
so tapping a window name selects it. The latest explicit launch mode applies to
the shared managed session. It must preserve status and mouse settings of an
existing unmanaged tmux session and never change global tmux configuration.
Within AI-managed sessions the launcher enables session-local `set-titles` and
uses the literal `#{pane_title}` format, forwarding the selected pane's native
Codex title to attached Termux clients. Upstream `/title` selects and orders
terminal-title items; `/rename` changes the conversation name. The launcher
forwards native title output rather than implementing another title composer.
It does not rename tmux windows. Qualified native identity leaves upstream title
composition unchanged; historical title-based focus retains its required thread ID.
Unmanaged sessions and global title options remain unchanged; no title watcher
is introduced.
New panes use
current caller color preferences, including explicit unset values, rather than
stale server environment; tmux retains native TERM/TERM_PROGRAM ownership.

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

After a successful probe, release-builder revalidates the source as a regular
owner-executable file within the existing 64 MiB bound, snapshots it through
the existing private bounded path, sets and verifies mode `0755`, binds the
snapshot digest in `manager_artifact_digest`, and includes `manager` in the
signed release inventory. A snapshot/read failure or probe mismatch publishes
no generation and leaves an existing destination unchanged.

Core does not repeat the probe during ordinary launch or update. Its existing
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
Core-only generation and reports Manager unavailable. The automatic local
fallback may produce only such a Core-only generation unless an explicit
already-qualified Manager artifact is supplied through the bounded release
producer; it never builds or discovers one on-device.

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
complete immutable Release asset set, including `manager` when the signed
inventory lists it; publish `update-index-v1.sig`; then publish
`update-index-v1`. A missing or mismatched Manager asset, failed or timed-out
upload, private-key exposure, or index update out of order fails the operation
without advancing the index and without undoing a successful local
activation. No OpenAI repository, live runtime, auth, profile, session, or
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

## 9A. Post-Core Termux compatibility extension

The upstream runtime remains the official `aarch64-unknown-linux-musl` Codex
artifact adapted by this product. Running that Linux-target executable inside
Termux does **not** activate upstream compile-time `target_os = "android"`
branches. Compatibility decisions therefore belong to an explicit Core-owned
Termux boundary rather than to accidental Linux desktop behavior.

This extension does not reopen or weaken the accepted Core completion
invariants. The resolver, signed-generation trust model, wrapper-owned update,
coordinated Core/generation activation and rollback, profile/session/auth
boundaries, sandbox policy, and no-on-device-compiler rule remain unchanged.
It also does not authorize a new raw-runtime byte patch: patch policy
`termux-fd-remap-v1` remains exact. Any additional binary rewrite requires a
separate normative amendment that names its exact bounded substitutions and
qualification evidence.

### Browser and external URL intents

An upstream request to open an HTTP or HTTPS URL on Termux must use one
Core-qualified Termux opener capability rather than Linux desktop discovery.
The preferred local capability is the absolute Termux URL opener under
`$PREFIX/bin`; the adapter passes exactly one already-formed URL argument and
must not evaluate a shell command string. Opener selection is child-local: it
must not write user shell configuration, desktop configuration, profile state,
or upstream config files. If the capability is unavailable or fails, the
operation must fail soft at the browser-open boundary and preserve the
upstream flow that exposes or otherwise allows manual use of the URL; it must
not make authentication state or the runtime unusable.

The same policy covers primary login, TUI onboarding/history URL actions, and
MCP OAuth browser intents. A fix limited to one login call site is incomplete.

### MCP OAuth registration and provider-policy boundary

MCP OAuth client registration and browser opening are separate compatibility
boundaries. An authorization-server response such as
`invalid_client_metadata` with `redirect_uri is not allowed by the account
configuration` is provider/account policy evidence, not by itself a Core
compatibility defect, when Codex submitted a syntactically valid callback that
matches the selected MCP registration mode and the server simply has not
allowed that callback. Core must not rewrite, relax, or bypass an
authorization server's redirect allowlist merely to turn that response into a
successful registration, and source qualification must not mutate external
OAuth account configuration.

For the selected upstream release, qualification must revalidate the actual
MCP registration path. Codex may use advertised CIMD when its prerequisites are
met or Dynamic Client Registration otherwise. Native loopback DCR can register
an HTTP callback on `127.0.0.1`/`localhost` with the active listener port and,
when issuer-bound responses are unavailable, a server-specific callback path.
The authorization server used for end-to-end qualification must therefore be
configured outside Core to accept the exact callback form that the selected
upstream release legitimately requires. A provider-policy rejection before
that prerequisite is met is recorded as an external configuration blocker,
not patched in Core.

TC-2 acceptance begins only after that provider prerequisite is satisfied. It
then proves the full MCP OAuth path: registration strategy selection,
authorization URL production, the shared Termux browser opener, loopback
callback receipt and validation, token exchange, and credential persistence in
a disposable `CODEX_HOME`. If the server is proven to allow the exact callback
and registration still fails because Codex generated a callback inconsistent
with the server's advertised metadata or the applicable OAuth/MCP contract,
that becomes a client compatibility defect and may be patched only after the
normative contract is updated with the observed mismatch. Credential values,
client secrets, authorization codes, and tokens never become test evidence.

### App-server daemon and remote-control authority

No Termux command may create, select, execute, update, or trust an unmanaged
Codex installation under `$CODEX_HOME/packages/standalone` or
`$CODEX_HOME/packages/app-server-daemon`, and no Termux app-server path may
fetch or execute an upstream installer or self-updater. Upstream 0.160.0 uses
the latter daemon package root with legacy standalone fallback; both remain
independent package authorities rather than signed Core generations. Bare
`codex update` and all runtime replacement remain Core-owned signed-generation
operations.

For interactive local upstream TUI/resume/fork launches, Core may start one
shared app-server directly from the already qualified active generation before
unchanged upstream exec. This server receives the same FD-33 resolver, FD-34
system configuration, FD-35 Termux temporary directory and qualified child environment. Its private profile-local
socket is exposed at the upstream default control socket through one validated
Core-owned socket symlink into a private short generation/profile namespace.
Both the Core rendezvous and the upstream physical socket pathname must fit
the 107 encoded-byte bound. In qualified 0.160.0, the physical path is the
canonical FD-35 temporary root plus UID and a 64-hex digest; the standard Termux
temporary root fits. Disposable proof must provide a separate private temporary
root short enough for this native path rather than nesting it under a long
fixture root. Core does not bypass the native private socket ownership checks;
profile identities and authentication remain distinct. Core binds a reused
server to its recorded PID, kernel socket peer PID, and exact qualified executable
through `/proc`, and
serializes startup with a kernel-held directory lock. Generation changes choose
a new namespace and preserve existing clients on the old server; they never
kill user turns as a side effect of launch. Server system configuration is a
private immutable snapshot of the effective Core defaults/optional fragment and
requirements for that identity. Another launch never refreshes a live snapshot. A substituted socket,
record, directory or runtime is rejected. Startup is bounded; failure does not
invoke an installer or execute any package outside signed generations. Explicit
user embedded/remote/configuration selections retain their upstream semantics.

A qualified local TUI may submit its upstream-resolved model catalog
to Core's private `__app-server-catalog-v1` operation on stdin (at most4 MiB).
For deferred startup without a resolved catalog, the frontend requests the same
native server through private no-argument `__app-server-default-v1`. This operation
does not consume stdin or interpret frontend configuration. Both operations return
only a validated socket and share the existing coordinator. Failure after deferred
selection is explicit; it never retries through an embedded executor or starts a
second native server. A catalog request remains distinct from an absent catalog;
an empty/invalid catalog is not a request for native/default behavior.
Core returns only the exact qualified server socket, and uses the existing server
coordinator, environment, descriptor and retirement path. Catalog bytes, profile
home, runtime and effective system config/requirements jointly identify reuse; a changed catalog at the same source
filename must not reuse stale metadata. Core stores an immutable private snapshot
and supplies its path only to backend startup, never to ordinary upstream argv.
It does not interpret profile TOML, resolve provider credentials or start another
executor. The frontend remains responsible for loading selected user/CLI/project
configuration with upstream code. Existing native servers and active siblings
remain intact. Remote/embedded launches are unaffected; failure to prepare a
requested catalog is explicit, never a fallback to an unqualified embedded backend.
Catalog server records live under Core's private `catalogs` namespace and never
retarget the default `servers` rendezvous. Current Manager discovery
validates both families, including exact catalog content, runtime and peer PID;
generation retirement applies the same existing-client/writer protections to both.
The existing server-record codec has one implementation shared by Core and
Manager: owner fields, family/namespace binding, opaque catalog bytes and bounds.
Core owns this execution-record contract; Manager consumes decoded bindings for
its task UX rather than independently defining Core's catalog/storage format.
This is a dependency-free source library compiled into each executable, not an
installed shared runtime or a third management authority. Core never imports
Manager, and Core-only construction and execution require no Manager artifact.
Callers retain their filesystem trust checks and revalidate live process/peer
identity before use. The codec performs no writes, process control, model-schema
interpretation or provider selection. This introduces no registry, service or
second runtime authority.
Core launches no upstream daemon updater and never creates a packages tree.
The historical public daemon/remote-control lifecycle fence remains in force.

Configuration-bound execution records add the `config-v1` owner marker after
home/runtime, followed only by an optional `catalog-v1` marker. They retain the
existing `servers`/`catalogs` families. Their namespace also binds the exact
opaque `config/config.toml` and optional `config/requirements.toml` bytes, with
absence distinct from an empty file. Each config file retains the16-KiB bound.
Core and optional consumers share this codec; it interprets no Manager/provider
policy. Reuse compares the complete requested snapshot, not only its short routing
hash. A live snapshot is immutable. Use one current codec, without historical
format emulation or a staged reader/writer release. Unsupported old experimental
records authorize no operation. Preserve their running processes and user data;
profile mutation and takeover still require native process/writer checks. Test
the complete new composition, its install/update/rollback, discovery and retirement
directly. Compatibility with superseded experimental codecs is not an admission
requirement.

A daemon-backed app-server command, including `remote-control start`, may run
only if it is bound to the currently qualified signed generation and cannot
enter an upstream daemon-package or standalone installation/update loop. If
that binding is not available, the command must fail closed with a stable Termux-specific
unsupported result before creating unmanaged package state or performing
network I/O.
The foreground `remote-control` form that owns a private temporary socket is a
separate path and remains usable when otherwise qualified.

An app-server Unix control socket for a supported Termux path must use a
private, profile-distinct short runtime namespace. Its encoded pathname must
be at most 107 bytes so it fits the Termux/Linux `sockaddr_un.sun_path[108]`
bound including the terminating NUL. The short runtime identity may derive
from the logical `CODEX_HOME`, but it must not relocate, alias as authority, or
merge profile auth/config/session state. Different logical profiles must not
collide, and symlinks or pre-existing untrusted entries must not become socket
or profile authority. Default and Manager-created profiles use the same rule.

### Linux-target Android capability mismatches

The Linux-target runtime must not treat X11, Wayland, a desktop Secret Service,
or another Linux desktop facility as present merely because the compile target
is Linux. Each observed desktop-only capability is either adapted through a
bounded Termux/terminal mechanism or reported unavailable without corrupting
TUI state.

Image clipboard paste must not reach a usable Linux desktop or WSL clipboard
transport on a normal Termux session unless a separately qualified backend
exists. For an upstream artifact compiled as Linux, Core may satisfy this
without a new runtime byte patch only when release-specific source review proves
the image path is bounded to desktop `arboard` plus the enumerated WSL fallback:
the child-local capability projection must make X11 and Wayland transport
unreachable before runtime launch and remove inherited WSL selector variables.
Qualification must also reject a positive WSL kernel signature when the exact
upstream kernel probe is readable. If that source is unreadable under the same
credentials the child will inherit, qualification may instead rely on that same
probe remaining unavailable together with removal of every remaining upstream
WSL selector. The upstream Linux function may then execute only far enough to
fail before a desktop connection; no PowerShell/WSL subprocess fallback may
become reachable. In the
absence of a separately qualified clipboard backend the user-visible outcome is
a clear unavailable result. Text copy may use a terminal-mediated fallback such
as the already supported OSC 52 path; native clipboard failure alone must not
make the TUI unusable. Compatibility code must not bypass Android permission or
application boundaries. This capability fence does not widen the raw-runtime
byte-patch allowlist.

The release qualification for every newly supported upstream version must
inspect material `target_os = "android"` versus `target_os = "linux"`
behavior in the upstream source used by the artifact. A new Android-specific
upstream guard that affects a Termux-visible feature is a compatibility review
input even though the shipped binary is Linux-target.

### Host-tool and credential fallback discipline

Because the accepted package adaptation excludes upstream `codex-path/rg`, a
fresh Termux installation must not silently assume that system `rg` exists for
ordinary product correctness. A feature that still requires it must either
have a qualified signed helper/fallback or degrade explicitly with bounded
doctor/user-facing evidence. The bootstrap and Core recovery paths must not install a
package manager dependency automatically. Bundled zsh remains excluded unless
a later accepted feature makes it an explicit requirement.

MCP OAuth `Auto` credential storage must apply one coherent authority across
load, save, refresh, and delete. If keyring storage is unavailable and file
storage is the resolved fallback, logout/delete must be able to remove the
resolved file credential without requiring a functioning desktop keyring.
Credential contents never become compatibility evidence.

### Compatibility acceptance gate

A compatibility slice is accepted only with focused tests for its exact
boundary plus the existing protected-state and workspace regressions. Tests
use disposable roots and may not mutate the live resolver, launcher/runtime,
auth, profiles, sessions, Manager state, or network configuration. For any
app-server slice, acceptance additionally proves no standalone Codex tree or
upstream installer request is created, custom-profile socket paths stay within
the pathname bound, profile identities do not collide, and foreground
remote-control behavior outside the daemon path is preserved. For browser and
clipboard slices, acceptance covers every enumerated upstream surface rather
than one observed call site.

## Release execution

One maintained release workflow owns construction and publication. main is a
small immutable-source caller, with no second implementation or manually mirrored
producer. The caller selects an accepted exact implementation commit; nested
publication logic comes from that same source. Job-specific permissions separate
read-only construction/smoke from signing/public mutation. Signing secrets appear
only at the signing boundary. A manual publication needs explicit authorization;
scheduling authorizes only a qualified strictly newer upstream pair. An explicitly
authorized current-version rebuild uses its exact qualified upstream/archive and
normal next sequence, with all ordinary integrity/publication gates intact.

Remove completed one-shot acceptance/deployment code paths, old mutation workflows
and experimental preview construction. Existing immutable migration releases and
required consumer compatibility remain ordinary product support, not executable
historical producer modes. Preserve normal signature/inventory verification,
LKG continuity, candidate site size, complete public readback, disposable consumer
proof and non-forced exact-parent CAS. No public release or installed activation
is implied by a development-plane change.
