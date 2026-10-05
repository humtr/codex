# Rust Core Workboard

SPEC -> GOAL -> WORKBOARD; goal-md bound; workers OFF.
Current branch experiment/profile-tui, frozen accepted baseline rewriteb949afa.
Public/installed40 and retained39 remain protected.

Current bundle PROFILE-TUI-EXPERIMENT, user-authorized feasibility prototype.

0. Native baseline: exact upstream0.160.0 source; unmodified AArch64-musl
   hosted build/executable proof and owned-device execution. RED: exact event
   1ec630b/hosted37310353892 fails Cargo --locked (exit101) before compilation.
   Root cause: release manifest0.160.0 vs workspace lock0.0.0. Only local
   version metadata normalization admitted; external bindings unchanged, --locked
   retained. Two focused helper regressions pass; native rebuild still pending.
   Actual source normalization passes:154 workspace versions adjusted,1313
   external package structures unchanged. Original lock5553f065; normalized28b14a75.
   Second exact807a89c/hosted37311480006 also fails --locked before compile.
   Local-version normalization is insufficient; isolated Cargo workspace-update
   diagnosis must enumerate the remaining delta. No build may consume that delta
   without a separately reviewed bounded correction. No UI behavior added.
   Complete local diagnostic requires exactly5 more inherited local versions:
   path dependencies omitted from workspace.members. Normalize all159 local
   release manifests, including these5; external packages remain unchanged.
   Focused regression now covers non-member path dependencies and references.
   Third hosted37312598358 diagnostic also exposes missing offline index entries;
   failure diagnosis uses online resolution only in the disposable diagnostic copy.
   Focused helper gate briefly red: rewritten inventory check accepted a missing
   declared member manifest. Restored explicit member-manifest presence check;
   all3 focused tests must pass before another hosted build.
   Production
   definitions/behavior changes frozen until this target is runnable.
1. Native /profile entrypoint and bounded Manager profile display. Pending.
   Paths: experimental upstream TUI patch and owned Manager bridge; focused proof
   actual slash dispatch, list/current/default/error/cancel; protected stage0.
2. Reuse native history/agent selection with account-aware presentation. Pending.
   Proof: actual upstream lists/hierarchy, current writer labels, no private index.
3. Same-terminal explicit selection/reconnect/transfer. Pending.
   Proof: two owned accounts, same conversation, no extra terminal, original
   argv/TTY/streams; cancellation/failure leaves a usable source or target.
4. Active-owner safety and final feasibility acceptance. Pending.
   Proof: running-turn/retained-subscriber cases, cancellation/native lock release,
   failed destination, scoped force only when explicitly selected; protected40/39,
   resolver/auth metadata and user PID/start census unchanged. No live cutover.

Do not preserve a mock/wrapper as completion evidence. Record runnable baseline,
focused nonzero gates and actual diffs here; final experiment disposition in GOAL.
