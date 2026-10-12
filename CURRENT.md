# Operating state and next work

Implementation: rewrite/rust-core. Publication: main's immutable source caller.
AGENTS.md owns development rules; SPEC.md owns product behavior and boundaries.
This repository carries no external workflow skill or model configuration.

## Operating baseline

- Public and installed stable48: local-hosted-0-161-0-b4126438d87d-s48,
  CLI0.161.0, accepted implementationb412643. Publication38139742400 passed all
  eight jobs, including Android execution, signing, public readback/disposable
  update and non-forced promotion. main caller86927c1 pins that exact source.
  Source acceptance38139038173 passed all four jobs. Frontend source73f77de
  removes the bare-registration omission; native27/lint/pristine Core proof passed.
- Local composed full acceptance passed Core193/one pre-existing ignored,
  Manager42+55+three actual path gates, builder25, Python18+59 and fmt/Clippy/diff.
  The two first-launch startup fixtures now remove inherited window-ready state;
  their focused rerun passed after the inherited environment skipped the advisory.
- Initial public device update hit the existing300-second large-payload transfer
  limit and fell through to local-derived update, whose same-version frontend
  boundary refused it; stable45 remained intact. This was a transport/fallback
  failure, not rejection of the paired signed stable46 release. No trust boundary
  was weakened to activate it.
- Device activation used the normal signed `codex update --local` path, with six
  identical files copied from stable47 against the signed inventory. Only metadata
  and the new frontend helper needed downloading. All eight hashes/modes and
  the public manifest signature/key verified before activation. stable47 is the
  bounded rollback target; older busy images stay retained. A subsequent ordinary
  update was a no-op. Core/Manager/upstream doctor passed, twelve configuration/auth
  files and fifteen installed process PID/start/image identities stayed unchanged.
  Private proof: ~/.cache/codex/acceptance/stable48-device.json.
- /switch frontend raw00842f35/adaptede29c5625, official source97901140.
  Native27/lint/pristine Core qualification38135821930 at73f77de applies to the
  exact admitted frontend. New launches use the new signed
  generation; current running clients keep their old image until normal exit.

## Independent window return and optional tmux

- `codex termux launch` selects an existing profile with Up/Down and Enter;
  Esc/Ctrl-C cancels, the saved default is unchanged, --profile is explicit and
  upstream bytes after -- are preserved. Default is tmux off. Ordinary interactive
  `codex` uses the same optional window routing without a profile picker.
- Native-capable Termux runs in the invoking terminal. Confirmed unsupported native
  API uses one random named dedicated terminal per launch without requiring tmux.
  Inconclusive AM failure never becomes a constructor fallback. Stock manual rename
  or close races may create an inert empty terminal, never restart/resume work.
- Bare foreground FD bindings and tmux pane bindings share notification focus.
  UUID changes use the live FD rather than labels/titles or stored conversations.
  `/switch`, /new, /resume and /title do not change the Android origin.
- Hidden/status requests inside tmux refuse before picker, launch descriptors,
  Android commands or work. Both Manager and AI removed their inside-session
  new-window branch; no nested session, extra pane or replacement workload is
  created. Independent Android terminals can each own a separate container.
  `codex termux attach SESSION` rebinds only detached Manager work without replay.
- tmux and Android starters use private single-consumption bounded descriptors for
  raw argv/caller environment; terminal identity stays new-terminal owned. The
  descriptors are removed before exec. Lazy cleanup requires verified departure.
  Temporary descriptors can contain caller auth environment; permanent bindings,
  Android intents, notifications and logs contain none of it.
- Native Termux extension on the existing authenticated AM socket captures exact
  handle/PID/start from peer ancestry and rechecks the app's live table on focus.
  It stops at the app boundary and introduces no new daemon/socket/registry.
  Installed package com.termux/code118, 0.118.0+8629e63.origin2, base8629e632.
  Existing APK acceptance/build38099833533 remains applicable; app inputs unchanged.
  APK SHA256d32973c555ff9d8d726306948a0e0af1c2b41ccd2938a22edcd848bf7a278453.
  Original and accepted APK remain in Download/termux-origin-candidate.
