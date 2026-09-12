# Rust Core Workboard

This file owns only the active implementation or repository-maintenance bundle.
Accepted evidence and historical disposition belong in `GOAL.md`; normative
behavior belongs in `SPEC.md`.

## Current routing

- Repository: `humtr/codex`.
- Active branch: `rewrite/rust-core`.
- Preparation base: `275b8725c4ee0924ef00a10c3e2f3995421ec8f2`.
- Original Rust Core goal status: **complete**; the accepted closure is not
  reopened by this work.
- Active Goal Lift: **TERMUX-COMPAT — Linux-target upstream compatibility on
  Termux**.
- Active implementation bundle: **TC-1 — app-server authority fence and short
  socket boundary**.
- Later bundles are not active: TC-2 covers shared URL opening plus
  Linux-target/Android clipboard behavior; TC-3 covers the system-`rg`
  dependency and disposable MCP credential fallback proof/correction.
- `legacy/monolith` remains sealed at
  `bf30a7dc94d4dad7f58836c69028160856e63c58`.
- Worker mode remains OFF.

## TC-1 — app-server authority fence and short socket boundary

### Entry conditions

1. Re-read `SPEC.md` Section 9A and the active Goal Lift in `GOAL.md` before
   product-code mutation.
2. Revalidate the exact upstream source behavior for the version selected by
   the implementation session. The audit evidence is bound to upstream
   `0.153.4`; if a different target has materially changed managed-install,
   updater-loop, daemon, remote-control, or socket derivation behavior, update
   the authority documents before adapting code.
3. Preserve patch policy `termux-fd-remap-v1`. TC-1 does not authorize an
   additional raw-runtime byte substitution. If a correct implementation
   requires one, stop the bundle and amend `SPEC.md` first.

### Scope

- Fence every daemon-backed app-server entry reachable through the Termux Core,
  including `remote-control start`, before upstream can select
  `$CODEX_HOME/packages/standalone/current/codex`, create standalone install
  state, fetch `https://chatgpt.com/codex/install.sh`, or enter its updater
  loop.
- Prefer binding supported daemon execution to the currently qualified signed
  generation using Core-owned argument/environment/runtime projection. If that
  cannot be proven without violating the existing signed-generation or patch
  contracts, fail the daemon-backed form closed with a stable Termux-specific
  unsupported result. Do not introduce a second updater or a shadow
  installation merely to preserve the command.
- For any daemon-backed path that remains supported, derive a private
  profile-distinct Unix socket namespace whose encoded final path is at most
  107 bytes. The namespace must not relocate or merge logical `CODEX_HOME`
  auth/config/session state, trust a symlink as authority, or collide across
  accepted Manager profile IDs.
- Preserve the existing foreground `remote-control` path and its private
  temporary socket unless a focused regression proves an independent defect.
- Add focused tests at the owning Core/Manager boundaries. Use disposable roots
  and synthetic upstream/app-server fixtures; no live app-server daemon,
  credential store, profile, session, resolver, runtime, or network mutation is
  part of source acceptance.

### Required TC-1 proof

- Daemon-backed admission cannot execute or create an unmanaged standalone
  Codex tree and cannot issue the upstream installer request.
- The safe outcome is deterministic: either current signed-generation binding
  with the updater path unreachable, or fail-closed unsupported before side
  effects.
- Supported app-server socket paths satisfy the 107-byte maximum for default,
  shortest custom, longest accepted custom, and adversarial profile IDs.
- Distinct profiles map to distinct private socket identities; malformed,
  symlinked, colliding, or pre-existing untrusted runtime entries fail closed.
- Foreground `remote-control` behavior remains outside the daemon fence and its
  prior short temporary-socket behavior is preserved.
- Existing Core signed update/rollback, Manager profile isolation, resolver
  non-mutation, auth/session boundaries, and sandbox behavior remain green.
- Finish with focused tests plus repository-wide locked check/test, warnings-
  denied clippy, formatting, and `git diff --check`. Record accepted evidence
  in `GOAL.md` before closing TC-1 and selecting TC-2.

### Explicitly out of scope for TC-1

- URL/browser opener implementation.
- Clipboard/image-paste implementation or TUI copy-visibility changes.
- Bundling, installing, or otherwise resolving `rg`.
- MCP OAuth credential-store code changes unless a TC-1 test accidentally
  exposes a direct dependency; such a finding opens TC-3 rather than expanding
  TC-1.
- Upstream version upgrade, live cutover, release/index publication, source
  push, remote `main` promotion, or repository environment cleanup.

## Planned follow-up routing

- **TC-2 — shared URL + Android capability policy:** one Termux URL opener
  policy across login/TUI/MCP surfaces, plus explicit Linux-target-on-Android
  clipboard/image-paste behavior and release-time Android-guard review.
- **TC-3 — host tools + credential fallback:** remove the silent fresh-system
  `rg` assumption by the bounded SPEC-permitted resolution, then reproduce the
  MCP `Auto` keyring/file logout boundary in disposable roots and correct it
  only if the latent asymmetry is real.

Only TC-1 is authorized as the next source slice. Later bundles become active
only after TC-1 acceptance updates `GOAL.md` and replaces this routing.

## Resume rule

A fresh implementation session resumes by reading `SPEC.md`, then `GOAL.md`,
then this file and checking the current branch/HEAD. It revalidates the
selected upstream target before code changes and implements only TC-1. If
repository truth has moved past this preparation commit, reconcile the
contracts before mutation rather than assuming this workboard is still current.

Remote `main` promotion, source push, release/index publication, and live
mutation remain separate explicitly authorized operations. Historical accepted
evidence stays in `GOAL.md`; do not repopulate this file with closed bundle
history.
