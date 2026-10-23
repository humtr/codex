# Codex for Termux

This repository is the clean rewrite of the Termux compatibility layer for the
upstream Codex CLI.

The product has one public command, `codex`, and four components:

- a minimal native Rust Core that makes the upstream runtime work correctly on
  Termux and owns installation, update, diagnosis, activation, and rollback;
- an optional Manager reached through `codex termux` for accounts, quota,
  notifications, windows and task assistance;
- an optional TUI, extending the upstream terminal UI and integrating Manager
  and provider capabilities;
- an optional Rust Provider for ChatGPT model connection, with a minimal
  app-side JavaScript bridge.

Every Termux Codex execution passes through Core. Actual tools, reviewer,
conversation storage, compaction and resume remain upstream-owned. Optional
preparation/configuration isolation and no-Manager TUI/Provider execution have
source and bounded Android proof; signed composition and operating activation
are distinct. See [current status and work order](CURRENT.md#product-completion-scope-and-work-order).

The optional **Provider** contains the ChatGPT–Codex bridge: model and native
tool adapters plus shared Chat/session management. ChatGPT supplies intelligence;
native Codex executes, approves and records. MCP connects tool calls/results;
its app connection is named **Termux Bridge**. That connection is part of Provider,
not another product component. Provider release sequence numbers identify signed
publications, not different component names.

The common managed development loop is Codex -> Linux app API -> work Chat ->
MCP or app-injected local functions -> the same native work. ChatGPT general
and direct development use the same native execution path through MCP. The
primary Codex thread retains durable history while Chat contexts can be replaced.
Origin, controller identity and original tool-return bindings survive handoff.

## Start here

Install **Full** for the usual terminal experience (Core, Manager and TUI).
Install **Base** when you want only Core and upstream Codex. ChatGPT Provider
is an optional separate package in either edition.

| Where you start | Who reasons about the task | Required setup |
| --- | --- | --- |
| Terminal: ordinary Codex | Your selected native Codex model | Base or Full; native Codex login |
| Terminal: Codex using Termux Bridge tools | Your selected Codex model; a managed Chat only for work explicitly delegated there | Native Codex plus Provider's local MCP connection; app/login for managed Chat work |
| Terminal: Codex with ChatGPT | A managed Chat in the provisioned Linux ChatGPT app | Base or Full, Provider, app/login; MCP connection if using its MCP return path |
| ChatGPT: direct local development | The Chat you are currently using | Base or Full, Provider/native execution binding, reachable MCP connection; no Linux app for primary direct work |
| ChatGPT: delegate managed development | A separate managed work Chat; the current Chat requests and receives the work | Direct-entry setup plus the Linux app/login and an explicit ChatGPT project |

Core is installation/runtime support. Manager provides account/window conveniences.
TUI is the terminal interface. Provider owns the ChatGPT connection. **Termux
Bridge** is the app's MCP connection name. These are components and connection
roles; release numbers do not create differently named products.

A working native installation does not require Provider. Direct ChatGPT work
needs a connected MCP service, but its primary current-Chat reasoning does not
need the Linux app. Managed Chats and app-backed reviewers do need that app.
See [fresh install](#fresh-install), [daily use](#usage-by-entrypoint), and
[the remaining first-time setup gap](#bootstrap-and-minimum-setup-status).

## Current status

A–H are complete at their
[recorded scopes](CURRENT.md#product-completion-scope-and-work-order).
[CURRENT](CURRENT.md#operating-baseline) records accepted sources, signed releases,
device selections and actual running processes. This device selects Full96 and
ChatGPT Provider release sequence4; existing sessions keep their pinned executable
until normal exit. Ordinary account configuration does not force a model.

Fresh Core installation exists. Provider app provisioning, first login and remote
connection setup still require the [manual steps below](#fresh-install) and the
[Provider guide](providers/chatgpt/README.md#first-time-setup). The shared minimum
setup flow and repository separation remain
[deferred implementation work](CURRENT.md#deferred-setup-and-repository-work).
Diagnosis and local recovery use `codex doctor` and `codex update --rollback`.

To run the direct Core path for one invocation while Manager is installed:

```sh
CODEX_TERMUX_CORE_ONLY=1 codex
```

This keeps signed Core execution and its safety preparation while skipping
Manager profile/window policy, the optional TUI and Manager notification hooks
for that run.

## Fresh install

These steps are for a **new** Termux installation. On an existing managed
installation, use `codex update`; fresh bootstrap deliberately refuses existing
authoritative state or a different existing `codex` executable.

The Android/AArch64 release needs Termux's shell, curl and OpenSSL. If the latter
two are missing, install their Termux packages explicitly:

```sh
pkg install curl openssl-tool
```

No repository clone, Rust compiler, release private key or GitHub login is needed.
The accepted immutable installer selects the signed stable release. Run this
block for Full; change `--variant full` to `--variant base` for Base:

```sh
(
  set -eu
  codex_install_tmp="$(mktemp -d "${TMPDIR:?}/codex-install.XXXXXX")"
  trap 'rm -rf -- "$codex_install_tmp"' EXIT
  "$PREFIX/bin/curl" -q --fail --silent --show-error --location \
    --proto '=https' --proto-redir '=https' --tlsv1.2 \
    --connect-timeout 15 --max-time 60 --max-filesize 131072 \
    --output "$codex_install_tmp/install-online.sh" \
    'https://raw.githubusercontent.com/humtr/codex/eba60a7a72cacc9f6be126606ee992c00e662f5d/install-online.sh'
  "$PREFIX/bin/sh" "$codex_install_tmp/install-online.sh" --variant full
)
```

The frontend verifies the pinned public key, signed index and release inventory;
`install.sh -> bootstrap/codex-bootstrap -> Core` owns persistent installation.
For a downloaded signed bundle, the existing local/offline `install.sh` path
remains available. Full does **not** install Provider or the Linux ChatGPT app.

For native-model Codex, log in and then open your real project (direct ChatGPT
entry can skip this native subscription login):

```sh
codex login
codex login status
cd /absolute/path/to/your-project
codex
```

Replace the project path with an existing project. If you intentionally use device
login, `codex login --device-auth` is also supported. Native Codex owns this login;
it does not sign into the separate Linux ChatGPT app. Existing login should be
reused rather than logging out/recreating it. To conditionally invoke native
login from a setup script, use `codex login status || codex login`.

Full's optional named account example is:

```sh
codex termux profile create work
codex termux profile use work -- login
codex termux profile use work
```

`work` is an example account ID. `codex termux profile default work` explicitly
makes it the default for future launches. A Manager account and upstream
`--profile chatgpt` are different concepts: the first chooses account/settings;
the second selects generated ChatGPT model routing within that account.

## Usage by entrypoint

### Terminal with native Codex

From your project, run `codex` and give the task. Use `codex resume` to select
an earlier thread or `codex resume THREAD_ID` for a known thread. `codex exec "TASK"` is the noninteractive entrypoint. These are upstream commands through
Core. Full also offers `/switch` for configured accounts. Ordinary configuration
does not force a model or reasoning effort.

### Terminal with Termux Bridge tools

You can also register Provider's `mcp-stdio` wrapper as a local Codex MCP server.
For this configured device, an explicit registration in the selected account is:

```sh
codex mcp add termux-bridge -- "$HOME/.local/state/codex/providers/chatgpt/mcp-stdio"
```

On a fresh device, first prepare Provider and its running service as described
in the [Provider guide](providers/chatgpt/README.md#first-time-setup); use that
installation's wrapper path. MCP registration belongs to the active Codex account
and is not automatically added by the background services. This local stdio
connection uses the process's standard input/output; it needs no remote tunnel.

The model selected in Codex remains the primary decision maker. MCP supplies
tools; it does not select ChatGPT as the primary model. Explicitly delegated
managed Chat work additionally needs the app/login and project configuration.

### Terminal with ChatGPT reasoning

First complete [Provider setup](providers/chatgpt/README.md#first-time-setup).
In Full, run `codex`, open `/model`, and choose the Provider's ChatGPT model and
effort. A fresh idle session can switch before its first message; no initial
prompt or thread name is required. The same thread is retained without an empty
checkpoint. After updating Full, exit and reopen an existing Codex client to use
the updated picker. For Base or explicit configuration selection:

```sh
cd /absolute/path/to/your-project
codex --profile chatgpt
```

Give the task in that same terminal. Native Codex still owns tools, approval,
history and the final result; Provider connects the managed app Chat. Existing
readable-thread model transitions retain their qualified scopes. An old foreign
opaque checkpoint needs its own supported handoff; do not discard history to
force a switch. `--profile chatgpt` neither selects a Manager account nor logs
into the Linux app.

### Connect ChatGPT once

First complete the [direct connection setup](providers/chatgpt/README.md#direct-chatgpt-connection-without-a-linux-app).
Provider's authenticated service needs a working remote transport. In ChatGPT
web, open Plugins, add a custom MCP server named **Termux Bridge**, choose your
reachable Server URL or the configured Secure MCP Tunnel, and complete its
authentication/consent. Install the resulting plugin. Workspace access controls
still apply. See the [official OpenAI connection instructions](https://developers.openai.com/api/docs/guides/custom-mcp-server).

In a conversation, type `@` and select the installed Termux Bridge entry from the
picker, then describe the task. Do not assume that typing its name alone enables
an unavailable connection. The local `127.0.0.1` URL and `mcp-stdio` command are
not remote Server URLs; a configured tunnel can invoke that local command.

### ChatGPT with direct local development

With Termux Bridge selected, ask:

> Work in /absolute/path/to/your-project from this Chat. Fix [problem] and verify it.

This Chat supplies reasoning (`reasoning=current`); native Codex executes,
approves and records. The assistant handles the advertised attachment and native
tool protocol, including project instructions, existing changes and actual final
results. You do not need to spell out protocol fields, call sequences or private
credentials. This primary path needs neither a Linux app nor an extra native
Codex subscription login. Separately configured app-backed reviewers retain
their own prerequisites.

### ChatGPT delegating managed development

Use the same MCP connection, with a provisioned/authenticated Linux app and
[explicit project configuration](providers/chatgpt/README.md#managed-chat-project).
With Termux Bridge selected, ask:

> Delegate [task] in /absolute/path/to/your-project to a separate work Chat.
> Bring back its progress and original final result.

The current Chat requests/observes the work; a separate managed Chat does the
reasoning (`reasoning=managed`). Managed work can use MCP (default) or explicitly selected
`local_function` internally. That internal transport choice does not create a
second direct ChatGPT entrypoint. `work wait`/events retrieve original results;
uncertain calls are observed by their existing handles rather than resent.

### Manage existing work from ChatGPT

Keep Termux Bridge selected and use ordinary requests:

| Request | What the assistant should do |
| --- | --- |
| Show work and its current state on this device | List existing work and inspect its status; do not start another task |
| Get the original result for this work | Read the existing work's events/history; do not rerun it |
| Continue this completed work with [next task] | Reconcile its state and reuse its work/native thread at a supported boundary |
| Stop this work | Request interruption and report confirmed versus pending Stop state |

When several works match, select the returned work ID before an action. Local
status/result controls already exist; there is no shipped project picker or
task dashboard. A reusable plugin skill and an optional compact work panel are
[proposed follow-ups](providers/chatgpt/README.md#chatgpt-entry-and-task-management),
not prerequisites or installed features. A ChatGPT project instruction can hold
your workspace and preferred direct/delegated route where available, but it does
not connect MCP, establish native control or replace local project instructions.
See the [official project guide](https://learn.chatgpt.com/docs/projects).

## Bootstrap and minimum setup status

| Layer | Exists now | What still needs manual setup |
| --- | --- | --- |
| Base/Full bootstrap | Signed online installer and audited local bootstrap | Termux/prerequisite packages; native upstream login |
| Native login | Upstream first-run/login/device-login flow | User completes the official account authentication/consent |
| Provider installation | `codex provider install/update/status/rollback chatgpt` | Selecting an execution home/workspace and preparing routing/service |
| App-backed reasoning | Rust host/service/viewer commands | Official app/rootfs and optional viewer assets must already be provisioned; first app login |
| Direct ChatGPT entry | Authenticated MCP stdio and native work controls | Tunnel/remote transport credentials and adding the connection in ChatGPT |
| Minimal combined setup | No installed `setup` command yet | A small reusable configuration/login wrapper is the next requested convenience |
| ChatGPT task convenience | Select the connected MCP plugin and ask short tasks | A bundled workflow skill and optional work panel remain proposals |

The useful next step is a **small minimum-setup flow**, not a new onboarding
framework: collect only missing execution/app/connection inputs, reuse existing
settings/login, call the existing installers and official login/viewer controls,
and check readiness without starting a model task. Native and app login stay
separate and are requested only for the chosen entrypoint. User authentication
and consent remain in their official UI. The agreed follow-up is one shared
implementation, run explicitly or called after a fresh interactive install.
Implementation is deferred for now; ordinary Codex launch must not acquire apps,
change configuration or start model work implicitly.

The existing device was migrated manually using preserved profiles; that is not
proof of a complete fresh-user app provisioning/login flow. Detailed manual
steps and precise prerequisites are in the [Provider guide](providers/chatgpt/README.md#first-time-setup).

## Background services on this device

Provider contains the bridge and exposes its MCP service as **Termux Bridge**.
The **OpenAI Secure Tunnel runtime** transports that MCP traffic; it is a separate
program, not the bridge. Keeping an interactive Codex conversation open is not
required for ChatGPT entry.

This device explicitly registers these services beside tdev in
`$PREFIX/var/service/`:

| Service | Responsibility |
| --- | --- |
| `codex-chatgpt-provider` | Foreground Provider `serve`, restarted by runit if the process exits |
| `codex-chatgpt-tunnel` | Existing OpenAI `tunnel-client` runtime/profile, with the same remote tunnel and credential references; renew local transport after Provider owner changes |
| `codex-chatgpt-host` | Observe Provider and acquire its provisioned Linux app/Xvnc when the owned host is confirmed absent; no model request |

Xvnc, the app, private pipes and optional viewer remain Provider-owned. There is
no separate competing Xvnc service. Present-but-unready/unknown host state is
preserved for inspection rather than automatically killed. No viewer window,
new task, login, update or model selection is triggered by service startup.

Standard `termux-services`/runit starts enabled services at the next shell startup
and restarts individual exited services. This device additionally installs the
owner's `~/prj/termux-services` recovery layer: `$PREFIX/bin/service-daemon`,
the shell startup hook and `~/.termux/boot/00-start-services`. Its independent
15-second monitor restores a lost runit root. That extra layer is not required
for normal runit operation; it improves root-loss recovery. Termux:Boot must be
available for its boot hook to run. A fully killed Termux UID requires the next
Boot/login execution opportunity; no service runs while the whole environment
is stopped.

Inspect the actual services with:

```sh
sv status "$PREFIX/var/service/codex-chatgpt-provider" \
  "$PREFIX/var/service/codex-chatgpt-tunnel" \
  "$PREFIX/var/service/codex-chatgpt-host"
codex provider chatgpt status
codex provider chatgpt host status
```

`sv status` proves a process is supervised, not app login or remote tunnel health.
Use the existing tunnel profile's local health/readiness endpoint as well.
The previous tmux-managed runtime is retired; this device uses runit, not
`tunnel-client runtimes connect/stop`, for local lifetime management. Remote
tunnel registration is unchanged.

This device's private `~/.local/state/codex/providers/chatgpt/runit/environment`
holds the executable paths, state and tunnel profile. Service/MCP wrappers read
that common file; Provider configuration selects the native execution home and
workspace. Interactive account switches do not retarget the services. The tunnel
must use an Android-compatible runtime; this device selects native0.0.14. The
immutable Provider observer path must be rebound during deliberate package/service
maintenance after pending work has settled.

For deliberate maintenance, use `sv-disable` in dependency order: tunnel, host,
then Provider. Re-enable Provider, then host if wanted, then tunnel with
`sv-enable`. An enabled supervisor restarts a process stopped directly; disable
it first. Keep active native work and pending calls settled before shutdown or
package changes. Disabling only the host keeper leaves an already acquired app
running; inspect/stop that idle owned host explicitly if desired. Direct ChatGPT
primary reasoning does not need an always-on Linux app.

An idle native catalog can retain an inherited Provider package lease after the
service exits. If selection reports busy, inspect the actual FD/lock holder and
its loaded threads/jobs. Preserve metadata and settle work before stopping that
exact idle owner. Service shutdown alone does not prove the package is unused.

This is an explicit configuration of the existing device. Future shared minimum
setup should offer service registration/enabling for the selected entrance,
including dependency checks, private logs and disable/restore instructions.
Neither the current installer nor ordinary `codex` launch installs these service
definitions automatically. See the [Provider service details](providers/chatgpt/README.md#device-runit-services).

## Updating

After installation, ordinary updates remain Core-owned:

```sh
codex update
```

The default route consumes the signed stable channel. It does not delegate to
the upstream self-updater, and publication credentials are not device update
authority.

Update output separates the upstream Codex version from the Termux edition and
release number, for example `Full #57 -> Full #59 · Base #58`. Base means the
standalone Core/upstream edition; Full includes Manager/enhanced TUI. The signed
release date is the UTC date that the immutable release was prepared, not the
device installation date. Older releases or unavailable optional metadata show
`unavailable`; valid updates still work. An already-current update also shows
the installed edition/release information, without installing again.

### Older browser-bridge releases

Published browser-bridge releases before the Manager TUI may reject a direct
update with “R10 browser helper bridge contract is invalid.” Run this one-time
migration from the immutable signed Release, then ordinary updates. The existing
Core verifies its trusted signature and complete inventory; profiles, conversations
and running clients are preserved. The temporary download is always cleaned.

```sh
(
  set -eu
  migration="$(mktemp -d "${TMPDIR:?}/codex-migration.XXXXXX")"
  trap 'rm -rf -- "$migration"' EXIT
  gh release download local-hosted-0-161-0-ab644771ac89-manager-tui-bridge \
    --repo humtr/codex --dir "$migration" \
    --pattern core --pattern generation.meta --pattern manager --pattern runtime \
    --pattern codex-code-mode-host --pattern helper-0 --pattern helper-1 \
    --pattern release.manifest --pattern release.sig
  chmod 0644 "$migration/generation.meta" "$migration/release.manifest" "$migration/release.sig"
  mkdir "$migration/helpers"
  mv "$migration/helper-0" "$migration/helpers/0"
  mv "$migration/helper-1" "$migration/helpers/1"
  chmod 0755 "$migration/core" "$migration/manager" "$migration/runtime" \
    "$migration/codex-code-mode-host" "$migration/helpers/0" "$migration/helpers/1"
  codex update --local "$migration"
  codex update
)
```

## Profile selection and optional tmux

### Login accounts and configuration profiles

Profiles support creation, deletion, renaming and a saved default. A damaged saved account choice stops launch with an error; it never silently selects a different account. All lifecycle commands are available in the installed Manager:

```sh
codex termux profile create work
codex termux profile rename work personal
codex termux profile default personal
codex termux profile default default
codex termux profile delete personal
```

`profile default` without an ID shows the saved choice. It affects later ordinary
launches; an explicit `CODEX_HOME` takes priority. Deletion removes that account's
login and settings while preserving shared conversations. Rename/delete reject
an account still in use or selected as the saved default; choose another default
and close its running clients/servers first.

Upstream configuration profiles (`codex --profile NAME`) are separate from these
login accounts. Optional shared configs live at
`~/.local/share/codex/core/config/profiles/NAME.config.toml`; Manager registers a
missing filename in each owned execution home. Existing local configs take
priority, and base settings/login files stay unchanged. Existing registrations
and account-local upstream profiles work without Manager; new shared registration
requires Manager. See the
[Provider guide](providers/chatgpt/README.md#first-time-setup) for the shared
ChatGPT profile. Package selection and explicit profile preparation are
separate; neither upgrades an already running process.

### Terminal selection and tmux

```sh
codex termux launch
codex termux launch --profile work -- resume
codex termux launch --tmux=hidden
codex termux launch --tmux=status --profile work
```

Choose a profile with Up/Down and Enter; Escape or Ctrl-C cancels. Selection does
not change the saved default. Default execution uses no tmux. Ordinary `codex`
uses the same window routing with its usual account resolution and no picker.
`codex termux tmux` is a shorthand for hidden tmux. tmux is optional for keeping
work running while detached; detach with Ctrl-B then D. `tmux ls` lists sessions;
`codex termux attach SESSION` reconnects detached Manager work and registers the
new connection window without restarting Codex.
Requesting another tmux container from inside tmux is refused; no extra pane or
workload is created in that terminal. Use a separate Termux terminal for independent work.
Changing execution mode requires a safe new launch, rather than migrating a live
Codex process. Independent launches never share a workload or cover another launch.

## Notifications (Full)

`codex termux help` lists the installed Manager commands. Profiles separate
execution preferences and authentication while sharing upstream conversation
storage. `notify show/set` controls Termux notification/toast delivery;
`notify test` sends fixed test text and reports each selected provider's result.
Notifications return to the live workload's registered window by default, with or
without tmux (`notify set --focus window`). Native-origin capable Termux runs in
the invoking terminal. Official APK fallback creates one named dedicated terminal
per launch, then reuses it. A tap consumes the notification; use a fresh one for
another return. Return never starts/resumes Codex. Stock APK lacks atomic existing-
window selection: manual rename or a closing-window race can open an inert empty terminal. `--focus termux` only brings the app forward.
The `UserInputRequest` notification selector covers structured questions and
follow-up input requests, with their question text. Each conversation updates one
notification, so a generic input reminder cannot remain beside its follow-up body.

## Active work across accounts

When upstream resume reports an active writer in another account, run:

```sh
codex termux task
```

The helper shows current writer-lock holders and their execution accounts, then
offers reconnect, confirmed stop, or stop and resume in the current account. A UUID can restrict
selection: `codex termux task <THREAD_UUID>`. History browsing remains upstream
`codex resume`.
Ownership follows the current kernel lock holder; an original author or a
read-only window left open by a previous account is not treated as the owner.

For explicit actions, use `task status [THREAD_UUID]`, `task reconnect THREAD_UUID`,
`task stop THREAD_UUID`, or `task takeover THREAD_UUID --profile PROFILE_ID`
after `codex termux`. Omitting `--profile` uses the fresh-launch account: explicit
`CODEX_HOME`, then the saved default, then the native default.
A completed cancellation can still leave a writer held by another connected
window. Reconnect or close that connection before ordinary takeover.

Force takeover stops the entire owning server, potentially including other
work. The interactive helper shows that scope and requires confirmation; the
explicit form uses the exact displayed `--force-server PID:START` token. It
never deletes the writer lock. Manager is optional; ordinary Codex remains usable
without this helper. See SPEC for authoritative scope and failure behavior.

## Documents

- [Provider guide](providers/chatgpt/README.md) — connection, login and work controls
- [SPEC.md](SPEC.md) — normative product and architecture contract
- [CURRENT.md](CURRENT.md) — operating baseline, proof and deferred work
- [AGENTS.md](AGENTS.md) — repository-local execution and safety rules

## Branches

- `main` — publication authority; not the implementation base
- `rewrite/rust-core` — independent orphan Rust Core implementation lineage
- `legacy/monolith` — sealed predecessor at `bf30a7d`

The implementation branch remains independent from the publication branch.
Current stable-channel promotion changes only the signed public index authority
on `main` through the accepted non-forced CAS release path; implementation
development continues on `rewrite/rust-core`.

Repository separation/cleanup is
[deferred](CURRENT.md#repository-separation-and-cleanup): retain the old repository
as an archive, move development to a fresh private `codex`, and use a separately
named public distribution repository. Names and endpoints have not changed.
The owner accepts a brief installation/update outage during the eventual move;
the new bootstrap and Core/Provider update paths still need verification.

## Account usage and automatic switching (Full)

`/switch` shows each registered profile's remaining Codex5h/weekly usage, reset
countdown and freshness/error state. It opens cached rows first and refreshes
without sending a model request. Unknown values stay unknown. API-key and other
provider allowances are not Codex account quota.

Open its settings to choose the account pool, enable automatic switching, select
5h reset/weekly reset/combined/remaining/custom ranking, adjust reserves and
weights, set account priority, pause a member or disable rotation for this client.
Rotation is off by default and new/replaced accounts are not enrolled implicitly.
Two profiles of the same account share one allowance. Manual selection still works.

```sh
codex termux usage --refresh
codex termux rotation show
codex termux rotation preview gpt-6.1-sol
codex termux rotation set '{"accounts":["default","work"],"enabled":true,"policy":"combined"}'
```

Automatic switching waits for settled native work and writer release. It retains
the conversation, terminal, CWD and session settings. A confirmed quota failure
starts one checkpoint-continuation turn on an eligible account; it does not resend
the original request or replay completed tool calls. Proactive idle switching
submits no prompt. Unknown tool outcomes, conflicting writers, stale quota,
cooldowns or missing destination capabilities pause safely with a reason.

## Development

The minimum installation is the qualified upstream runtime plus Core. Core owns
Termux execution compatibility and signed install/update/rollback; it does not
depend on Manager, the enhanced TUI or a Chat provider. Manager adds account/task/
notification/window conveniences; the TUI integrates `/switch` and provider model
selection. The provider owns its transport and host lifetime. The small shared
runtime-binding crate is compiled source, not another installed dependency.
See [the component boundary](SPEC.md#46-final-component-boundary-and-core-independence).
TUI is the component name. Existing `frontend/manager-tui`, signed artifact
identities and build flags retain their compatibility spelling. This terminology
does not yet remove the deployed Full TUI's Manager pairing requirement.

`release/upstream.json` is the single exact version/source/archive/toolchain
recipe. Core/runtime qualification runs it without acquiring or compiling Manager
or the enhanced TUI. Native frontend qualification runs independently. Source and
Android proofs are required before the controller can register immutable frontend
inputs and advance the accepted product-source pointer.

Public **base** (Core + adapted upstream UI) and **full** (+ Manager/enhanced TUI)
have separate signed heads. Ordinary updates follow the installed signed public
baseline; losing an optional binary does not move a full installation onto base.
Both publication lanes run real Android smoke, signing, public readback,
disposable update/rollback and non-forced promotion. A scheduled controller
discovers official upstream stable releases and qualifies base independently.
Immutable GitHub Releases store common base files once; full pins those exact
files plus its additional components. Existing Pages URLs remain valid. If full
fails after base advances, the previous full stays available while the next
add-on repair targets the latest successful base. Unexpected failures open a
deduplicated GitHub attention issue assigned to the owner.
Unsupported upstream transformations fail with the accepted release preserved;
automation does not invent a semantic port or skip its qualification.
Actual standalone lifecycle proof is reproducible with
`scripts/qualify_core_independence.py --help`, using disposable roots and a fresh
fixture signing key, with no real account or Chat request.

The repository owns its development rules in `AGENTS.md`, optimized for
GPT-6.1 Sol. Use concise context, direct implementation and calibrated reasoning.
`scripts/check.sh docs`, `python`, `rust`, `provider`, or `full` selects the relevant
gate. The independent `provider` gate needs qualified Bun for app-bridge and
development CDP tests;
ordinary Core checks do not.
Compiler output lives in the XDG cache; test data is disposable.
Runtime/security/installer changes and release candidates require full acceptance.

Implementation and reusable release logic live on `rewrite/rust-core`. The small
`main` caller pins accepted workflow implementation and resolves the exact accepted
product SHA from `release-source-v1`. Its controller advances that pointer only
after the source and independent Android gates succeed; frontend registration
requires its own exact build/execution proof. Workflow implementation changes need
an accepted caller repin; recipe/output updates do not. The controller explicitly
dispatches the existing proof workflows on `rewrite/rust-core`; GitHub token commits
are not assumed to trigger push workflows. Each proof/publication workflow
explicitly signals completion; the receiver joins its terminal run and rechecks
the real proofs. Default-branch workflow registrations support discovery and
initial dispatch, reject execution on `main`, and contain no implementation or
publication authority. Actual proofs run on the source branch.

Scheduled publication checks newer official backends. Manual `rebuild_current`
qualifies the accepted source recipe even when its version has not changed;
manual `publish` defaults to false. `variant` selects base, full, or both. No
signing, Release, Pages or stable mutation occurs in a dry run. Initial controller
installation and caller pinning use an accepted exact source; no signing key is
available to the input controller.


The optional [ChatGPT Provider source](providers/chatgpt/README.md) lives under
providers/chatgpt with its own source qualification. Its experimental installer
is retired. Reusable qualification tools, app/viewer assets, original evidence
and installed user data remain preserved. [CURRENT](CURRENT.md) owns the operating
baseline and deferred follow-ups; [SPEC](SPEC.md) owns public behavior.
No automatic fallback after uncertain execution is allowed.
