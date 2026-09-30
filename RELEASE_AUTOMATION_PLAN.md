# Release Automation and Local-Derived Update Plan

Status: **active only for AUTHORITY-COMPACTION-V1 / RUNTIME-ALIGNMENT**.
Historical production runs and one-shot gates are accepted evidence in
`GOAL.md`; they are not current routing in this file.

Authority remains `SPEC.md` -> `GOAL.md` -> `WORKBOARD.md`. This file is the
current release drift-control plan. It never overrides those authorities.

## Current production baseline

- Public-control branch: `main=4d7418085265073bf1dada737a364aee54b374bd`.
- Signed stable sequence: **23**.
- Signed stable generation:
  `local-hosted-0-159-0-b164e5b61cb4`.
- Upstream version: `codex-cli 0.159.0`.
- Sequence-23 producer source:
  `b164e5b61cb438913cdb1633d74c62d6c96f6523`.
- Accepted source-authority product implementation:
  `4fd14ed8aafcf29602f43e00c314eb8da39a6e5f`.
- Live Termux consumer: exact `0.159.0`, active sequence-23 generation above.
- The sequence-23 immutable GitHub Release is retained.
- Authenticated download-size resources are public at
  `<generation>/compat/download-size-v1` and
  `<generation>/compat/download-size-v1.sig`; they are signature-verified,
  bound to the exact `release.manifest` SHA-256, and are control resources
  rather than installed generation inventory.

The sequence-23 producer source is intentionally treated as a production
artifact, not as current source authority. It differs from the accepted source
implementation in Core/Manager product files, so the selected maintenance bundle
must realign public/live runtime rather than declaring those states equivalent.

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
- the sidecar never enters installed generation inventory;
- older pre-feature signed generations may legitimately lack the sidecar.

## Selected runtime-alignment gate

This is a one-shot manual same-version correction. It does not authorize a new
feature or a different upstream release.

Required preconditions:

- event is explicit/manual publication authorization for this bounded operation;
- current public stable is exactly sequence 23 generation
  `local-hosted-0-159-0-b164e5b61cb4`;
- public `main` is re-read immediately before mutation and its exact parent is
  used for CAS;
- official stable remains exact `0.159.0`;
- accepted product source is exact
  `4fd14ed8aafcf29602f43e00c314eb8da39a6e5f`, unless a later docs-only
  authority commit is first proven product-tree-identical;
- next release sequence is exactly **24**;
- the candidate uses a fresh generation identity;
- source-owned Core/Manager outputs must bind to the accepted source;
- upstream runtime, code-mode host, helper payloads, and other source-independent
  load-bearing bytes/modes must either match the authenticated sequence-23
  baseline or have a separately proved reason for deterministic rebuild
  difference; unexplained drift fails closed.

Then run the frozen release invariants above in order: build -> native smoke ->
sign -> independent verify -> immutable Release -> LKG Pages -> every-byte public
readback -> disposable ordinary update/no-op -> non-forced exact-parent CAS.

After promotion, independently re-read the signed stable pair. Only then run the
live consumer through ordinary `codex update`, verify the active generation and
health, and run exact-current again.

## Ordinary future upstream intake

This maintenance gate is not reusable for a future version. When official
upstream moves beyond `0.159.0`, rebind current authority and select a new
Workboard bundle if source/release changes are needed. The established scheduled
or manual producer may be used only if its exact pins and gates match the then
accepted source and public baseline. Stale historical one-shot selectors must
never be interpreted as current authorization.

The next genuinely newer live update is also the first opportunity to complete
the deferred user-visible TTY acceptance of authenticated
`downloaded / total` progress from the installed 0.159.0-era Core.
