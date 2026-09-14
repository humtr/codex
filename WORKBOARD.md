# Rust Core Workboard

This file owns only the active implementation or repository-maintenance bundle.
Accepted evidence and historical disposition belong in `GOAL.md`; normative
behavior belongs in `SPEC.md`.

## Current routing

- Repository: `humtr/codex`.
- Active branch: `rewrite/rust-core`.
- Original Rust Core goal status: **complete**.
- TERMUX-COMPAT TC-1, TC-2, and TC-3 are accepted and recorded in `GOAL.md`.
- TC-LIVE-BRIDGE source repair commit is
  `1b51902775c89c79522ef868921e4b95229a35fb` and has been pushed to
  `origin/rewrite/rust-core`.
- Selected upstream remains `rust-v0.154.0`, upstream commit
  `6b9826e3aa83b1a5947db50f4332cb9c65f1b340`.
- TC-LIVE-BRIDGE repository-native two-step live cutover is accepted and
  complete.
- Final live runtime is `codex-cli 0.154.0` with current generation
  `local-20260914-tc3-canonical-1` and retained previous generation
  `local-20260914-r10-browser-bridge-1`.
- Final live launcher SHA-256 is
  `0055ec0ecc762e4a4c878be62fd26f93118785218f004b26fe12b178cd3380eb`.
- Active implementation bundle: **UPDATE-CHANNEL-LATEST** — stable channel
  publication plus exact-current update no-op repair.
- Authorized operational next step: after source gates, publish the already
  accepted signed sequence-8 bridge, prove retained R10 bare update, then
  advance the signed stable index to the accepted sequence-9 canonical release.
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

Source acceptance and the decisive disposable migration/rollback proof are
recorded in `GOAL.md`. The final repository gate passed Core 141/0/1,
release-builder 15/15, Manager 20/20 plus 11/11, locked workspace check/test,
clippy `-D warnings`, formatting, and `git diff --check`.

Live preflight rebound the exact R10 baseline and final signed candidates. The
installed R10 updater then admitted signed bridge sequence 8; only after bridge
health and protected-state checks passed did the new Core admit canonical
sequence 9. Final doctor is healthy, resolver and protected auth/config/profile/
session identities are unchanged, canonical nested browser helpers are active,
and the bridge is retained as the one rollback generation. No live rollback was
performed solely for evidence.

## Active UPDATE-CHANNEL-LATEST bundle

Scope is deliberately bounded to the two defects exposed by the post-cutover
update smoke. Core may treat an authenticated candidate as already current only
when its release sequence equals the installed current sequence and both its
generation identity and signed release manifest exactly match current. Lower
sequences and equal-but-different releases remain fail-closed. No-op success must
not stage, probe, rewrite the launcher, or mutate activation state.

The stable channel repair reuses only the accepted signed TC-LIVE-BRIDGE
artifacts: publish immutable sequence 8 bridge assets and point the signed index
to them, prove a retained R10 0.153.4 bare `codex update`, then publish immutable
sequence 9 canonical assets and advance the signed index to canonical. The final
public stable target must be `local-20260914-tc3-canonical-1`. No live runtime
replacement, rollback, package-manager action, PATH/resolver change, signing-key
rotation, or credential/provider mutation belongs to this bundle.

Done means focused anti-rollback/no-op regressions and the full repository gate
are green, source is accepted on `rewrite/rust-core`, both staged public-channel
transitions are independently smoke-proved in disposable HOME/PREFIX roots, the
final stable index targets canonical sequence 9, and the real live 0.154.0 state
remains healthy and byte-identical at protected boundaries.

## Resume rule

A fresh session resumes by reading `SPEC.md`, then `GOAL.md`, then this file and
verifying branch/HEAD plus the final live state before selecting any new work.
No further TERMUX-COMPAT implementation or live-migration bundle is implicitly
selected. Historical accepted evidence stays in `GOAL.md`.
