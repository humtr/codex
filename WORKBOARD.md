# Rust Core Workboard

Authority: SPEC.md -> GOAL.md -> WORKBOARD.md. User-approved equivalent primary
Lead; workers OFF. No implementation bundle is selected.

SHARED-CONFIG-BOUNDED-RETENTION-V1 is accepted and closed in GOAL.md.
Implementation: bdb02cc7d10095fb923f8199d069beb995dabbcf, pushed on the independent
rewrite/rust-core lineage. Acceptance-only ledger updates preserve product bytes.
Public stable: f71324348719c1a416f42956074d925004c70ab5; production run 37003873759.
Public/live: sequence 29, Codex 0.160.0,
local-hosted-0-160-0-bdb02cc7d100-bounded-retention; sequence 28 is rollback.

Accepted gates: exact scripts/check.sh (217 Rust, 90 Python; one explicit device
ignore), release build, protected-state checks, signed public/native production
proof, actual bare shared-server/full-access, same-PID hook refresh with loaded
thread/rollout inode preserved, real two-account CWD 6/All 22 pickers and existing/
new cross-account resume, cleanup/prefix/inode proof and idempotent live update.
Actual cleanup and disposition are recorded only in GOAL.md.

Two older generations remain referenced by three existing external tdev tunnel
app-servers. Preserve those live clients; Core maintenance reclaims unreferenced
files on a later ordinary launch/update. No abandoned staging/publication roots
remain. No further protected live-state mutation or implementation is selected.
