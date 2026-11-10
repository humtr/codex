# 5. Termux runtime contract

> Normative module of [SPEC.md](../../../SPEC.md). Check [CURRENT](../../../CURRENT.md) for implementation/evidence.

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

Shared-server reuse binds the complete immutable FD-34 settings snapshot under
the existing namespace coordinator lock. Equal settings may reuse the server;
changed Manager hooks or other effective bytes select a matching configuration
namespace without rewriting a live server's files or restarting its active work.
New launch/resume uses the newly selected snapshot. Existing threads retain their
actual upstream configuration until an upstream-supported transition applies it.
Do not add CLI overrides or mutate sibling settings to force a policy refresh.

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

Approve for me selects the upstream automatic reviewer for native approval
requests. It does not promise a model review for every command. Preserve
OnRequest eligibility and establish a specific missing reviewer invocation
before changing approval behavior. Do not restore Untrusted or broaden admission
merely because ordinary allowed commands showed no review.

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
