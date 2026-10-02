# Release Automation and Local-Derived Update Plan

Status: **active only for STARTUP-ADVISORY-SINGLE-KEY-TRANSIENT-V1 / SEQUENCE-27-CORRECTIVE**.
Historical production runs and one-shot gates are accepted evidence in
`GOAL.md`; they are not current routing in this file.

Authority remains `SPEC.md` -> `GOAL.md` -> `WORKBOARD.md`. This file is the
current release drift-control plan. It never overrides those authorities.

## Current production baseline

- Accepted hotfix product source:
  `75aed30c7ad6c427d30115b0667957130da198f5`.
- Public-control branch before the corrective trigger:
  `main=bf833822c41cb5ee817922b2ed08e9316f288a40`.
- Signed public stable sequence: **26**.
- Signed public stable generation:
  `local-hosted-0-160-0-4fd14ed8aafc`.
- Signed public stable upstream version: `codex-cli 0.160.0`.
- Live Termux consumer sequence: **24**.
- Live generation:
  `local-hosted-0-159-2-4fd14ed8aafc`.
- Live upstream version: `codex-cli 0.159.2`.
- Fresh official upstream preflight on 2026-10-02 resolves exact `0.160.0`
  with archive SHA-256
  `7f0fe42ff22ecfa3a47bc4a34f5b22c4218b431a4ec0aba51c7d98299f07900c`.
- Next public release sequence: **27**.

The public sequence-26 generation remains authoritative until every corrective
publication gate below passes. The live sequence-24 consumer remains untouched
until public promotion is independently re-read green.

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

## Selected startup-advisory sequence-27 corrective gate

This is a bounded same-version corrective production operation. It exists only
to publish the accepted startup-advisory product fix and does not weaken the
ordinary newer-version path.

Required preconditions:

- user authorization for the startup-advisory fix is explicit in the active
  Workboard bundle;
- public `main` is re-read as exact
  `bf833822c41cb5ee817922b2ed08e9316f288a40` before the one-shot trigger;
- authenticated public stable is exact sequence **26**, generation
  `local-hosted-0-160-0-4fd14ed8aafc`, version `0.160.0`;
- product source is exact
  `75aed30c7ad6c427d30115b0667957130da198f5`;
- official latest remains exact `0.160.0` with archive SHA-256
  `7f0fe42ff22ecfa3a47bc4a34f5b22c4218b431a4ec0aba51c7d98299f07900c`;
- next release sequence is exactly **27**;
- the same-version override is accepted only under an exact one-shot selector
  bound to the source, public parent, current generation/sequence, target
  sequence, upstream version/digest, and publication authorization;
- candidate generation identity is fresh and existing public generations are
  immutable.

Then run the frozen release invariants in order: build -> native smoke -> sign ->
independent verify -> immutable Release -> LKG Pages -> every-byte public
readback -> disposable ordinary update/no-op -> non-forced exact-parent CAS.

After promotion, independently re-read the signed stable pair and generation
manifest. Only then run the user's live sequence-24 consumer through ordinary
signed public `codex update`. Direct-jump acceptance must prove sequence 27 is
active, `codex-cli 0.160.0` executes, Core/Manager/runtime remain healthy, and a
second exact-current update causes no state delta.

## Ordinary future upstream intake

This sequence-27 selector is one-shot and must not be reused. After acceptance,
future upstream releases return to the ordinary scheduled/manual producer under
freshly rebound source/public authority. Historical one-shot selectors never
become current authorization merely because their code remains in history.
