# Codex for Termux

This repository is the clean rewrite of the Termux compatibility layer for the
upstream Codex CLI.

The rewrite has one public command, `codex`, and two internal layers:

- a minimal native Rust Core that makes the upstream runtime work correctly on
  Termux and owns installation, update, diagnosis, activation, and rollback;
- a separate Manager layer reached through `codex termux` for execution
  profiles and notifications. Conversation
  discovery and resume remain upstream-owned.

## Current status

See `GOAL.md`'s Current Operating Baseline for the accepted source, signed public
release and installed generation. The Rust Core owns signed installation,
update, diagnosis, atomic activation, rollback, and recovery; Manager remains
optional behind `codex termux`.

`codex termux help` lists the installed Manager commands. Profiles separate
execution preferences and authentication while sharing upstream conversation
storage. `notify show/set` controls Termux notification/toast delivery;
`notify test` sends fixed test text and reports each selected provider's result.
Notification taps foreground Termux. With `notify set --focus tmux` and AI tmux
launch, a qualified live conversation selects one named Android terminal attached
to its existing tmux session, creating it only if absent. Repeated taps reuse it;
closing it permits one replacement. No new Codex workload or `resume` is started.
The `UserInputRequest` notification selector covers structured questions and
follow-up input requests. Diagnosis and recovery remain available through Core
`codex doctor`, `codex update`, and `codex update --rollback`.
Exact contracts belong to `SPEC.md`.

Release-automation phases through RALD-7 are accepted in `GOAL.md`. RALD-6
proved the public fresh-install surface and same-client signed update delivery.
RALD-7 completed full repository acceptance, activated the bounded six-hour
official producer, and promoted the first real newer official candidate through
the complete build/smoke/sign/Release/Pages/readback/runtime/non-forced-CAS
path.

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
