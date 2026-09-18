# Codex for Termux

This repository is the clean rewrite of the Termux compatibility layer for the
upstream Codex CLI.

The rewrite has one public command, `codex`, and two internal layers:

- a minimal native Rust Core that makes the upstream runtime work correctly on
  Termux and owns installation, update, diagnosis, activation, and rollback;
- a separate Manager layer reached through `codex termux` for profiles,
  sessions, notifications, and other Termux conveniences.

## Current status

The signed public stable channel is release sequence 11 generation
`local-hosted-0-154-0-37fbbd8033b8-rald45-transition`, serving upstream
`codex-cli 0.154.0`. The Rust Core owns signed installation, update, diagnosis,
atomic activation, rollback, and recovery; Manager remains behind
`codex termux`.

Release-automation phases through RALD-6 are accepted in `GOAL.md`. RALD-6
proved the public fresh-install surface from an empty disposable Termux-shaped
environment and proved that the same installed client can follow the normal
signed `codex update` path to a newer production-authority fixture. RALD-7
full acceptance/activation has not started.

## Fresh install

A fresh Termux environment needs the existing Termux shell, curl, and OpenSSL.
No Rust compiler, package-manager install, release private key, or GitHub account
is required on the device.

The accepted public entrypoint is the immutable RALD-6 online installer:

```sh
"$PREFIX/bin/curl" -q --fail --silent --show-error --location --proto '=https' --proto-redir '=https' --tlsv1.2 --connect-timeout 15 --max-time 60 --max-filesize 131072 'https://raw.githubusercontent.com/humtr/codex/9972a3288c0531ba744e9bd1356273d9a080aa79/install-online.sh' | "$PREFIX/bin/sh"
```

The network frontend is only acquisition. It pins the accepted bootstrap public
key, verifies the signed stable index and release manifest before using their
network-selected values, downloads the bounded signed inventory, and delegates
all persistent writes to the existing local
`install.sh -> bootstrap/codex-bootstrap -> Core` path.

For an already downloaded signed bundle, `install.sh` remains the local/offline
frontend defined by `SPEC.md`.

## Updating

After installation, ordinary updates remain Core-owned:

```sh
codex update
```

The default route consumes the signed stable channel. It does not delegate to
the upstream self-updater, and publication credentials are not device update
authority.

## Documents

- `SPEC.md` — normative product and architecture contract
- `GOAL.md` — success threshold and acceptance ledger
- `WORKBOARD.md` — current milestone and next work only
- `AGENTS.md` — repository-local execution and safety rules

## Branches

- `main` — publication authority; not the implementation base
- `rewrite/rust-core` — independent orphan Rust Core implementation lineage
- `legacy/monolith` — sealed predecessor at `bf30a7d`

The implementation branch remains independent from the publication branch.
Current stable-channel promotion changes only the signed public index authority
on `main` through the accepted non-forced CAS release path; implementation
development continues on `rewrite/rust-core`.
