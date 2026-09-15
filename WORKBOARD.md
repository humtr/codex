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
- UPDATE-CHANNEL-LATEST is accepted and closed. Final source acceptance before
  the closure ledger is `057f078a091441c1f624c7b229649bd1463ef774`.
- Public stable is signed sequence 10 generation
  `local-20260914-update-channel-bridge-1` at the verified GitHub Pages release
  base; retained R10 no-argument update and exact-current no-op are accepted.
- AUTO-UPSTREAM-ROLLBACK is accepted and source-closed at
  `21bb1cd78d4a6e6ef8e124b7f07230206c5aa5ea`.
- Active implementation bundle: **RELEASE-AUTOMATION-LOCAL-DERIVED (RALD)**.
- RALD-1 local-derived Core contract is accepted on 2026-09-15; the next
  incomplete phase is **RALD-2 — detach official production from Core
  `codex update`**.
- The drift-control execution contract is `RELEASE_AUTOMATION_PLAN.md`.
- Remaining authorized order: RALD-2 detach official release production from
  `codex update`; RALD-3 GitHub-hosted scheduled producer; RALD-4 Actions-secret
  signing; RALD-5 Release/Pages LKG-preserving publication and runtime-proven
  promotion; RALD-6 fresh-install/update delivery E2E; RALD-7 full acceptance
  and scheduler activation.
- Public stable remains fixed during source development and may advance only after
  the selected plan's focused/full gates, public readback, disposable public-update
  runtime smoke, and exact non-forced promotion checks pass.
- `legacy/monolith` remains sealed at
  `bf30a7dc94d4dad7f58836c69028160856e63c58`.
- Worker mode remains OFF.

## Accepted RALD-1 disposition

Core now exposes exact `codex update --build-local`, and only automatic signed-
channel transport unavailability may enter that same local-derived construction.
The build uses exact official upstream metadata/archive qualification and normal
Termux adaptation/probes, creates a fresh private 0600 Ed25519 key in disposable
staging, signs the immutable local candidate, deletes private key material before
successful activation, and persists only the public verifier in the current
verifier slot. `activation-state-v3.update_key` remains the official authority;
the local release carries the authenticated public baseline sequence rather than
allocating a public sequence.

Special local-derived admission is internal to that construction path. Ordinary
`--local`/`--remote` signed admission cannot use a self-signed local-derived
marker. Local-derived activation is blocked while an authenticated rollback hold
is active. Rollback from local-derived to retained official state creates no
public hold or rollback-Core guard, while rollback from official state and exact
held `--force` behavior remain unchanged. A later authenticated greater public
sequence supersedes local-derived current through the normal official authority.
No local-derived path invokes `gh` or official publication.

Acceptance gate is green: RALD-1 process E2E 4/4; existing ARH/signed-channel
regressions 4/4; locked workspace check; workspace clippy `-D warnings`; format
and `git diff --check`; full locked workspace tests with Core 153 passed / one
explicit live-Termux smoke ignored, Manager 20/20 plus 11/11 integration, and
release-builder 15/15. Tests ran only in the isolated worktree/disposable roots.
The live installation and public stable remain unchanged. RALD-2 is next; the
remaining official producer in Core is intentionally not re-designed in RALD-1.

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

## Accepted UPDATE-CHANNEL-LATEST disposition

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

Acceptance is complete. Focused anti-rollback/no-op regressions and the full
repository gate are green; the final gate is release-builder 15/15, Core
142/0/1, Manager 20/20 plus 11/11, locked workspace check/test, clippy
`-D warnings`, formatting, and `git diff --check`. Sequence 10 was built and
signed from the accepted 0.154.0 source, then staged as GitHub Release ID
388405334. Pages workflow run 34848065440 succeeded, and complete HTTPS readback
byte-matched the 292,960,783-byte signed tree. The final signed stable index at
`main` head `40692fd63b4b5408f4600b338e854225d8b653c5` targets sequence 10. A disposable retained R10
performed no-argument `codex update` directly from 0.153.4/sequence 7 to
0.154.0/sequence 10; a second no-argument update returned exit 0 with the exact
already-current message and no launcher/state/generation-tree mutation. The real
live sequence-9 canonical runtime remained healthy and unchanged at the measured
launcher, activation-state, resolver, and protected metadata boundaries. No live
runtime replacement, rollback, package-manager action, PATH/resolver change,
signing-key rotation, or credential/provider mutation occurred in this bundle.