- User tapped all four fresh Manager return notifications. A/B bare and hidden tmux C with synthetic native workloads recorded exit0, selected=true and count5; D's seeded stock named
  route returned exit0/count5 on the custom APK. D proves named-service reuse,
  not an installation of an official APK. All four owned workloads exited normally;
  user-requested cleanup restored baseline count1 and removed the fake install trees.
  Private proof: ~/.cache/codex/acceptance/window-device-88b0a09.json. This return
  proof remains applicable to8ba5997: the follow-up only removes nested construction;
  fresh launch, binding and focus code is unchanged. Actual Android signed installed
  Manager also refused all three nested entrypoints before any state creation.
- A fresh notification is required for each trial: Android consumes it on one tap.
  Never ask for a second tap on an already consumed notification.
- The owned direct signed-stable46 title trial has no PTY intermediary or copied
  authentication. The user confirmed normal rendering and Termux title propagation;
  upstream persisted the selected activity/app-name/model/run-state/current-dir
  order. `/title` configures title fields/order, separately from route identity.
  A discarded intermediary trial rendered badly; it is not product acceptance.
- Bare same-PID frontend re-entry needs replacement of the old qualified FD
  binding. The affected real-exec path now passes native and named-route return,
  UUID changes and preservation of unknown bindings. Local composed full acceptance
  passed Core193/one pre-existing ignored, Manager42+55+three path gates, builder25,
  Python18+59, fmt/Clippy/diff. Two startup fixtures now clear inherited window-ready
  state, so a connected caller cannot silently bypass the first-launch advisory
  test. Hosted acceptance and signed stable47 installation passed. A fresh real frontend
  trial exposed a separate missing bare-registration call; stable48 corrects it.
  The user confirmed the fresh return selected `title switch48` after `/switch`.
  Two notification actions exited0, changed selected=false to true and kept
  terminal count3; the latest fresh tap has explicit user confirmation. The first
  tap's screen observation was inconclusive. PID/start/TTY and Android origin stayed unchanged while the FD
  binding changed across re-entry. This is actual signed frontend/Manager proof,
  with tmux off, not merely the synthetic FD fixture.
  An owned version-only reproduction also identifies a trial-only PATH-alias
  warning: upstream refuses a CODEX_HOME beneath TMPDIR. A separate scratch
  directory removes that warning without weakening the upstream check. Future
  trial launches use this separation; the user's fleeting text was not captured.

## Source and installed AI

- AI main e8d49c3 formally includes the installed profile/auth/Manager adapter and
  migration support; the approved normal push completed, with a clean source tree.
  Enhanced full verification passes50/warn0/fail0, including TUI166 and isolated
  installed Manager profile lifecycle. Migration stages complete accounts before
  publication, restores the source on failure and refuses unreadable live owners;
  unique AGY profile state is preserved. No real account migration was performed.
  All thirteen installed ai_* modules match the published source; only three
  modules needed atomic replacement, and seventeen other runtime/config hashes
  remained unchanged. Private proof: ~/.cache/codex/acceptance/ai-formal-source.json.
- App source remains the separate formal ~/prj/termux repository, with official Git
  history and read-only upstream. AI has no competing Codex profile/window authority.
- AI follow-up1f958e5 removes its AGY shim/installation path; enhanced full verification
  passes48/warn0/fail0. AGY main ac228cf publishes the formerly installed native Android
  support. Its full suite passed and the formal setup installed wrapper261011.01,
  upstream1.2.17, with one $PREFIX/bin/agy entrypoint. The raw input and all9302
  protected profile/authentication files retain their hashes; installed library and
  builder match published source. Native allocator/syscall patterns stay unchanged,
  while explicit Linux compatibility remains available. Default installation does
  not require glibc/proot. Private proof: agy-formal-source.json in the same cache.

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

## Remaining limits

- The former native frontend gated FD registration on TMUX/TMUX_PANE despite
  Manager supporting bare bindings. Synthetic native workload path proofs did
  not cover this real-frontend call. The admitted stable48 frontend removes
  that condition, and its native child gate exercises both bare and tmux processes.
  Native compilation/all27 regressions/lint/pristine Core proof38135821930 and
  qualification binding passed at73f77de. Both bare and tmux children exercise the
  call. Qualified raw frontend00842f35 and input archive805a71f9 have immutable
  public readback proof; the producer now pins this exact source/run/input pair.
  Signed publication/activation passed; the real phone frontend publishes its
  bare FD binding with the matching original Android route. The fresh physical
  return across `/switch` is accepted by the user and has selected/count proof in
  stable48-device.json. `/title` rendering itself is also accepted by the user.
