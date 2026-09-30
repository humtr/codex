# Rust Core Workboard

This file owns only the current implementation, repository-maintenance, release,
or live-consumer bundle. Normative product behavior belongs in `SPEC.md`.
Accepted and historical evidence belongs in `GOAL.md`. Release mechanics belong
in `RELEASE_AUTOMATION_PLAN.md`.

## Current routing

- Repository: `humtr/codex`.
- Source authority branch: `rewrite/rust-core`; always re-read the remote head
  before work. Do not use a remembered SHA as current authority.
- Selected maintenance bundle: **AUTHORITY-COMPACTION-V1 / RUNTIME-ALIGNMENT**.
- Product-code mutation in this bundle: **none**. Authority compaction is
  documentation-only. Runtime alignment must use already accepted product source
  and the existing signed publication/activation boundaries.
- Accepted product implementation for the current feature set is source
  `4fd14ed8aafcf29602f43e00c314eb8da39a6e5f`. Subsequent closeout commits on
  `rewrite/rust-core` changed authority documents only.
- Current signed public stable before alignment is sequence **23**, generation
  `local-hosted-0-159-0-b164e5b61cb4`, upstream `codex-cli 0.159.0`, with
  public-control `main=4d7418085265073bf1dada737a364aee54b374bd`.
- Current live Termux consumer is the same `0.159.0` / sequence-23 generation.
- Sequence 23 was built from isolated production source
  `b164e5b61cb438913cdb1633d74c62d6c96f6523`. That commit is a child of
  `4fd14ed8...` but changes Core and Manager product files; therefore the live
  sequence-23 runtime is **not** claimed to be source-equivalent to the current
  accepted source authority.
- The signed sequence-23 Release and its authenticated
  `compat/download-size-v1` control resources remain retained and valid.

## Phase A — authority compaction

1. Keep `SPEC.md` normative and unchanged unless an actual product contract must
   change.
2. Keep `GOAL.md` as the durable acceptance ledger; add a short current-state
   section ahead of dated evidence so historical use of words such as “current”
   cannot override fresh routing.
3. Replace the accumulated historical body of this file with only current
   routing, the selected bundle, and resume rules.
4. Replace the accumulated run-by-run release-plan history with the reusable
   current production contract plus this bundle's exact runtime-alignment gate.
5. Validate that the compaction is documentation-only, commit it, and publish it
   to `rewrite/rust-core` before live/public mutation.

## Phase B — signed runtime alignment

The user explicitly authorized runtime alignment after authority compaction.

- Do not rewrite product code for this phase.
- Build from exact accepted product source
  `4fd14ed8aafcf29602f43e00c314eb8da39a6e5f` (or prove any later selected
  source has an identical product tree before use).
- Treat public sequence 23 / generation
  `local-hosted-0-159-0-b164e5b61cb4` as the exact authenticated baseline.
- Upstream must still resolve to exact `0.159.0`; ordinary newer-version
  comparison must not be repurposed to bypass a same-version gate.
- The correction must use a fresh generation identity and exact next sequence.
  It may not overwrite or repurpose the sequence-23 Release, Pages tree, tag, or
  signed index bytes.
- Before signing, prove unchanged upstream/runtime payload identity where the
  source correction does not own a change, and bind every changed Core/Manager
  payload to the accepted source.
- Preserve production-authority signing, independent verification, immutable
  Release staging, last-known-good Pages continuity, public HTTPS every-byte
  readback, disposable ordinary update/no-op proof, and non-forced exact-parent
  stable-index CAS.
- Mutate the live Termux consumer only after public promotion has independently
  re-read green, and only through ordinary signed public `codex update`.
- Final acceptance requires live `codex --version == codex-cli 0.159.0`, active
  generation equal to the newly promoted generation, healthy Core/Manager/runtime,
  and a second exact-current update with no state delta.
- After acceptance, record the exact sequence/generation/run/source evidence in
  `GOAL.md`, set this Workboard back to “no active bundle,” and leave future
  download-size TTY acceptance to the next genuinely newer upstream release.

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
2. Rebind remote `rewrite/rust-core`, public `main`, signed stable index, and
   live activation state before mutation.
3. Never replay a completed effect. Continue from the freshest verified frontier.
4. A new feature, cleanup, public release, or live-state mutation requires an
   explicitly selected bounded bundle here.
5. If no bundle is selected, source/product/public/live mutation is not
   authorized.
