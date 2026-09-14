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
- Authorized operational next step: after source gates, build and sign the new
  sequence-10 R10-readable bridge-layout generation, publish it through the
  verified Pages transport, prove retained R10 bare update plus exact-current
  no-op, then advance the signed stable index to that sequence-10 generation.
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

The stable channel repair publishes one new signed sequence-10 generation,
`local-20260914-update-channel-bridge-1`, with current repaired Core and exact
`creation_metadata = "r10-browser-helper-bridge-v1"`. Its signed helper inventory
remains `helpers/0` and `helpers/1` so the retained R10 sequence-7 parser can
consume the final public target directly. The selected upstream/runtime and TC1/
TC2/TC3 behavior remain 0.154.0-equivalent; this is a transport compatibility
layout, not a second updater or a rollback of canonical local policy.

GitHub Release assets remain staging for the large top-level files. A fixed
GitHub Pages workflow reconstructs the exact signed tree under
`https://humtr.github.io/codex/<generation_id>/` after binding the pinned public
key and verifying signature, bridge metadata, exact inventory, helper identities,
and every signed digest. The automated Core Release publisher remains fail-closed
for nested signed inventory. The workflow may be mirrored to `main`, but release
bytes never use the Contents API. Failed deployment/readback cannot advance the
signed index. Raw Git-tag publication, slash-bearing Release assets, and
companion-release routing are rejected qualification paths, not fallbacks.

Done means focused anti-rollback/no-op regressions and the full repository gate
are green, source is accepted on `rewrite/rust-core`, sequence 10 is built and
signed from the accepted 0.154.0 source, Pages HTTPS readback byte-matches its
complete signed tree, a disposable retained R10 performs bare `codex update`
directly from sequence 7 to sequence 10, a second bare update on sequence 10
returns exit 0 with the already-current message and no state mutation, the final
stable index targets sequence 10, and the real live 0.154.0 state remains healthy
and byte-identical at protected boundaries. No live runtime replacement,
rollback, package-manager action, PATH/resolver change, signing-key rotation, or
credential/provider mutation belongs to this bundle.

## Resume rule

A fresh session resumes by reading `SPEC.md`, then `GOAL.md`, then this file and
verifying branch/HEAD plus the final live state before selecting any new work.
No further TERMUX-COMPAT implementation or live-migration bundle is implicitly
selected. Historical accepted evidence stays in `GOAL.md`.
