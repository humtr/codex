# ChatGPT Provider

This optional Rust component lives on `rewrite/rust-core`. Core launches the
verified upstream runtime; Provider owns Chat connection, model configuration,
Responses/tool-return mapping and host lifetime. Upstream Codex executes tools,
applies its approval/reviewer policy and owns canonical history. Neither Core nor
Manager links Provider implementation. Manager and enhanced TUI are not required
for the upstream configuration route.

[CURRENT](../../CURRENT.md#operating-baseline) distinguishes installed software,
qualified source and release/activation. The installed adapter remains the preserved
JS/Bun/Python experiment; the Rust candidate has real native/app/development proof
but is not a signed, permanently activated Provider release. Source integration,
a successful build and a model listing do not change that state.

## Workflow under review

The [ChatGPT-operated Codex proposal](../../CURRENT.md#proposed-chatgpt-control-through-mcp)
would let ChatGPT use MCP to manage local sessions/context, discover tools, develop,
report and finish, with either Codex or ChatGPT as the entrance. It is not the
current two-tool MCP return transport, an implemented control API or a selected
default. Origin/control context would use existing non-user tool returns; local
bindings own the actual reply destination. No extra user prompt is proposed.

The single [work order](../../CURRENT.md#product-completion-scope-and-work-order)
stabilizes shared primitives and a recoverable Rust baseline before comparing the
alternative, then completes the selected product and signed delivery. It does not
require polishing every existing adapter policy before experimentation. Existing
Rust local functions are an explicit fallback candidate; no uncertain mid-task
switch or silent legacy-runtime fallback is allowed. SPEC retains the accepted
model-adapter behavior until an explicit replacement contract and qualification.

## Candidate configuration and commands

The source executable is `codex-chatgpt-provider`; these are candidate developer
commands, not commands promised by the current signed installation. Its settings
and catalog have one authority:
`$XDG_CONFIG_HOME/codex/providers/chatgpt.toml`, default
`~/.config/codex/providers/chatgpt.toml`. The installed JS adapter does not read it.

- `models` reads `catalog.models`; `catalog` generates native model JSON.
- `config show|effective|validate|edit` inspects or edits settings. Effective output
  reports file/default/per-model origins and `runtime_application=not_queried`.
- `config set FIELD VALUE [FIELD VALUE ...]` validates dependent values in one
  atomic transaction; `config unset FIELD ...` restores defaults. Cooperating
  writers serialize, stale editors refuse overwrite, and invalid edits preserve
  the old file. `config edit` uses `VISUAL`/`EDITOR`.
- `prepare [--home PATH] [--state PATH]` generates an immutable catalog and shared
  command-auth profile for an existing selected home, preserving local overrides,
  base settings and login. It may authenticate/start the configured Rust service;
  it submits no Chat or project creation. No bearer is written in the profile.

Initial catalog data is in `default-models.toml`, not a Rust model-family list.
GPT-6 and manual-only Pro are retained; GPT-5.6 is excluded. A hidden Instant alias
is compatibility data. Model/effort/context entries are editable without rebuilding
unless the host protocol changes. Listing a route does not prove entitlement.

Per-model context fields include `window_tokens` and `auto_compact_token_limit`;
explicit upstream invocation overrides win. The user's1,000,000-token override is
preserved policy, not measured Chat capacity. An unset threshold delegates to
upstream and does not disable model/hash-triggered compaction. Saving settings does
not mutate a bound native catalog; prepare for the next startup. Provider reads
compatible host-route/history changes at request admission.

`/model` is the primary enhanced-TUI selection surface; upstream `--profile chatgpt`
is the configuration fallback, distinct from Manager login accounts. Current exact
source/device composition proof and pending real-Chat TUI/lifecycle gates are in
[CURRENT](../../CURRENT.md#remaining-acceptance-and-delivery).

## Service and observation

| Candidate command | Effect |
| --- | --- |
| `serve [--descriptor PATH] [--state PATH] [--port PORT]` | Run the loopback Responses service using a provisioned owned host or explicit qualification relay |
| `token [--state PATH]` | Authenticate and optionally start that same configured executable; never paste its credential output into chat/logs |
| `status [--state PATH] [--thread ID]` | Read local process/last lane observation; never starts, queries, resumes or cancels Chat |
| `reveal --thread ID [--state PATH]` | Select the exact bound saved Chat conversation through an existing service; no model message/approval/viewer startup |
| `stop [--state PATH]` | Request PID-bound graceful Provider service shutdown; preserve journal/app profile/config and report unconfirmed shutdown accurately |
| `view [--state PATH]` | Open the optional Rust viewer against configured noVNC assets/private RFB; may start service, never submit Chat |
| `mcp-stdio [--state PATH]` | Forward bounded multiplexed MCP JSON-RPC to an already-running verified Provider for an existing tunnel |

Run qualification services from private immutable binary snapshots, not Cargo
outputs replaced by later builds. Bridge-source revision, process/account/workspace
identity and exclusive app-home lease bind the owner. Failed readiness of a live
owner never authorizes restarting it. Startups serialize; restart retains the
journal's loopback port and rotates its local credential. An occupied saved port
fails before app startup. Content-free startup errors stay private.

`host.rootfs` selects an already provisioned official-app rootfs. Rust uses PRoot,
Xvnc and private Chromium pipes, with app home under Provider state and temporary
display resources. No existing profile is copied; no rootfs/app install or login is
implicit. `--descriptor` is an explicit qualification relay, not a production
Bun/Python dependency or automatic fallback. Ordinary Base/Full startup/update must
not download or log into the app. Third-party app binaries/auth/user data are not
bundled here. Korean fonts and pinned icons of the existing qualification desktop
are not proof of a changed Rust viewer or new user login.

Status separates endpoint liveness, last host observation, pending native calls,
host confirmation and stop requested/acknowledged/unknown. An old journal or a
content-free SSE keepalive proves neither current model progress nor task
completion. Provider service shutdown does not prove a native tool stopped or undo
its effects; native task control has a separate upstream authority. Physical CLI
progress and confirmed task Stop remain explicit acceptance requirements.
Service shutdown is confirmed by the pinned kernel process handle; losing the
recorded executable path during teardown leaves stop unconfirmed until actual exit.

The optional viewer requires `host.viewer_assets`, shares the owned RFB socket and
uses a separate temporary capability. Missing assets do not block inference.
Mobile keyboard/scroll/Esc/fullscreen controls require physical acceptance beyond
HTTP/WebSocket/RFB fixtures. Revealing a conversation does not prove the visible
Android screen changed. Do not restart a live owner to apply host settings.

## Existing result transports

`transport.return_path` chooses `local_function` (current default) or explicit
`mcp`. This is a result-transport setting within the existing model adapter, not
selection of the proposed ChatGPT-operated workflow. Return path/connector changes
require settled service restart; uncertain dispatch never triggers fallback.

Local functions return actual native results on their original app calls.
The MCP path exposes `native_catalog` and `native_tool` under a per-turn capability,
projects native calls through the same execution/history path and returns results
on the original MCP invocation. It does not expose general session/context control.
Neutral discovery describes capabilities; catalog access is read-only metadata and
execution advertises possible state changes. Exact logical IDs/inputs/results bind
retries independently of JSON-RPC IDs. A detached MCP waiter is not an execution
cancel; the owning Responses request retains its own cancellation semantics.

Set `transport.connector_name` for the installed connector; the default is
`Termux Codex`. Historical `Codex Native2` naming is not required. Tunnel IDs,
clients and targets remain independent from Tdev. A changed tunnel alias may still
run its old stdio child: reconnect after settlement and verify actual executable/
state argv instead of relying on readiness metadata. No replay follows uncertainty.

Host approval/authentication retains the exact request and shows a waiting report.
Use the signed app's actual consent control; existing explicit bounded delegation
may authorize selecting it, never fabricating consent. A correlated host permission
reply is counted separately and does not retire the retained MCP capability. A
stopped/restarted owner never silently reactivates old pending work.

The bounded actual R11 MCP trial completed two commands/effect+verification and
public/final native output, with one user object and two app dispatches including
one real Allow reply. This is not full-control or long-development qualification.
App dispatches, user-role objects, tool returns and charged Chat message limits
are different quantities; the last remains unknown. Neither same-call returns nor
project grouping establishes a free allowance or absent remote history.

## Context, projects and transcripts

The existing adapter tracks warm-prefix deltas and native checkpoint acknowledgment.
Summary policy/results are transient until upstream commits the offered checkpoint;
identical summary text in unchanged old input is not acknowledgment. Public `ocx1:`
checkpoints do not decode or share OpenAI's foreign opaque reasoning format.
Native compaction and model/hash transitions remain upstream lifecycle operations.

`checkpoint --thread ID --from-model ID --model ID --effort EFFORT` reads a complete
idle public snapshot from stdin and emits an external-context message. It starts no
app/inference and writes no native history. TUI uses the upstream API to persist it,
preserving original history/thread and source readability. Same-thread or declared
fork/parent lineage and current policy/later input are validated. Historical calls
are quoted observations, never executable replay. A synthetic API/PTY handoff does
not establish all real-Chat TUI/downshift/cold-resume transitions.

Public initial/intermediate/final reports and real tools use native output events.
Known MemoryCite/box presentation converts to append-only terminal Markdown while
literal code, unknown markup and exact tool data remain unchanged. No formatting
model request or private reasoning export occurs. A rejected single gateway envelope
may receive at most three original-call error corrections before any native effect;
uncertain delivery/identity/compaction errors never use that path. Native function
argument validation stays native; correction model starts remain real usage.

Independent native reviewer lanes preserve upstream decision schemas and exclude
the primary project; self-review is not equivalent. Existing allow/deny fixtures
have synthetic replies, not qualified real Chat judgment. A question fixture also
returns the actual answer on the original call; physical question UI is separate.

Project mode is explicit, for example:

```sh
codex-chatgpt-provider config set history.mode project history.project_name "Codex"
```

`history.project_id` disambiguates an exact owned private/unshared project;
`history.create_if_missing` controls creation. Preserve existing memory scope,
conversations and personalization. Duplicate/inaccessible destinations fail, and
uncertain create is reconciled before retry. Destination changes affect new lanes.
Project grouping reduces clutter, not stored history or established message costs.
Archive/delete/temporary retention is not admitted. Full contracts are in
[SPEC](../../SPEC.md#approved-provider-configuration-context-and-remote-project-behavior).

## Verification and migration

```sh
BUN=/absolute/path/to/qualified/bun scripts/check.sh provider
```

This runs Rust fmt/tests/clippy/release plus retained JS/Python regression suites
in disposable roots without Chat submissions. Automated qualification never
samples Pro or GPT-5.6; live trials first declare purpose, scope and expected
submissions. Missing Bun is an unfinished
reference-suite gate, not a Core dependency. Actual Android and app execution
proof, signed composition/lifecycle and bounded activation remain separate.

The Android qualification job retains an unsigned snapshot for30 days, including
the executable, exact source/run/toolchain identity, dependency lock and licenses.
Before copying its executable into a private immutable qualification location,
read back every declared file against its inventory and source commit:

```sh
python3 -B providers/chatgpt/candidate.py verify \
  --artifact /absolute/path/to/provider-android --source . \
  --source-sha EXACT_40_HEX_COMMIT
```

The verifier rejects missing/extra/nonregular files, changed content and source
assets. Its unsigned inventory establishes consistency with the selected build
record, not publisher authentication or device acceptance. Preserve that record
with the build/test and disposable restart/rollback proof. It never installs,
starts a host or changes the operating Provider.

The existing no-MCP Rust path already passed bounded direct `~/prj/ai` development
and supervisor verification. Future materially different paths need their own real
trial there under its AGENTS/Git baseline, without a temporary worktree or a
supervisor-written substitute patch. Preserve unrelated user edits; exercise
analysis, failure/correction, tests and latest-state resume. The single
[acceptance matrix](../../CURRENT.md#remaining-acceptance-and-delivery) owns open
gates; existing proof does not qualify the new MCP control workflow.

The preserved experimental installer is a development/migration reference, not
signed Provider delivery or an automatic fallback. Port behavior/proof before
removing runtime responsibilities; preserve unique experiment source/user1m edits,
licenses, credentials and conversations. No whole historical Core/TUI/release
snapshot is merged. Retire superseded JS/Bun/Python product requirements only after
accepted Rust replacement; qualification tooling may remain. See [NOTICE](NOTICE.md)
for reused-source provenance and licenses.
