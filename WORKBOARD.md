# Rust Core Workboard

This file owns only the active implementation or repository-maintenance bundle.
Accepted evidence and historical disposition belong in `GOAL.md`; normative
behavior belongs in `SPEC.md`.

## Current routing

- Repository: `humtr/codex`.
- Active branch: `rewrite/rust-core`.
- Original Rust Core goal status: **complete**.
- TERMUX-COMPAT TC-1, TC-2, and TC-3 are accepted and recorded in `GOAL.md`.
- TC-LIVE-BRIDGE source repair is accepted on top of pushed base
  `3e6899262d1fa4ce3952c89341aa6b89a58364ed`; final commit SHA is assigned only
  after the accepted five-file diff is committed.
- Selected upstream remains `rust-v0.154.0`, upstream commit
  `6b9826e3aa83b1a5947db50f4332cb9c65f1b340`.
- Active implementation bundle: **none**.
- Authorized operational next step: exact fast-forward push of the accepted
  bridge source, then repository-native two-step live cutover: signed R10-readable
  bridge sequence first, canonical signed generation second.
- `legacy/monolith` remains sealed at
  `bf30a7dc94d4dad7f58836c69028160856e63c58`.
- Worker mode remains OFF.

## Accepted TC-LIVE-BRIDGE disposition

The bounded repair retains existing v3/v4 trust and activation authority. Only
exact `creation_metadata = "r10-browser-helper-bridge-v1"` with the two accepted
browser helper identities in exact order may use signed `helpers/0` and
`helpers/1`. Normal generations use only `browser/open/curl` and
`browser/manual/curl`. Marker/layout mismatch fails closed. No release format,
key, bootstrap authority, package install, PATH authority, raw-runtime patch, or
direct launcher replacement was added.

Acceptance evidence is recorded in `GOAL.md`. The decisive disposable proof used
the real installed R10 updater in an isolated HOME/PREFIX and completed R10 ->
bridge -> canonical -> rollback with doctor success at each state while leaving
actual live launcher, resolver, and protected auth/config/profile/session
identities unchanged. The final repository gate passed Core 141/0/1,
release-builder 15/15, Manager 20/20 plus 11/11, locked workspace check/test,
clippy `-D warnings`, formatting, and `git diff --check`.

## Live cutover boundary

Before live mutation, rebuild release artifacts from the final accepted source,
regenerate and sign both candidate releases with the existing update authority,
and rebind the remote base, live launcher, activation state, resolver, protected
state fingerprint, selected upstream/runtime digest, and candidate manifests.

Step 1 uses the existing R10 `$PREFIX/bin/codex update --local` on the signed
bridge release. Require exit 0, 0.154.0, healthy doctor, current=bridge,
previous=R10, exact indexed helpers, and unchanged protected identities before
continuing. Step 2 uses the newly installed Core on the canonical signed release.
Require exit 0, healthy 0.154.0, current=canonical, previous=bridge, exact nested
browser helpers, and unchanged protected identities. Any failure stops with the
last verified generation active. Do not perform a live rollback merely to prove
it; disposable rollback proof is already accepted.

## Resume rule

A fresh session resumes by reading `SPEC.md`, then `GOAL.md`, then this file;
verifying exact branch/HEAD, remote tip, selected upstream, and live baseline;
and continuing only the authorized operational cutover until its evidence is
recorded. Historical accepted evidence stays in `GOAL.md`.
