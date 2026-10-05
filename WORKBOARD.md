# Rust Core Workboard

SPEC -> GOAL -> WORKBOARD; goal-md bound; workers OFF.
Current branch experiment/profile-tui, frozen accepted baseline rewriteb949afa.
Public/installed40 and retained39 remain protected.

Current bundle PROFILE-TUI-EXPERIMENT, user-authorized feasibility prototype.

0. Native baseline: exact upstream0.160.0 source; hosted AArch64-musl build,
   executable proof and owned-device execution. Current RED: exactd006419/
   hosted37318131485 passes --locked, then tikv-jemalloc-sys fails atomics probes
   under the improvised musl-gcc toolchain. No UI behavior added.
   KEEP exact source and locked graph; COLLAPSE improvised native build setup into
   upstream's exact release Zig0.14 + musl-tools script. DELETE obsolete lock
   update fallback after graph restoration. Retain failed native config logs.
   Focused helper3/3 passes. Exhaustive normalization changes159 local inherited
   versions;1313 external packages unchanged. Original5553f065; finalf0ea6b03.
   Production definitions/behavior frozen until native executable target runnable.
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
