# Active work order — Codex TUI presentation Phase 1

Prepared 2026-11-27 at the user's request (plan correction and readiness).
Authority: `AGENTS.md`, `SPEC.md`, `docs/INDEX.md`,
`docs/spec/CHATGPT_PROVIDER.md`, and [ROADMAP §11](docs/ROADMAP.md#11-deferred-codex-presentation-improvements).
This is the single active preparation work order, not a new product contract.
**State: READY FOR CANDIDATE IMPLEMENTATION; product source not edited.**
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
| P1-02–05 — semantic projection | NEXT candidate work: one source-aware parser/projection, protected Markdown, literal data and conditional state, native GFM/relations/chart data, truthful static controls and explicit unavailable fallback. |
| P1-06–07 — integration/regression | Pending: current `stream.rs::ResponsesEncoder::flush_ready`, utility isolation, incremental/settled equivalence, emitted prefix and resume/compaction, no MCP side effects. |
| P1-08 — qualification | Pending: exact 0.172.0 upstream feature inspection, Rust/JS/CI, real official TUI default-daemon/diagnostic isolated PTY, actual candidate AppHost/account lane and physical UI separately. |
| P1-09 — closeout | Pending: review docs/source, applicable open §10 risks, signed candidate publication/readback/activation/rollback only as separately authorized. |

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
- Stop scope at this preparation checkpoint; source implementation is the
  next task. Do not claim Phase 1, physical TUI or product qualification
  completed from the current synthetic failures.
