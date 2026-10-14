# Operating state and next work

Implementation: rewrite/rust-core. Publication: main's immutable source caller.
AGENTS.md owns development rules; SPEC.md owns product behavior and boundaries.
This repository carries no external workflow skill or model configuration.

## Operating baseline

Signed production authority is [rewrite/rust-core CURRENT.md](https://github.com/humtr/codex/blob/rewrite/rust-core/CURRENT.md).
The current device/public generation is stable51 (CLI0.161.0); older installed
processes retain their loaded images. This branch owns opt-in provider experiments,
not a parallel production activation pointer or release authority.

## ChatGPT internal transport and shared registration

Signed stable51 includes the qualified model menu and shared configuration path;
full source acceptance38557124149 and signed publication38557559491 pass. Native
qualification38552383479/source5f8ff12 passes39 actual TUI tests, lint and pristine
Core/dependency proof. Actual legacy/named model round trips preserve PID/PTY,
account home, native thread and permissions; installed new-home projection plus
real native commands passes without copied auth or base config changes.

Installed experimental provider0de70e3072a4b8e87a31 uses source46db05e. The signed
app's submit/cancel/state own separate temporary conversations per native thread
and guardian child; credentials stay in the app. Continuations explicitly link to
the lane's completed correlated assistant. Internal and GUI transports share the
qualified external-tool/context instructions. App startup waits for its actual
Scope before considering an unavailable-capability fallback. The app and display
remain dependencies; no debug TCP port or extracted first-party token is introduced.

Real GPT-6 overlap/cancel-sibling, two native clients/four commands and two actual
reviews pass. App acceptance timestamps measure overlap directly. Tiny Instant replies can
finish before the sibling starts; a bounded Medium trial proves actual overlap
and cancellation/sibling survival. Earlier non-overlap trials stay failed evidence. Manual
Medium compaction passes persisted tool-only state, four native commands and both
Core/whole-app restart/resume. GitHub catalog search plus the actual discovered
read-only repository call passes in wrlab. A controlled no-inference native catalog
comparison returns8 GitHub matches in wrlab and0 in szob; app login does not grant
another native account's connector tools. Failed/timeout trials remain evidence.

29 protocol/app tests and15 desktop/profile tests pass for the current source.
Packing/admission/accounting changes are not yet installed. GPT-6 Instant/Medium/High/
Extra High actual tool loops pass; Pro inference is prohibited and was not sent.
GPT-5.6 remains deferred. Automatic/fault compaction under the new internal path,
multimodal input, hours-long load, physical reboot and phone layout remain separate
gates. Historical GUI qualification is not blanket internal-path acceptance.
Private proofs: ~/.cache/codex/acceptance/chat-internal-checkpoint-startup-wait.json,
chat-internal-discovered-tool-wrlab.json, native-account-catalog-comparison.json,
chat-final-concurrent-guardian.json, chat-final-app-acceptance-timing.json,
chat-final-medium-overlap-cancel.json and chat-final-installed-inventory.json.

## ChatGPT model errors: next work

Status: runtime repair unresolved; controlled diagnosis and adapter changes on
2026-10-14 are source-only. User-visible failure:
`stream disconnected before completion: error sending request`. Device stable51
and provider 0de70e3072a4b8e87a31/source 46db05e were inspected without changing
runtime, configuration, conversations or authentication. The installed protocol,
app transport, Responses server and profile source matched the maintained files
before this source change; installation remains unchanged.

Confirmed observations:

- At 13:26:26-13:26:27 UTC, the shared backend logs unknown model
  `chatgpt-desktop/gpt-6` and uses fallback metadata. Its fresh-thread request
  reaches loopback `/v1/responses`, which returns 400 `unsupported_desktop_request`.
  The error covers multiple admission checks; the exact rejected field is not
  captured. Do not assume a particular check failed from the generic code alone.
  Frontend picker visibility/provider selection does not prove that the backend
  loaded the intended model catalog and request capabilities.
  The no-inference `native_catalog_request_probe.py` now reproduces this boundary
  through the installed official Android backend in disposable homes: per-thread
  catalog override still produces unknown-model metadata and an unsupported routed
  `web_search` declaration (43,348 JSON characters in the owned fixture); the same
  catalog loaded at server startup produces an admitted request (16,894 characters).
  This proves a controlled mechanism, not the uncaptured historical rejected field.
  Actual wire bodies are42,543 and16,387 bytes respectively; the character counts
  above reserialize the owned JSON and are not the measured HTTP byte lengths.
  Production model lookup ignores the per-thread `ModelsManagerConfig.model_catalog`;
  frontend projection alone cannot repair the startup-scoped catalog.
- At 13:26:59 UTC, the existing long thread's pre-turn ModelDownshift compaction
  sends model `gpt-6.1-sol` to the newly selected loopback Chat endpoint. The HTTP
  send fails (`is_timeout=false`, `is_connect=false`) and the backend records
  `Failed to run pre-sampling compact`. Upstream previous-model compaction uses
  `with_model(previous_model)` while preserving the current provider; a provider
  transition therefore needs an explicit compatible compaction handoff.
- `desktop_responses_server.mjs` sets `maxRequestBodySize:2*1024*1024` before
  context staging. Two deliberately invalid local requests, which cannot reach
  Chat submission, return 400 at approximately 1 KiB and 413 above 2 MiB. The adapter
  proof file remains byte-identical. Active-history payload serialization before
  the failure totals 2,277,814 bytes since the last compact; this is not a measured
  HTTP body. The ceiling is a confirmed limitation and a plausible contributor
  to the send failure, not a proven explanation of that exact failed request.
  `desktop_server_admission_probe.py` reproduces both a header-only 413 and a
  BrokenPipeError while sending an oversized body. The missing historical body
  size remains unproved; the ceiling is unchanged pending handoff/cold-replay proof.
- The adapter is alive and its read-only readiness response verifies against
  its existing private binding. Zero admitted-request/submission counters do not
  mean zero HTTP arrivals: the observed 400 occurs before admission accounting.
  No Chat inference or chat-limit consumption experiment was performed. Nothing
  observed establishes a Chat message-limit, Cloudflare or login rejection.

Evidence remains private in `~/.codex/logs_2.sqlite` and the affected rollout's
metadata. Use the timestamps and error targets above to inspect narrowly; do not
publish prompts, tool results, credentials or raw conversation records.

Required follow-up, before claiming ordinary `/model` use is qualified:

1. Apply selected metadata at the official backend startup boundary while preserving
   native siblings' defaults and exact runtime ownership. Reuse Core's server path;
   do not add an executor/model service or disable tools for fallback metadata.
   Prefer a bounded catalog-specific startup scope to duplicating the upstream
   config loader or patching pristine Core. Qualify catalog content changes, reuse
   and retirement; a filename alone is insufficient and active siblings must survive.
   Honoring config.model_catalog during upstream lookup is a smaller upstream fix,
   not authority to source-build a parallel backend.
2. Establish a compatible old-provider compaction/new-provider resume boundary
   for long histories, preserving canonical history, permissions and sibling
   routes. Do not bypass the defect by deleting history or silently truncating it.
3. Measure actual serialized request bytes and handle the body ceiling explicitly
   before app context staging. Distinguish oversized input, unsupported shape
   and network failures in the user-visible error; increasing the ceiling alone
   does not solve route/catalog or compaction incompatibility.
4. Qualify fresh selection, long native-to-Chat and Chat-to-native switching,
   pre-turn compaction, restart/resume, sibling isolation and preserved native
   tools on the actual installed Android path. Earlier direct-profile tool loops,
   manual internal compaction and inference-free menu round trips retain their
   narrow scope; they are not proof for these failing combinations.

Runtime/catalog/compaction repair and real Chat inference remain deferred; this
source change supplies admission/packing/accounting groundwork below. Explain
trial necessity and expected submissions before performing real inference.
Chat message submission/limit accounting is a separate review, not a diagnosis
or resolution of these model-use failures.

## ChatGPT provider cost and debt reduction

The selected objective is to improve ChatGPT as a model inside local Codex with
the least maintenance and message-submission cost. tdev progress/control proposals
belong to a separate session; neither its runtime nor a new workflow controller,
display or autonomous agent is a prerequisite for this provider.

Current findings and source changes on 2026-10-14:

- An offline synthetic packet of 50,410 bytes (five tool definitions, 24 history
  messages and instructions) previously became27 stages:26 context ACKs and one
  work request. Ordered packing now produces2 stages: one context ACK and one
  work request. This evaluates the previous commit and current code on the identical
  fixture using only `desktopStages`; no Chat message was submitted and no
  quota was measured. Splitting every old message and issuing an empty tool-reset
  ACK separately are avoidable submission overhead, not a provider requirement.
- The signed app has `extraDeveloperInstructions`, `prependMessages` and a real
  tool-result continuation export (`bRt`/`Nwi`). These are investigation candidates,
  not acceptance. Its generated local function signatures are currently only
  Work/Codex `handoff` and are empty in Temporary Chat; `Jui` supplies no arbitrary
  local tool names. `toolOrigin` marks an SDK follow-up prompt, so changing the
  author label alone does not establish native tool continuation or quota savings.
- The installed payload already has one bundled wire bridge and one adapter.
  UI fallback still carries a second submission/retry/ownership implementation;
  noVNC/display control and numerous qualification sources are shipped alongside
  inference code. Keep necessary signed-app/X dependencies until independently
  qualified alternatives exist; a headless or token-extraction shortcut is unproved.

Source-only implementation packs whole schemas/history with one final work phase,
preserves oversized instruction/message fragments and explicit empty tool resets,
and refuses an oversized whole schema before submission. Existing warm deltas
and canonical history ownership remain. Fixed admission codes reveal no request
or error bodies; authenticated arrivals/rejections are distinct from admitted work.
Work submissions retain their counter; setup submissions are observed before ACK,
so an accepted setup with a failed ACK is still counted. Total observed submissions
separate app acceptance from weaker GUI observation; retries and completed ACKs
remain separate. None of these counters measures chat-limit deductions.

Remaining order; no runtime activation or real Chat submission performed:

1. Repair the recorded catalog/admission, provider-crossing compaction and body-size
   boundaries. Preserve ordinary upstream execution and one authoritative history.
   Do not hide incompatibility by dropping tool/context features, silently trimming
   history or forking pristine upstream Core.
2. Qualify implemented ACK packing/accounting through actual submission, preserving
   the first tool-list reset and warm deltas. Assess the app's native
   instruction/history parameters before assuming the old UI character limit is
   necessary internally. Preserve data roles, whole schemas and final-work separation.
   Offline replay/byte-boundary tests precede any real submit. Measure work, setup,
   tool-result, guardian, compact and retry submissions independently of quota.
3. Qualify a minimal native-Chat MCP continuation in the existing adapter/tunnel
   path, without tdev as executor or another agent. Chat-issued calls should become
   ordinary Core tool calls; actual Core results should satisfy the original MCP
   call while the same Chat turn continues. This is a transport replacement, not
   a script that invokes Codex recursively. Preserve all advertised native tools,
   reviewer/approval, cancellation, call identity and independent thread ownership.
   The app turn must survive an expected Responses tool boundary but stop on actual
   cancellation; account/plugin/Temporary Chat availability and MCP timeouts are
   explicit feasibility gates. Do not build a general broker before this tiny proof.
4. After acceptance, keep one inference transport, make GUI viewing optional and
   package reachable runtime dependencies separately from tests/history. Remove
   a replaced path only after its full affected acceptance. If native continuation
   is unavailable, retain the optimized text adapter with its explicit per-judgment
   submission ceiling rather than accumulating speculative compatibility layers.

The first real native-MCP proof would use one user message for two harmless Core
tool calls and a final answer, versus three work submissions through today's text
loop, plus any separately counted setup/review overhead. These are submission
expectations, not chat-limit deduction claims. Explain the exact candidate and
budget before real inference/provider changes; Pro inference remains prohibited.

Private no-inference proofs: `~/.cache/codex/acceptance/chat-stage-packing.json`,
`chat-catalog-request.json`, `chat-adapter-admission.json` and
`chat-debt-full-check.log`. HTTP proof covers five admission rejections,401 without
auth, the body ceiling, and zero admitted work/Chat submissions. Protocol/app tests
pass29/174 assertions and desktop/lifecycle tests pass15. This is not a real Chat
quality, ACK/cancellation observation or provider-switch qualification. User data,
credentials, active runtime and conversations are preserved.
`scripts/check.sh full` passes77 Python and323 Rust tests, formatting, Clippy and
diff checks; the separate Android HTTP/catalog proofs above cover the affected
real request paths without inference. An installed/live Chat gate remains open.

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
  trial exposed a separate missing bare-registration call, which blocks its
  `/switch` notification-return acceptance until the frontend is replaced.

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

- The installed native frontend still gates FD registration on TMUX/TMUX_PANE,
  despite Manager supporting bare bindings. Existing synthetic native workload
  path proofs do not cover this real-frontend call. The maintained patch now removes
  that condition, and its native child gate exercises both bare and tmux processes.
  Native compilation/tests/lint, qualification binding, signing/publication and
  fresh physical return across `/switch` are pending; stable47 is not evidence that
  bare real-frontend notification return is complete. `/title` rendering itself
  remains accepted by the user.

- Broader authorized normalization continues separately: genuine official-APK
  fallback execution, independent AGY/OpenCode
  notification hooks, isolated real ChatGPT Web acceptance and the final home/usr
  authority audit. Existing synthetic browser proofs do not establish account,
  model, tool-loop, cancel, compact or resume acceptance. Photos v3 cleanup90b8546
  is published: retired snapshots remain recoverable in Git, active data/source
  bytes are preserved and clean-tree packaging/service-smoke passed.

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
