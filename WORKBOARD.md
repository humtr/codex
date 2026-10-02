# Rust Core Workboard

This file owns only the current implementation, repository-maintenance, release,
or live-consumer bundle. Normative product behavior belongs in `SPEC.md`.
Accepted and historical evidence belongs in `GOAL.md`. Release mechanics belong
in `RELEASE_AUTOMATION_PLAN.md`.

## Current routing

- Repository: `humtr/codex`.
- Source authority branch: `rewrite/rust-core`; always re-read the remote head
  before work.
- Selected maintenance bundle:
  **STARTUP-ADVISORY-SINGLE-KEY-TRANSIENT-V1 / SEQUENCE-27-CORRECTIVE**.
- Accepted hotfix product source is exact
  `75aed30c7ad6c427d30115b0667957130da198f5`. Later authority-only commits
  must not replace this product-source pin.
- Current public-control branch is exact
  `main=bf833822c41cb5ee817922b2ed08e9316f288a40`.
- Current signed public stable is sequence **26**, generation
  `local-hosted-0-160-0-4fd14ed8aafc`, upstream `codex-cli 0.160.0`.
- Current live Termux consumer is sequence **24**, generation
  `local-hosted-0-159-2-4fd14ed8aafc`, upstream `codex-cli 0.159.2`.
- Fresh official upstream preflight on 2026-10-02 resolves exact `0.160.0`
  with archive SHA-256
  `7f0fe42ff22ecfa3a47bc4a34f5b22c4218b431a4ec0aba51c7d98299f07900c`.
- The next public release sequence is therefore exactly **27**.

## Active bounded hotfix — startup advisory single-key/transient behavior

The user reported that the five-second bare-launch advisory was rendered as
persistent countdown lines and that typing `y` without Enter remained buffered,
producing output such as `1sy` and contaminating the subsequent upstream Codex
input stream. This bundle must preserve the existing five-second default-keep
semantics while making the advisory a true transient TTY interaction:

- no background `read_line()` thread may outlive the prompt;
- `y`/`Y` selects update immediately, without Enter;
- `n`/`N`, Enter, or another single input byte keeps the current runtime
  immediately;
- prompt input is not echoed into the terminal or forwarded to upstream Codex;
- the original terminal mode is restored before update execution or upstream
  launch;
- the countdown occupies one in-place transient line and is cleared on every
  exit path;
- non-TTY launches remain unchanged and bypass startup discovery/prompting.

The product source was validated with focused PTY coverage and the full repository
validation suite, then published as `75aed30c7ad6c427d30115b0667957130da198f5`.
Release/live activation is the remaining gate and must preserve the existing
signed-generation invariants.

## Release/live gate — sequence-27 corrective

This is a bounded same-upstream-version corrective release for the accepted
startup-advisory hotfix. It does not authorize unrelated product or Manager
changes.

- Re-read public `main`, the signed stable index/manifest, official upstream, and
  live activation immediately before mutation.
- Require public stable sequence **26**, generation
  `local-hosted-0-160-0-4fd14ed8aafc`, upstream `0.160.0`.
- Require exact hotfix product source
  `75aed30c7ad6c427d30115b0667957130da198f5`.
- Require official latest to remain exact `0.160.0` with archive SHA-256
  `7f0fe42ff22ecfa3a47bc4a34f5b22c4218b431a4ec0aba51c7d98299f07900c`.
- The ordinary semantic version comparison is expected to report no newer
  candidate; only this explicitly bounded same-version corrective gate may
  produce sequence **27**.
- Use a fresh generation identity. Never overwrite sequence 26 Release, Pages,
  or signed index bytes.
- Preserve unsigned qualification, production-only signing, independent
  verification, immutable Release staging, last-known-good Pages continuity,
  every-byte HTTPS readback, disposable update/no-op proof, and non-forced
  exact-parent stable-index CAS.
- Only after independent public confirmation may the live sequence-24 consumer
  take the ordinary signed public update directly to sequence 27. Verify
  `codex --version == codex-cli 0.160.0`, the sequence-27 generation is active,
  Core/Manager/runtime are healthy, and a second exact-current update is a
  no-op.
- Final live acceptance must also exercise a future-style startup advisory in a
  PTY fixture or equivalent installed-Core probe to prove single-key/no-Enter,
  no-echo, transient clear, input isolation, and terminal-mode restoration.

After acceptance, record exact source/public/live evidence in `GOAL.md` and
return this Workboard to no active bundle.

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
