# 9A. Post-Core Termux compatibility extension

> Normative annex of [SPEC](../../../SPEC.md); [INDEX](../INDEX.md) is navigation only and [CURRENT](../../../CURRENT.md) is observed status.

## 9A. Post-Core Termux compatibility extension

The upstream runtime remains the official `aarch64-unknown-linux-musl` Codex
artifact adapted by this product. Running that Linux-target executable inside
Termux does **not** activate upstream compile-time `target_os = "android"`
branches. Compatibility decisions therefore belong to an explicit Core-owned
Termux boundary rather than to accidental Linux desktop behavior.

This extension does not reopen or weaken the accepted Core completion
invariants. The resolver, signed-generation trust model, wrapper-owned update,
coordinated Core/generation activation and rollback, profile/session/auth
boundaries and sandbox policy remain unchanged. Ordinary signed installation
requires no on-device compiler; explicit source-build paths, including the tunnel
recovery contract, are separate opt-in operations.
It also does not authorize an unbounded raw-runtime byte patch: the selected
version's exact termux-fd-remap policy and retained-generation policies remain
bound to their qualified artifacts. Any additional binary rewrite requires a
separate normative amendment that names its exact bounded substitutions and
qualification evidence.

### Timezone compatibility

Resolve the musl runtime's user-visible timezone mismatch through Core's launch
environment. Respect explicit working POSIX/path TZ settings and resolve selected
IANA names, including the Android system timezone when TZ is unset, using the
active Android timezone data. The preferred method to qualify is a verified
zone-specific TZif supplied as `TZ=:/absolute/private-cache/zone.tzif` to the
existing runtime. Core's common child environment boundary supplies the same
selection to bare runtime, optional frontend and new shared-server launches.
The frontend uses that TZif through chrono's Local conversion, including absolute
resume-picker dates; it does not maintain another timezone database or parser.
Existing processes retain their launch environment until their ordinary restart.

Discover Android's active APEX tzdata first, then legacy updated tzdata and the
system image when preceding locations are absent. A present but unreadable or
invalid source is diagnosed rather than silently replaced by older data. Validate
the selected zone's index bounds and TZif structure before publishing it in a
private cache. Compare actual selected-zone bytes on each launch and atomically
replace changed data; no version-only cache hit may conceal a data update.
Explicit empty, POSIX-rule and file-path TZ settings remain under user control.
CURRENT records implementation and qualification separately; source changes do
not by themselves establish installed musl behavior.

Keep the upstream Local-based formatters and stored/server UTC timestamps. User
displays that explicitly format UTC require a separate narrow upstream-TUI fix;
do not add an unconditional offset or force Korean time on other users. Bare
Core must support the result without Manager or shell edits, and timezone
discovery/data failures must be diagnosable without preventing ordinary launch.

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
fetch or execute an upstream installer or self-updater. An upstream runtime may use
the latter daemon package root with a legacy standalone fallback; both remain
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
the 107 encoded-byte bound. For an FD-35-backed shared server, the physical path must be
formed beneath the canonical private temporary root using verified UID/identity; the standard Termux
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

A same-generation, signed optional frontend may advertise deferred native
startup through its non-mutating `--termux-backend-capabilities` query, which
must return exactly `codex-termux-resolved-startup-v1` before configuration,
terminal initialization or a server connection. Only a bounded, successful
probe enables `CODEX_TERMUX_BACKEND_STARTUP=resolved-v1` with Core's planned
socket; no server or secondary installer starts merely for this probe. The
frontend then resolves its upstream settings and asks Core for the qualified
local backend. Unknown or older helpers retain their supported eager path.
The helper and Manager must belong to the selected signed generation, not a
stale preview. Core does not interpret model policy or override user-supplied
upstream profile/CLI options; an incomplete optional frontend leaves the
independently qualified Core available.

A qualified local TUI may submit its upstream-resolved model catalog
to Core's private `__app-server-catalog-v1` operation on stdin (at most4 MiB).
For deferred startup without a resolved catalog, the frontend requests the same
native server through private `__app-server-default-v1`. Both private commands
also accept bounded --config VALUE/--profile VALUE pairs for the invocation;
their existing no-argument callers remain unchanged. The default command does
not consume stdin. Core validates the argument transport but delegates the
meaning, precedence and policy of their values to upstream. Both operations return
only a validated socket and share the existing coordinator. Failure after deferred
selection is explicit; it never retries through an embedded executor or starts a
second native server. A catalog request remains distinct from an absent catalog;
an empty/invalid catalog is not a request for native/default behavior.
Core returns only the exact qualified server socket, and uses the existing server
coordinator, environment, descriptor and retirement path. Catalog bytes, profile
home, runtime, invocation arguments and effective system config/requirements
jointly identify reuse; a changed catalog at the same source
filename must not reuse stale metadata. Core stores an immutable private snapshot
and supplies its path only to backend startup, never to ordinary upstream argv.
It does not interpret profile TOML, resolve provider credentials or start another
executor. The frontend remains responsible for loading selected user/CLI/project
configuration with upstream code. Existing native servers and active siblings
remain intact. Remote/embedded launches are unaffected; failure to prepare a
requested catalog is explicit, never a fallback to an unqualified embedded backend.
Catalog or invocation-configured records live under Core's private `catalogs` namespace and never
retarget the default `servers` rendezvous. Current Manager discovery
validates both families, including exact catalog content, runtime and peer PID;
generation retirement applies the same existing-client/writer protections to both.
The existing server-record codec has one implementation shared by Core and
Manager: owner fields, family/namespace binding, opaque catalog/invocation bytes
and bounds. Records without invocation overrides retain their existing encoding
and namespace for their live consumers. An invocation record adds its explicit
marker and private argument snapshot; reuse validates complete bytes, not only
the routing hash. The snapshot shares the existing16 KiB configuration-record
bound, including its UTF-8 arguments and NUL separators. Unsupported/malformed
arguments fail before server launch.
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
