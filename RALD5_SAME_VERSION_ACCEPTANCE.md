# RALD-5 Same-Version Publication Acceptance Bridge

Status: explicitly authorized by the user on 2026-09-17 for RALD-5 acceptance only.

This tracked decision is a bounded amendment to `RELEASE_AUTOMATION_PLAN.md` for the active RALD-5 bundle. It does not change `SPEC.md` product behavior, RALD-6, or RALD-7.

## Authorized acceptance stimulus

RALD-5 may use the currently official OpenAI stable `0.154.0` as one production-equivalent acceptance candidate instead of waiting for a later stable release, but only when all of the following are true:

- the event is an explicit `workflow_dispatch` on `refs/heads/rewrite/rust-core`;
- a dedicated same-version acceptance input is explicitly true;
- RALD-5 public publication authorization is explicitly true;
- the RALD-4 acceptance-only gate is false;
- the independently authenticated current public wrapper version is exactly `0.154.0`;
- the independently resolved official OpenAI stable is exactly `0.154.0`;
- the candidate is rebuilt from the exact official metadata/archive and archive digest through the ordinary accepted producer source and qualification path;
- a distinct generation identity is used and the public release sequence is derived normally from authenticated current public state;
- the ordinary official signing authority, immutable Release staging, LKG-preserving Pages deployment, complete public HTTPS byte/signature readback, disposable no-argument update/launch/doctor/second-update no-op proof, and final non-forced exact-parent CAS gates remain unchanged.

The dedicated input may only convert the exact authenticated `0.154.0 == 0.154.0` equality case into the RALD-5 acceptance candidate. It must not allow an older, different, malformed, unauthenticated, or arbitrary version/archive to enter publication.

## Production invariants

Scheduled runs and ordinary manual runs continue to require a genuinely newer official upstream stable. With the dedicated same-version acceptance input false, exact-current equality remains a no-op. The RALD-4 acceptance-only comparison bypass remains separate and cannot enter RALD-5 publication.

Any failure before promotion leaves the previous signed stable index authoritative and the current public LKG reachable. Promotion remains non-forced and fail-closed on any compare-and-swap mismatch or ambiguous reread. Force push is forbidden.

This authorization does not start or authorize RALD-6 or RALD-7.
