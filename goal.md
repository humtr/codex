# Fast defect-fix loop

Resolve candidates **one by one, then continue immediately**. `SPEC.md` owns contracts, `AGENTS.md` development rules, and `CURRENT.md` the NC-01..29 matrix, evidence and release state. This file only tracks the active issue and pending batch gate.

## Loop

1. Pick the first reproducible defect or verified gap. Inspect the original caller, owner and relevant code; reuse exact existing evidence.
2. Fix the smallest owned behavior, with one discriminating regression where needed; reject false candidates with evidence instead of workarounds.
3. Run cheap checks (`git diff --check`, affected syntax/format/static checks). **Do not rebuild, rerun all tests, or redo mutation/live proofs after every issue.** Batch related source fixes, then run required compiled tests, strict lint and mutation evidence **once at the batch acceptance boundary**. Run an early focused test only when it resolves uncertainty.
4. Note `fixed-source`, `already-fixed`, `rejected` or `blocked` and missing proof briefly in `CURRENT.md`. Advance immediately; signing and activation are not per-issue prerequisites.
5. At batch acceptance, verify the combined inputs with the applicable tests/Android/client paths. Signed publication, device activation, `scripts/d3_refresh.mjs` (full schemas) and new Native round-trip are distinct final gates. If one fails, return to its owning issue.

Preserve dirty work, original Native writer/history, live sessions and user files. No second executor, unproven replay, unrelated service restart or Pro trial. An issue without a safe immediate fix must not stall independent candidates.

The active-Steer retirement defect is fixed, qualified and delivered.
Provider25 and Full148 are publicly signed/promoted and selected on the device.
The Jan service/MCP and8 referencing profile pins now use signed Provider25.
Canonical full-schema client refresh and a new actual Native work, including
exact retry with one append and confirmed completion, passed. No native CI was
restarted for metadata-only changes. CURRENT.md owns the detailed evidence and
remaining original-turn/NC boundaries.

## Queue

0. **P0 active Steer / Queue edit** — `qualified`: direct upstream Native writer
   integration plus Provider same-Chat/input-prefix/confirmed-Stop retirement.
   Final artifact exposed native_lane_busy in rapid cancellation; fixda5bf62 now
   waits for canceled tasks to retire before reserving the successor's slot and
   retains Stop/controller/effect fences. The held-Stop Android test passes with
   both instructions once on the original turn; real Queue/Shift+Left works and
   no false queue-edit hint appears. Synthetic Chat; zero models; no physical UI.
1. **Provider work/lane alias error** — `qualified`: exact work-ID precedence and
   original failure propagation are retained in the accepted Provider batch.
2. **Paginated blank history** — `already-fixed`; retained frontend gates pass.
3. **AppHost read-only observation** — `already-fixed` on observed paths;
   broader multi-window acceptance remains separate.
4. **codex exec Native owner** — `blocked`: native_thread_owner_unavailable;
   requires supported upstream writer rendezvous, not another executor.
5. **Android ENFILE** — `blocked` for attribution; preserve original evidence.

## Delivery and proof reuse

Original dirty0814be7b workspace is protected. Implementation is in
$TMPDIR/steer-batch-v8lgrrmr/source (e8e2c1b, including frontend registration).
AGENTS.md must be reread after recovery before editing/reviewing that actual diff.

Do not restart the large TUI CI. Native41834657202 atdb307ec passed144 tests,
retained upstream features and format/lint; artifacte286df94 is already built and
was used by final Android proofs. The qualifier fix9adc841 reuses that successful
run across an expected-count correction and literal job-time-budget changes.
Current required count144, source/commands/toolchains/audit/artifact validation
remain enforced. Reuse/controller tests and targeted mutation pass. Registered
input bundle is in release/upstream.json; exact-source Source/Core CI passed.
The user's objection canceled unnecessary full rerun41850957815. The earlier
41842123715 passed the new Steer fault step but remains a canceled job; it is not
used as successful build authority. No unchanged legacy mutation suite is redone.

Provider retirement fixda5bf62 passed official Source/Core/Provider CI, local
186-library/integration/strict Clippy/release/Bun/Python, targeted cancellation-vs-
retirement mutation, final held-Stop Android, Native tools/cold resume/Stop/control,
package/TLS and all three installed compositions. Diagnostic code was removed.

Provider25 (chatgpt-da5bf622bc49-s25) is signed, publicly read back and accepted by
14 disposable Android consumer steps. It is promoted non-forced at maind64e70e.
GHA signing41854609332 succeeded but release creation got403; the existing owner
credential published the retained, reverified signed files without rebuilding or
resigning. Its failed job status remains preserved. Public assets are immutable.

Full publication41858194993 succeeded with the accepted source/artifact:
Base147 and Full148 public readback, disposable consumer and non-forced stable
promotion passed. Device update selected Full148/Base147 and Provider25.

Jan activation reused existing service/tunnel lifecycle and the exact connector.
The old Provider owned the one CDP connection; serial maintenance allowed the
existing temporary relay and canonical d3_refresh.mjs to refresh all17 full
schemas. The relay was kernel-confirmed stopped and signed Provider25 reconnected
directly. The refreshed connector completed new Native work2d7a652d..., append,
exact retry (one write), readback and finish; final idle/completed has0 pending
calls and0 unconfirmed effects. Default service, original Native/app/browser
owners and user data were preserved. Backups remain in live-cutover.

Original unresolved turns and broader NC gaps remain as listed in CURRENT.md.
Their recovery is not inferred from this new-work proof. Pro remains untouched.
The requested refactoring direction is to preserve capability while reducing
independent state, duplicate ownership, preparation and recovery paths; avoid
adding another permanent layer around each newly observed symptom.
