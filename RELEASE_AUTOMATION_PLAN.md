# Release Automation and Local-Derived Update Plan

Status: **active only for RUNTIME-ALIGNMENT-UPSTREAM-0.159.2**.
Historical production runs and one-shot gates are accepted evidence in
`GOAL.md`; they are not current routing in this file.

Authority remains `SPEC.md` -> `GOAL.md` -> `WORKBOARD.md`. This file is the
current release drift-control plan. It never overrides those authorities.

## Current production baseline

- Source authority compaction commit:
  `28cea0b97c7b4ae32e7b37816f2f1d277fdb07a6`.
- Public-control branch before the production trigger:
  `main=4d7418085265073bf1dada737a364aee54b374bd`.
- Signed stable sequence: **23**.
- Signed stable generation:
  `local-hosted-0-159-0-b164e5b61cb4`.
- Current stable/live upstream version: `codex-cli 0.159.0`.
- Sequence-23 producer source:
  `b164e5b61cb438913cdb1633d74c62d6c96f6523`.
- Accepted source-authority product implementation:
  `4fd14ed8aafcf29602f43e00c314eb8da39a6e5f`.
- Live Termux consumer: exact `0.159.0`, active sequence-23 generation above.
- The sequence-23 immutable GitHub Release is retained.
- Authenticated download-size resources are public at
  `<generation>/compat/download-size-v1` and
  `<generation>/compat/download-size-v1.sig`.

Fresh official upstream preflight on 2026-09-30 resolved:

- version: **0.159.2**
- exact archive SHA-256:
  `05a524a463cadf7e3e22c7f923539c0d0b74c3e78b1f5f1fab52e50e6fb3312f`

The previously planned 0.159.0 same-version sequence-24 correction was blocked
by this pretrigger check before any public or live mutation and is superseded.
The selected alignment is now an **ordinary newer-stable 0.159.2 sequence-24
release** from the accepted source.

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

## Selected 0.159.2 runtime-alignment gate

This is a bounded ordinary newer-stable production operation. It does not
authorize unrelated product changes.

Required preconditions:

- user authorization for runtime alignment is already explicit;
- public stable remains exact sequence 23 generation
  `local-hosted-0-159-0-b164e5b61cb4`;
- public `main` is exact
  `4d7418085265073bf1dada737a364aee54b374bd` before the one-shot production
  trigger commit and is re-read immediately before that commit;
- accepted product source is exact
  `4fd14ed8aafcf29602f43e00c314eb8da39a6e5f`;
- official latest remains exact `0.159.2` with archive SHA-256
  `05a524a463cadf7e3e22c7f923539c0d0b74c3e78b1f5f1fab52e50e6fb3312f`;
- ordinary comparison from authenticated stable 0.159.0 to official 0.159.2
  returns `candidate=true`;
- next release sequence is exactly **24**;
- candidate uses a fresh generation identity;
- the production trigger is a one-shot exact-parent/exact-message push bridge
  that also pins `CODEX_SOURCE_SHA` back to the accepted source. Any parent,
  message, baseline, source, version, sequence, or candidate mismatch fails
  closed.

Because upstream changed versions, runtime/code-mode-host and other
upstream-derived payloads are not required to be byte-identical to sequence 23.
They must instead be derived from and qualified against the exact authenticated
0.159.2 archive. Core/Manager remain source-bound to exact `4fd14ed8...`.

Then run the frozen release invariants in order: build -> native smoke -> sign ->
independent verify -> immutable Release -> LKG Pages -> every-byte public readback
-> disposable ordinary update/no-op -> non-forced exact-parent CAS.

After promotion, independently re-read the signed stable pair. Only then run the
live consumer through ordinary `codex update`, verify active sequence 24 and
`codex-cli 0.159.2`, and run exact-current again. Capture the authenticated
TTY download progress as live acceptance evidence if present.

## Ordinary future upstream intake

This one-shot alignment bridge is not reusable. After sequence 24 acceptance,
future upstream releases return to the ordinary scheduled/manual producer under
freshly rebound source/public authority. Stale historical one-shot selectors
must never be interpreted as current authorization.
