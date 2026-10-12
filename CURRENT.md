# Operating state and next work

Implementation: rewrite/rust-core. Publication: main's immutable source caller.
AGENTS.md owns development rules; SPEC.md owns product behavior and boundaries.
This repository carries no external workflow skill or model configuration.

## Operating baseline

- Public and installed stable49: local-hosted-0-161-0-952c3eacb22c-s49,
  CLI0.161.0, accepted implementation952c3ea. Publication38218954233 passed all
  eight jobs, including Android execution, signing, public readback/disposable
  update and non-forced promotion. main caller71b48bf pins that exact source.
  Source acceptance38218520434 passed all four jobs. Frontend source73f77de
  removes the bare-registration omission; native27/lint/pristine Core proof passed.
- Local composed full acceptance passed Core193/one pre-existing ignored,
  Manager42+56+three actual path gates, builder25, Python18+59 and fmt/Clippy/diff.
  The two first-launch startup fixtures now remove inherited window-ready state;
  their focused rerun passed after the inherited environment skipped the advisory.
- Initial public device update hit the existing300-second large-payload transfer
  limit and fell through to local-derived update, whose same-version frontend
  boundary refused it; stable45 remained intact. This was a transport/fallback
  failure, not rejection of the paired signed stable46 release. No trust boundary
  was weakened to activate it.
- Earlier stable48 device activation used the normal signed `codex update --local` path, with six
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

## Question notification consolidation

- Manager now presents the allowlisted sync/async question tools' actual text,
  never their options or other arguments. Empty question reminders are no-ops.
  A validated conversation ID selects one Android notification per group/thread;
  completion updates that notification rather than leaving a generic input alert.
  Separate conversations and existing exact window return remain independent.
- Candidate native Android Manager delivery passed against the real Termux:API:
  sync/async question replacement, native completion replacement, two separate
  conversations, empty-reminder suppression and removal of all owned alerts.
  The reusable `scripts/qualify_notification_delivery.py` uses disposable state
  and synthetic content only. Local full acceptance passes Core193/one pre-existing
  ignored, Manager42+56 and three actual path gates, builder25, Python18+59,
  fmt/Clippy/diff. Hosted acceptance38218520434 and publication38218954233 passed.
  Ordinary signed `codex update` activated stable49; installed Manager's real
  Android delivery rerun passed and doctor exited0. Twelve protected file hashes
  and fourteen preexisting live PID/start/image identities stayed unchanged;
  stable48 is the rollback target. No running conversation needed restarting.
  The live structured plugin-installation question also produced exactly one body
  notification, with the current conversation tag and no generic input reminder.
  Proof: ~/.cache/codex/acceptance/notification-consolidation-device.json.

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

