# Codex TUI — source and qualification

The TUI is an **independent optional upstream-based frontend**. Core is the
only native runtime launcher/executor. Manager provides separate optional
account/terminal features; TUI's Manager integration is not a dependency
for ordinary Native execution.

[Current state](../../CURRENT.md) ·
[UI and Bridge contract index](../../docs/spec/INDEX.md) ·
[Termux path and ownership constraints](../../docs/TERMUX_CONSTRAINTS.md)

## Source authority

The maintained source is `frontend/tui`; `native.patch` integrates the
exact official upstream checkout pinned by `release/upstream.json`.
Release Builder owns common Rust preparation and Native patch composition.
Do not apply a TUI patch by itself or inject separate backend/owner code.
Canonical session writer, tool, approval, model-setting and resume semantics
remain upstream/Native-owned.

The external identifiers `manager-tui-native`,
`termux-manager-tui-v1`, and accepted `--manager-tui` builder flags
remain compatibility contracts; they do not make this a Manager subprocess.
Builders can use `--tui`, `--tui-version` and `adapt-tui` as the internal
maintained names. Preserve published artifact IDs/signatures and aliases.

## Assembly and validation

On a *disposable* copy of the exact official upstream source, obtain the
Release Builder's shared `prepare-source --frontend provided` output
`PREPARATION` and use the existing assembler once:

```sh
python3 frontend/tui/apply.py UPSTREAM   --builder BUILDER   --native-preparation PREPARATION
```

The arguments above are placeholders for the builder and exact preparation
produced for this source; they are not permission to edit an existing live
installation. Preflight patch-base, upstream commit, source scope, formatting,
compiler diagnostics and affected native API contracts. Verify the final
candidate at its own exact SHA in whole TUI CI and actual Android Core
execution. A passing CI for an earlier SHA cannot qualify a later candidate.

## Existing-thread re-entry intent

A qualified profile or model re-entry binds its private launch intent to the
current process before replacing it with `exec`. Both actual upstream entry
points consume and remove that intent before arg0 dispatch starts threads.
Only initial startup receives the typed restore choice. Later ordinary resume
keeps its explicit model and service-tier override contract. Core also removes
the intent from each Runtime child, including Native servers and probes.

The PID binding scopes UI launch intent; it grants no Native ownership or account
permission. Core, Manager and owner re-entry preserve the same PID through their
existing `exec` paths. Spawned children, malformed bindings and older unbound
markers cannot replace an ordinary invocation's explicit selection. This private
handoff requires compatible producers and consumers; it does not rewrite older
signed generations or acquire an existing Writer.

## Product acceptance boundaries

Qualify the exact changed source through full TUI CI and the affected Native
product paths. Synthetic responses can prove picker routing, profile transitions,
existing-thread history and actual Native command effects; they do not prove
inference with an authenticated ChatGPT app. Successful operating App/Native
inference likewise does not qualify a different candidate installation.

Before a live fixture, follow the
[mandatory preflight](../../docs/TERMUX_CONSTRAINTS.md): exact dispatcher and
generation, the actual configured Provider/AppHost owner, a short private socket
`TMPDIR` (107 encoded bytes maximum) and distinct evidence scratch. A private
candidate cannot borrow an operating AppHost journal, registry or Writer to
satisfy that ownership boundary. If candidate activation needs an operating
change, preserve the source and isolated proofs and report that separate gate.
Do not replace signed generations, profiles, services or account assets for a
qualification alone, and do not re-submit a possibly accepted turn.
