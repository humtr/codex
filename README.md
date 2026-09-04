# Codex for Termux

This repository is the clean rewrite of the Termux compatibility layer for the
upstream Codex CLI.

The rewrite has one public command, `codex`, and two internal layers:

- a minimal native Rust Core that makes the upstream runtime work correctly on
  Termux and owns installation, update, diagnosis, activation, and rollback;
- a separate Manager layer reached through `codex termux` for profiles,
  sessions, notifications, and other Termux conveniences.

## Current status

The Rust Core implementation through M2 and the authorized local install/cutover
are accepted in the `GOAL.md` acceptance ledger. The R4 wrapper-owned update
boundary, signed-channel admission, doctor presentation, and code-mode
companion placement are also accepted. R5 adds the Rust release-builder fetch
path for official upstream build inputs. R6 adds the non-installed publisher
that creates the signed wrapper index and adapted release tree consumed by
Core; its acceptance evidence is recorded in `GOAL.md`.

The R4 Core launcher is installed in the working Termux runtime through a
bounded, digest-checked device cutover. Ordinary `codex update` is now owned by
Core and can activate only a signed generation prepared by the wrapper release
pipeline; it never delegates to the upstream self-updater. The existing signed
v1 generation remains active, so `codex doctor` reports its legacy `compat/`
layout as `migration_required` until a newly signed root-level companion
generation is delivered. Publishing that signed channel to the external wrapper
distribution surface remains a separate operational gate.

`codex-release-builder fetch --version <MAJOR.MINOR.PATCH>` obtains only the
official versioned OpenAI archive, prints its exact SHA-256, and leaves the
installed Core out of the build step. The release pipeline passes that archive
and digest to `codex-release-builder build`, then passes the unsigned generation
to `codex-release-builder publish` with the current release private key. The
publisher emits `update-index-v1[.sig]` and `releases/<generation-id>/` using
the Core `codex-release-v3` format; it performs no upload to OpenAI or any
remote service. A deployment must publish that generated tree through the
wrapper's chosen distribution surface before live `codex update` can succeed.

The publication command is:

```text
codex-release-builder publish --generation <ABSOLUTE_DIRECTORY> \
  --release-sequence <POSITIVE_DECIMAL> \
  --release-base <HTTPS_BASE_URL> \
  --private-key <ABSOLUTE_FILE> \
  --openssl <ABSOLUTE_EXECUTABLE> \
  --output <ABSENT_ABSOLUTE_DIRECTORY>
```

The private key is read only for Ed25519 derivation/signing and is not copied
to the output, repository, or installed device. The current live update remains
fail-closed until an authorized operator publishes a bundle signed by the
active Core `update_key`.

Implementation is intentionally split into two milestones:

1. local Rust Core execution and compatibility contracts;
2. secure fresh installation, self-update, generation activation, and rollback.

Independent product review follows the completed Milestone 2 candidate rather
than interrupting ordinary implementation checkpoints.

## Documents

- `SPEC.md` — normative product and architecture contract
- `GOAL.md` — success threshold and acceptance ledger
- `WORKBOARD.md` — current milestone and next work only
- `AGENTS.md` — repository-local execution and safety rules

## Branches

- `main` — publication authority; not the implementation base
- `rewrite/rust-core` — independent orphan Rust Core implementation lineage
- `legacy/monolith` — sealed predecessor at `bf30a7d`

The implementation branch starts at an empty root and has no Git ancestry in
common with `main` or the legacy implementation. Promotion to `main` is an
explicit ref replacement after acceptance, not a merge.
