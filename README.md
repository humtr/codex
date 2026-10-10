# Codex for Termux

The native Core runs upstream Codex on Termux and owns install, update, diagnosis
and rollback. Optional Manager provides profiles, /switch and notifications.

[Usage and development](https://github.com/humtr/codex/blob/rewrite/rust-core/README.md),
[product contract](https://github.com/humtr/codex/blob/rewrite/rust-core/SPEC.md)
and [current work](https://github.com/humtr/codex/blob/rewrite/rust-core/GOAL.md)
live with the implementation. main owns the signed stable index and an immutable
caller; it has no mirrored product spec, goal ledger or release implementation.

A fresh Termux install needs curl and OpenSSL:

```sh
"$PREFIX/bin/curl" -q --fail --silent --show-error --location --proto '=https' --proto-redir '=https' --tlsv1.2 --connect-timeout 15 --max-time 60 --max-filesize 131072 'https://raw.githubusercontent.com/humtr/codex/9972a3288c0531ba744e9bd1356273d9a080aa79/install-online.sh' | "$PREFIX/bin/sh"
```

Then use `codex`, `codex update`, `codex doctor`, or `codex termux help`.
Older browser-bridge installations needing the immutable migration Release should
follow the [migration instructions](https://github.com/humtr/codex/blob/rewrite/rust-core/README.md).

The six-hour schedule publishes only qualified newer upstream pairs. Manual
`publish` defaults to false: unsigned build and smoke only. `rebuild_current`
selects the authenticated qualified current backend at the next sequence; signing,
public readback, disposable update and stable promotion require `publish=true`.
Neither control edits device profiles, sessions or installed runtime.
