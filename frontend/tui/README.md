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

## Live /provider acceptance remains open

Earlier whole [TUI Native CI](https://github.com/humtr/codex/actions/runs/42742653586)
passed on `f1ba6719`; later TUI qualifier fixes at `5384d938` passed
40 Python and 10 assembly checks. Synthetic Core→TUI→Chat→Native
`sameThread=true` is **not** a real model admission.

In the candidate test, `/provider` selected GPT-6. Its first real request
then returned `native_thread_owner_unavailable`, because the operating
Provider's Core/home/workspace did not match the isolated candidate's Native
writer. This does not independently implicate Core or Manager. The next test
must bind the same actual writer, complete two real GPT-6 turns on the same
Native thread and independently prove stream-cut/HTTP 413 recovery and
unrelated-work concurrency. Do not re-submit a possibly accepted turn.

Before any live fixture, use the
[mandatory preflight](../../docs/TERMUX_CONSTRAINTS.md): real dispatcher
`$PREFIX/bin/codex`, actual generation, correct configured Provider owner,
short private socket `TMPDIR` (107 encoded bytes maximum) and distinct
per-work evidence scratch. Never change operating signed Full159/Full153,
profiles, services or Android sdcard for qualification alone.
