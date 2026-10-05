# Rust Core Workboard

SPEC -> GOAL -> WORKBOARD; goal-md bound; workers OFF.
Current branch experiment/profile-tui, frozen accepted baseline rewriteb949afa.
Public/installed40 and retained39 remain protected.

Current bundle PROFILE-TUI-EXPERIMENT, user-authorized feasibility prototype.

0. Native baseline: exact upstream0.160.0 source; unmodified AArch64-musl
   hosted build/executable proof and owned-device execution. Pending. Production
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
