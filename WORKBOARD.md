# Rust Core Workboard

This file owns only the current implementation, repository-maintenance, release,
or live-consumer bundle. Normative product behavior belongs in `SPEC.md`.
Accepted and historical evidence belongs in `GOAL.md`. Release mechanics belong
in `RELEASE_AUTOMATION_PLAN.md`.

## Current routing

- Repository: `humtr/codex`.
- Source authority branch: `rewrite/rust-core`; always re-read the remote head
  before work. Do not use a remembered SHA as current authority.
- Selected maintenance bundle:
  **AUTHORITY-COMPACTION-V1 / RUNTIME-ALIGNMENT-UPSTREAM-0.159.2**.
- Authority compaction completed and was pushed at
  `28cea0b97c7b4ae32e7b37816f2f1d277fdb07a6`.
- Product-code mutation in this bundle: **none**. Runtime alignment must use the
  already accepted product source and existing signed publication/activation
  boundaries.
- Accepted product implementation for the current feature set is exact source
  `4fd14ed8aafcf29602f43e00c314eb8da39a6e5f`. The later authority commits are
  documentation-only.
- Current signed public stable before alignment is sequence **23**, generation
  `local-hosted-0-159-0-b164e5b61cb4`, upstream `codex-cli 0.159.0`, with
  public-control `main=4d7418085265073bf1dada737a364aee54b374bd`.
- Current live Termux consumer is the same `0.159.0` / sequence-23 generation.
- Fresh official upstream preflight on 2026-09-30 resolved exact `0.159.2`
  with archive SHA-256
  `05a524a463cadf7e3e22c7f923539c0d0b74c3e78b1f5f1fab52e50e6fb3312f`.
- Sequence 23 was built from isolated production source
  `b164e5b61cb438913cdb1633d74c62d6c96f6523`. That commit is a child of
  `4fd14ed8...` but changes Core and Manager product files; therefore the live
  sequence-23 runtime is **not** source-equivalent to current accepted source
  authority.
- The signed sequence-23 Release and its authenticated
  `compat/download-size-v1` control resources remain retained and valid.

## Phase A — authority compaction — complete

The historical Workboard/release-plan bodies were compacted into current routing
plus reusable gates, while `GOAL.md` remains the durable evidence ledger.
Validation was green and the exact docs-only result was pushed to
`rewrite/rust-core`.

The initially planned 0.159.0 same-version sequence-24 correction was never
published or applied. Its pretrigger upstream check observed official 0.159.2 and
failed closed before public or live mutation. It is superseded by Phase B below.

## Phase B — ordinary newer-stable runtime alignment

The user explicitly authorized runtime alignment after authority compaction.

- Do not rewrite product code for this phase.
- Build from exact accepted product source
  `4fd14ed8aafcf29602f43e00c314eb8da39a6e5f`.
- Treat public sequence 23 / generation
  `local-hosted-0-159-0-b164e5b61cb4` as the exact authenticated baseline.
- Official upstream must resolve to exact `0.159.2` and exact archive SHA-256
  `05a524a463cadf7e3e22c7f923539c0d0b74c3e78b1f5f1fab52e50e6fb3312f`.
- The ordinary semantic version comparison must report a newer candidate. Do not
  use a same-version override to create this release.
- The candidate must use fresh generation identity and exact next sequence
  **24**. It may not overwrite or repurpose the sequence-23 Release, Pages tree,
  tag, or signed index bytes.
- Core and Manager must be cross-built from the accepted source. Upstream-owned
  runtime/code-mode-host bytes may legitimately change because upstream moved
  from 0.159.0 to 0.159.2; every such byte remains bound to the exact official
  archive and full candidate qualification.
- Preserve production-authority signing, independent verification, immutable
  Release staging, last-known-good Pages continuity, public HTTPS every-byte
  readback, disposable ordinary update/no-op proof, and non-forced exact-parent
  stable-index CAS.
- Mutate the live Termux consumer only after public promotion has independently
  re-read green, and only through ordinary signed public `codex update`.
- Final acceptance requires live `codex --version == codex-cli 0.159.2`, active
  generation equal to the newly promoted sequence-24 generation, healthy
  Core/Manager/runtime, and a second exact-current update with no state delta.
- Because the live sequence-23 Core already implements
  `UPDATE-DOWNLOAD-SIZE-V1`, this real 0.159.0 -> 0.159.2 transition is also
  the first live acceptance opportunity for authenticated
  `downloaded / total` TTY progress.
- After acceptance, record exact sequence/generation/run/source evidence in
  `GOAL.md` and set this Workboard back to “no active bundle.”

## Preserved architecture

- SCS-1 through SCS-6 are accepted and closed. `$HOME/.codex` is the canonical
  user-global conversation state; Manager profiles are execution/auth/config
  identities, not conversation owners.
- Steady-state Manager remains a thin wrapper. It must not reintroduce a second
  thread store, routine SQLite/transcript parsing, conversation ownership, or a
  parallel session index.
- Core owns install/update/rollback/doctor and signed generation activation.
- Historical accepted bundles remain evidence in `GOAL.md`; they are not active
  routing merely because their original text used “current” or “active.”

## Resume rules

1. Read `SPEC.md` -> `GOAL.md` -> `WORKBOARD.md`; read
   `RELEASE_AUTOMATION_PLAN.md` before release or live-runtime work.
2. Rebind remote `rewrite/rust-core`, public `main`, signed stable index,
   official upstream, and live activation state before mutation.
3. Never replay a completed effect. Continue from the freshest verified frontier.
4. A new feature, cleanup, public release, or live-state mutation outside the
   selected bundle requires a new bounded Workboard selection.
5. If no bundle is selected, source/product/public/live mutation is not
   authorized.
