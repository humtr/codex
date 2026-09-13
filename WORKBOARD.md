# Rust Core Workboard

This file owns only the active implementation or repository-maintenance bundle.
Accepted evidence and historical disposition belong in `GOAL.md`; normative
behavior belongs in `SPEC.md`.

## Current routing

- Repository: `humtr/codex`.
- Active branch: `rewrite/rust-core`.
- TERMUX-COMPAT implementation base: TC-2 accepted source commit
  `ad5ccd654a783c4d5f67a5253eea5705a4c86577`.
- Selected upstream target for current compatibility work: `rust-v0.154.0`,
  upstream commit `6b9826e3aa83b1a5947db50f4332cb9c65f1b340`. Revalidate again if the
  supported upstream target moves.
- Original Rust Core goal status: **complete**; the accepted closure is not
  reopened by this work.
- Active Goal Lift: **TERMUX-COMPAT — Linux-target upstream compatibility on
  Termux**.
- TC-1 and TC-2 are accepted and recorded in `GOAL.md`.
- Active implementation bundle: **TC-3 — host tools + disposable MCP credential
  fallback**.
- `legacy/monolith` remains sealed at
  `bf30a7dc94d4dad7f58836c69028160856e63c58`.
- Worker mode remains OFF.

## TC-3 — host tools + disposable MCP credential fallback

### Entry conditions

1. Re-read `SPEC.md` Section 9A and the active TERMUX-COMPAT Goal Lift in
   `GOAL.md` before product-code mutation.
2. Revalidate the exact selected upstream release's ordinary-correctness uses of
   external `rg`, including the behavior when no `rg` is available, and the MCP
   OAuth credential-store `Auto` load/save/delete/logout paths relevant to
   keyring-to-file fallback.
3. Preserve TC-1 daemon fencing, TC-2 browser/clipboard policy, the signed
   generation/update authority, Manager profile isolation, resolver
   non-mutation, and the existing raw-runtime patch allowlist. Amend `SPEC.md`
   first if a new authority or byte patch would otherwise be required.
4. Use disposable roots and non-sensitive fixture credentials for credential
   reproduction. Never read, copy, mutate, or record live credential contents.

### Scope

- Remove the silent fresh-system dependency on an undeclared system `rg` for
  ordinary correctness. The accepted resolution is bounded by the Goal Lift:
  use a qualified signed helper/fallback, or provide explicit bounded
  degradation/diagnosis where the operation cannot be supported. Do not invoke
  a package manager automatically and do not silently broaden PATH authority.
- Enumerate the selected release's material `rg` call sites before choosing the
  resolution. Keep unrelated upstream tools and optional convenience behavior
  out of this bundle unless they are direct prerequisites for the same
  correctness boundary.
- Reproduce the MCP OAuth `Auto` credential-store authority entirely in
  disposable roots: keyring unavailable/failing, file fallback save/load, then
  delete/logout. If file-backed credentials can be stranded because delete
  returns on keyring failure, correct only that demonstrated asymmetry so
  logout removes the resolved fallback authority.
- Credential values, authorization codes, tokens, client secrets, and account
  identifiers must never enter test output, diffs, acceptance evidence, or
  logs. Tests should prove presence/removal through bounded shape or sentinel
  assertions rather than exposing secret material.

### Required TC-3 proof

- In a fresh/disposable environment with no usable system `rg`, every in-scope
  operation has deterministic behavior: qualified signed fallback/helper use or
  explicit bounded degradation/diagnosis. No automatic package-manager install,
  unqualified PATH search widening, or hidden ordinary-correctness dependency
  remains.
- If a signed helper/fallback is introduced, its identity, digest, staging path,
  and qualification remain inside the existing signed generation/update
  authority and fail closed on substitution or missing qualification.
- Disposable MCP OAuth proof demonstrates the actual `Auto` authority path used
  when keyring storage is unavailable. Save/load succeeds through the accepted
  fallback, logout/delete removes that same fallback, and a second load proves
  the credential is gone. If the suspected asymmetry is not reproducible on the
  selected release, record the no-change disposition instead of patching it.
- No live `CODEX_HOME`, OS/account credential store, provider/account setting,
  resolver, installed runtime/helper, or process environment is modified.
- TC-1 and TC-2 focused regressions remain green. Finish with focused TC-3 tests
  plus repository-wide locked check/test, warnings-denied clippy, formatting,
  and `git diff --check`; record accepted evidence in `GOAL.md` before closing
  the bundle.

### Explicitly out of scope for TC-3

- Automatic Termux package-manager installation or mutation of live `$PREFIX`.
- Reworking TC-1 daemon authority or TC-2 opener/clipboard behavior absent a
  demonstrated regression that must first be routed through authority.
- Live OAuth/provider/account qualification, real credential migration, or
  mutation of the user's keyring/file credential state.
- Upstream version upgrade beyond revalidation, live cutover, release/index
  publication, source push, remote `main` promotion, or unrelated feature work.

## Planned follow-up routing

TC-3 is the final preplanned bundle in the current TERMUX-COMPAT lift. After its
acceptance gate, update `GOAL.md` with the accepted evidence and determine from
the Goal Lift thresholds whether the lift closes or whether a new bounded bundle
must be planned. No additional implementation bundle is implicitly authorized by
this file.

## Resume rule

A fresh implementation session resumes by reading `SPEC.md`, then `GOAL.md`,
then this file and checking the current branch/HEAD and selected upstream target.
A single user direction to proceed is sufficient to implement only TC-3 while
all work remains source/disposable and within these authority boundaries. Do not
expand this routing into live package installation, live credential mutation,
provider changes, release publication, source push, or runtime cutover.

Remote `main` promotion, source push, release/index publication, and live
mutation remain separate explicitly authorized operations. Historical accepted
evidence stays in `GOAL.md`; do not repopulate this file with closed bundle
history.
