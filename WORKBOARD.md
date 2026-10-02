# Rust Core Workboard

Authority SPEC.md -> GOAL.md -> WORKBOARD.md; goal-md bound, approved equivalent
primary, workers OFF. Prior notify/color bundle accepted signed34; see GOAL.

Selected TMUX-STATUS-CHOICE-TERMINAL-FOCUS, 2026-10-02.
Bind Codex4b00b8d/rewrite, public07d6255/signed34/upstream0.160.0;
AI7656358/main with pre-existing provider/TUI/smoke dirty identity9fe1bac80363.
Preserve unrelated dirty hunks; only bounded status UI/code changes are committed.
Native tmux5-test baseline passed before mutation. No red baseline gates.
Resume binds Codex dirty diff425cf1c32338 and AI staged tree
372e3050ffd121915ee98633959c790c27a44dc6. Existing unrelated working diff
3acdf081e07b is preserved. Primary continues directly; workers OFF.

Ordered slices:
1. AI exposes off / on-hidden / on-status in CLI and TUI; legacy bool maps to
   hidden/off; upstream arguments untouched. Owned status mode controls one row
   and mouse window selection; foreign/global settings preserved. Paths AI CLI,
   plan, TUI, tmux, native tests, TUI smoke, README; SPEC contract changed first.
   Focus nonzero native regressions + TUI render/command/normalization cases;
   syntax/actual diff review before next behavior. State: FOCUSED GREEN. Native6
   test functions include actual PTY SGR status mouse selection; TUI162 pass.
   Latest test-only strengthening checks visible request inside a foreign session.
   One trailing-whitespace diff-check red was corrected before commit; no product
   behavior was added while red. Exact staged candidate passes isolated final37
   gates/0 warnings, TUI161 and all6 standalone test scripts. Working runtime
   with the already-installed unrelated UI/provider work passes final37/TUI162
   and native6. Actual staged and runtime diffs reviewed; installation delta is
   only CLI/plan/tmux and four tmux TUI hunks. State: ACCEPTED/INSTALLED e6a91e6.
   Installed public selectors pass; protected15 and pane/PID2 unchanged.
2. Exact native Termux terminal routing without new terminal/process on click.
   State: DIAGNOSIS. Decompiled installed TermuxService confirms named reuse
   falls back to creation if missing. TermuxSession.execute creates a native
   terminal even with nonexistent executable. Reject invalid-executable guard
   and precheck-only race as acceptance proof; no live routing mutation yet.
   Activity has no existing-terminal identity selection extra. Installed Termux is
   0.119.0-beta.3/versionCode1022, signed by F-Droid, SHA256 certificate
   228fb2cfe90831c1499ec3ccaf61e96e8e1ce70766b9474672ce427334d41c42.
   A locally signed patch cannot update this app in place. Scope clarification
   sent to user while status work continues; no app replacement/reinstall or
   preference hack. Investigate minimal native app capability; do not install APK,
   modify native preferences, kill user tasks or inject terminal keyboard input.
   User-directed legacy inspection confirms old RunCommandService opens a new
   native terminal running tmux attach. Its action-only tests do not prove exact
   existing terminal selection. No source copied. User clarification pending:
   permit historical new-terminal attach, or retain existing-only native target.

Status gate complete: isolated grouped37/TUI161/all6 scripts, working37/TUI162/
native6, actual diffs, installed selectors and protected fingerprints. Push source
and authority closure; terminal routing remains the next implementation decision.
Single-line notification reverified through installed public emit to actual native
providers; toast keeps newlines. Own synthetic notification removed. Cache
follow-up remains deferred, thread unload delay0 remains intact.

Session recovery slice: user explicitly requests recovery of the previously
closed tmux conversation. Kernel FD inspection binds its writer to the old
signed33 shared server; FD34 already specifies delay0, but thread/read reports
active. Public client closure left its running turn alive, so idle expiry cannot
release it. One target-only upstream turn/interrupt succeeds, status becomes
idle and upstream removes its lock normally. No lock deletion, server kill,
session rewrite or timeout change. Rebind after interruption confirms lock absent
and existing tmux pane/PID identities unchanged. State: RECOVERED; background
turn preservation remains the native contract, not an automatic cancellation.
