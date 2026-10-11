# Operating state and next work

Implementation: rewrite/rust-core. Publication: main's immutable source caller.
AGENTS.md owns development rules; SPEC.md owns product behavior and boundaries.
This repository carries no external workflow skill or model configuration.

## Operating baseline

- Public and installed stable46: local-hosted-0-161-0-8ba5997cb723-s46,
  CLI0.161.0, accepted implementation8ba5997. Publication38115752305 passed all
  eight jobs: Android build/smoke, signing, immutable Release, Pages prepare/deploy,
  complete public readback/disposable update and non-forced stable promotion.
  main callerb2a4ad2 pins that exact reusable producer/source; histories stay separate.
- Source acceptance38115440009 passed all four jobs, including actual Android Core
  execution and locked workspace gates. Local full acceptance passed Core193
  (one pre-existing ignored), Manager42 unit +55 integration +three public path
  gates, release-builder25, Python18+59, fmt/Clippy/diff checks.
- Initial public device update hit the existing300-second large-payload transfer
  limit and fell through to local-derived update, whose same-version frontend
  boundary refused it; stable45 remained intact. This was a transport/fallback
  failure, not rejection of the paired signed stable46 release. No trust boundary
  was weakened to activate it.
- Device activation used the regular signed `codex update --local` path. The admitted
  local public key verified manifest and candidate index signatures. Five identical
  large/helper files were copied from stable45 against the signed inventory,
  generation.meta came from the immutable Release, and only Core/Manager payloads
  needed downloading. All eight file hashes/modes verified before activation.
  stable45 is now the bounded rollback target; older busy images remain retained.
  A second ordinary `codex update` reported already up to date with no change.
- Core, Manager and upstream doctor report healthy with exit0. The pre-existing
  upstream duplicate-thread warning remains; protected conversation data was not
  rewritten. Installed launch capability and focus=window report correctly.
  Other protected files2050 and all eleven pre-existing installed process PID/start
  and image identities were unchanged. Only the intended AI module changed.
- /switch frontend rawdcf76b3c/adaptedcd4a6619, official source97901140.
  Native27/lint/pristine Core qualification38032317709 at29b2584 remains applicable:
  all six maintained frontend inputs are unchanged. New launches use the new signed
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
- User tapped all four fresh Manager return notifications. A/B native bare and
  hidden tmux C recorded exit0, selected=true and count5; D's seeded stock named
  route returned exit0/count5 on the custom APK. D proves named-service reuse,
  not an installation of an official APK. All four owned workloads exited normally;
  user-requested cleanup restored baseline count1 and removed the fake install trees.
  Private proof: ~/.cache/codex/acceptance/window-device-88b0a09.json. This return
  proof remains applicable to8ba5997: the follow-up only removes nested construction;
  fresh launch, binding and focus code is unchanged. Actual Android signed installed
  Manager also refused all three nested entrypoints before any state creation.
- A fresh notification is required for each trial: Android consumes it on one tap.
  Never ask for a second tap on an already consumed notification. Title display
  after the next ordinary production launch remains a separate user observation.

## Source and installed AI

- AI maine4bbeba includes the qualified Manager adapter5caf2e8 and same-terminal
  refusal. Both user-composed and clean publication trees passed full verification38
  with zero warnings/failures; TUI166 and affected native tests pass. Temporary-index
  commits excluded unrelated profile/recovery changes and kept all user working bytes.
  Explicitly approved main pushes completed normally.
- A minimal install-script request was automatically rejected for concern about its
  unrelated directory-deletion branch. The safe replacement installed ai_tmux.py
  alone with an atomic file replacement; all other seventeen library files and AI
  launcher retained their hashes. Installed provider list and nested refusal pass.
  Existing profile/auth recovery behavior is preserved in the composed live module.
- App source remains the separate formal ~/prj/termux repository, with official Git
  history and read-only upstream. AI has no competing Codex profile/window authority.

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