- The user separately confirmed `/switch` to the owned other profile rendered
  normally in the same Android terminal on stable47. This confirms screen/profile
  transition, not missing bare FD registration or notification return. Private
  proof: title-switch-device.json in the acceptance cache.
- Official unmodified Termux8629e632 production inputs, real Android/x86_64 APK,
  accepted Manager24ad60d and real AM client1.5.0 passed hosted38137172382. Two bare
  launches retain their caller terminals and each construct one dedicated terminal;
  six A/B existing returns preserve count and workload count. A dead binding refuses
  before constructor and the other still returns. Only the model frontend is a
  synthetic FD fixture; an emulator-only libc shim exposes the same renameat2 kernel
  syscall already used by production arm64. This is actual stock APK/Manager proof,
  separately from phone/native Codex frontend acceptance. The reproducible manual
  workflow and fixture are retained under .github/stock-window.

- Broader authorized normalization continues separately: independent AGY/OpenCode
  notification hooks, isolated real ChatGPT Web acceptance and the final home/usr
  authority audit. Existing synthetic browser proofs do not establish account,
  model, tool-loop, cancel, compact or resume acceptance. Photos v3 cleanup90b8546
  is published: retired snapshots remain recoverable in Git, active data/source
  bytes are preserved and clean-tree packaging/service-smoke passed.
- Termux source/runtime audit: Tdev's CLI uses ~/.local/share/tdev/active, the old
  composition root is absent, actual authenticated local MCP tools/list returns13
  tools and both default/janmori connections are enabled with control-plane polling.
  Controller/retained House databases pass read-only SQLite quick_check. Runit root
  and15-second monitor preserve their original PIDs; service/AM installer ownership
  checks pass. Package-owned inactive GUI/Cloudflare service entries remain down.
  Four inactive home source/recovery trees were relocated intact to the private
  ~/.local/share/termux-recovery/20261011/original-trees after425 files were extracted
  and hash-verified from a private recovery archive. Automatic review rejected
  deletion; the reversible relocation preserves every original byte instead.
  Legacy credential configuration is unchanged. Empty prj/artex was removed.