- Web trial tunnel DNS is now qualified separately from startup health. The
  standalone global Linux/CGO-disabled CLI selected /etc/resolv.conf and retried
  localhost53; Tdev's own native Android/CGO binary was already correct and is
  unchanged. Exact OpenAI source0f870e5 built on-phone with Android/arm64 CGO
  resolves api.openai.com and its owned trial records successful HTTP204 long
  polls. The existing key reference/tunnel ID are reused, with env-i/private HOME
  and no active TurnBroker. Local ready/healthy alone is insufficient. The same
  alias/profile/source remain a temporary experiment, not an installed service.
  Native artifact/proof are retained under XDG cache. The user confirmed plugin
  creation/installation; real model/tool-loop acceptance remains open and the
  execution broker is not yet connected.
  Proof: ~/.cache/codex/acceptance/web-native-tunnel.json; experiment8fce479.

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
- Independent experiment/web-termux preserves the failed native-host trial. Production remains stable49;
  the restored mobile viewer ise72f1e8 with user-accepted023c263 controls. Original
  viewer/Tunnel connectivity and Native2 installation are confirmed. The rejected
  dca030e width behavior is inactive. Fresh navigation uses --refresh; browser
  login/profile and owned normal-window480x785 fitting are preserved.
  The stale cookie-only seed had24 account cookies versus27 in the current
  experimental profile. A read-only SQLite snapshot excludes challenge cookies
  and other domains and reaches an authenticated unique composer. One prepared
  empty Temporary Chat is handed off by exact owned CDP target ID and consumed
  once; stale targets, drafts, user turns and ambiguous composers are refused.
  This removes the redundant navigation between inspection and actual Core work.
  Actual signed stable49 Core,6.1.8/7cc8b8d server and TurnBroker in disposable
  roots reach authenticated surface, model/effort, exact Native2 selection, full
  prompt/files and Send. The conversation POST returns403 plus
  cf-mitigated:challenge; the adapter reports chatgpt_security_check. The same
  result occurs using the unchanged full experimental browser profile in place.
  This is the model-submission failure, not a disconnected viewer or Tunnel.
  Real reply, two model-initiated tools, cancel, compact and resume remain open.
  Native TypeScript passes. Six affected suites pass177/0 with1005 assertions,
  including five actual native Chromium DOM tests. Account-free private-pipe
  proof passes descriptor/auth/origin/concurrent-owner refusal, prepared-page
  readback/consumption, stale-target/draft/user-turn refusal, persistent context,
  reconnect/network and owned-process/descriptor cleanup. These gates do not
  establish real service success. No private screenshots/conversations/keys are
  published and no CAPTCHA solution or browser-property override is used.
  Automatic approval review rejected an indefinite native-host hold of the live
  experimental profile; it was not executed. The bounded human-view variant was
  allowed: max120 seconds, no model requests while waiting, owner verification,
  private recovery checkpoint, automatic ordinary-browser/marker restoration and
  stale-alert removal. The latest timed view displayed a challenge title without
  a composer, expired and restored ordinary Chrome. The user confirmed clicking verification returns to Verify repeatedly. This
  fails the native-host human gate; it is not missing user input. DNS/public
  validation JavaScript access work (HTTP200). An account-free public diagnostic
  shows JS/cookies/WebCrypto available but no WebGL without unsafe fallback.
  Verified apt154 was compared in TMPDIR with installed152: both support WebGL
  with Playwright's unsafe fallback and both lack it when the flag is excluded.
  An upgrade is therefore not proven to fix verification. No package/operating
  graphics flag was changed; downloaded154 files were removed. The particular
  triggering bot signal remains unproven. Keep ordinary Chrome/Native2 Tunnel
  running; require a new technical basis before another production check.
  Private aggregates: ~/.cache/codex/acceptance/web-current-login.json,
  web-same-profile.json, web-human-check.json and web-warm-page.json.
  Experimental browser data belongs to ~/.local/share/codex-web-experiment;
  current isolated sources/checkouts remain in TMPDIR. There is no new boot
  service, merged provider or production installer admission. Operating Tdev
  config/key hashes and the native trial Tunnel remain unchanged.

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

- Official alternatives to the failed native-browser human gate were checked.
  Linux desktop preview now publishes ARM64; signed 26.1007.21434 installed and
  --version passed in an owned Debian13/PRoot rootfs. The full-size shell and
  bundled 0.162.0-alpha.17.2 stdio app-server started, but the observed UI remained
  loading/blank at first; a longer bounded wait with --disable-dev-shm-usage
  reached the actual sign-in screen. That is renderer proof, with no user login
  or model acceptance. This is not Android support or a new production generation.
  All account-free GUI trial launchers were stopped; the guest rootfs and temporary
  source checkouts were removed. Verified package and aggregate proof stay in XDG
  cache, with no new global command or auto-start service.
  Official SIWC ChatGPT plan usage can instead feed the public Responses API to
  Codex app-server with a user-granted dynamic client. Account/model entitlement,
  protected token storage and real model/tool-loop acceptance remain unqualified;
  existing Codex/browser credentials are not repurposed. Hosted Web tools are
  not feature-equivalent on this route. Details and primary sources remain in
  experiment/web-termux's experiments/web-provider/README.md.
