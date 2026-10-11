# Operating state and next work

Implementation: rewrite/rust-core. Publication: main's immutable source caller.
AGENTS.md owns development rules; SPEC.md owns product behavior and boundaries.
This repository carries no external workflow skill or model configuration.

## Operating baseline

- Public and installed stable45: local-hosted-0-161-0-6c5819c3813a-s45,
  CLI0.161.0, accepted implementation6c5819c. Publication38102005296 passed
  actual Android build/smoke, signing, immutable Release, Pages, complete public
  readback, disposable current-to-candidate update and non-forced stable promotion.
  main callerb2aa15e pins that exact reusable producer/source; histories stay separate.
- Device activation used the signed --local path: six inventory-identical assets
  were copied from stable44 and two downloaded. Signed stable index, manifest and
  complete file/mode inventory were verified before activation. Ordinary second
  update is a no-op. stable44 is now the bounded rollback generation.
- Core and Manager doctor healthy. Ten account/config hashes and the one native
  workload present at activation remained unchanged. Upstream doctor separately
  warns about one duplicate rollout thread ID; protected conversation data was
  preserved rather than rewritten as part of a terminal-routing deployment.
- /switch frontend rawdcf76b3c/adaptedcd4a6619, official source97901140.
  Native27/lint/pristine Core qualification38032317709 at29b2584 remains applicable:
  all six maintained frontend inputs are unchanged. New ordinary launches use
  signed-generation helpers/2; existing native clients remain on their old image.

## Origin-bound launch and return

- `codex termux tmux` works without AI: existing-profile arrow/Enter picker,
  Esc/Ctrl-C cancellation, unchanged saved default, explicit --profile ID and
  upstream argv after --. Attachment stays in the invoking terminal. Fresh
  outside launches own independent tmux sessions; inside launches target the
  invoking TMUX_PANE's exact session. Status is hidden; native titles pass through.
- The native Termux extension uses its existing authenticated AM socket and live
  session table. References bind handle plus kernel PID/start. Same-UID ancestry
  stops at the owning app boundary; protected Android system parents are not read.
  No additional daemon, socket or persistent terminal registry is introduced.
- Origin capture precedes detachment. A session-local @termux_origin_v1 retains
  the validated reference; labels, conversation IDs and connection order are not
  Android terminal addresses. Missing optional capability does not block launch.
- Manager __terminal-focus-v1 qualifies the actual native FD/pane/image before
  selecting a live existing origin. Notifications use that Core/Manager path;
  exact return has no AI, window-constructor or Activity fallback. An unavailable,
  stale or closed target is refused instead of creating another window/workload.
- Corrected native app candidate ca061a7 is installed and physically accepted.
  Base8629e632fcb95da272221be327db653fb24befe9, package com.termux/code118,
  0.118.0+8629e63.origin2. Build38099833533 compiled the actual APK, ran six
  focused Java tests and verified compatible package/certificate. APK SHA256
  d32973c555ff9d8d726306948a0e0af1c2b41ccd2938a22edcd848bf7a278453.
  Original APK and accepted candidate remain in Download/termux-origin-candidate.
- Actual device gate used two disposable native workloads with the real Manager
  and AM socket. Both direct attachments captured distinct origins and kept the
  native root PID equal to their tmux client PID. The user tapped a fresh A return
  notification once and a fresh B notification once; both selected their own
  existing terminal and count stayed three. Wrong-start, stale-thread and closed
  origins were refused. Owned fixtures exited normally; original count one restored.
  This is physical disposable-workload proof, not a claim of testing account data.
- Manual Termux rename can enable the legacy named-service reuse path; OSC title
  output cannot register an unnamed originating terminal. The extension supplies
  automatic origin lookup/selection. Manual named reuse does not require it.

## Independent window/container candidate

- Ordinary interactive Core launches now delegate only optional window routing to
  the qualified same-generation Manager. Maintenance/non-TTY paths and Manager-
  unavailable Core remain independent. The handoff strips inherited loader
  overrides; the one-time READY guard prevents recursive convenience dispatch.
