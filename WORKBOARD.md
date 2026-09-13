# Rust Core Workboard

This file owns only the active implementation or repository-maintenance bundle.
Accepted evidence and historical disposition belong in `GOAL.md`; normative
behavior belongs in `SPEC.md`.

## Current routing

- Repository: `humtr/codex`.
- Active branch: `rewrite/rust-core`.
- TERMUX-COMPAT implementation base: TC-1 accepted source commit
  `6abb70bf33144468d9420d7880f25f5afc0b5684`.
- Selected upstream target for current compatibility work: `rust-v0.154.0`,
  upstream commit `6b9826e3aa83b1a5947db50f4332cb9c65f1b340`. Revalidate again if the
  supported upstream target moves.
- Original Rust Core goal status: **complete**; the accepted closure is not
  reopened by this work.
- Active Goal Lift: **TERMUX-COMPAT — Linux-target upstream compatibility on
  Termux**.
- TC-1 is accepted and recorded in `GOAL.md`.
- Active implementation bundle: **TC-2 — shared Termux URL opener and
  Linux-target/Android capability policy**.
- TC-3 is not active: it covers the system-`rg` dependency and disposable MCP
  credential fallback proof/correction.
- `legacy/monolith` remains sealed at
  `bf30a7dc94d4dad7f58836c69028160856e63c58`.
- Worker mode remains OFF.

## TC-2 — shared Termux URL opener and Linux-target/Android capability policy

### Entry conditions

1. Re-read `SPEC.md` Section 9A and the active TERMUX-COMPAT Goal Lift in
   `GOAL.md` before product-code mutation.
2. Revalidate the exact selected upstream release's browser-open call sites,
   MCP OAuth discovery/registration strategy, callback construction and
   issuer/metadata binding, plus material `target_os = "linux"` versus Android
   compile-time guards that affect clipboard/image-paste behavior.
3. Preserve the existing signed-generation/update authority and raw-runtime
   patch policy. TC-2 does not authorize a new binary byte substitution; amend
   `SPEC.md` first if one becomes necessary.
4. Treat authorization-server/account callback allowlisting as external policy.
   A response such as `invalid_client_metadata: redirect_uri is not allowed by
   the account configuration` is not a Core defect when Codex submitted the
   exact legitimate callback for the applicable advertised protocol/metadata.

### Scope

- Implement one bounded Termux URL-opening policy used by every enumerated
  upstream browser-open intent, including primary login, TUI onboarding/history
  URL actions, and MCP OAuth. The policy must invoke only a qualified absolute
  opener under `$PREFIX/bin`, pass exactly one already-formed HTTP/HTTPS URL
  argument, avoid shell evaluation, and preserve the URL for safe manual use
  when opening is unavailable or fails.
- Do not mutate logical `CODEX_HOME` auth/config/session state merely to select
  or execute the opener. The opener boundary is capability projection, not a
  new configuration authority.
- Make Linux-target-on-Android clipboard/image-paste behavior explicit. Normal
  Termux execution must not enter unqualified Linux desktop/WSL image clipboard
  paths. Either adapt the confirmed surface through a bounded Termux capability
  or make image paste explicitly unavailable while keeping the TUI stable.
  Preserve the existing terminal-mediated text-copy fallback.
- Review the selected release for other material Android-gated behavior that is
  inactive in the shipped `aarch64-unknown-linux-musl` runtime and disposition
  each relevant finding without expanding into unrelated feature work.
- MCP OAuth end-to-end qualification must use an authorization server or
  disposable fixture already configured to allow the exact legitimate callback
  required by the selected release. Prove DCR/CIMD strategy selection as
  applicable, authorization URL production, Termux browser opening, loopback
  callback validation, token exchange, and disposable credential persistence.
- If that exact callback is allowed and the flow still fails because Codex
  constructs a URI inconsistent with advertised metadata or the applicable
  protocol, promote only that concrete mismatch to a focused TC-2 client
  compatibility correction. Do not change the redirect URI merely to evade a
  provider allowlist.
- Never record credentials, authorization codes, tokens, client secrets, or
  account identifiers in acceptance evidence.

### Required TC-2 proof

- All enumerated browser-open surfaces traverse one shared bounded Termux opener
  policy; no shell-string/eval path or desktop-only opener bypass remains.
- Opener absence/failure is deterministic and leaves a usable manual URL without
  mutating live auth/profile/session/provider state.
- Unsafe schemes, malformed URLs, unqualified opener paths, and argument
  injection fail closed at the owning boundary.
- Linux-target Termux image-paste behavior is either qualified through the
  accepted capability or explicitly unavailable without destabilizing the TUI;
  terminal text-copy fallback remains usable.
- Release-sensitive Linux-versus-Android guards relevant to this bundle are
  reviewed and captured by focused tests or an explicit no-change disposition.
- With the provider/fixture already allowing the legitimate callback, MCP OAuth
  proves registration/metadata strategy, authorization URL, Termux opener,
  loopback callback, token exchange, and disposable credential persistence end
  to end. Provider-policy rejection by itself is not repaired in Core.
- Existing TC-1 daemon fence, signed update/rollback authority, Manager profile
  isolation, resolver non-mutation, auth/session boundaries, and sandbox
  behavior remain green.
- Finish with focused TC-2 tests plus repository-wide locked check/test,
  warnings-denied clippy, formatting, and `git diff --check`. Record accepted
  evidence in `GOAL.md` before closing TC-2 and selecting TC-3.

### Explicitly out of scope for TC-2

- Bundling, installing, or otherwise resolving the system `rg` dependency.
- MCP credential-store delete/logout fallback correction; that remains TC-3
  unless TC-2 qualification exposes a direct prerequisite that must first be
  recorded and routed rather than silently expanded.
- Upstream version upgrade beyond revalidation, live cutover, release/index
  publication, source push, remote `main` promotion, live provider/account
  configuration changes, or live auth/network mutation.

## Planned follow-up routing

- **TC-3 — host tools + credential fallback:** remove the silent fresh-system
  `rg` assumption by the bounded SPEC-permitted resolution, then reproduce the
  MCP `Auto` keyring/file logout boundary in disposable roots and correct it
  only if the latent asymmetry is real. Credential contents must never become
  evidence, and no automatic package-manager install is permitted.

Only TC-2 is authorized as the next source slice. TC-3 becomes active only
after TC-2 acceptance updates `GOAL.md` and replaces this routing.

## Resume rule

A fresh implementation session resumes by reading `SPEC.md`, then `GOAL.md`,
then this file and checking the current branch/HEAD and selected upstream target.
A single user direction to proceed is sufficient: implement only the currently
active bundle, run its acceptance gates, record accepted evidence in `GOAL.md`,
replace this routing with the next preplanned bundle, and continue without a
routine user pause while all work remains source/disposable and within these
authority boundaries. TC-2 must honor the MCP OAuth provider-policy boundary
above rather than treating provider allowlist configuration as a Core patch.

Remote `main` promotion, source push, release/index publication, and live
mutation remain separate explicitly authorized operations. Historical accepted
evidence stays in `GOAL.md`; do not repopulate this file with closed bundle
history.
