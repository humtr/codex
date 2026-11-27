# Active work order — Codex TUI presentation Phase 1

Prepared 2026-11-27 at the user's request (plan correction and readiness).
Authority: `AGENTS.md`, `SPEC.md`, `docs/INDEX.md`,
`docs/spec/CHATGPT_PROVIDER.md`, and [ROADMAP §11](docs/ROADMAP.md#11-deferred-codex-presentation-improvements).
This is the single active preparation work order, not a new product contract.
**State: ISOLATED SOURCE CANDIDATE CORRECTED AFTER INDEPENDENT REVIEW; final independent review and live/physical gates remain open.**
The previous Native-return work order is preserved byte-for-byte in Git
(`git show 3f12f956:HANDOFF.md`), not marked completed. Its replacement
route and unresolved acceptance risks are recorded in
[ROADMAP §10](docs/ROADMAP.md#10-chatgpt-provider-native-tool-return-and-structured-context).
No replay or new settlement attempt was performed.

## Pinned preparation baseline

- `main` refreshed at `e8fa37af` (initial preparation snapshot at
  `3f12f9567dc84bcd30a93e88e1ab15fef902bf81`). Active `~/.codex/ais.config.toml`
  and `~/.codex/chatgpt.config.toml` remain outside the repository (stale
  repo-root copies removed). Recheck ownership and diff before implementation.
- Official Wrapper `$PREFIX/bin/codex`: SHA-256
  `339ab995b4195b96b79213fd1655483bd77254a27906df8999822a558fe7ba0a`.
  `codex-cli 0.172.0`, selected `releases/0.172.0-faf17b`; adapted
  upstream executable SHA-256
  `1775f7912f595d840ef53acb56e5552d06cbecb2333f8bd483aaa1cbd059048d`.
  Public `rust-v0.172.0` tag peeled to
  `3d86e43b075adeba2193ec5c22ac92f55afd82e6` by read-only ls-remote;
  current-version upstream feature inspection and real PTY remain open.
- AppHost selected `26.1120.50957`. Provider selected/running
  `0.1.0-368a1e0d` (`sequence=53`, PID `2497`, actual executable SHA-256
  `bbf84d4caf459dcd7fb04f80d33b125c149dad18ec85aee0ae8621b904c0f003`);
  `termux-shell-mcp` selected/running `0.1.0-eb19e206` (`sequence=16`, PID
  `29393`, SHA-256
  `a8ac379adee837816f868ea6f2992144c648270688f6be98ad1faac9910cccf2`);
  Orch selected/running `0.1.0-5a865216` (`sequence=25`, PID `10032`, SHA-256
  `2e22ee1cbac82f06eae54ef78089a0560046ea31e3d0002f1d567f68887763e2`).
  Provider health/status returned `ok=true`, `host_running=true`,
  `auth_state=authenticated`, `completion_ready=true`.
- Private synthetic evidence: `$PREFIX/tmp/tui-presentation-prep.K8eknh/`.
  No existing audit artifacts modified. `presentation.rs` standalone tests
  4/4 PASS; `providers/chatgpt/tests/fixtures/presentation_preflight.rs`
  oracle **2/18 PASS, 16 FAIL, exit 1** (re-verified against `e8fa37af`).
  The oracle is a standalone qualification fixture, not a failing automatic
  Cargo integration test. See `unit.log`, `baseline-regressions.txt`,
  `oracle.log`, and `source-sha256.txt`; commands and acceptance details are
  in ROADMAP §11.

## Execution order

| Gate | State / next action |
| --- | --- |
| P1-00 — baseline/scope | Prepared. Recheck checkout/ownership and update relevant Provider annex before observable behavior changes. |
| P1-01 — regression oracle | Initial 18-case fixture ready; preserve expected failures until fixed, extend historical/current PTY corpus. |
| P1-02–05 — semantic projection | Candidate implemented: shared parser/projection, protected Markdown, supported local literal data/conditions, native GFM/chart data and static controls; unsupported dynamic expressions and geometry remain explicit. |
| P1-06–07 — integration/regression | Candidate integrated with the existing encoder; automated continuity, machine-output and prefix gates pass. |
| P1-08 — qualification | Local source pin, Rust/JS/host/docs, isolated CLI/Provider and official synthetic PTY observed; live candidate AppHost/account lane, physical UI, terminal reconstruction, clipboard and natural TUI exit not qualified. |
| P1-09 — closeout | Independent review pending; preserve applicable open §10 risks. Publication/readback/activation/rollback require separate authorization. |

## Implementation boundaries

- Phase 1 only: standalone `providers/chatgpt` renderer/encoder and
  relevant tests/docs. Preserve official upstream Codex, Wrapper/ELF
  adaptation, cdx, all MCPs, AppHost files and selected/running services.
  Do not restore local-tools or introduce a browser viewer, custom TUI,
  additional executor, conversation store or separate stream state machine.
- Current owner path: `page_driver.js` → LaneMailbox → TurnAssembly →
  ResponsesEncoder → presentation projection → SSE/JSON → upstream Codex.
  Request classification in `responses.rs`; binding in `turn.rs`/
  `session.rs`. Utility output, raw AppHost history, reasoning, tool
  payloads, identity and settlement must stay intact.
- Preserve original unresolved Native/MCP receipts. Incomplete prior
  qualification does not forbid isolated presentation work and must not
  be silently treated as passed. No retries, new call IDs or re-created
  work to conceal ambiguous effects. Any intersecting acceptance
  blocker must be resolved or explicitly recorded before promotion.
- Candidate tests use owned disposable roots, loopback fixture data, and
  no operating account/profile. Recheck current upstream/host before live
  tests; live inference using an operating account, switching services,
  signed publication and Phase 2 are not authorized by preparation.
- Source candidate evidence is recorded below. Do not claim full Phase 1,
  physical TUI, authenticated AppHost or release qualification from synthetic
  evidence. Do not switch the running generation.

## Candidate implementation evidence (2026-11-27)

- Work: `work_78a0bb522ddfa34b`, isolated branch
  `phase1/tui-work_78a0bb522ddfa34b`. Candidate includes the concurrent
  main update `ef02c3fe`; main itself was not changed by this Work.
- Initial W1–W3 authentication failures and W4/W5 timeout failures are
  retained in the orchestration ledger. W7 and W8 returned independently
  scoped parser/renderer edits. W6 qualified the exact upstream source and
  installed Wrapper using isolated fixture-driven PTYs.
- Original 18-case oracle 18/18 (only Cargo test-entry annotation added);
  additional 23/23 groups, 48/48 deterministic malformed UTF-8-prefix cases.
  Legacy rendering tests restored; obsolete silent-omission assertions now
  require explicit unknown markers. Both oracles run under Cargo.
- `$PREFIX/tmp/p1-final-gates.IRbIKI/`: fmt, strict clippy, locked
  offline Rust under umask 0022 and 0077 (53 main tests + two 14-test
  oracle targets passed each), JS 17/17, host 8/8, doc links, standalone
  oracles. The ignored official CLI integration test was run separately
  against the candidate loopback MockDriver and passed (native-provider.log).
- `$PREFIX/tmp/phase1-independent-w-TmbIar/`: official 0.172.0 source
  and isolated default-daemon/no-daemon PTY, streaming, transcript and
  resume probes. `$PREFIX/tmp/p1-candidate-pty.U2bNRc/`: actual
  candidate-projected fixture streamed through installed official Wrapper
  at 100/80/48/32 columns (end marker at all widths). These are raw/stripped
  PTY observations, not a reconstructed or physical screen. Copy remained
  unconfirmed and the interactive processes were harness-terminated
  (wait status 15); do not count these as clipboard or clean-exit passes.
- Scope limitations: deterministic literals, local bindings and explicitly
  supported primitive operations only; unknown iteration/external bindings,
  unsupported expressions and interactions are static/unavailable. A
  128-level nesting guard retains the deep literal region with an explicit
  marker; 128/512-depth and ~100 KiB cases were exercised. No arbitrary
  runtime, external retrieval or invented chart values/geometry.
- V1 review of 8d268653 was inconclusive. W9 independently reran all
  12 gate commands (exit 0, $PREFIX/tmp/p1-independent-rerun.MFaNTi).
  W10/W11 independent review found parser and encoder/continuity defects.
  W12/W13 repaired disjoint parser/renderer paths in isolated worktrees;
  Lead integrated and simplified the table traversal, corrected visible
  answer settlement through page_driver/assembly/encoder, normalized
  compaction bypass, and persisted failed-publication receipts using the
  existing ThreadBinding. Executable regressions cover compatible raw
  rewrites, incompatible revisions/removal, durable readback, replay
  rejection and exact-prefix continuation. Original oracle unchanged.
  The older PTY evidence applies to the earlier candidate, not this
  corrected source. Numeric-dollar/prose and identifier-adjacent markup
  use a conservative grammar; genuinely ambiguous literal markup may
  remain buffered and unresolved expressions stay explicit.
- No authenticated live candidate AppHost, Android physical UI, signed
  package, publication, activation or rollback test is authorized or
  qualified by this candidate. Independent verdict remains necessary.