- `codex termux launch` selects a profile with tmux off by default; hidden/status
  are explicit containers. Native-capable APK runs in the invoking window.
  Confirmed stock API uses one random named dedicated terminal per launch without
  requiring tmux. Inconclusive AM failure never becomes a constructor fallback.
- Bare native foreground FD bindings and tmux pane bindings share notification
  focus. UUID changes read the existing FD, rather than titles or stored session
  content. `codex termux attach` rebinds only detached Manager-created work, without
  restarting it. Native return remains existing-only; stock rename/close races can
  create an inert empty terminal, never a new/resumed Codex workload.
- tmux and Android starters use private single-consumption bounded descriptors to
  retain raw argv and caller environment. Terminal identity remains new-terminal
  owned. Descriptors are deleted before exec; lazy cleanup requires verified process
  departure. Credentials may occur temporarily in the private launch descriptor,
  never in permanent bindings, intents, notifications or logs.
- Focus defaults to window; explicit termux still brings only the app forward.
  Earlier tmux focus state is accepted. AI's advertised-command adapter retains
  its chosen authentication supervisor; unrelated AI worktree changes are preserved.
- Local targeted proof: Manager42 unit tests; actual PTY/native FD paths cover
  bare A/B, changed/ambiguous UUID, stock one-shot launch, raw non-UTF8 argv and
  synthetic auth/color/CWD, inert reuse, tmux status, detached-work rebind and
  inconclusive capability. Core PTY dispatch regression proves automatic routing,
  its recursion guard, absent Manager, maintenance and non-TTY boundaries.
- Local full acceptance passed Core193 (one pre-existing ignored), Manager42
  unit +55 integration +three public path gates, release-builder25 and Python18+59,
  fmt/Clippy/diff checks. Actual Android disposable launches A/B bare, C hidden
  tmux and D seeded stock named route retained synthetic env and consumed their
  private launch data. Exact-source CI, fresh single-tap physical observations and
  signed publication remain pending. Stable45 above is still installed/public.
  Disposable real Android trials are separate from the fixture transport tests.

## Source and installed AI

- AI main22e8fd5 incorporates the six client-only paths from candidate e1245e9;
  app source is maintained as a separate formal project at ~/prj/termux, preserving
  official Git history and read-only upstream remote. Old public AI experimental
  commits remain historical evidence, not current app source authority.
- The composed AI implementation passed isolated full verification38 with zero
  warnings/failures, plus existing recovery/profile tests. User changes remain
  uncommitted and preserved; only the clean six-path client change was published.
- The bounded --ai-only installer applied ai_tmux.py and ai_terminal.py. Thirty-seven
  other runtime/launcher/account/config files remained byte-identical, and the
  installed provider-list path works. No unrelated user feature was overwritten.

## Reusable acceptance

- Exact source6c5819c acceptance38087812283 passed every job, including actual
  Android/AArch64 Core execution and locked workspace tests/Clippy/fmt.
  Local Core192 passed/one pre-existing ignored; Manager40 unit +55 integration
  +two public tmux/native-FD paths passed; release-builder25 and Python18+59 pass.
  Two superseded AI-action assertions were updated before the final Manager gate.
- Scoped d18f796 acceptance38100063268 covers documentation; runtime jobs skipped.
  Its unchanged6c runtime proof is reused rather than described as a fresh build.
- Repository-local development rules are accepted; main is a thin source caller.
  Standard checks are scripts/check.sh docs, python, rust or full, chosen by scope.

## Remaining physical observations

- A tmux session started before native origin registration cannot infer its
  original detached Android ancestor. Use a normal new launch with
  `codex termux tmux` or the updated AI launcher; do not recreate a window on tap
  or terminate an existing job merely to retrofit registration.
- /title display after the next normal production launch remains a separate
  user observation. Preserve installed preview assets while native clients use them.
- Any further notification trial needs a new notification: this Android version
  removes the notification after one tap. Never request two taps of the same item.
