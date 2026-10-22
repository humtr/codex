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
- an optional Provider for ChatGPT model connection, currently experimental;
  its approved target is a Rust adapter with a minimal app-side JavaScript bridge.

Every Termux Codex execution passes through Core. Actual tools, reviewer,
conversation storage, compaction and resume remain upstream-owned. Optional
preparation/configuration isolation and no-Manager TUI/Provider execution have
source and bounded Android proof; signed composition and operating activation
are distinct. See [current status and work order](CURRENT.md#product-completion-scope-and-work-order).

The optional Provider is the **ChatGPT-Codex bridge**: model adapter, native tool
adapter and shared Chat/session management. ChatGPT supplies intelligence; native
Codex executes, approves and records. The common managed development loop is
Codex -> Linux app API -> work Chat -> MCP or app-injected local functions -> the
same native work. ChatGPT general development submits work into that same loop
through MCP; ChatGPT direct development uses the current Chat to control native
work through MCP. Local functions are an inner managed-Chat choice, not a second
direct ChatGPT entrance. Linux app removal is not the objective.

[E design and functional decisions](CURRENT.md#agreed-bridge-structure-and-e-work)
come first, including Linux app/host management research and concrete acceptance.
The settled [selected contracts](SPEC.md#e-selected-common-interfaces-and-lifecycle)
cover identity/context mapping, existing-Chat input/control transfer, coordinated
agents and host management, with source capability findings in CURRENT. E design
and common idempotency source work are complete. F1–F9 combined implementation
and bounded integration qualification are also complete: shared roles, three
usage modes, saved-Chat reading/input, coordinated agents, host management and
matched development comparisons. F qualified/applied the Rust idempotency policy,
preserving all other annotation values. Actual, synthetic and recovered-result
evidence and their limitations are recorded in CURRENT's F final review; existing
C/C2, D2/D3 and direct ChatGPT two-turn proof retain their own scopes.
Alternate app-free clients remain optional research. G's public signed
Base89/Full90 and optional Provider2 delivery, readback and actual Android
update/rollback consumption pass on exact source c65d3f8. Broader declared
runtime/real-Chat TUI/lifecycle qualification remains active, while device
activation and H retirement retain separate gates. Installed/bare-config defaults,
legacy `execution.mode` names and active runtime remain unchanged.
The same Codex thread retains long-term history while replaceable Chat conversations
receive summaries plus confirmed incremental work and current state. Origin/control and
handoff information travels through non-user tool returns. Current execution,
cancellation and updates are unchanged.

## Current status

See `CURRENT.md` for the accepted source, signed public
release and installed generation. F's common bridge, shared reading, agent and
host integration has bounded acceptance; its exact actual/synthetic scopes are
recorded there. G product/lifecycle/delivery and device activation remain separate;
installed Provider defaults are unchanged. The Rust Core owns signed installation,
update, diagnosis, atomic activation, rollback, and recovery; Manager remains
optional behind `codex termux`.

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
Diagnosis and recovery remain available through Core
`codex doctor`, `codex update`, and `codex update --rollback`.
Exact contracts belong to `SPEC.md`.

To run the direct Core path for one invocation while Manager is installed:

```sh
CODEX_TERMUX_CORE_ONLY=1 codex
```

This keeps signed Core execution and its safety preparation while skipping
Manager profile/window policy, the optional TUI and Manager notification hooks
for that run.

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
requires Manager. The opt-in ChatGPT pilot
uses this common registration; `/model` selects its model and supported effort,
while `--profile chatgpt` remains the fallback. The pilot's signed Linux app still
owns its login and Chat inference. Active source and focused qualification now live in
[the optional Provider](providers/chatgpt/README.md). Historical experiment source
and unique drivers remain preserved for reviewed integration/closure; provenance
belongs to the Provider NOTICE and bounded proof to CURRENT.
The installed adapter remains JS/Bun/Python. The independent Rust candidate
already implements transport/configuration/host lifetime and has separate actual
authenticated-app, native tools, project and direct AI-development proofs. It is
not permanently activated or a signed Provider release; its optional app-renderer
bridge remains part of the qualified app transport. Neither source integration
nor a model listing replaces the app's login or actual runtime qualification.
G adds an independent signed optional payload and source Core commands for explicit
install/status/rollback and `codex provider chatgpt ...` dispatch. These candidate
commands do not change the currently installed Core. Package selection preserves
existing profiles until explicit `prepare`; see the [package interface](providers/chatgpt/README.md#optional-package-selection-g-candidate).
The MCP app's display name is **Termux Bridge**, configurable through
`transport.connector_name`. The CLI provider identifier remains `chatgpt`.

The recorded installed-pilot transition failure is `unsupported_desktop_route`
when old-model compaction reaches the Chat endpoint; foreign opaque checkpoints
also require readable handoff. Preserve existing conversations. Source API/PTY
handoff and real Chat summary have separate bounded proofs; they do not qualify
every long conversation or a new signed operating composition. See
[current diagnosis](CURRENT.md#confirmed-operating-chat-blocker).

The Rust candidate owns editable configuration/catalog data at
`~/.config/codex/providers/chatgpt.toml` (honoring `XDG_CONFIG_HOME`), including
model/effort/context and remote ChatGPT project settings. The installed JS adapter
does not read this file. The preserved1,000,000-token setting is an operator
override, not a measured Chat capacity. Model/effort changes within one selected
Provider use native session settings; cross-provider history requires its own
qualified handoff. Project mode reuses/creates an explicitly configured remote
project for new chats; it does not move/delete existing chats or prove lower Chat
message costs. See [configuration](CURRENT.md#context-and-easy-configuration),
[project behavior](CURRENT.md#chatgpt-projects-and-history) and the
[remaining acceptance](CURRENT.md#remaining-acceptance-and-delivery).

## Fresh install

A fresh Termux environment needs the existing Termux shell, curl, and OpenSSL.
No Rust compiler, package-manager install, release private key, or GitHub account
is required on the device.

The accepted public entrypoint is the immutable online installer:

```sh
"$PREFIX/bin/curl" -q --fail --silent --show-error --location --proto '=https' --proto-redir '=https' --tlsv1.2 --connect-timeout 15 --max-time 60 --max-filesize 131072 'https://raw.githubusercontent.com/humtr/codex/eba60a7a72cacc9f6be126606ee992c00e662f5d/install-online.sh' | "$PREFIX/bin/sh"
```

The network frontend is only acquisition. It pins the accepted bootstrap public
key, verifies the signed stable index and release manifest before using their
network-selected values, downloads the bounded signed inventory, and delegates
all persistent writes to the existing local
`install.sh -> bootstrap/codex-bootstrap -> Core` path.

For an already downloaded signed bundle, `install.sh` remains the local/offline
frontend defined by `SPEC.md`.
The online installer defaults to full. Save the same script and run
`sh install-online.sh --variant base` for the standalone Core/upstream edition.
Component acquisition is automatic; both editions install a complete signed
generation through the same lifecycle.

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

## Profile selection and optional tmux

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

- `SPEC.md` — normative product and architecture contract
- `CURRENT.md` — operating baseline, proof and remaining work
- `AGENTS.md` — repository-local execution and safety rules

## Branches

- `main` — publication authority; not the implementation base
- `rewrite/rust-core` — independent orphan Rust Core implementation lineage
- `legacy/monolith` — sealed predecessor at `bf30a7d`

The implementation branch remains independent from the publication branch.
Current stable-channel promotion changes only the signed public index authority
on `main` through the accepted non-forced CAS release path; implementation
development continues on `rewrite/rust-core`.

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
gate. The independent `provider` gate needs its qualified reference-test Bun;
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


The optional [ChatGPT provider source](providers/chatgpt/README.md) is integrated
under providers/chatgpt, with its own offline source qualification. Its development
installer and existing app/user data remain separate from signed Base/Full delivery.
Source integration does not activate a new operating Provider. CURRENT is the
single authority for the revised work sequence, completed proof and remaining
context/project/lifecycle/packaging gates. The broader MCP control path has opt-in
source and separate synthetic/real-Chat qualification. The shared bridge structure,
three usage modes and remaining work are recorded in CURRENT: E settled design,
features and app/host management; F implemented and boundedly qualified the
bridge and common saved-Chat discovery/selection/read from Codex and ChatGPT.
G product/delivery qualification is active; H retires obsolete paths after accepted replacement;
bare configuration still uses model_adapter/local_function. Documentation does not
activate the combined design or change the installed Provider.
No automatic fallback after uncertain execution is allowed.
