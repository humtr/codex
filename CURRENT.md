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

- Two independent official alternatives are prepared in experiment/web-termux
  at6bbb383.
  Signed Linux ARM64 desktop 26.1007.21434 installs in a Debian13/PRoot candidate
  under XDG cache; account data stays separately in
  ~/.local/share/codex-web-experiment/linux-desktop/home. The login page rendered
  in bounded trials. Candidate HTTP(S) MIME defaults now route the app's sign-in
  to its scoped Android handler rather than back into chatgpt.desktop. Guest GIO
  dispatch reached that handler using an unrecognized account-free target, which
  was refused. Signed-app URL construction revealed a separate refusal: it uses
  /oauth/authorize. An actual guest button click also identified the default
  chatgpt.com/codex/desktop-auth wrapper carrying authorize_url. Both paths are
  accepted now, with exactly one nested OpenAI target and no URL rewriting.
  Five Android MIME/routing/target cases pass, including the panel helper under
  candidate HOME and owned browser window selection. A private tint2 panel shows
  Browser, ChatGPT and clock. Browser now opens candidate Debian Chromium in the
  same Linux display, separate from the Android authentication handoff. Actual
  panel clicks open one browser, reuse its same window and fit 480x743. Its
  /root/browser-profile is separate from the app and existing browser accounts;
  controller-owned browser processes are included in shutdown cleanup.
  Linux user login completed and candidate auth is ChatGPT with a Business-family
  team claim. The bundled Linux Core passed a real-service ephemeral turn in an
  isolated guest workspace: two successful shell calls wrote/read a correlated
  marker and the final model reply matched it. The owner-only aggregate is
  linux-desktop/guest-core-proof.json; test files/logs/processes were removed.
  No ordinary Termux profile was imported. Actual native GUI input and visually
  inspected distinct uppercase reply now pass, including on the current detached
  controller. The ChatGPT task button returns from Chromium to the response.
  Clipboard readback remains unaccepted. Desktop Work parity and runtime manifest
  404 remain separate unresolved concerns; successful Core/GUI replies do not
  establish complete product parity. SIGTERM now enters
  owned-process cleanup; four older orphan groups were safely stopped.
  Fresh notifications resolve the current endpoint at tap time and verify owner
  PID/start/executable. Default seconds=0 now keeps the authorized interactive
  desktop open until the app/controller closes; positive bounds up to 3600 still
  support disposable trials. The start action detaches the same controller from
  tool-command lifetime after a previous foreground owner vanished and left its
  children alive. Old owned orphan groups and unused private screens were safely
  cleaned. No new system service or global launcher was added. Browser aggregate
  proof is linux-desktop/browser-panel-qualification.json; viewer HTTP 200 passes.
  Native SIWC public-client qualification has a separate loopback consent page
  and owner-only records in ~/.local/share/codex-web-experiment/siwc. It uses the
  unmodified official devkit OAuth/SSE modules, with the official guide's 0600
  file storage rather than the devkit's encrypted high-level store. Current
  Termux Keystore supports signatures, not encryption; no new key service or
  existing credential reuse is introduced. Its fresh notification starts manual
  OpenAI consent. Native Node compilation/vendor 27, Android persistence/privacy/UI 6
  and repository Python 18+59 pass. Signed stable49's account-free app-server gate
  passes two correlated actual shell calls and direct-route body-shape checks.
  Two new actual Android MIME/routing cases also pass; current viewer HTTP and
  delivered page JavaScript pass. That Core fixture is synthetic service proof.
  The user's external sign-in returned 3p_login_tenant_policy_denied, preserved
  separately from verified credentials. ChatGPT workspace Apps settings do not
  govern this tenant policy; Global admin External access does. Official plan
  usage eligibility documents Plus/Pro, so policy access alone is not proof of
  Business entitlement. No organization setting or selected account was changed.
  The user now selects their separate Plus account for the SIWC qualification;
  a fresh alert requests OpenAI login and plan-use consent, with no Admin/API key.
  SIWC real account/model/tool-loop gates remain unpassed. Linux first-party
  login and guest Core now pass independently of the external identity rejection.
  Production remains stable49; no global launcher or boot service changed.

  Public Responses research: SIWC uses api.openai.com/v1/responses and currently
  documents Plus/Pro plan eligibility. Business Codex access tokens are official
  CLI/app-server credentials, not general public API credentials. Workspace
  Agents offers Business preview with an independent api.chatgpt.com trigger,
  scoped token and asynchronous status polling; final answer retrieval is not
  supported. An MCP result callback is a possible bridge, not implemented proof.
  Ordinary API-key inference has separate API charges; no paid request or
  organization-policy change was made. See experiment README's sourced comparison.