- Independent experiment/web-termux has native Android Bun1.4.0, locked bridge6.1.5
  and actual Chromium152 DOM proof. The earlier wire correction applies to this
  exact bridge: original9 contract failures became26 passing HTTP/parser tests
  after correction, plus full TypeScript verification. Parser tests use an inert
  configuration executable; they are not model/runtime-service acceptance.
  Native headed Chromium/X11, private Unix RFB and capability-protected mobile
  viewer HTTP/WebSocket roundtrips pass. Independent experiment/web-termux now
  publishes5a0fcfd: compact40px manual keyboard/automatic switch, one-finger
  scroll/mouse-drag mode, remote Esc and fullscreen controls. Actual native X11/RFB
  fixtures pass repeated input,2-second initial-focus retention, independent
  keyboard controls, one/two-finger scrolling, mouse dragging and remote HTML-modal
  cancellation. The user's native Save dialog was separately closed with a checked
  WM_DELETE request; automated native Save creation remains unqualified. Completed
  account-free fixtures and services were cleaned; aggregate proof/toolbar image
  stay under ~/.cache/codex/acceptance. The first physical scroll attempt switched
  X11 desktops because the saved-account Chrome window was508x345 within a480x785
  viewer. Owned PID/start/image/private environment/ancestry fitting now maximizes
  the normal browser before return. Native regression proves stale-owner refusal,
  lower-area page scrolling after viewer resize and no desktop change. The live
  phone browser now fills the785px desktop height; fresh phone scroll/Esc acceptance
  remains pending. No global desktop binding was removed to conceal the defect.
  A follow-up exposed Chromium's500px minimum cropping the480px phone width; the
  owned fitter now clamps only that normal window's size hints and uses fullscreen.
  Native window geometries match360x700/480x785 and horizontal swipe changes real
  scrollX without changing desktops. Phone width/swipe acceptance remains pending.
  Saved-state automated Chromium repeatedly hit Cloudflare human verification.
  An ordinary persistent browser now opens without automation/debugging flags,
  reusing private account login while excluding challenge-clearance imports. Its
  existing supervisor refits only on display-size changes. Account data stays
  local; no stealth or automated challenge solving is added. Actual manual
  verification and automated-provider real-service acceptance remain open.
  The user accepted matched width and absence of Cloudflare in the ordinary
  persistent browser. Bounded Playwright reopening reached a composer once but
  proved no reply; follow-up diagnostics returned document403/200/challenge title
  without a composer. No challenge was clicked or solved; the ordinary browser is
  restored. Private web-real-service.json records the failure separately from
  manual acceptance. Actual native stdio MCP catalog proof returns six unique
  object-schema tools with zero executed tools; it passes on6.1.5 and6.1.8.
  Latest6.1.8/7cc8b8d is staged separately in TMPDIR with the same frozen lock and
  cleanly rebased Termux/wire patch. Full TypeScript passes. Six affected suites
  passed134/skipped one Windows case; four authority cases initially failed due
  to inherited CODEX_SQLITE_HOME and passed their focused isolated rerun. The
  browser-creation path remains unchanged, so a version bump is not a403 fix.
  The existing stopped surface-probe Tunnel is attached through independent alias
  codex-web-termux-trial and a private TMPDIR profile; native0.0.14 reports running,
  ready and healthy after connect/status. Operating Tdev config/key hashes remain
  unchanged. This is temporary trial state, with no boot service; stop/remove its
  local runtime metadata when the trial finishes. ChatGPT-side Codex Native2
  plugin creation/selection is pending user menu confirmation. Actual response,
  two model-initiated tools, cancel, compact and resume remain unqualified.
  The user then reported unusable scroll/keyboard reflow and an app-setup rate limit.
  Actual Android metrics changed desktop785→933px on IME open. The agent-owned
  layout poller was removed without signalling/restarting Chromium; its actual
  PID/start/image now owns the private browser record, preserving the current form.
  Current viewer holds desktop height across keyboard/address-bar changes, defaults
  to manual keyboard (newv2 opt-in setting), cancels auto focus on any gesture
  movement and sends Esc without focusing hidden input. Fullscreen fitting is
  idempotent and no longer polls. Native account-free regression passes actual
  vertical/horizontal scroll and models the reported IME geometry; an out-and-back
  swipe does not focus keyboard or resize canvas/X11. This is simulated IME layout,
  not physical Android acceptance by itself. The user subsequently reported normal
  usability with this exact change; physical scroll/IME follow-up is now accepted
  separately in web-viewer-keyboard.json. The same ordinary browser remains alive
  and trial tunnel is ready/healthy; operating config/key hashes are unchanged. App setup
  was not retried or bypassed; its red rate-limit message remains unresolved.
  The user confirmed the logged-in composer. Normal owned-browser exit captured
  and verified new experimental auth on2026-10-12, with private mode600 state/marker;
  the bridge removed its login profile only after successful verification. Saved
  auth reopens the experimental display. No pre-existing auth was imported and no
  production route/service was changed. Account detection reports sol/extraHigh/pro
  unavailable, so real model/capability/tool-loop/cancel/compact/resume acceptance
  remains open. Browser data belongs to ~/.local/share/codex-web-experiment.

- Public update currently fetches complete payloads with a300-second per-payload
  budget. Slow large downloads can hit that limit and trigger local-derived
  fallback; the existing same-version frontend rule can then refuse fallback.
  A verified inventory assembled from identical installed bytes and immutable
  Release assets is accepted by the ordinary `codex update --local` path. Efficient
  public acquisition reuse and clearer transport/fallback reporting remain separate
  Core work; do not change signing/admission to work around network speed.
- Existing old tmux work does not infer a detached original Android ancestor.
  New registered launch or explicit detached Manager attach supplies the origin;
  no notification creates replacement work to retrofit old jobs.
- Switching a live bare PTY into/out of tmux is not supported; choose a new safe
  launch. No user job was stopped to change execution mode.
