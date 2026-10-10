# Operating state and next work

Implementation: rewrite/rust-core. Publication: main's immutable source caller.
Development rules: AGENTS.md; product contracts: SPEC.md. This record is local
to this repository and carries no external workflow or model configuration.

## Operating baseline

- Public/installed stable44: local-hosted-0-161-0-05e7504a905b-s44, CLI0.161.0,
  accepted source05e7504. Successful publication38056887993 includes actual
  Android build/smoke, signing, Release, Pages, complete public readback,
  disposable current-to-candidate update and non-forced stable promotion.
  Device wrapper/Manager doctor healthy; ordinary second update is a no-op.
  Stable43 remains the installed rollback generation. Large direct transfer
  failed on device; signed --local installation reused6 exact-matching files
  and downloaded2, with complete inventory verification before activation.
- /switch frontend rawdcf76b3c/adaptedcd4a6619, official source97901140.
  Native27/lint/pristine Core: successful qualification38032317709 at29b2584.
  Its six maintained inputs are unchanged by this development-plane revision.
- User confirmed two notification taps return original1 without window growth;
  two installed focus calls also reuse that client. Account/config12 hashes
  remain unchanged after deployment;9 existing Codex processes remain alive.
  Existing preview clients/assets remain protected until exit.
- New ordinary launches use signed-generation helpers/2. Initial AI registration
  interoperability is accepted at0963635/7ad8479; AI owns its own source/policy.

## Development-plane acceptance

- Product source153e4f0: source acceptance38051226302 successful; cache refinement
  e205875: source acceptance38051413777 successful. Local Rust309 passed,
  1 pre-existing ignored; Clippy/fmt green. Python migration/check/scope18 and
  hosted59 passed; workflow syntax/action pins and scope/cache/failure proof pass.
- maincd15746 publishes stable44 through the exact05e7504 reusable workflow
  and source, pinned by caller1eb9712. Duplicate main specs, goal/workboard
  and producer implementation are removed.
- Unsigned real caller execution38051722501 passed actual Android build and
  executable smoke (publish=false, rebuild_current=true). Signing and all public
  mutation jobs were skipped; signed stable index/signature remain unchanged.
- Repository-local rules/state and scope-test revision9865efe: successful scoped
  source acceptance38052220630; Android/workspace builds correctly skipped.
  CURRENT.md replaces the former goal record with no external skill dependency.
  Unchanged runtime/native proof above is reused, not presented as a new run.

## Independent launch correction

- AI source7df68b9 removes reuse of the shared humtr-ai session for outside-tmux
  launches. Each fresh launch owns a separate session/native return name. Inside
  tmux, launch targets the invoking TMUX_PANE's exact session, not another client.
- The old implementation reproduced the existing-session selection change.
  Regression proves two independent sessions/workloads/return names, preserved
  legacy selection, service failure without replay, repeated attach reuse, exact
  inside-session targeting and missing-pane refusal. Real isolated PTY clients
  keep independent screens; closing one leaves the other unchanged.
- AI final verification38 passed, zero failures/warnings; focused committed-source
  gates also passed. The installed tmux module passed5 registration/client checks.
  Only that module was replaced through the bounded AI installer;31 other runtime,
  launcher, account/config files and the live tmux pane/selection snapshot match.
  Existing uncommitted AI development is preserved and excluded from this commit.
- Codex installed/public stable44 is unchanged; this correction belongs to AI.
  User device observation disproves initial-terminal acceptance: fresh launch
  opens a new Android terminal rather than the invoking one. Source7df68b9
  fixes cross-launch overlay only. Named service-window reuse does not prove
  automatic registration of an existing unnamed terminal; that remains unresolved.

## Origin-bound design decision

- Connection count/order is not routing authority. SPEC defines the proposed
  workload-to-origin invariant separately from the installed name-based path.
  Launch stays in its invoking terminal; notification selects an existing bound
  origin only. tmux/conversation identities and display labels cannot address
  Android windows. Core remains outside terminal control.
- Exact installed Termux source8629e63 matches the inspected Service and terminal
  client source. The native session table/internal handle lookup is established;
  a callable resolve-origin/focus-existing interface has not been qualified.
  Minimal native exposure is a candidate, not an implemented or deployed result.
- Next work is qualification of that actual capability plus A/B native-terminal
  launch/return proof. Do not replace initial dispatch with direct attach and call
  the full requirement complete, or cycle back to create-or-reuse registration.
  This design revision makes no runtime, running-session or Termux APK changes.

## Product observations still open

- First-launch use of the invoking Android terminal plus exact notification
  return is unresolved. Removing the initial service call alone would leave the
  original-terminal registration gap; do not call that a complete correction.
  User confirms earlier initial execution stayed in the invoking terminal,
  while later launches reused that first terminal. The exact earlier routing
  branch has not been established; do not infer manual setup as its cause.
  Preserve independent workload isolation while resolving this actual device path.
- /title display after next normal launch remains a separate physical observation.
- Retire installed preview assets only after remaining native clients exit.
- Current APK cannot select an arbitrary originating bare terminal. App/current
  terminal return and named tmux reuse work; notification taps do not start work.

## Origin-bound implementation candidate

- Source implements `codex termux tmux`: interactive existing-profile picker,
  arrow/Enter selection, explicit profile/native-argv separation, direct caller
  attachment, independent outside sessions and exact inside-session targeting.
  No AI installation or Android window constructor is needed to launch.
- Manager owns `__terminal-focus-v1 UUID`, qualifying the actual native FD/pane
  identity and selecting only a validated existing Termux origin. Notifications
  no longer depend on HOME/bin/ai or constructor/Activity fallback in exact mode.
- Local Core192 passed/1 existing ignored; Python18+59 passed. After correcting
  two superseded AI-action assertions, Manager40 unit +55 integration +2 public
  tmux/native-FD paths passed. Release-builder acceptance and workspace Clippy
  pass. Core proof uses unchanged inputs; no repeat after test-only corrections.
- AI experimental origin client e1245e9 (experiment/origin-terminal) passed full
  isolated verification38 and focused protocol, direct-PTY, title/native-FD,
  color/status and runtime-qualification checks. Old window-construction tests
  and helpers were removed. Original AI worktree/user changes remain untouched.
- Native Termux candidate f10d4ec, build38086750520, pins installed source8629e63.
  Actual APK build and3 focused source tests passed; package com.termux/code118,
  apt-android-7 and certificate match the installed APK (local keytool comparison).
  It reports real selected-view match/count, and refuses expired/closed handles.
- Reviewable APK: /sdcard/Download/termux-origin-candidate/termux-origin-f10d4ec.apk
  SHA256 c389b4b5e6e441e381787f0d2d3d3d8813a698ea6c7590b163274e496698e005.
  Original APK is retained beside it. User confirmation for app update is pending
  because current jobs may stop; no APK or installed wrapper cutover has occurred.
- Closure still requires native app installation, real A/B terminal launch and
  notification return with stable counts, then qualified wrapper deployment.
  Source/PTY/build evidence is not a claim that this Android gate has passed.
