# Operating state and next work

Implementation: rewrite/rust-core. Publication: main's immutable source caller.
AGENTS.md owns development rules; SPEC.md owns product behavior and boundaries.
This repository carries no external workflow skill or model configuration.

## Operating baseline

- Public and installed stable51: local-hosted-0-161-0-17e1b52b0d77-s51,
  CLI0.161.0, accepted implementation17e1b52. Publication38557559491 passes all
  eight jobs: Android execution, signing, complete public readback/disposable
  update and non-forced promotion. main caller49b304f pins that exact source;
  promotion53bd199 preserves the caller and independently advances the signed index.
- Frontend source5f8ff12/native38552383479 passes39 actual TUI regressions, lint
  and pristine official Core/dependency proof. Its raw SHA256 is
  947225e0667f4f976a754b2e5d8eae694e29a2769beaf7e93d88516b696f1447;
  installed FD-adapted SHA256 is
  cbf04684fe9e938076e96a989a3c7c71f5cbfa76a47bd469de1a018c526cc127.
  Producer admission binds the immutable input0b5149e0 to that exact39-test run.
- Full local acceptance passes Core197/one pre-existing ignored, Manager42+56
  and three actual path gates, builder25, Python18+59, fmt/Clippy/diff.
  Hosted full acceptance38552682462 and38557124149 pass. Proof-only effb9fd
  passes scoped38557658154 without claiming another runtime build.
- Ordinary signed `codex update` activates stable51; its repeat is a no-op.
  All eight installed hashes/modes and manifest signature verify; twelve base
  config/auth files and six existing PID/start/image identities are unchanged.
  Immediate activation gate preserved all six process identities. A later user
  /switch re-execs the same frontend PID/start onto stable51, as expected.
  Later proof: stable51-device-after-profile-switch.json in the acceptance cache;
  the original immediate green gate is recorded in this session tool output. stable50 is the
  rollback generation. Current running clients keep their image until normal exit.
- Core/Manager doctor is healthy with zero failures. Official upstream doctor
  reports unknown configured title item thread-name and three rollout/DB
  consistency issues; user title/history files were not edited to conceal them.
  Upstream0.162.1 is available but unqualified; this release intentionally keeps
  the accepted0.161 backend. These state/version notes are separate follow-ups.

## Termux permission resume correction

- Source795a064 fixes native local resume dropping Core's no-sandbox default and
  restoring obsolete saved workspace permissions. Approval policy/reviewer remain
  independent; explicit restricted settings fail before local thread/turn requests.
- Native38530530528 passes exact ARM64 build,29 actual TUI tests, lint and pristine
  upstream Core/dependencies. Raw frontend SHA256c2a75d4c594c3008001bd71c5fc5308c75d7e79a7dbd9b8a4aa42296a6b64523.
  This is the historical stable50 input; stable51 admits the combined39-test set.
- Android legacy and named-profile fixtures each pass three actual commands and
  three guardian requests through fresh launch, cold resume and /btw. Seeded saved
  workspace settings become the current supported selection; named identity and
  on-request/auto_review remain intact. An older frontend negative control restores
  workspace settings instead. The deterministic model tests approval plumbing,
  not model judgment, and submits no real inference or account credentials.
  Private aggregate proofs: ~/.cache/codex/acceptance/permission-lifecycle-{native-model,named,stale}.json.
  The reusable script owns its signed disposable roots and needs no Chat provider
  installation or external Python package. Local full checks pass; changed producer
  admission and all Python18+59 tests also pass. Signed stable50 is published and
  installed; the running old client needs a normal restart to apply it. `/switch`
  uses same-conversation native resume in the destination account home, so this
  correction also covers its qualified local cold-resume path. Existing live
  writer reattachment keeps that writer's permissions and does not take it over.

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
- Historical stable49 Web trial in experiment/web-termux preserves the failed native-host trial;
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


## ChatGPT model pilot

- Ownership correction: the active Chat provider reuses miuuyy6.1.5 request/SSE,
  tool mapping, queue, turn identity and summary helpers. Our signed-app
  transport/lifecycle and Core/Manager integration do not make it an independently
  written Responses implementation. Full-mode MCP source is present but not wired
  into the active text relay; tool results currently cause another Chat user
  submission. `Codex Native2` was an upstream browser default copied into an old
  probe, not an MCP wire requirement. Experimental browser entrypoints now require
  the existing connector's explicit display name without a fixed brand/prefix;
  active app routing needs no such name. Existing connections, installed payload
  and dependency licenses remain unchanged. Three selector/CLI tests, five browser
  helper tests and the installed6.1.5 provider mapping pass with zero Chat submits.