## Accepted AUTO-UPSTREAM-ROLLBACK bundle

ARH-1: `codex update --rollback` remains the sole Core-owned public rollback
selector; no top-level `codex rollback` command is added. Rollback success writes
a separate exact-format hold for the rolled-back-from
generation/sequence without changing activation-state v3. After trust and
anti-rollback validation, plain update refuses candidates that do not exceed the
held sequence. A committed greater sequence removes the obsolete hold/guard;
`codex update --force` requires that valid hold, retries exactly its held sequence
for one invocation, retains the normal hold after success, and bypasses no other
admission/probe/activation check.
The disposable first-migration smoke proved that restoring a legacy pre-ARH Core
would bypass that hold on the next process. ARH-1 therefore also owns the bounded
rollback-Core guard repair: if and only if the rollback target lacks exact
`codex-update-hold-v1` capability, keep the held generation's signed Core launcher
as the control plane while rolling back the previous signed payload/pointer. Bind
that exception to exact target/held identities, held sequence, verifier authority,
and held signed Core digest; use the guard as crash-window hold intent, and remove
it after force/greater activation or a normal hold-aware Core rollback.

ARH-2: with no self-hosted Actions runner and no private-key export, `codex update`
is the maintainer-side automation controller only when the default signed channel,
secure matching local signing key, and authenticated local GitHub CLI are all
present. Ordinary clients only consume stable. Exact-current or hold-only public
outcomes may then trigger official OpenAI stable discovery and local adaptation/
signing only for a genuinely newer version. GitHub Release stages already signed
bytes; Actions reconstructs/deploys current stable plus candidate in one Pages
site. Full candidate readback and disposable public update must pass before one
non-forced Git commit atomically replaces the signed index and signature. Any
pre-commit failure leaves stable untouched; an indeterminate final ref result is
not guessed. A known committed promotion survives a later local activation failure,
which is reported as deferred and recovered by the next `codex update`.

Acceptance evidence is complete: focused ARH routing/hold/force/publisher tests
and the disposable legacy-guard process flow pass; release-builder is 15/15; Core
is 156 passed, 0 failed, 1 explicitly ignored real-Termux smoke; Manager
integration is 11/11; locked workspace check/test, clippy `-D warnings`, formatting,
and `git diff --check` all pass. Controlled fixtures prove the newer-version and
publication paths without artificially advancing public stable, and the acceptance
run did not mutate the installed live Codex or protected user/provider state. The
source closure for this accepted ledger state is the corresponding clean
fast-forward commit on `rewrite/rust-core`.

## Selected RELEASE-AUTOMATION-LOCAL-DERIVED plan

This follow-up separates official release production from device-side release
consumption. `codex update`, `codex update --rollback`, rollback hold/guard, and
bounded `codex update --force` retain their accepted consumer/recovery semantics.
A new explicit `codex update --build-local` is always local-derived regardless of
whether an official private key happens to exist on the device; it preserves the
official `update_key` and can never publish official stable.

Official release production moves to a GitHub-hosted scheduled producer. The
selected signing input is repository Actions secret `CODEX_RELEASE_SIGNING_KEY`,
which must derive the exact accepted public update authority before signing. The
secret value must never enter repository content, logs, artifacts, or device
state. GitHub-hosted publication must preserve the currently authoritative
successful public stable generation while a candidate is staged, and upload or
Pages success alone is never enough for promotion: full public readback plus a
disposable real `codex update`/launch/doctor/no-op smoke must pass before the
signed stable index can advance atomically.

`RELEASE_AUTOMATION_PLAN.md` freezes the comparison evidence, trust split,
last-known-good rule, implementation phases, failure matrix, acceptance gates,
and resume protocol. It is the selected plan for this bundle but does not override
`SPEC.md`; RALD-1 must update the normative contract before behavior changes.
This documentation-selection step does not mutate `main`, the public stable
index, GitHub Releases/Pages, Actions secrets, or the installed live runtime.

## Resume rule

A fresh session resumes by reading `SPEC.md`, then `GOAL.md`, then this file and
verifying branch/HEAD plus the final live state before selecting any new work.
No further TERMUX-COMPAT implementation or live-migration bundle is implicitly
selected. Historical accepted evidence stays in `GOAL.md`.
