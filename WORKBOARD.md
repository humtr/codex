# Rust Core Workboard

Authority SPEC.md -> GOAL.md -> WORKBOARD.md; approved equivalent primary;
workers OFF. Goal-md resolver binds codex-goal.md to this GOAL.md/rewrite lineage.

Selected NOTIFY-SINGLE-LINE-TMUX-COLOR-V1, 2026-10-02. Bound clean Codex
08b67cfec7d0afd9e62f730936298219ac619fa4, signed/public/live33, upstream0.160.0.
AI main5ce014ce7bbcffc916ee2c1a1c5e8a000e231ce9 has pre-existing dirty
lib/ai_provider.py, lib/ai_tui.py, verify/ai-tui-smoke.sh; preserve/exclude them.
User prioritizes single-line notification (toast unchanged) and tmux rich/color
rendering; additionally hide native status row only for AI-managed sessions.
Prompt-cache retention investigation is deferred; keep delay0.
Baseline Codex grouped gate runs in one disposable root; AI final-verify passed
37 gates/0 warnings. AI dirty identity9fe1bac80363 preserved.

Ordered vertical slices:
1. Notification title/body always fold whitespace to one line; configured toast
   newline policy stays intact. Paths Manager lib/profile_commands tests, SPEC.
   Focus: actual both-provider multiline dispatch and Unicode folding regression.
   Protect hook selection/action, payload limits, config shape, auth/content privacy.
   State: CLOSED source slice. Exact focused dispatch1/1 and complete Manager
   21 unit/15 integration pass; production diff reviewed. Notification-only fold
   is at provider boundary; no toast/config/hook/action mutation.
2. AI tmux launches use current caller color preferences, including explicit
   absence, rather than stale tmux server environment. Preserve native TERM,
   upstream argv/CWD/profile, explicit NO_COLOR and existing-pane focus. Paths
   AI ai_tmux.py/tests/test_tmux_launch.py/README. Focus actual isolated tmux
   stale-NO_COLOR removal, explicit preference preservation and ANSI rendition.
   Protect unrelated dirty AI work, existing user panes/server/config/auth/history.
   State: CLOSED source slice. Native tmux tests5/5 pass, including stale values
   for all6 color keys, explicit NO_COLOR/FORCE_COLOR preservation, native TERM
   and actual captured ANSI. Actual diff reviewed; no global server mutation.
3. AI-owned session hides native bottom status row; foreign sessions preserve
   their setting. Paths AI launch/test/README. Proof actual owned status off and
   foreign status on before/after launch/focus. State: CLOSED source slice. Native
   owned/foreign status and all5 tmux regression functions pass.

Source acceptance: grouped230 Rust/90 Python (one explicit device ignore),
locked workspace release, AI37 gates/0 warnings and native5 tests pass. Actual
installed AI7656358 is pushed; native bare Codex has RGB/bold/dim, zero argv
overrides, NO_COLOR absent, native screen-256color and owned status off. Existing
owned live status is off with unchanged pane identities/global status; protected
15 unchanged. Actual product/authority diffs reviewed.

Next: implementation commit/push and signed Manager-only publication/activation,
protected fingerprints, implementation commits/push; bounded AI-only install,
signed Manager-only Codex production/public ordinary update, native synthetic
notification and isolated real Codex tmux ANSI proof. Existing user TUI cannot
have its startup environment changed in place; do not terminate active work.
Remove own probes/staging without backup after acceptance. No global tmux config
change, credential access, transcript rewrite, lock manipulation or cache pings.
