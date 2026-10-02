# Rust Core Workboard

Authority: SPEC.md -> GOAL.md -> WORKBOARD.md. Approved equivalent primary Lead;
workers OFF. Selected bundle: MANAGER-NOTIFY-INPUT-REQUEST-V1.
Bound clean HEAD db7411bd0cc0432b5dc94b52933ccc31f2e1664d; public/live sequence 29.
Baseline: 217 Rust pass, one explicit device ignore; no red gates. Temporary
build/proof root /data/data/com.termux/files/usr/tmp/codex-manager-notify-6js8vzy7.

Ordered vertical slices:
1. REVIEW (complete): current repair plan qualifies signed runtime/layout and
   returns none/healthy; it is a narrow legacy-layout migration path. Manager
   list contains only default while current reports external: known external
   identity visibility is a useful future improvement. No profile mutation.
2. DELIVERY (complete): Manager notify test reports each selected provider's
   result, fixed text only; five-second process-group bound; emit remains silent
   best-effort. Production Manager lib/help and integration tests only. Proof:
   explicit success/failure/missing/timeout, descendant cleanup, both independent,
   disabled hooks testable, invalid args/state non-mutating, output private.
   Focused nonzero proof: six unit + four integration notification tests pass;
   new definitions/branches map to three notification_test_* regressions; actual
   Manager diff inspected. Existing silent emit failure regression remains green.
3. INPUT (source complete; native proof in slice 4): Core projects UserInputRequest via actual upstream PreToolUse
   anchored request_user_input(_async) matcher, suppresses redundant handler under
   broad PreToolUse; fifteen-second hook timeout. Manager accepts/canonicalizes
   selector. Proof: parser/record, real Core projection, native official matcher
   behavior and real question/approval/Stop hook paths. Preserve argv/shared server.
   Core notification_* plus actual mgr3 production-exec projection tests and
   Manager selector roundtrip pass; changed branches/timeout map to those gates.
   Actual Core/Manager diff inspected; native lifecycle acceptance remains open.
4. RELEASE/APPLY (selected): grouped scripts/check.sh/release build, protected
   auth/config/resolver verification and fault/recovery acceptance; commit/push,
   signed publication/ordinary activation; apply both + PermissionRequest,
   UserInputRequest,Stop preferences; actual delivery and lifecycle proof, native
   shared-server/resume preservation, bounded artifact cleanup and close ledger.

Do not mutate protected live state before slice 4. Provider probes in review are
read-only. No workers, credential/session parsing or unrelated profile changes.