- Current work minimizes debt and message submissions in the standalone ChatGPT
  provider. tdev progress/control proposals are separate-session work, not a
  dependency. Source-only ordered packing reduces an owned 50,410-character
  fixture from27 submissions to2 while preserving schemas/history; no Chat quota
  deduction is measured. Admission errors and context/work submission accounting
  are now distinct on the experiment branch. First repair actual model-use boundaries.
  The pinned bridge contains full-mode MCP result continuation, which our adapter
  has not connected. At the user's direction, independently write our continuation
  using miuuyy's flow and AgentDock's lifecycle as references, adapted to Core's
  exact turn APIs; do not import their executors or another management plane.
  Ten further source comparisons support app-native submission plus targeted MCP
  continuation as the current candidate, not wholesale launcher/runtime adoption.
  Browserless Codex proxies and official SIWC use a different inference/allowance
  route; no superior ordinary-Chat protocol or quota-saving measurement is proved.
  AgentDock/CCCC add execution-lifetime and delivery/completion references. Dots
  is excluded from current work at the user's request; its historical source
  comparison does not create a provider dependency or implementation follow-up.
  Then qualify native Chat tool-result continuation through the existing adapter
  before expanding its architecture. Detailed scope and the no-inference stage-cost
  measurement live in experiment/web-termux CURRENT.md. No provider/runtime changes
  or new Chat inference are authorized by these review notes.
  The experiment source now repairs bounded identity/zstd HTTP decoding and
  pre-parser rejection of unreadable compaction. Actual Android offline HTTP,
  merged-catalog and existing-broker IPC proofs pass. Experiment678dbaa adds
  upstream-resolved catalog transfer through Core's existing server coordinator;
  ab35549 records actual Android admission with12 tools, content-bound reuse,
  changed-catalog isolation, retained thread access and Manager owner discovery.
  Native siblings/default sockets remain distinct, without duplicating the config
  loader or routing native inference through the optional adapter. Local77 Python
  and323 Rust tests pass (one existing ignored resolver smoke), plus fmt/Clippy.
  Release frontend38654727667 builds; actual Android legacy/named and upstream
  --profile chatgpt round trips pass with backend catalog binding and no inference.
  That first CI failed compiling a new test before execution; a20c58d fixes its
  error conversion. Native qualification38657053854 remains pending. This is not
  acceptance of long-history cross-provider compaction or real model-use repair.
  A second read-only native connection also observes the exact persisted turn
  become interrupted during an owned local command, whose process exits; stop
  sends no additional provider HTTP request. This control candidate is not wired
  to MCP, and ephemeral threads reject the tested history APIs. Keep cancellation
  behavior until those gaps are resolved. No installed runtime change or Chat
  inference was performed; production remains stable51/provider46db05e.
  Review-only CoS reference: totec448-spec/chat-on-steroids2.1.37 at2d1c86e adds
  precise caller attribution, retained result-delivery and ambiguous-dispatch
  invariants. It is its own MCP executor, not a Responses provider. Automatic
  Continue/Goal may add user submissions; session_finish's HELD is a model
  instruction, not a provider finalization lock. Detailed source paths, limits
  and independent implementation consequences are in experiment/web-termux
  CURRENT.md. Reference code/tests were inspected but not executed; no Chat
  submissions or runtime changes occurred.
  Additional output-silence review preserves execution when rendering/observation
  fails: internal app-store reads reduce visible-DOM dependence but still require
  the app renderer/Scope/CDP. Current finished-only reads, generic abort-on-error
  cleanup and provider-independent heartbeat need separate lifecycle/visibility
  treatment. Plan default factual native status plus optional expanded output and
  exact-conversation viewer; observer/display recovery must add no user prompts,
  restart no actions and never equate heartbeat with progress. Actual rendering,
  observer-loss, heartbeat-only, stop and resume gates remain unqualified; detailed
  findings and fault matrix are in experiment/web-termux CURRENT.md.
- `/model` is the primary opt-in selection; `--profile chatgpt` is its fallback.
  stable51 includes the qualified native menu and generic shared config projection.
  The provider remains an opt-in experiment, not a bundled Core dependency.
  One private core/config/profiles/chatgpt.config.toml owns the registration;
  six existing homes have aliases. An empty new home on installed stable51
  automatically receives its alias and completes two native commands/exact reply
  without creating base config or copying authentication. Manager is unnecessary
  for projection; local profile overrides win. Native model/account defaults stay
  unchanged, and app Chat login remains independent of Codex account selection.
- Actual Android `/model` round trips in legacy and named-permission fixtures
  enter Chat GPT-6/Instant and return to OpenAI. Same PID/PTY/home/thread,
  immutable frontend, official backend and configuration/permissions are retained;
  thread/read verifies both routes and a sibling retains server defaults. No model
  inference or Pro selection occurs in this menu gate. Combined frontend fresh,
  cold-resume and /btw fixtures pass six native commands and six guardian requests,
  retaining no-sandbox and independent auto_review selection. The lifecycle proof
  waits for the delivered fork response; an earlier unsynchronized sample failed.