- Native Termux ChatGPT provider now has real experimental acceptance on
  experiment/web-termux at6a1715e. The signed Linux app's actual ChatGPT / Chat / Instant
  surface supplies the backend; desktop Codex mode, a mock, extracted first-party
  token and public API-key inference are not substitutes. A private FD3/FD4 relay
  reuses the existing native Bun host. The local authenticated /v1/responses
  adapter reuses qualified bridge92a356f parsing, wire names and SSE; native signed
  stable49 retains approval and all actual local tool execution.
  Real disposable native Core proof completed two separate shell operations,
  returned exact results and matched the final nonce. Actual submission was then
  cancelled before any tool; the first Core exited, a new Core resumed the same
  native thread and completed both commands/final reply. Installed
  `codex --profile chatgpt exec` also completed two native commands and exact
  readback/reply. Its initial profile now advertises native tool search, reducing
  eager tools from220 to16 while leaving existing plugin configuration intact.
  A90,000-character native context passed11 acknowledged setup packets before the
  real task, then completed the same two-command loop. Whole schemas and ordered
  lossless instruction/message fragments are retained; subsequent requests send
  only native context additions and changed definitions. Virtualized DOM code
  blocks are matched by correlation instead of index. Setup-ack and actual-work
  instructions are separate. The sender waits for the app’s actual generation
  and Send readiness; cleanup clears only an exactly matched own draft.
  Actual upstream TUI /model
  displayed ChatGPT / Instant under this configuration profile.
  The installed profile now defaults to actual Chat GPT-6. Physical selector
  readback passes Instant, Medium, High and Extra High for GPT-6 and GPT-5.6 Sol,
  mapped to native none/medium/high/xhigh. GPT-6 Sol is a Work/Codex model and is
  not advertised as Chat. Installed GPT-6 Instant, Medium, High and Extra High
  each pass two actual native write/read calls and exact final reply. The last
  two use gpt6-high-proof.json and gpt6-xhigh-proof.json in the acceptance cache.
  GPT-5.6 Sol executes one real
  tool but fails continuation in Instant and Medium; it is explicitly deferred
  at the user's request. These failures remain evidence, not accepted capability.
  Separate Pro choices carry an execution-unverified label. The user forbids
  automated Pro inference; maintained qualification drivers explicitly select
  only non-Pro models. Pro was neither selected nor submitted during these tests.
  The separate model-menu.patch overlay augments the upstream picker and uses
  same-home/thread/window Core re-entry for launch-scoped provider changes;
  active tools, queued input and side conversations refuse. --profile chatgpt
  remains the fallback. Exact-source native38510380995 atd1b433c passes ARM64
  build,35 actual TUI tests, lint and pristine upstream Core checks. The revised
  overlay uses the official app-server for profile/invocation settings
  through upstream per-thread configuration, rather than introducing an embedded
  source-built execution backend. Explicit --no-daemon keeps the ordinary
  official runtime path. Local full checks pass Core194/one pre-existing ignored,
  Manager42+55+three path gates, builder25, Python18+59 and fmt/Clippy/diff.
  Installed routing uses the document parser and native return explicitly selects
  OpenAI instead of restoring the saved Chat provider. Real phone menu gates pass
  for base configuration and a selected named-permission profile: same PID/PTY,
  home/thread, unchanged configuration, admitted frontend and official backend;
  thread/read confirms both model/provider changes and an independent ephemeral
  thread retains server defaults. Private proofs are chat-model-menu-device-legacy.json
  and chat-model-menu-device-named.json in the acceptance cache. The fixture
  completes normal upstream screen-reader initialization before config hashing.
  Prototype source410231f remains separate; production admission is still pending.
  No experiment code has entered signed stable49.
  The continuation architecture keeps Core history/checkpoints authoritative and
  disposable adapter cursors. Route/account/model/effort or verified connection
  generation changes require replay; unchanged owned context receives deltas.
  Server KV hit/miss/unknown remains independent from correctness and local context
  estimates. Scenario serialization costs and native/desktop boundaries live in
  the experiment README/SPEC. The current route refuses unsupported model/effort
  tuples before inference instead of ignoring requested effort. Supported
  family/power selection uses the signed app's actual controls and checks them
  before every continuation. A new GPT-6 checkpoint fault trial passed compression,
  four tools and persisted resume; the first whole-app restart failed on a
  restored expanded selector.158401f waits for a fresh owned view and reuses that
  menu. Its GPT-6 checkpoint rerun passes four actual tools, committed tool-only
  memory, Core restart/resume and whole-app restart (gpt6-checkpoint-proof.json).
  The installed experimental adapter atcc28b07 also passes a real GPT-6 native
  guardian review: an owned prompt rule requires approval, one guardian request
  interleaves, parent canonical context replays, two native commands complete and
  the exact final reply matches with zero owner conflicts. Child/parent ledgers
  remain separate; unrelated active owners still refuse. Native structured output
  reaches the Chat model without an adapter-generated approval decision. The new
  guardian-lane checkpoint proof passes four actual tools, summary persistence,
  Core restart and whole signed-app restart. Private aggregate proofs are
  ~/.cache/codex/acceptance/gpt6-autoreview-rule-proof.json and
  gpt6-checkpoint-guardian-lanes-proof.json. Home recovery uses the app's public
  new-temporary-chat shortcut only with no visible composer or Stop control.
  Live unsupported tuples returned400 without an inference request; actual
  catalog lookup/tool execution and automatic compaction plus four native calls
  and restart/resume passed with the route-aware cursor. Live draft protection
  refused replay before sending and preserved the generated draft fixture.
  Account/multi-model selection remains distinct acceptance; a synthetic route
  transition test is not an entitlement or cache-hit proof. Bounded adapter and
  whole-controller process restart now pass actual installed CLI tool turns.
  This is an opt-in upstream
  configuration profile, separate from Manager login
  accounts. Current0.161 uses chatgpt.config.toml, not a legacy inline profiles
  table. Base config hashes stay unchanged; ordinary Codex routing and signed
  production binaries remain stable49. Local relay authentication is private and
  command-backed, with PID/start/image and loopback checks. The opt-in auth command
  now serializes bounded on-demand GUI/adapter bootstrap. Its address/capability
  stay fixed across restarts; no profile rewrite is required. Runtime/source/Bun,
  pinned exported PRoot and noVNC assets live in ~/.local/lib/codex-web-experiment;
  runtime.json in private native-provider data is the installation authority.
  Only transient GUI sockets/control roots remain in TMPDIR. Reinstall reuses
  verified permanent dependencies after temporary checkouts disappear. No boot
  service or second production launcher is installed. Base config, marked trial
  config and auth hashes stay unchanged through faults. Android denies /proc/net
  listener inspection, so fresh HMAC readiness challenges prove possession before
  transmitting the bearer; foreign listeners receive no capability. The bind
  probe uses SO_REUSEADDR to distinguish ordinary TIME_WAIT from a live owner.
  Six concurrent starts, adapter termination/profile recovery and whole-controller
  termination/recovery each passed, with actual native write/read calls. Proof is
  native-provider/{lifecycle,lifecycle-migration}-proof.json. Physical phone reboot,
  hours-long load and expired-login recovery remain distinct qualification. After
  learning a secret solely through a real tool result, removing its original
  fixtures, compacting and running two more native shell calls, a new Core resumed
  the checkpoint. The owned Linux controller was then stopped: the same living
  adapter recovered the app and recalled the secret without another shell call.
  lifecycle-checkpoint-proof.json confirms four native calls, actual committed
  summary and checkpoint survival through both Core and whole-app restart. Final path
  acceptance also invokes the auth command for all three marked homes privately,
  verifies its capability without exposing stdout and requires the installed
  source path rather than the calling checkout. lifecycle-auth-path-proof.json
  records that all helpers and the adapter image match the selected installation.
  Clear send failures now use the app's Retry control with1/2-second backoff and
  at most two extra attempts. Cancellation, already received correlated replies
  and ambiguous delivery prevent retry. Bounded retries and their failure policy
  have protocol tests; real induced UI Retry recovery is still unqualified.
  Capability-free/browser-origin HTTP probes returned401 and an active second
  native lane returned409 without a model request. Concurrent-request reservation
  now begins before body parsing; malformed requests release it. Only one physical
  desktop lane is supported; cross-session parallelism is not production-admitted.
  Native Responses/memento compaction now passes in the experiment: a
  secret learned only from actual shell output survives summary/persistence,
  two new native calls and Core stop/resume without repeating it in user input.
  Four shell calls and an exact same-thread summary checkpoint applied. Core
  owns history replacement; only the offered summary’s exact replay retires old
  live call checks. Automatic pre-turn and mid-turn compaction also pass using
  lower thresholds only in disposable tests. Four real native calls per proof,
  matching compaction item IDs, persisted tool-only state and Core restart/resume
  passed. Cancellation after observed actual Chat submission and locally rejected
  relay authentication each committed no partial summary; restart preserved old
  state, then successful compaction and another restart passed. Missing desktop
  token usage previously became zero, suppressing accumulated-context accounting.
  The adapter now supplies UTF-8 size estimates for native context budgeting and
  marks terminal metadata termux_usage_source=estimated_context. These counts
  are neither authoritative server usage nor evidence of quota/billing deduction.
  Text, advertised function/custom tools and native search
  wire mapping are present; image/file/audio context is refused. Actual native
  catalog search now passes: a model-requested lookup returns
  a correlated successful native result, nonempty loaded schemas and an exact
  final nonce, with full permissions in the disposable qualification workspace.
  CLI JSON omits search items; the probe observes the actual matched Responses
  result rather than relying on CLI display. Actual discovered-tool execution also
  passes: installed --profile chatgpt performs native catalog search, executes
  the returned read-only GitHub repository metadata tool for humtr/codex and
  receives the real result/final reply. Image/file/audio support, hours-long load
  and physical phone reboot remain separate gates. Process lifecycle passes above.
  Cancellation retires
  the interrupted ChatGPT temporary lane;
  native resume replays its history into a fresh owned lane.
  Proof: ~/.local/share/codex-web-experiment/native-provider/{installed-cli,
  staged-cli,cancel-resume,compact,auto-compact,mid-turn-compact,interrupt-compact,
  failed-compact,tool-search,discovered-tool,tui,isolation,http-concurrency}-proof.json.
  Experimental
  protocol tests18/85 assertions, lifecycle/profile/routing Python12+5 and repository Python
  18+59 passed. Unchanged production Rust/release evidence above remains applicable;
  no new signed production publication is claimed.
  User reports phone horizontal width still mismatched despite X11 480px proof;
  that physical-layout issue remains explicitly deferred. Plus SIWC/public
  api.openai.com/v1/responses remains a separate unauthenticated qualification
  track; desktop success does not grant public API access.

  Quota proof is separate: this opt-in provider's primary requests go to the
  signed app's Chat/Instant mode. Ordinary Chat allowance is the expected
  classification. A bounded device comparison completed one installed ChatGPT
  profile reply and one ordinary native gpt-6.1-sol reply using the app's Business
  login. This development session uses a different Plus account and was excluded.
  The control used an admitted signed Core in disposable state, with no usable
  refresh token; the original login file remained unchanged. Backend usage and
  token-profile reads before/after and delayed reads retained 0% primary/weekly
  usage and unchanged token counters for both paths. The positive Codex control
  also produced no visible increment, so the small observation cannot distinguish
  billing classification or prove zero deduction. App-server rounds percentages
  to integers; raw server percentages were also integer-valued in this trial.
  Aggregate proof: native-provider/quota-comparison-proof.json in the private
  experiment data root. No quota was burned to force an increment. The Responses
  wire version is v1 (/v1/responses), separate from the runtime bridge 6.1.5 at
  92a356f, native CLI 0.161.0 and signed desktop app 26.1007.21434. The independent SIWC
  public Responses grant explicitly uses Codex/Work usage; Work shares Codex
  allowance. Current development in this original Codex session remains separate.
