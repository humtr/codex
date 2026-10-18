# ChatGPT provider

This optional provider is owned by `rewrite/rust-core`. Core executes verified
Codex on Termux; this directory owns Chat transport, model capabilities,
authentication context and host/conversation lifetime. Manager is not required.
TUI supplies native transcript/model-selection integration. No provider code is
linked into Core or Manager.

## Approved Rust migration

The production target is a standalone Rust Provider package here
(`Cargo.toml`, `src/`), with minimal app-side JavaScript and separately inventoried
optional viewer assets. The initial Rust package now validates and edits one private
configuration file with disposable-root tests. It does not yet execute Responses,
tool transport, CDP or host lifetime; today's installed runtime remains JS/Bun plus
Python. Experiment closure and Rust runtime admission are still pending. The repository-wide ordered work
is in [CURRENT.md](../../CURRENT.md#product-completion-scope-and-work-order);
this file does not define a competing roadmap.

| Responsibility | Target implementation |
| --- | --- |
| Configuration, effective values and model capabilities | Rust; one validator and generated catalog/profile outputs |
| Responses admission, streaming, tools, deltas and compaction | Rust; preserve upstream history/approval/execution ownership |
| App connection, host lifetime, cancellation and reconciliation | Rust; bounded CDP and process ownership |
| App-local function exposure, events and result delivery | Minimal JS executed in the app renderer; typed/versioned wire boundary to Rust |
| Optional remote viewer | Browser JS/TS/CSS assets; not a mandatory inference path |
| Build/research/qualification helpers | Rust, Python or shell as appropriate; not product runtime dependencies |

Current code demonstrates an app JS/React connection point; it does not establish
the language of every part of the desktop app. Moving the adapter to Rust must not
move credentials out of the app, duplicate tool execution, add model requests or
promise a different Chat allowance. Rust owns policy; the injected bridge only
implements the app operations/events needed by the qualified transport.

Port existing deterministic boundary tests and real-Core drivers before removing
each legacy runtime responsibility. Keep the working installed provider unchanged
until candidate admission and a separate bounded activation. Preserve data paths,
account/workspace/conversation identity, context overrides and pending-call state;
do not restart live work or guess whether an uncertain submission completed.
After the Rust product path passes equivalent acceptance, retire superseded
runtime JS/Python and Bun requirements. Retained signed generations supply explicit
rollback, not a second automatically selected implementation. Keep provenance and
licenses for reused or translated code. No whole historical Core/TUI/release
snapshot is merged from the experiment.

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
transient unless native history commits its checkpoint. Identical summary text in
the previous prefix is not a new checkpoint acknowledgment; actual history
replacement is required before retiring a lane. Bounded content-free
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

This gate runs Rust formatting, tests, lint and release build, then the existing
JavaScript/Python suites in disposable roots; it performs no real Chat submissions.
Missing Bun is an unfinished legacy regression gate, not a skipped success. Normal
Core checks do not require the optional provider runtime. Actual Android binary and
app-bridge execution gates are still needed before Rust runtime admission.
A live trial requires explaining its purpose, scope and expected submissions;
automated tests never sample Pro or replay an uncertain submission.

Configuration remains generated upstream `--profile chatgpt` fallback and TUI
`/model` routing. One user-owned provider settings file and configured remote
ChatGPT project placement are approved work, not implemented features yet.

See [NOTICE](NOTICE.md) for dependency provenance and licenses.