- Real-use failures on 2026-10-14 remain unresolved. The shared backend logs an
  unknown Chat model; a fresh request reaches the local adapter but returns 400
  `unsupported_desktop_request`. Switching the existing long thread attempts
  previous-model `gpt-6.1-sol` compaction through the newly selected Chat endpoint
  and fails before sampling. The adapter's 2 MiB body ceiling independently
  returns 413 in a no-inference local probe; retained active-history serialization
  exceeds that ceiling, but the failing HTTP body's actual size is unmeasured.
  These are integration gaps, not evidence of a Chat message-limit rejection.
  Prior menu-only and direct-profile proofs do not qualify long-thread switching
  with actual inference. Detailed evidence, uncertainty and required regression
  scope are maintained in [experiment follow-up](https://github.com/humtr/codex/blob/experiment/web-termux/CURRENT.md#chatgpt-model-errors-next-work).
- Provider0de70e3072a4b8e87a31 is installed from experiment source46db05e.
  The signed Linux app26.1007.21434 owns actual
  Chat submit/cancel/state, login and token refresh. Independent temporary Chat
  conversations/ledgers/cursors serve native threads and guardian children through
  one private FD/CDP connection; no UI Send clicks or extracted first-party tokens
  are required. App/display remain dependencies. UI fallback is serialized and
  allowed only before submission; ambiguous failures never replay a native action.
- Real overlapping GPT-6 replies, cancelled-lane/surviving-sibling proof, two native
  Android clients/four commands and two actual guardian reviews pass. The initial
  incomplete guardian sibling remains failed evidence; its bounded retry passes.
  Final two-client/four-command concurrency and two guardian reviews pass on the
  corrected transport. Installed guardian and native catalog-search proofs pass.
  App/adapter restart
  followed by actual submit/cancel, cold same-thread Core resume and two native
  commands/exact reply also passes. Native Core owns local execution, approval,
  history and checkpoints; the adapter converts only advertised text/tool wire.
- Instant/Medium/High/Extra High each pass two actual native commands and exact
  final reply through the new internal transport. Pro entries stay execution
  unverified: automated Pro inference is prohibited and was not performed.
  GPT-5.6 remains deferred after earlier continuation failures. Internal Medium
  compaction now passes four commands, actual summary/secret persistence and both
  Core and whole-app restart/resume. The candidate's shortened execution/context
  instructions were restored to the qualified shared UI prompt; reply parenting
  is explicit and startup waits for the actual app scope before choosing fallback.
  GitHub catalog search and the actual discovered read-only repository call pass
  in wrlab. Compared szob returns no GitHub matches for the same native query;
  a no-inference controlled comparison returns0 GitHub matches there and8 in
  wrlab. Native account tool availability is separate from shared Chat app login.
  Failed/timeout trials stay retained as evidence. Automatic/fault compaction,
  multimodal context, hours-long load and physical phone reboot remain separate
  internal qualification. Phone horizontal width remains deferred. Opaque agent
  payloads/routes refuse, but a newly reproduced bundled-parser gap replaces an
  unreadable native compaction checkpoint with a note. Readable canonical handoff
  or explicit refusal remains required; this is not qualified lossless resume.
- `/v1/responses` is the wire interface, not an API entitlement. Requests use the
  signed app's Chat mode; measured native context usage is an estimate, not a
  billing/KV-cache proof. Earlier quota comparison observed no increment even in
  the positive Codex control, so it cannot prove zero Codex deduction. Plus SIWC
  public Responses access is a separate unqualified track; desktop Chat acceptance
  does not grant it. No paid API-key inference or organization-policy change occurred.
- Maintained setup and detailed contracts live on experiment/web-termux. Private
  evidence in ~/.cache/codex/acceptance includes chat-model-shared-device-{legacy,
  named}.json, chat-combined-permissions-{synchronized,named}.json,
  chat-internal-{cancel-resume-retry,installed-guardian,installed-search,
  installed-medium,installed-high,installed-xhigh}.json, chat-stable51-new-home.json
  and chat-{app-concurrency-cancel,native-dual-shared-profile,
  native-dual-guardian-retry}.json. Final internal proofs include
  chat-internal-checkpoint-startup-wait.json, chat-internal-discovered-tool-wrlab.json,
  native-account-catalog-comparison.json, chat-final-concurrent-guardian.json
  chat-final-app-acceptance-timing.json, chat-final-medium-overlap-cancel.json
  and chat-final-installed-inventory.json. App submission timestamps now come
  from response acceptance inside the app; polling timestamps were too late
  to prove overlap for very short replies. Tiny Instant responses can also finish
  before the sibling is accepted; those overlap trials remain false rather than
  becoming proof of a transport defect. Medium proves actual overlapping app
  acceptance, and the independent native guardian gate proves concurrent requests.
  Cancellation and surviving sibling succeeded in the earlier short-reply trials. No user credentials or
  conversation bodies are published with the proofs.
