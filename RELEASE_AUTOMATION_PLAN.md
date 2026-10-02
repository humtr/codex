# Release Automation and Local-Derived Update Plan

Status: **inactive; no release/live bundle is currently selected**.
STARTUP-ADVISORY-SINGLE-KEY-TRANSIENT-V1 / sequence 27 is accepted evidence in
`GOAL.md`; historical production runs and one-shot gates are not current routing.

Authority remains `SPEC.md` -> `GOAL.md` -> `WORKBOARD.md`. This file is the
current release drift-control plan. It never overrides those authorities.

## Current production baseline

- Last accepted product source:
  `75aed30c7ad6c427d30115b0667957130da198f5`.
- Signed public stable sequence: **27**.
- Signed public stable generation:
  `local-hosted-0-160-0-75aed30c7ad6-startup-advisory`.
- Signed public stable upstream version: `codex-cli 0.160.0`.
- Live Termux consumer sequence: **27**.
- Live generation:
  `local-hosted-0-160-0-75aed30c7ad6-startup-advisory`.
- Live upstream version: `codex-cli 0.160.0`.
- Stable promotion commit:
  `2dc79bd11842c9e5970b8c73c470886b780f2ff8`.
- Official upstream archive used by this accepted generation has SHA-256
  `7f0fe42ff22ecfa3a47bc4a34f5b22c4218b431a4ec0aba51c7d98299f07900c`.

Any future release must rebind source, public stable, official upstream, and live
state afresh. Sequence-27 one-shot admission is historical and must not be
reused.

## Frozen release invariants

Every public generation must preserve all of the following:

1. **Exact source and upstream identity.** Candidate source is pinned to an exact
   commit; official upstream version/archive identity and digest are immutable
   inputs.
2. **Unsigned qualification before signing.** Build/adaptation and the native
   Android/AArch64 executable smoke occur before official signing.
3. **Production signing authority only.** Public release signatures come only
   from the accepted GitHub Actions signing authority. Device-local ephemeral
   signing remains local-derived only and never acquires public authority.
4. **Independent verification.** Signed manifest/index/control bytes are
   reverified independently before publication or activation.
5. **Immutable release location.** A generation tag/Release and its assets are
   immutable. Failed or superseded attempts get a fresh generation identity;
   existing public bytes are never repurposed.
6. **Last-known-good Pages continuity.** Candidate Pages deployment must retain
   the currently authoritative stable generation while staging the candidate.
7. **Every-byte public readback.** Public HTTPS bytes, signatures, manifest,
   modes, inventory, and authenticated controls must match the signed candidate
   before promotion.
8. **Disposable consumer proof.** Ordinary update into the candidate and an
   exact-current second invocation must pass against public bytes before stable
   promotion.
9. **Non-forced exact-parent CAS.** Stable promotion changes the signed index pair
   only through a non-forced compare-and-swap from the exact expected
   public-control parent.
10. **Live consumer is a separate gate.** A public promotion does not itself
    mutate the user's Termux runtime. Live activation follows only after
    independent public confirmation and uses ordinary signed public
    `codex update`.
11. **No secret or trust-key shortcuts.** Never copy private signing material into
    source, logs, artifacts, or device state; never replace the accepted public
    key to make a candidate pass.
12. **Failure is non-promoting.** Any build, smoke, signing, Release, Pages,
    readback, disposable-update, or CAS failure leaves the previously signed
    stable authoritative.

## Download-size control contract

For generations that implement `UPDATE-DOWNLOAD-SIZE-V1`:

- publication emits signed `download-size-v1` plus
  `download-size-v1.sig`;
- the body binds the exact release-manifest digest, exact ordered file inventory,
  per-file byte counts, file count, and total bytes;
- public placement is `<generation>/compat/`;
- Core may use it only after authenticating the control bytes and must fail
  closed on authenticated size mismatch;
- actual progress is based on GET-written bytes, not HEAD;
- the sidecar never enters installed generation inventory.

## Last accepted startup-advisory sequence-27 corrective

The bounded same-version corrective completed on 2026-10-02. Production run
`36965656433` passed the frozen release invariants and promoted signed sequence
27 generation `local-hosted-0-160-0-75aed30c7ad6-startup-advisory`. Live
sequence 24 then consumed the result through ordinary signed public
`codex update`, reached exact `codex-cli 0.160.0`, and passed durable
exact-current no-op verification.

The selector is closed. It does not authorize any future same-version release.

## Ordinary future upstream intake

This sequence-27 selector is one-shot and must not be reused. After acceptance,
future upstream releases return to the ordinary scheduled/manual producer under
freshly rebound source/public authority. Historical one-shot selectors never
become current authorization merely because their code remains in history.
