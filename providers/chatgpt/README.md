# ChatGPT provider

This optional provider is owned by `rewrite/rust-core`. Core executes verified
Codex on Termux; this directory owns Chat transport, model capabilities,
authentication context and host/conversation lifetime. Manager is not required.
TUI supplies native transcript/model-selection integration. No provider code is
linked into Core or Manager.

The active runtime was selectively ported from `experiment/web-termux` at
`ab41733`; the user's existing uncommitted1m catalog/profile assertions were
included here and remain untouched in that checkout. Historical experiment Core,
Manager, TUI, release workflows, probes and account data were not imported.
[Product contract](../../SPEC.md) and [current work](../../CURRENT.md) own the
remaining configuration, project placement, context and release requirements.

The local-function transport returns native tool outputs on the original Chat
call. Native Codex owns approval, actual tool execution, writer locks and history.
Independent MCP transport is retained in source with offline tests; it is not an
automatically admitted live fallback. User submissions, tool returns, model starts
and Chat allowance deductions are different quantities.

Warm requests reuse their owned prefix. A successful summary instruction remains
transient unless native history commits its checkpoint. Bounded content-free
observations record append/replay, changed instructions/tools and host-acknowledged
conversation counts; they contain no prompt text. The1m native context setting is
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

This gate runs the actual provider JavaScript/Python suites in disposable roots,
with no real Chat submissions. Missing Bun is an unfinished gate, not a skipped
success. Normal Core checks do not require the optional provider runtime.
A live trial requires explaining its purpose, scope and expected submissions;
automated tests never sample Pro or replay an uncertain submission.

Configuration remains generated upstream `--profile chatgpt` fallback and TUI
`/model` routing. One user-owned provider settings file and configured remote
ChatGPT project placement are approved work, not implemented features yet.

See [NOTICE](NOTICE.md) for dependency provenance and licenses.
