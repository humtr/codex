# Current Operating State and Verified Evidence

[SPEC.md](SPEC.md) and its annexes (`docs/spec/*`) own required behavior;
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) maps what the current codebase
implements; [docs/ROADMAP.md](docs/ROADMAP.md) tracks deferred Stage 2 work;
[docs/INDEX.md](docs/INDEX.md) defines document authority and status vocabulary.
This file records **current live state, active open work, and recent verified
evidence (newest-first)**. Earlier 2026-11-16 – 2026-11-18 Stage 1 checkpoints
and retired work orders are archived in
[docs/history/STAGE1_MILESTONES.md](docs/history/STAGE1_MILESTONES.md).

---

## Current Live Operating Snapshot

Observed on 2026-11-25. The latest targeted Provider preflight below
supersedes the earlier same-day Provider and tunnel snapshot; other component
qualification receipts have not been independently refreshed in this preflight.

| Boundary | Observed State | Notes |
| --- | --- | --- |
| **Published and installed Wrapper (`$PREFIX/bin/codex`)** | Stable Ed25519-signed `wrapper` tag at `f49e1117` (`sequence=2`, key SHA-256 `0df170da...`). Published archive SHA-256 `c9368338...`. Installed `bin/codex` SHA-256 `f83899d6...` | Signed `SHA256SUMS.sig` readback, `install.sh` CI ([run 45449340483](https://github.com/humtr/codex/actions/runs/45449340483)), and live Termux `--require-signed --all` install matched on 2026-11-25 |
| **Active / rollback upstream generation** | Active `0.171.0-d8c43b-2` (`ino=9741792`, SHA-256 `26d9f90b...`). Rollback `0.170.0-3f7e1a-2` | Preserved across signed Wrapper & `install.sh` cutover; `codex wrapper status --verify --json` (`qualified=true`) and `codex doctor --json` (`summary=healthy`) agreed |
| **Live managed daemons (`default` / `janmori101` / `szob`)** | All three homes aligned to `$HOME/.codex/packages/standalone/current`; `janmori101` & `szob` running | `cdx profile list` sees all 8 profiles (`default` + 7 normalized `codex-manager-profile` homes) |
| **Signed rollout snapshot — doctor and external services** | `codex doctor --json` reports `summary.status=healthy`. All six Stage 2 `runit` services (`chatgpt-provider` PID `10705`, `termux-shell-tunnel` `8182`, `termux-codex-tunnel` `8485`, `termux-chatgpt-tunnel` `9191`, `termux-orch-relay` `9581`, `termux-orch-tunnel` `9687`) verified active with `/proc/<pid>/exe`, child MCPs, and `/readyz`/`/healthz` | The retired `0.167.2` archive remains in place with protected account/session data; all `previous` rollback generations remain intact |
| **Stage 2 signed rollout (Slices 1–7 + streaming redesign)** | **Released and verified at signed cutover in the unified XDG `codex/` namespace (`0.1.0`)**: `codex-ext` (`sequence=11`, `0c6de280`, SHA-256 `4a1c2ac3...`); `tunnel-client` (`sequence=5`, `v0.0.21-0c6de280`, `04c52620...`, prev `v0.0.21+f3c9334e`); `termux-shell-mcp` (`sequence=8`, `0.1.0-0c6de280`, `eeb67e96...`, prev `0.1.0+d3d24d11`); `termux-codex-mcp` (`sequence=5`, `0.1.0-0c6de280`, `abc0b04a...`, prev `0.1.0+f3c9334e`); `host/chatgpt-desktop` (`26.1117.41037`, GPG-verified) + **`chatgpt-provider`** (`sequence=14`, `0.1.0-921cbb9e`, `890c7847...`, prev `0.1.0-66f4ce05`); `termux-chatgpt-mcp` (`sequence=5`, `0.1.0-0c6de280`, `e7dae7e2...`, prev `0.1.0+f3c9334e`); `termux-orch` (`sequence=10`, `0.1.0-0c6de280`, `89d10069...`, prev `0.1.0+7b+d3d24d11`, `@TermuxOrch` connector `asdk_app_6b03de61dd748191bf56fef2b1029a2e` `ACTIVE`) | All 8 installed binaries match their signed GitHub Release archives; all 4 MCP tunnels return `200 ready` on `/readyz`, `termux-orch-relay` returns `ok:true` on `/healthz`, and `chatgpt-provider` passes live `gpt-6-2` `xhigh`/`max` streaming & `model-catalog.json` (8 latest-first families, `supports_reasoning_summary_parameter=true`) checks |
| **Milestone records** | Stage 1 WP0–WP6 and Stage 2 Slices 1–7 are archived | [Stage 1 milestones](docs/history/STAGE1_MILESTONES.md), [Stage 2 milestones](docs/history/STAGE2_MILESTONES.md) |


**Targeted refresh (2026-11-27):** the table above retains the earlier
2026-11-25 snapshot for components not refreshed here. The selected upstream
is now `releases/0.172.0-faf17b` (`codex-cli 0.172.0`). Signed live generations
now active include Provider `0.1.0-449b061c` (`sequence=60`, PID `12516`)
and `termux-orch` `0.1.0-449b061c` (`sequence=31`, relay PID `12518`,
tunnel PID `12515`, MCP child PID `12545`). Both retain
`0.1.0-71078da1` as previous. The selected Shell MCP is still
`0.1.0-eb19e206` (`sequence=16`); unrelated services were not restarted.
The Wrapper remains the signed sequence-160 binary at `612d2261`.
Actual target executable SHA-256 values match the signed rollout records below.
AppHost selected `26.1120.50957`; Provider health/status reports `ok=true`,
`host_running=true`, `auth_state=authenticated`, and `completion_ready=true`.
The earlier local E01/Thin Option B activation is historical, not the active
route. This refresh did not independently requalify unmodified components.

**Targeted refresh (2026-11-28):** Provider and Orch now select the signed
`0.1.0-4a030b2f` generations (sequences `64` and `35`). The running Provider
(`30670`), relay (`30668`) and Orch MCP child (`30693`, tunnel `30669`) match
their signed binaries. Both `previous` links now select `0.1.0-e6f24ca2`.
Wrapper bytes, upstream activation state, protected directory identities and
the three unrelated tunnel processes were preserved. See the 2026-11-28
cutover receipts below.

### Unverified / Remaining Boundaries

- **Provider presentation Phase 1 (2026-11-27).** The signed Phase 1
  implementation is active and the scoped Work is completed after independent
  verification. Physical screen reconstruction, native clipboard, natural
  interactive TUI exit, and an actual rollback switch remain unqualified.
  Phase 2 remains a separate authorization. See
  [ROADMAP §11](docs/ROADMAP.md#11-deferred-codex-presentation-improvements)
  and the release receipts below.
- **Retired Native-return transition / active MCP continuity.** Old
  pending-call migration and uncertain-effect reconciliation, remote MCP
  approval boundary, latest CLI/TUI/app-server and physical UI qualification
  remain separate open gates. Preserve the original work order in Git
  (`3f12f956:HANDOFF.md`); do not re-enable the retired route or infer PASS.
  [Remaining verification](docs/ROADMAP.md#10-chatgpt-provider-native-tool-return-and-structured-context)
- **R-2.** The live two-account handoff via daemon RPC preserved the same UUID
  and an unrelated active source turn. The installed launcher passed an
  authenticated same-UUID switch with an idle source and rejects a switch
  while another source turn is active. Concurrent handoff through the
  launcher remains open.
  [Verification criteria](docs/ROADMAP.md#7-follow-up-verification-session-handoff-and-session_indexjsonl)
- **R-3.** Termux `/copy` is accepted. External `codex remote-control` client
  pairing is tracked as long-term verification.
- **R-4.** A new Termux-visible process map scan found no references to the
  retired Provider archive. It still holds protected account and session data
  and is retained.
- **R-5.** The manual read-only canary passed for official Codex `0.171.0`,
  including a 37-step Termux run. The physical-device promotion gate remains
  deferred.
  [Phase 2.6](docs/ROADMAP.md#3-phase-26--read-only-upstream-discovery-and-qualification)
- **R-6.** Installer recovery after an Android Termux force-stop passed on the
  API 37 device with 4 KiB pages. Physical power removal and execution on a
  16 KiB page-size device remain long-term checks.
- **Slice 7 redesign promotion.** The user accepted the candidate under a
  provisional assumption for the remaining authenticated scenarios. Those
  scenarios retain their current evidence status and move to long-term
  verification. The signed generation is active.
  [Long-term Slice 7 checks](docs/ROADMAP.md#slice-7-long-term-authenticated-checks)

---

## Recent Verified Evidence (Newest-First)

### 2026-11-28 — Signed MCP short-wait, pre-validation, receipt pruning, and brief Provider pacing

- **Source and publication.** `4a030b2f` added bounded MCP short-wait (up to
  3.5s after a live relay accepts `/wake`, falling back to `queued`/`processing`),
  local pre-validation before `requests/pending/` admission, newest-512 pruning
  on `requests/finished/`, and single-pass brief admission pacing (`<= 1500ms`,
  waiter cap `< 4`, `waited` counter) in `providers/chatgpt/src/page_driver.js`,
  with normative updates in [docs/spec/ORCHESTRATION.md](docs/spec/ORCHESTRATION.md).
  Both signed workflows passed: [Provider run 46096668824](https://github.com/humtr/codex/actions/runs/46096668824)
  · [Orch run 46096671657](https://github.com/humtr/codex/actions/runs/46096671657).
- **Local gates.** `mcp/termux-orch` and `providers/chatgpt` passed `cargo fmt --check`,
  locked `clippy --all-targets -- -D warnings`, and both `umask 0022` and `0077`
  `cargo test --locked` (Orch 13 unit + 17 E2E = 30 per umask; Provider Rust 93
  pass + 1 ignored per umask), `node --test providers/chatgpt/tests/page_driver.test.mjs`
  (21/21), and `.github/scripts/check_doc_links.py` (24 tracked documents).
- **Live cutover.** Installed via `./install.sh --require-signed --release wrapper --components chatgpt-provider,termux-orch --skip-upstream --no-setup`
  after stopping the three target services; `previous` retains `0.1.0-e6f24ca2`.

  | Running process | PID | Signed sequence | Executable SHA-256 |
  | --- | ---: | ---: | --- |
  | `chatgpt-provider` | 30670 | 64 | `b3772cc9a64600cbed0b162f5e07b78d4ce5edb82d475773b60774a5db229b07` |
  | `termux-orch-relay` | 30668 | 35 | `f56ba832b89ac13bf2126630812a9fea7d7f39c6eeae51ce839ecce0ec5a5ece` |
  | `termux-orch` MCP child (tunnel `30669`) | 30693 | 35 | `f56ba832b89ac13bf2126630812a9fea7d7f39c6eeae51ce839ecce0ec5a5ece` |

  All `/proc/<pid>/exe` digests match installed signed binaries; Provider
  `/control/status` reports `auth_state=authenticated`, `completion_ready=true`,
  `pinned_send=true`, `rate_limiter.{read,send}.waited=0`; relay `/healthz`
  reports `ok=true`, `rate_limit_hits=0`, `local_rate_limit_hits=0`; all 4 MCP
  tunnels report `200 ready`.

### 2026-11-28 — Signed relay-owned orchestration and Provider admission limits

- **Source and publication.** `e6f24ca2f682417f69af77838ae2fc9bc00f1fd3`
  moved owner calls out of MCP request handling. MCP persists requests and
  exposes operation receipts. Relay owns remote dispatch and cooldown. The
  Provider applies shared request budgets and signals relay on tracked turn
  settlement. Both signed workflows succeeded. Independent readback verified
  the peeled tags, stable release metadata, Ed25519 manifests and downloaded
  archive digests. [Provider run 46087532939](https://github.com/humtr/codex/actions/runs/46087532939)
  · [Orch run 46087532413](https://github.com/humtr/codex/actions/runs/46087532413)
- **Local gates.** Both workspaces passed formatting and locked Clippy with
  warnings denied. Both umasks, `0022` and `0077`, passed 28 Orch tests and
  93 Provider Rust tests per run. The existing device-dependent Provider test
  stayed ignored. All 20 page-driver tests and the tracked Markdown link
  validation passed. Compiled fixtures covered blocked owner I/O, restart
  without replay, authenticated wake requests and duplicate call IDs.
- **Live cutover.** Stopped the Provider and both Orch services, used
  `./install.sh --require-signed --release wrapper --components chatgpt-provider,termux-orch --skip-upstream --no-setup`,
  and brought the three services up. Hashes of the running executables
  through `/proc/<pid>/exe` matched the independently verified bundles.

  | Running process | PID | Signed sequence | Executable SHA-256 |
  | --- | ---: | ---: | --- |
  | `chatgpt-provider` | 7972 | 62 | `08058c30ea75b324749a4b63656152ad97c64aed716e858cef86154f845d7384` |
  | `termux-orch-relay` | 7974 | 33 | `fb6a5b9dc103f410d1278c1be29cc5082ba58bc32ec39063224aceb1cbf1199b` |
  | `termux-orch` MCP child of tunnel PID 7973 | 7997 | 33 | `fb6a5b9dc103f410d1278c1be29cc5082ba58bc32ec39063224aceb1cbf1199b` |

- **Live readback.** Provider `/control/status` reported authenticated,
  `completion_ready=true` and the configured `rate_limiter` (read 5 burst,
  0.5/s, 25/minute and send 3 burst, 0.2/s, 10/minute). One direct project
  lookup returned the same ID and name as the list entry. Relay `/healthz`
  reported `ok=true`, `rate_limit_hits=0`, `local_rate_limit_hits=0` and no
  tick errors at observation. All four MCP tunnels returned `200 ready`.
- **Preservation and scope.** The installer replaced the Wrapper executable
  with identical signed bytes (SHA-256
  `339ab995b4195b96b79213fd1655483bd77254a27906df8999822a558fe7ba0a`).
  The upstream active `0.172.0-faf17b` and rollback `0.171.0-d8c43b-2`
  remained in the same activation record. Unrelated tunnel PID and start
  identities and the legacy archive, Codex home and AppHost profile directory
  identities matched their preflight values. Existing rollback generation
  files were retained. This rollout exercised authenticated reads and live
  service readiness. Model dispatch, a live completed-turn wake, and
  rate-limit rejection under production traffic remain unverified on the
  running generation.

### 2026-11-27 — Signed `front_marker` 429 storm & early `sendMessage` cooldown fix

- **Exact source:** `449b061c0b57874f74534202c0ac4a95fe3158a0` on `main`.
  Bounded `termux-orch` `resolve_front_chat_by_marker` to check up to 5 recent
  chats in the bound `project_id` + remaining budget up to 8 total from
  top-level chats (down from 120 top-level-only reads) and abort immediately on
  `backend_http_429` or active `OwnerGate` cooldown. Guarded
  `providers/chatgpt/src/page_driver.js` `sendMessage()` with an early
  `retryAfterUntil` cooldown check before `/me` and account-verification calls
  so cooldown returns `backend_http_429` instead of `pinned_account_unavailable`.
- **CI and publication:** [signed Provider release](https://github.com/humtr/codex/actions/runs/46005744266)
  and [signed Orch release](https://github.com/humtr/codex/actions/runs/46005751252)
  passed against `449b061c`. Installed via `./install.sh --require-signed --release wrapper --components chatgpt-provider,termux-orch --skip-upstream --no-setup`
  after stopping the three target services cleanly; previous pointers retain
  `0.1.0-71078da1`.

  | Active target generation | Sequence | Executable SHA-256 | Live process |
  | --- | ---: | --- | --- |
  | `termux-orch 0.1.0-449b061c` | 31 | `72d50921f8a431cbb7a2e634b13f6e12c381d1c5a256a6bcca669261edb7f0c0` | relay 12518; MCP 12545 (tunnel 12515) |
  | `chatgpt-provider 0.1.0-449b061c` | 60 | `0fe104db6b6dbe736b56559e6687106510d709e54f93ea3276ef5697cacf04f6` | 12516 |

- **Live device state repair & checks:** Rebound persisted `humtr/codex`
  project scope from stale `"Wrapper"` (`g-p-6b044ac30e8c8191bf4afa1fbda4603c`)
  to `"Codex"` (`g-p-6b075b896c6881919d2575fb599973ff`) and cancelled orphaned
  `work_6b8c4a48d7fbadee`. Local gates passed (`termux-orch` fmt/clippy +
  12 unit + 15 E2E; `chatgpt-provider` fmt/clippy + 92 Rust + 17 `page_driver.test.mjs`).
  Live `/proc/<pid>/exe` hashes match installed binaries, all 4 MCP tunnels
  return `200 ready`, relay `/healthz` reports `ok=true` / `rate_limit_hits=0`,
  and Provider `/control/status` reports `auth_state=authenticated`,
  `completion_ready=true`, `pinned_send=true`, `active_turn_count=0`.

### 2026-11-27 — Signed Phase 1 presentation and Orch verdict repair

- **Exact source:** Phase 1 candidate `265f81eeb43a3351fde3f5ee0c1a4020196f782d`,
  Orch repair `e69df028a8b0c1bbe8b6c9536fc2dbdeb68cc239`, integrated release
  `71078da16327c66e0708fcb8943e939c64f9243a`. The integrated Provider tree
  is identical to the independently reviewed candidate. The first Orch review
  exposed shared synthetic notification timing interference; isolating the
  fixture ledger and receipt namespace retained all assertions and passed
  independent corrective review under both umasks.
- **CI and publication:** [Wrapper](https://github.com/humtr/codex/actions/runs/45965481318),
  [Orch](https://github.com/humtr/codex/actions/runs/45965481389),
  [Provider](https://github.com/humtr/codex/actions/runs/45965481203),
  [signed Orch release](https://github.com/humtr/codex/actions/runs/45965527021),
  and [signed Provider release](https://github.com/humtr/codex/actions/runs/45965532694)
  all passed against the exact release SHA. Downloaded manifests, Ed25519
  signatures, canonical tags, archives and inner binary checksums verified.

  | Active target generation | Sequence | Executable SHA-256 | Live process |
  | --- | ---: | --- | --- |
  | `termux-orch 0.1.0-71078da1` | 29 | `fd4b7186c5a8d1d87122e9421b61636e89e20c4cc02524510e33bab97c388f05` | relay 12477; MCP 12498 (tunnel 12483) |
  | `chatgpt-provider 0.1.0-71078da1` | 58 | `7538d032390753f99935e8cc3472c69a7e8456c9cbd1853bb3fca18c6ceb95bd` | 12899 |

- **Activation:** The signed installer with target components, `--skip-upstream`
  and `--no-setup` staged both generations. The supervisor required an
  explicit idle check before the authorized coordinated switch. Both
  running executable hashes match the signed archives; previous pointers
  select `0.1.0-7423a6dc`. All four MCP tunnels are ready; relay health and
  `codex doctor` are healthy. Wrapper and upstream activation-state bytes,
  the ChatGPT profile, and unrelated service PIDs were unchanged.
- **Qualification:** Exact candidate gates 12/12 exit 0; original standalone
  oracle 18/18; adversarial oracle 23/23; Rust 58+17+17 under each umask
  (one ignored main test), JavaScript 17/17, host 8/8. Official 0.172.0
  loopback Native regression 1/1; synthetic managed-daemon PTYs at four
  widths (100/80/48/32) reached completion markers. The actual signed
  Provider and official `--no-daemon` CLI passed two authenticated
  `gpt-6-2` xhigh turns in a disposable Codex home, including same-thread
  resume, badge projection, table/code preservation and raw remote readback
  with exactly two user and two assistant messages.
- **Durable closeout:** `work_78a0bb522ddfa34b` is `completed` (ledger
  readback seq54). V3/V4 incomplete reviews retained their inconclusive
  verdicts and evidence; V5/V6 independently verified the missing release,
  live, and regression boundaries and recorded PASS on the same current
  subject. No participant replay or manual ledger rewrite. Final evidence
  root `$PREFIX/tmp/phase1-orch-release.blYrsd`, including
  `ci-final.json`, `verify-release-local.log`, activation receipts and
  `live-native/`. The old active handoff is preserved in Git at
  `71078da1:HANDOFF.md`.
- **Not qualified:** Physical screen reconstruction, native clipboard,
  natural interactive TUI exit, actual rollback execution, and Phase 2.
  The authenticated no-daemon CLI proof is not an authenticated default-daemon
  TUI proof. The synthetic PTY completion markers do not qualify physical UI.

### 2026-11-27 — All-the-way MCP single-turn `chatgpt-provider` refactoring, `local-tools` archival, and live 3-MCP + Codex CLI verification

Transitioned `providers/chatgpt` from local function-call round-trips (`local_function_names` / `pending_calls` / `min_request_interval_ms`) to **All-the-way MCP single-turn execution** on regular ChatGPT (`gpt-6-2` / `gpt-6-2-thinking`, preserving regular quota and projects), while archiving the retired `612d2261` local-tools implementation verbatim under `providers/chatgpt/archive/local-tools/` (outside active Cargo/Node builds and CI) and preserving a clean `TurnStrategy` seam in `turn.rs`:

- **Modular decomposition & code hygiene (`providers/chatgpt/src/`):** Split the 3,997-line `responses.rs` into single-responsibility modules: `catalog.rs` (677 LOC; `1M` context / `95%` effective / `800k` auto-compact, `supports_parallel_tool_calls: false`), `session.rs` (488 LOC; lean schema v3 `response_threads.json` with legacy compatibility, project cache, 5-min TTL active MCP connector cache excluding retired `@TermuxBridge`), `turn.rs` (741 LOC; `TurnStrategy::McpSingleTurn`, hidden Codex preambles, utility isolation, auto-project routing, opt-in one-turn MCP role-bridge directive via `[mcp] bridge_guidance = true`), `stream.rs` (721 LOC; 3-state `TurnAssembly`, monotonic `ResponsesEncoder`, 12s SSE keepalive), and thin `responses.rs` (713 LOC). Slimmed `page_driver.js` (2,637 LOC) by removing all `local_function_*` / `min_request_interval_ms` branches and adding secret-free MCP progress streaming (`Running ...` / `Completed ...`). Fixed `supervisor.rs` `check_singleton()` so dead (`ESRCH`), other-UID (`EPERM`), and recycled non-ChatGPT PIDs (`/proc/<pid>/cmdline`) are treated as stale `SingletonLock` symlinks.
- **Static & unit gates:** `cargo fmt --check` PASS; `cargo clippy --all-targets -- -D warnings` PASS (0 warnings); `cargo test` **35/35 PASS** (1 ignored official-Codex test, including `check_singleton_allows_missing_dead_and_non_chatgpt_recycled_pids`); `node --test providers/chatgpt/tests/page_driver.test.mjs` **14/14 PASS**.
- **Runtime environment restored on `janmori101` (`4319`):** Stood down temporary `szob` (`4320`, `profile-szob` preserved on disk), kept retired `@TermuxBridge` (`0.167.2`) server/tunnel stopped without deleting its ChatGPT connector, cleared stale `Singleton*` symlinks for dead PID `14708`, and verified all 4 Stage 2 MCP tunnels + relay returning `HTTP 200 ready` (`termux-shell-tunnel`, `termux-codex-tunnel`, `termux-chatgpt-tunnel`, `termux-orch-tunnel`, `termux-orch-relay`).
- **Live 3-MCP + official Codex CLI E2E on `127.0.0.1:4319` (`janmori101`, `gpt-6-2` `high`, debug candidate binary):**
  - **`model-catalog.json`:** Verified `gpt-6-2` emits `context_window: 1000000`, `effective_context_window_percent: 95`, `auto_compact_token_limit: 800000`, `supports_parallel_tool_calls: false`.
  - **`TermuxShell` (3 commands in 1 turn, `POST /v1/responses`, `37.13s`, `fn_calls=0`):** Executed `git rev-parse --short HEAD`, `wc -l SPEC.md`, and `uname -m` in a single ChatGPT turn (`chat_id=6b08e537-e3e4-83ee-a419-9752b64427a7`, `"Codex"` project), streaming live reasoning titles + `Worked for 7s` and returning exact ground truth (`612d2261`, `221 SPEC.md`, `aarch64`).
  - **`TermuxChatGPT` (`chatgpt_status` in 1 turn, `POST /v1/responses`, `11.89s`, `fn_calls=0`):** Verified `chat_id=6b08e544-f49c-83ee-b5ae-f033a977a2eb` (`Worked for 6s`, `host_running: true`, `auth_state: authenticated`, `completion_ready: true`, `model_count: 28`).
  - **`TermuxCodex` (`codex_status` in 1 turn, `POST /v1/responses`, `12.28s`, `fn_calls=0`):** Verified `chat_id=6b08e553-b5d4-83ec-bec4-f1c779734558` (`Worked for 4s`, `daemon_running: true`, `profile: default`, `default_provider: openai` — confirming physical profile isolation from `chatgpt_desktop`).
  - **Official Codex CLI (`OpenAI Codex v0.172.0`, disposable `CODEX_HOME`, `gpt-6-2` `high`):** Passed end-to-end 1-turn `TermuxShell` execution (`chat_id=6b08e5f2-7a48-83e8-9e9c-2db5cd8b5fcc`, `Worked for 4s`, exact output `` `refactor/mcp-single-turn` | `221 SPEC.md` ``, `tokens used: 2,826`, exit `0`).
- **Signed GitHub Releases & live `--require-signed` activation (`5a865216`, 2026-11-27):**
  - Removed hardcoded fallback model versions (`config.rs` `default_models()` -> `["auto"]`) and aligned `mcp/termux-orch` (`owners.rs` Codex `turn/start` `effort` forwarding, `lead_model_provider`, MCP single-turn Lead/Front prompts, `turn.rs` bridge guidance including `TermuxOrch`, model-agnostic `ORCHESTRATION.md`). Local gates: `providers/chatgpt` `cargo test` **36/36 PASS** (1 ignored) + `node --test` **14/14 PASS**; `mcp/termux-orch` **10/10 unit + 15/15 E2E PASS** (`clippy -D warnings` 0 warnings on both).
  - Signed CI published & readback-verified: `chatgpt-provider` (`sequence=49`, `0.1.0-5a865216`, [run 45929524556](https://github.com/humtr/codex/actions/runs/45929524556), binary SHA-256 `25f350c877ad859fb89874e59f88f91a9d7b5f3f0f017bc6fb4ce487b71def6f`, rollback `0.1.0-612d2261`); `termux-orch` (`sequence=25`, `0.1.0-5a865216`, [run 45929528565](https://github.com/humtr/codex/actions/runs/45929528565), binary SHA-256 `2e22ee1cbac82f06eae54ef78089a0560046ea31e3d0002f1d567f68887763e2`, rollback `0.1.0-72348a3b`).
  - Live `runit` activation verified via `/proc/<pid>/exe` exact SHA-256 match (`chatgpt-provider` PID `9973`, `termux-orch-relay` PID `10032`, `termux-orch-tunnel` PID `10041`); all 4 MCP tunnels return `HTTP 200` on `/readyz`, `@TermuxOrch` connector (`asdk_app_6b03de61dd748191bf56fef2b1029a2e`) refreshed `ok`, and signed live single-turn E2E passed for both `TermuxOrch` (`orch_call(relay_status)` + `orch_works(list)`, `22.98s`, `fn_calls=0`, `Worked for 6s`, `active_works=0`) and `TermuxShell` (`11.33s`, `fn_calls=0`, returned exact `5a865216`).
  - **AppHost upgrade to GPG-verified `26.1120.50957` (`Chrome/158.0.8127.89`, `codexBuildNumber=30866`, deb SHA-256 `d11adb9a...`, rollback `previous -> 26.1117.41037`):** Passed `app.asar` contract check (21/21 symbols intact, `-40.9 MiB` smaller `app.asar`), isolated CDP boot probe (`6,509ms`), isolated `page_driver.js` probe (`app_host_http_fetch`), and live authenticated activation on port `4319` (`auth_state=authenticated`, `completion_ready=true`, 1-turn `TermuxShell` E2E `24.7s`, `fn_calls=0`, `Worked for 4s`).
  - **Signed `chatgpt-provider` `0.1.0-93312687` (`sequence=51`, [run 45936301266](https://github.com/humtr/codex/actions/runs/45936301266), SHA-256 `41a88042672c3c671a1e8ef5106aff12356c4a9ea232529404b9f9316aa90aec`, PID `2935` `/proc/2935/exe` verified, rollback `0.1.0-5a865216`):** Fixed `turn.rs` `is_background_utility_request()` (catching Codex `0.172.0` `title_gen` even when `tools` and `request_kind=turn` are attached), fixed `page_driver.js` `:ext:` single-item reuse after early commits, and added `current_node` sync + `[ChatGPT 앱에서 이어진 대화 동기화]` `commentary` emission on Responses continuations. Verified local gates (Rust `36/36`, Node `16/16`, `clippy -D warnings`) and live on `:4319` (`title_gen` returned `'Fix ghost chat titles in provider'` with `0` persisted threads; App Turn 2 `MANGO_SYNC_7741` synced into Codex Turn 3 `commentary` and answered verbatim). Also empirically verified Codex `0.172.0` `thread/inject_items` in a disposable home (writes raw `response_item` to rollout with `0` TUI notifications and `turns: 0` on `thread/read`).
  - **Signed `termux-shell-mcp` `0.1.0-eb19e206` (`sequence=16`, [run 45938292220](https://github.com/humtr/codex/actions/runs/45938292220), SHA-256 `a8ac379adee837816f868ea6f2992144c648270688f6be98ad1faac9910cccf2`, PID `29393` `/proc/29393/exe` verified, tunnel PID `29378`, `/readyz 200`, rollback `0.1.0-baf81737`):** Updated `docs/spec/MCP_SHELL.md` and `mcp/termux-shell/src/main.rs` so valid NUL-free UTF-8 `stdout`/`stderr` omit redundant `stdout_base64`/`stderr_base64` (cutting ~58% response bloat and eliminating Base64 token pollution), while non-UTF-8, NUL-containing binary bytes, or text-truncated streams still attach lossless `*_base64`. Local gates (`cargo fmt`, `clippy -D warnings`, `7/7` unit + `1/1` stdio E2E) and live `@TermuxShell` E2E (`cat SPEC.md docs/ARCHITECTURE.md`: receipt `01bac5475863b464`, `46,910 B`, `stdout=46,077 B`, `has_b64=False`, `trunc=False`, `exit=0`, `22.2s`) passed.
  - **Signed `chatgpt-provider` `0.1.0-368a1e0d` (`sequence=53`, [run 45941004655](https://github.com/humtr/codex/actions/runs/45941004655), SHA-256 `bbf84d4caf459dcd7fb04f80d33b125c149dad18ec85aee0ae8621b904c0f003`, PID `2497` `/proc/2497/exe` verified, rollback `0.1.0-93312687`):** Made upstream SSE inactivity timeout configurable via `[server] sse_idle_secs` in `~/.config/codex/providers/chatgpt.toml` (default `660s` / 11 minutes to exceed `termux_shell` 600s max timeout, validated range `30..=7200s`, dynamic timeout detail message). Local gates (`cargo fmt`, `clippy -D warnings`, Rust `38/38` unit + Node `16/16`) and live activation on `:4319` (`auth_state=authenticated`, `completion_ready=true`, `host_running=true`, `/v1/models` 8 live families) verified.
  - **Signed `chatgpt-provider` `0.1.0-7423a6dc` (`sequence=56`, [run 45951615798](https://github.com/humtr/codex/actions/runs/45951615798), SHA-256 `1a04b7a6c0239a813c4e5836654a5d9616d12a6dff1910563de96f05d87b1d31`, PID `11531` `/proc/11531/exe` verified, rollback `0.1.0-368a1e0d`) + `termux-orch` `0.1.0-7423a6dc` (`sequence=27`, [run 45951620015](https://github.com/humtr/codex/actions/runs/45951620015), SHA-256 `c646732653467a376debe3177f8ec3fc4307ba3833701aae4b202147ef53b6f3`, relay PID `11534` `/proc/11534/exe` verified, rollback `0.1.0-5a865216`):** Hardened orchestration & provider rate-limit discipline after forensic diagnosis of the 2s `automatic_reason()` `GET /conversation/<lead>` 429 storm: centralized `page_driver.js` `apiFetch()` 429 global cooldown (default 60s when `Retry-After` absent) and prevented false `observeChat` `settled` on 429; added `OwnerGate` global 429 cooldown + telemetry, throttled/event-driven `automatic_reason()` (eliminating unconditional 2s `readChat` polling), non-fatal 429 worker observation, 3-state delivery state machine (`backend_http_429` in `rejected_before_send`, error propagation in `chatgpt_has_delivery_marker`, bounded unconfirmed readback escalation to `waiting_user` + notification), per-account ChatGPT concurrency cap (`ORCH_MAX_CONCURRENT_CHATGPT`, default `2`), and structured `termux-orch-relay` operational JSON logs + `/healthz` metrics. Local gates (`termux-orch` 11/11 unit + 15/15 E2E, `chatgpt-provider` Rust 38/38 + Node 17/17, `clippy -D warnings`, `check_doc_links.py` 25/25) and live activation verified (`auth_state=authenticated`, `completion_ready=true`, `6b091285` recovered from 429). Also applied `[features] memories = false` and `stream_idle_timeout_ms = 660000` to `~/.codex/chatgpt.config.toml`.

### 2026-11-26 — Phase 2 Thin Option B refactoring, utility isolation, auto-project routing, and live A/B qualification

Phase 2 unified `/v1/responses` onto **Thin Option B** (`CompletionClient.startCompletionStream`), archived the retired Option A prompt-flattening path into `docs/history/LEGACY_PROMPT_PATH.md`, removed the over-engineered per-round `GET /conversation/{id}` readback / `verifyCandidateSettlement` / permanent `dispatching: true` disk lockout, cached `GET /me` for 10 minutes, fixed zero-arg native tool calls, hid Codex's machine-generated `role: "user"` preambles (`# AGENTS.md`, `<environment_context>`, `<collaboration_mode>`, `<turn_aborted>`) via `is_visually_hidden_from_conversation: true` while preserving `role: "developer"` intact, routed background utility calls (`title_gen`, `memories`, `auto_review`) to `-instant` + temporary chats (`history_and_training_disabled: true`, `is_do_not_remember: true`) by default (or optional `utility_project_id` / `utility_project_name` created with `memory_scope: "project_v2"`), and added 3-tier automatic project routing (collapsing `/tmp`, `$PREFIX/tmp`, `$HOME`, `/` to shared project `"Codex"`).

The exact verified binary SHA-256
`1c5f93212d8ebcbe80d433acbe357024a8223bb9a5ea8a662a5a7f00643c963d` is active
locally at fixed installation version `0.1.0` on port `4319` (PID `9853`,
`/proc/9853/exe` SHA-256 verified; Phase 1 `62395f19...` backed up at
`generations/0.1.0-p1-62395f19`, with `0.1.0-e01-b6e3a5db`, `previous ->
0.1.0-local-native-0a1dfcb6`, `0.1.0-c9d0e245`, and `control.token` preserved):

- **Unit & lint gates:** `node --test providers/chatgpt/tests/page_driver.test.mjs` 17/17 PASS (asserting 0 `GET /conversation/{id}` polls and 1 cached `GET /me` across a 3-tool turn); `cargo fmt --check` PASS; `cargo clippy --locked --all-targets -- -D warnings` PASS; `cargo test --locked` (`umask 0022` & `0077`) 29/29 PASS (1 ignored).
- **Step 0 live canaries (real ChatGPT Desktop AppHost, isolated `127.0.0.1:4329`):**
  - **Canary 1 (hidden `role: "user"` preamble readability):** PASS (`4,222ms`; ChatGPT read and returned hidden `AGENTS.md` token `HIDDEN_CANARY_D6A7971B` verbatim).
  - **Canary 2 (temporary chat utility isolation):** PASS (`6,701ms`; 4-item 37KB Codex `title_gen` request returned concise title via `-instant` temporary chat with `new_permanent_chats=0` and `threads_written=False`).
  - **Canary 3 (project-only memory `memory_scope: "project_v2"`):** PASS (`1,054ms`; `POST /control/projects` accepted and returned `memory_scope="project_v2"` on `g-p-6b075b0f710c819193e499d164c1f6fc`).
- **Live A/B qualification vs Phase 1 (`c94dc121`):**
  - **Code size (`providers/chatgpt/src/`):** `14,089` → **`13,411` lines** (`-678` net lines; `responses.rs` `4,008` → `3,544`, `page_driver.js` `3,175` → `2,924`).
  - **HTTP calls per 3-tool user turn:** `15` → **`4`** (`-73.3%`; 4 `/f/conversation` + 0 cached `/me` + 0 `/conversation/{id}`).
  - **E02 (official Codex `0.171.0`, 3 sequential shell tool rounds, `gpt-5-6` + `high`):** PASS (`35,893ms` vs Phase 1 `44,648ms` = **-19.6%**, `cli_exit=0`, 3 commands, all 3 nonces consumed, `has_reasoning=true`, **`user_turn_count=1`** confirming hidden preamble, `chat_id=6b075b8d-50b0-83ee-8e47-20c4b50be8b9`).
  - **Auto-project routing:** PASS (`$PREFIX/tmp/...` and `$HOME` both resolved/reused cached `"Codex"` project `g-p-6b075b896c6881919d2575fb599973ff`).
  - **E04/E05/E15 + zero-arg tool (`ping_zero`) + empty `[]` output + mid-round `developer`:** PASS (`19,062ms`, 3 rounds, `user_turn_count=1`, spoof resisted).
  - **Tool-return round `Ctrl+C` mid-stream cancel + abandoned tool recovery:** PASS (HTTP 200 in `4,653ms`, `RECOVERED_OK`, zero `dispatching` lockout).
  - **E06 (official Codex `0.171.0` `-a on-request` + `execpolicy` forbidden rule):** PASS (`cli_exit=0`, `file_absent=true`).
  - **E08 (`input_image` 400), E10 (1-turn `instructions` expiry) + `/compact` (`4,782ms`, chat switched), E13 (mid-tool restart, HTTP 200):** ALL PASS.
  - **E11 (10 consecutive user turns on `gpt-5-6-thinking`):** PASS (`10/10`, `user_turn_count=10`, `chat_id=6b075c5e-10dc-83ec-a816-439afa330f61`, **`avg=5,059ms`**, `median=4,969ms`, `max=7,354ms`, `min=4,281ms` vs Phase 1 `avg=8,419ms` = **-39.9%**).
- **E12 (physical ChatGPT UI visual verification):** **PASS (2026-11-26, user-observed on `6b075b8d-50b0-83ee-8e47-20c4b50be8b9`):** Actual ChatGPT UI displays ONLY the single genuine user prompt (`Run THREE separate sequential shell commands...`), with zero visible `# AGENTS.md`, `<environment_context>`, or synthetic tool/result user bubbles across all 3 tool rounds.
- **Remaining open boundaries:** E03 live multi-call model emission (model serialized `local.*` calls), and P8 signed GitHub release readback/activation.

### 2026-11-25 — Phase 1 Option B hardening, local `0.1.0` activation on port 4319, and live A/B baseline

Phase 1 hardened the Option B unified `startCompletionStream` adapter against
real-world Codex CLI/TUI failure modes (`Ctrl+C` mid-stream disconnect recovery
via `restore_on_cancel` + `allow_parent_branch`, Codex `/compact` fresh-chat
handoff when prefix diverges without `previous_response_id`, duplicate tool
signature deduplication in both `responses.rs` and `page_driver.js`, empty tool
output arrays `[]`, and mid-round companion `developer` updates). The exact
verified binary SHA-256
`62395f1950e64ea52577fc8d613f935510a70737202208c9efa1a44773985e0c` is active
locally at fixed installation version `0.1.0` on port `4319` (PID `30080`,
`/proc/30080/exe` SHA-256 verified; previous E01 binary backed up at
`generations/0.1.0-e01-b6e3a5db`, `previous -> 0.1.0-local-native-0a1dfcb6`,
`0.1.0-c9d0e245`, `control.token`, and the original E02 receipt preserved):

- **Unit & lint gates:** `node --test providers/chatgpt/tests/page_driver.test.mjs` 24/24 PASS; `cargo fmt --check` PASS; `cargo clippy --locked --all-targets -- -D warnings` PASS; `cargo test --locked` (`umask 0022` & `0077`) 32/32 PASS (1 ignored).
- **Live port `4319` (`gpt-5-6-thinking`) & official Codex `0.171.0` baseline:**
  - **E02 (3 consecutive tool rounds, official Codex CLI + live `instructions` + reasoning):** PASS (`44,648ms`, `cli_exit=0`, 3 commands, all 3 runtime nonces consumed, `has_reasoning=true`, `user_turn_count=2` = 0 synthetic user turns, `chat_id=6b06bb3b-ca48-83e8-a452-4eec86393a04`).
  - **`Ctrl+C` mid-stream SSE cancel + immediate recovery:** PASS (client disconnected after 120B SSE chunk; same-session follow-up succeeded HTTP 200 in `7,946ms` with zero `native_reconciliation_required` deadlock).
  - **`/compact` fresh-chat handoff:** PASS (HTTP 200 in `8,607ms`, cleanly switched to a fresh ChatGPT chat).
  - **E04 + E05 + E15 + duplicate sig + empty `[]` output + mid-round `developer`:** PASS (`3` rounds on `6b06bee0-e530-83ee-83f4-3417712980df`, `user_turn_count=1`, custom/namespaced/error/spoof all verified).
  - **E06 (official Codex `0.171.0` `-a on-request` + `execpolicy` forbidden rule):** PASS (`cli_exit=0`, `file_absent=true`, `cmd_count=0`, model reported denial faithfully).
  - **E11 (10 consecutive user turns latency baseline on `:4319`):** PASS (`10/10`, `user_turn_count=10`, `chat_id=6b06bb8b-14a0-83ec-8bed-a89bbf4f7937`, `avg=8,419ms`, `median=7,965ms`, `max=11,852ms`).
  - **Background utility isolation (`memories` / `gpt-6-luna`):** PASS (HTTP 200, `4,846ms`).
- **Remaining open boundaries before Phase 2 slimming / P8 signed release:** E03 live multi-call model emission (model serialized `local.*` calls), E12 physical ChatGPT UI visual verification, Phase 2 legacy-prompt archival + slim verification A/B comparison, and P8 CI/signed GitHub release.

### 2026-11-25 — Option B unified `startCompletionStream` breakthrough and isolated authenticated qualification

Forensics on AppHost `26.1117.41037` (`app.asar` modules `QBVt` → `lNEL` → `Y86X`)
proved two structural capability gaps in Option C's `submitChatGPTLocalFunctionResult`
wrapper: (1) it populates `local_function_names`/`local_function_signatures` from
AppHost's own internal tools, dropping Codex's declared tools on Round 2+ (root
cause of the E02 `native_reconciliation_required` failure), and (2) it accepts
only a single tool result per invocation (blocking E03 batching). Option B —
sending AppHost's exact `a1Qm.Yb` structured `role: "tool"` envelope and post-user
`exclude_after_next_user_message: true` `role: "developer"` request instructions
directly via `CompletionClient.startCompletionStream` with Codex's
`local_function_signatures` preserved on every round — was selected and verified
against the real authenticated AppHost (`gpt-5-6-thinking`) and official Codex CLI
`0.171.0` using isolated candidate binary SHA-256
`b9e2c9b23b8ae772e53d327caf3cb8a997553e086c70899979747be7304ad462` on loopback
port `4329` (disposable state dirs `provider-optb-qual2.gl5psn9r` and
`provider-optb-p2.1q_1v4dv`; original port 4319 service, rollback generations, and
E02 pending receipt preserved):

- **E02 (3 consecutive tool rounds, official Codex CLI `0.171.0` with live `instructions`):** PASS (`cli_exit=0`, `38.6s`, 3 sequential `command_execution` rounds, all 3 runtime nonces consumed, `chat_id=6b06ab6f-f610-83ee-b26f-7d0b403648a8`, `user_turn_count=2` matching Codex's 2 initial user inputs = 0 synthetic user turns, `dispatching=false`).
- **E04 + E05 + E15 (custom tool `raw_probe` + namespaced `diag.check_path` + real execution failure + `[developer]` role-spoofing data):** PASS (`2` rounds, both custom and namespaced outputs/errors consumed verbatim, spoof resisted, `chat_id=6b06ab96-e640-83ee-882e-cf94815bd3a7`, `user_turn_count=1`).
- **E07 (large 64,672-byte lossless tool result + >1 MiB explicit bound):** PASS (1.1 MiB rejected pre-dispatch HTTP 400 `native_content_bound` without consuming pending call; 64 KiB payload delivered losslessly with tail marker verified).
- **E08 (unsupported structured content):** PASS (HTTP 400 `native_content_unsupported`).
- **E09 + E10 (durable `developer` vs request-scoped `instructions`):** PASS (`VAULT_d5a9a6d0` persisted across turns; `[REQ_T1]` applied on Turn 1, expired cleanly on Turn 2 via post-user `exclude_after_next_user_message: true`, replaced cleanly by `[REQ_T3]` on Turn 3).
- **E11 (10 consecutive user turns):** PASS (10/10 turns on one verified chat `6b06ad18-6e9c-83ee-962d-a8bdb578d46f`, `user_turn_count=10`, zero replay).
- **E13 (Provider process restart with pending tool call):** PASS (`chatgpt-provider` terminated and restarted mid-round; original pending call settled cleanly after restart on `6b06abc0-fb68-83e8-b47e-056e9a0da766`, `user_turn_count=1`).
- **E14 (independent parallel chats):** PASS (concurrent sessions succeeded with zero global busy or cross-delivery).
- **Background utility isolation (`memories` / `gpt-6-luna`):** PASS (HTTP 200 under `structured_native_return = true`).
- **Open boundaries:** E03 live multi-call model emission (adapter/verifier support N out-of-order batched results and pass unit tests; live `gpt-5-6-thinking` serialized `local.*` calls 1-by-1), E06 interactive Codex approval denial, E12 physical ChatGPT UI inspection, P6 obsolete-path cleanup, and P8 CI/signed release/authorized port-4319 cutover.

### 2026-11-25 — E02 stopped after the first tool execution

An isolated Codex CLI home and work directory exercised the existing E01
binary on port 4319 with a fresh chat. One tool command exited 0. While
returning its result, the Provider reported `native_reconciliation_required`
and the CLI exited 1. Readback of the original chat found that its head
advanced with the same two original Codex user-role inputs. The Provider
retains the original dispatch intent and one pending call. Result acceptance
and remote settlement remain unresolved. The command and result were not
replayed. See the historical `3f12f956:HANDOFF.md` for evidence and the unapplied source changes.

### 2026-11-25 — E01 proof binary active on port 4319

The existing runit service runs the exact E01 binary as local installation
`0.1.0`. PID 17215 was verified from `/proc` against the E01 binary SHA-256
`b6e3a5db9579187cf953ea358d2f30c10788690d3c8f0cdc26889f97c2244242`.
Loopback health passed. The configuration, control token and continuation
maps matched their snapshots across the restart. The immediate rollback
selection is `0.1.0-local-native-0a1dfcb6`. The binary already reported
package version `0.1.0`. Before the installation name was corrected,
official Codex CLI `0.171.0` verified the same binary on port 4319 by
returning the exact marker supplied only in its developer message and
exiting 0 with zero local commands.

### 2026-11-25 — Official Codex CLI native tool round trip

Codex CLI 0.171.0 through the Termux Wrapper sent developer text and
user-role inputs to the Android candidate. The exact command was
supplied only in the developer message. Codex executed it once,
submitted the structured result, returned the unpredictable marker,
and exited 0. Independent AppHost readback matched the committed head
and the two original user-role inputs. Candidate binary SHA-256
`b6e3a5db9579187cf953ea358d2f30c10788690d3c8f0cdc26889f97c2244242`.
The test restored the prior operating selection. Durable instruction
scope, batches and the remaining P0/P7 cases remain open.

### 2026-11-25 — AppHost native return round trip

Android proof binary SHA-256
`546a2e5c66ecc64685de2ff00f1da850a3e13551e4b527dc1d94667bdf34549e`
passed a direct Responses call and return on the authenticated AppHost.
The AppHost-native continuation returned one structured result for the
original call. The adapter committed the result message ID and remote
head. Independent remote readback confirmed the exact marker and one
genuine user turn. The temporary candidate restored the selected
operating generation. Integrated official Codex and physical UI remain
separate gates.

### 2026-11-25 — Local Native-return Provider selected

`0.1.0-local-native-0a1dfcb6` started on 4319 at PID 26342. Actual
binary SHA-256 `0a1dfcb61d904e32e3f71ee01d7388da6b506403ea855a7c28ea63c258c3da9b`.
The running AppHost reported authenticated and completion-ready. The
pre-activation token and legacy response map bytes matched after restart.
The original E01 chat readback showed one visible user message. The exact
call and return outcome remains unverified. The live Native dispatcher
returned HTTP 400 for a developer item before dispatch. The rollback
preflight passed for `0.1.0-c9d0e245`. This is an unsigned local activation.

### 2026-11-25 — Native-return handoff identity preflight

The operating AppHost was authenticated, ready and idle. It reported a
login user ID and omitted the account-scope ID. The launcher blocked
before creating a stop intent. The updated launcher passed against the
operating Provider, and the rebuilt candidate pins a uniquely verified
account through its existing AppHost account guard in synthetic tests.
Page Driver 18/18 passed. Updated Android candidate SHA-256
`009b5d04d5c11eaa67815e14edb1e901cccdc61b74e63d0b00a70f06b4698ceb`.
Real E01 dispatch and developer instruction scope qualification remain open.

### 2026-11-25 — Native-return candidate and handoff tests

Base `84d916f4` and working-tree changes built locally on Termux for Android
AArch64 API 24. Candidate ELF SHA-256
`df8706e733a586a3ee3c5c3260e4c5ce6664d0cf0b9e03e0fd650ad5a7ee6786`.
The binary passed the 16 KiB ELF gate and matched the independently
extracted bundle payload. Page Driver tests passed 17/17, AppHost Python
lifecycle 4/4, and locked Rust tests 29/29 under each required umask.
Fmt, strict clippy and the separate official Codex MockDriver integration
passed. Disposable `runsv` tests verified the operating-PID lease and
desired-state recovery plus idle candidate shutdown. The production Provider
and AppHost stayed running. The target E01 probe, exact AppHost wire support,
physical UI and signed deployment remain unverified.

### 2026-11-25 — Provider Native-return P0 baseline and isolation

Source `main` at `84d916f4` (Provider/AppHost source unchanged from
`c9d0e245`) was built in a private scratch root with a separate profile,
Provider state, Codex home and loopback port. The candidate debug Android ELF
matched its running `/proc` executable, `/healthz` reported healthy with the
AppHost running, and local control status reported `login_required`.
It exited 0, released its profile lease and closed its port. No model
requests were made. Page Driver 12/12, Python lifecycle 4/4, fmt and strict
clippy passed; locked Rust tests passed 25/25 under each umask (one ignored
integration run separately: 1/1 with official Codex 0.171.0 and MockDriver).
These are baseline checks, not live Native-return acceptance.

Latest observed operating Provider: selected/running `0.1.0-c9d0e245`,
PID 12838, supervisor PID 15625, both binary SHA-256
`d490f8c796b9df0518514cb3039a74733ae66b7eba66d1c64d549f2d1cb26a3e`;
retained `previous` is `0.1.0-74bc360b`. AppHost `26.1117.41037` retained.
Pre/post PID start ticks and binary digests matched, operating `/healthz`
returned HTTP 200, and generation links stayed unchanged. Orch relay PID
9581 remained up; the four MCP tunnels were already down before the probe
and remained down afterward. New public signing/readback, other service
qualification and live Native-return semantics were not verified here.

### 2026-11-25 — Full-stack signed release alignment, `install.sh` setup, and `chatgpt-provider` streaming redesign

All eight signed components are aligned, Ed25519-verified against [release-public-key.pem](release/release-public-key.pem), installed on-device, and verified against live processes and probes:

| Component | Seq | Commit | Archive SHA-256 | Installed SHA-256 | Active / Rollback Generation |
| --- | ---: | --- | --- | --- | --- |
| `wrapper` (`bin/codex`) | 2 | `f49e1117` | `c936833835846fed1d5a2c081c248b4e8e4226d4ee256e0704a96fc60df18ec5` | `f83899d654a23baf83a5bb995c3395e3a74c07f7fca8a77b0810a7069c7d2811` | `0.171.0-d8c43b-2` / `0.170.0-3f7e1a-2` |
| `codex-ext` (`bin/codex-ext`) | 11 | `0c6de280` | `7be3ef1fb67a3fe3151cb438badd212f27f0fc288cc43ffd7e31caa965b8f143` | `4a1c2ac3de14d4e0365c5a38c39e28062ed27af3c959b474b3f9cf2d0daab875` | `$PREFIX/bin/codex-ext` (`cdx`) |
| `tunnel-client` | 5 | `0c6de280` | `887b4ff55d6bf4573b9d6220164df49c4611363ef3cd0f38b58f83f688b46761` | `04c526200ff6f64e20499f33fb2a15e0515cb1b2a508371f7c0f088c069b6fb6` | `v0.0.21-0c6de280` / `v0.0.21+f3c9334e` |
| `termux-shell-mcp` | 8 | `0c6de280` | `3ee83a6f6d01a7c391e24bc155089acf8bcd113439f2a319e968aec585fe4d6e` | `eeb67e96cf16cf7155f69985ee9b9d27ac2b3a9fd8b1dd53b61840051ac79609` | `0.1.0-0c6de280` / `0.1.0+d3d24d11` |
| `termux-codex-mcp` | 5 | `0c6de280` | `454af83201edc65e12cbdd8c4040639519fb5ac8949a2159d3a303f91c102da3` | `abc0b04a5e7e585d4da835e254f4bab201cc367b81e88a68988eea5c76b35ba0` | `0.1.0-0c6de280` / `0.1.0+f3c9334e` |
| `chatgpt-provider` | 14 | `921cbb9e` | `6432e10b575ff8d03c1bda5e1bd9fae1191f83d20c0fd119b3168fb73d10e3c5` | `890c7847e4ab186ac52ec9c2e5348303d512b2ef75d75eaaf7019e0a01398bda` | `0.1.0-921cbb9e` / `0.1.0-66f4ce05` |
| `termux-chatgpt-mcp` | 5 | `0c6de280` | `f99b5e46a1a1b60cf049acacee1e5337fbb854a06037d006b0c21645999ebc15` | `e7dae7e29d10fac976daa432ba3c8b28fa64cf9c705eb3a8dbbe22ae6e263f9e` | `0.1.0-0c6de280` / `0.1.0+f3c9334e` |
| `termux-orch` | 10 | `0c6de280` | `941d1f2e1ba1bdeec2daea899dc48da209eceb14ee6f5faa328a92837d625dbf` | `89d10069c80c85be7c32c91a6146e6c2b5a022f1d843228471e7b0f9856ee390` | `0.1.0-0c6de280` / `0.1.0+7b+d3d24d11` |

- **Model picker, catalog, and installer setup (`0c6de280`, `f49e1117`):**
  `codex-ext` shares `-p` provider configuration across isolated sessions; ChatGPT's `<state_dir>/model-catalog.json` excludes Codex-only `*-wm`/`*-codex`/`hidden` entries, orders 8 canonical families latest-first (`gpt-6-2` through `research`), routes `effort` (`none` -> `-instant`, `low..max` -> `-thinking`/`-t-mini`), sets `supports_reasoning_summary_parameter: true`, and `install.sh` (`wrapper` CI [run 45449340483](https://github.com/humtr/codex/actions/runs/45449340483)) supports `--all` full-stack installation with interactive/device login setup (`Device Code` default `#1`).
- **`chatgpt-provider` single-adapter streaming redesign (`921cbb9e`, release CI [run 45460975231](https://github.com/humtr/codex/actions/runs/45460975231)):**
  - `page_driver.js` requests `supported_encodings: ["v1"]`, consumes `onDecodedEvent` as the single authoritative stream callback (with `onUpdate` fallback guarded against duplicate delivery), normalizes items per ID and channel (`reasoning_summary`, `commentary`, `answer`, `tool_call`), distinguishes `Mutable` vs `Committed` snapshots, preserves Unicode/Korean/whitespace, and exposes public AppHost reasoning (`thoughts[*].summary`, visible `rich_reasoning_titles`/`reasoning_titles`/`reasoning_title`, visible `reasoning_recap`, `channel === "commentary"`) while blocking `thought.content`, `raw_cot`, `hidden` groups, `analysis`, and `hide_all`.
  - `cdp.rs` replaces lossy `try_send()` with non-blocking `LaneMailbox` (coalescing mutable snapshots per `item_id` while losslessly queuing committed/terminal events with sibling-lane isolation).
  - `responses.rs` unifies streaming and non-streaming assembly around `TurnAssembly` + `ResponsesEncoder`, eliminates `chatgpt_stream_revised`, preserves existing live catalogs across `host_stopped` `/control/status`, hardens post-settlement continuation/cancellation and string-input replay (`response_input_already_sent`), and emits `model_reasoning_summary = "auto"` in `print-codex-profile`.
  - **Verification:** `node --test providers/chatgpt/tests/page_driver.test.mjs` (`9/9`), `cargo fmt`/`clippy -- -D warnings`, `umask 0022`/`0077` `cargo test --offline --locked` (`19/19`), ignored `native_codex_reads_provider_summary_and_answer_items` (`1/1`), live `gpt-6-2` `xhigh` (`12.81s`, `stream=true` & `stream=false`) + `max` (`29.4s`) + string replay rejection (`400 response_input_already_sent`), real `codex exec` against live `127.0.0.1:4319/v1` (`reasoning summaries: auto`, `LIVE_EXEC_OK`), and disposable PTY Codex `0.171.0` TUI rendering (`TUI_STEP_1_SUMMARY`, `TUI_COMMENTARY_PREAMBLE`, `TUI_FINAL_ANSWER`).
  - **Observed backend boundary:** Metadata-only AppHost tracing (`gpt-6-2-thinking` `max`, 12.18s) showed that when ChatGPT's backend emits zero `thoughts`/`commentary` and sets `reasoning_recap_type: "hide_all"`, no public reasoning summary exists to forward (and `hide_all` remains suppressed per spec rather than synthesizing or leaking hidden reasoning).
- **Live device services (verified 2026-11-25):**
  `chatgpt-provider` (PID `10705`, `healthz=200`, `host_running=true`, `auth=authenticated`, `28` models / `8` visible families); `termux-shell-tunnel` (`8182` -> child `8196` `termux-shell-mcp`, `:39795/readyz=ready`); `termux-codex-tunnel` (`8485` -> child `8505` `termux-codex-mcp`, `:34947/readyz=ready`); `termux-chatgpt-tunnel` (`9191` -> child `9207` `termux-chatgpt-mcp`, `:37605/readyz=ready`); `termux-orch-relay` (`9581`, `:44901/healthz` `ok:true`, `ticks:3172`); `termux-orch-tunnel` (`9687` -> child `9707` `termux-orch`, `:40255/readyz=ready`); `codex doctor --json` = `healthy`; `codex wrapper status --verify --json` = `qualified: true`.

### 2026-11-24 — Signed independent-audit fixes activated

`d3d24d11` was promoted as `cdx` sequence 9, Shell MCP sequence 7, and
Orch sequence 9. The Shell tunnel restarted. Orch relay and tunnel changed
generation together. The Provider and Codex and ChatGPT tunnels retained
their process identities. All eight public signed artifacts matched the
installed executable hashes, all four live MCP child images matched,
and the Wrapper remained qualified with doctor reporting healthy.

Four terminal Work records and twelve ledger files retained their hashes
through activation. On a disposable copy, the candidate processed all four
records with zero errors, and the prior Orch generation read the same
unchanged records afterward. A separate isolated Work proved additive field
preservation on a real MCP update. Schema-changing rollback and authenticated
cross-device scenarios retain their earlier qualification boundaries.
[Evidence and signed digests](docs/history/PRODUCT_POST_DEPLOY_AUDIT_20261124.md#signed-rollout-of-regression-fixes)

### 2026-11-24 — Independent review follow-up and repeat device audit

Signed `cdx` sequence 8 and Shell MCP sequence 6 from `ff2cafbf` are
installed. Shell alone restarted and retained its prior signed generation.
The other five services retained their PID and start ticks. The device audit
matched all eight downloaded signed archives to installed hashes and
verified the four live MCP child images. The Wrapper reports healthy and
qualified. The source-only supervisor correction is at `c4f307b0`.

The repeat audit passed 269 tests across 13 suites and a separate on-device
`runsv` two-service rollback fixture. The installed Shell MCP settled both
64 KiB binary streams losslessly and five concurrent calls with one call ID
produced one filesystem effect. The installed `cdx` rejected a mismatched
process identity and resumed once in an isolated launcher fixture. An
account-free AppHost probe reached one renderer and returned after clean
supervisor shutdown. Authenticated and remote checks retain their recorded
qualification status.
[Evidence and release digests](docs/history/PRODUCT_POST_DEPLOY_AUDIT_20261124.md#independent-review-follow-up)

### 2026-11-24 — Product post-deployment audit

Thirteen test batches passed across the Wrapper, shared crates, independent
components, and packaging scripts. The audit found four binaries whose
installed hashes differed from their public signed bundles. Two new signed
packages and three existing signed packages were activated. All eight
installed binaries then matched their verified release archives.

- `cdx` claims each pending handoff ticket by rename. Shell MCP closes
  capture readers at command end even when a detached process holds output
  open. Runit checks the actual MCP child before reporting activity.
- Four tunnels switched to the signed runner with their MCP children
  verified. Provider and Orch relay retained their prior process identities.
  Package versions and the official Codex version remained unchanged.

[Individual audit and release evidence](docs/history/PRODUCT_POST_DEPLOY_AUDIT_20261124.md)

### 2026-11-24 — Signed Provider and Orch cutover

Both stable tags use source `51a710c2ddf4e47e4118feff643a7d53753192f7`.
The public Ed25519 signatures, signed manifest and archive sidecars all
verified. Both installed components report `0.1.0+7b`.

| Component | Sequence | Active binary SHA-256 | Rollback generation |
| --- | ---: | --- | --- |
| ChatGPT Provider | 10 | `267108e63e64789059cce96f89484149f436e34b6645671711eee37788c3176c` | `0.1.0+7b-755f3bda` |
| TermuxOrch | 8 | `ddede475c278f076f9ed55d0c37da658062e05cbf73807e401c1740156e14707` | `0.1.0+7b-76a5d3fd` |

- The signed Provider passed the account-free AppHost probe. The signed
  Orch ran an isolated ordered plan with two zero exits and no Lead.
- Both Orch writers were stopped before the shared generation changed.
  The relay and tunnel MCP child now execute the same signed binary.
  The three unrelated MCP tunnel processes kept their original identities.
- The pre-cutover test chat resumed after deployment with its original chat
  ID and recalled its marker. The production test Work
  `work_8bd0dcb94e31f496` completed with shell exits `0, 0` and exactly one
  Front delivery in that chat. The three older Work records were read
  successfully and retained the same hashes. All six services were running
  and `codex doctor --json` returned healthy.
- The existing Orch connector stayed active. Refresh and tools/list probe
  both passed, with schema agreement and cache invalidation.

[Signed release evidence](docs/history/STAGE2_MILESTONES.md#2026-11-24--signed-provider-and-orch-redesign)

### 2026-11-24 — Slice 7 provisional acceptance

The user moved the remaining authenticated cases to long-term verification.
The isolated results below remain the record of what was observed. The
release candidate is eligible for signed publication and coordinated
activation without waiting for those checks.

### 2026-11-24 — Isolated CLI, connector, and Front follow-up

- The second supplied tunnel was absent from the isolated account's
  successful tunnel listing. ChatGPT rejected connector creation with HTTP
  424. Readback confirmed no connector was created. The test account never
  invoked the candidate through a ChatGPT connector.
- Basic `codex` answered the plain question with zero Works. For a natural
  request to continue after the terminal closed, it created one Work and
  exited in 20.311 seconds while the 45-second build was running. The relay
  completed the build and test with exit codes 0, sent one notification
  receipt, and spawned no Lead. `codex -p chatgpt` also answered without a
  Work for the plain question. In one background trial it scheduled and
  completed the two steps, but the follow-up response returned
  `pinned_account_unavailable`. In a later untuned trial it returned without
  calling Orch. A temporary account-diagnostic build completed a separate
  trial end to end and did not reproduce the account failure. With a local
  routing rule in the disposable repository and the original candidate
  restored, a plain question still created zero Works. The same background
  request created `work_e130345d1ce2032c`, acknowledged it in 34.942 seconds,
  and both dependent steps exited 0 with one notification and no Lead.
- A local MCP read with a unique user-message marker moved the Front to a
  second real ChatGPT chat. The final chat marker appeared once in the new
  chat and zero times in the first, and the Lead wake count stayed at zero.
  A nonexistent target was rejected. When a loopback fault injector dropped
  an already accepted send response, rebinding retained the first chat as
  that delivery's reconciliation target. The next report went to the new
  chat. Each report appeared once in its intended chat.
- A real `gpt-5-6-thinking` `xhigh` thread completed manual compaction in
  10.205 seconds. The next turn retained the Codex UUID and used a fresh
  ChatGPT chat. The first answer delta arrived at 8.947 seconds and that
  turn settled at 16.014 seconds. No public reasoning summary was received.
  Nine page-driver tests and fourteen Provider tests passed. A separate
  official Codex integration test consumed synthetic reasoning and answer
  items successfully. The diagnostic build settled three concurrent pinned
  requests. The cause of the intermittent error remains open.
- Fresh authenticated chats accepted synthetic inputs of 32,768 and 32,766
  bytes. The compiled recovery tests passed for both successful bounded
  rollover and a second `input_too_large` rejection. The latter remains
  synthetic coverage.

### 2026-11-24 — Slice 7 and Provider streaming follow-up

| Acceptance | Observed boundary | Remaining |
| --- | --- | --- |
| Plain question and automatic entry | Optional MCP registration and tool descriptions are available from both Codex profiles. Earlier model trials are recorded below. | Qualified natural-language selection with a disposable authenticated connector or model across the three entrypoints. |
| Fixed sequence and shared writer order | Compiled MCP and separate relay processes ran dependent shell steps, held the second writer until the first settled, and used a real exit receipt to complete without a Lead. | Live Front chat notification in the same authenticated run. |
| Parallel worker scope | A disposable Git worktree preserved the original tree and blocked completion after an undeclared write. Conflicting path detection is implemented. | Actual concurrent model-worker write intervals and integration of their branches. |
| Review and completion | Prose rejection did not pass. An isolated official Codex daemon returned a structured rejection from a synthetic Responses endpoint. Its completed turn produced `failed_verification` with evidence and zero Lead wakeups. | Evaluation of real independent model judgments. |
| Front migration | Loopback Provider fixtures exercised read-and-rebind, account/chat checks, and original-target delivery reconciliation. | Caller identity and notification migration on actual ChatGPT conversations. |
| Lost reply and relay restart | Compiled/synthetic tests reconciled the original dispatch and resumed shell plans. An actual detached shell that ignored TERM returned exit code 137 after cancellation. | Authenticated model delivery reconciliation after developer-controlled faults. |
| Restart, compaction and size errors | A fresh map instance resumed the stored chat for a matching prefix. Changed prefix selected a fresh chat. A compiled Provider fixture exercised one bounded rollover on `input_too_large` and a fresh-chat rejection with zero further attempts. | Authenticated restart with the new binary and real size-error reproduction. Earlier compaction evidence is below. |
| Live streaming and TUI | Simulated 250 seconds with live deltas. Official Codex 0.171.0 consumed the candidate's SSE and retained reasoning and answer in its rollout. With a synthetic server, the actual TUI showed live status and answer, and its expanded transcript displayed reasoning. An account-free candidate boot reached the real AppHost control status and exited cleanly. | The exact callbacks and their timing for a logged-in ChatGPT Desktop, a wall-clock xhigh live turn, and physical UI. |

Launching the candidate MCP and relay together on an empty ledger reproduced
an `EEXIST` startup race. The fix accepts a concurrent mkdir and validates
the winning directory by descriptor. A concurrent-open regression test also
checks that a symlink is rejected.

No authenticated account or profile is required by the above fixtures. The
component version identifiers stay at `0.1.0+7b`. Signed promotion and live
device activation require the remaining authenticated acceptance.

The installed AppHost renderer uses the latest `content.thoughts` entry's
`summary` as a live heading. The page driver now accepts that message shape.
The callback fixture checks early delivery, duplicate and replacement updates,
and exclusion of hidden headings and raw analysis. Authenticated callback
timing remains open.

### 2026-11-24 — Isolated authenticated Gate A

- The `d1bebefc` Provider ran on a new AppHost profile with a separate test
  account. The official Codex home, Orch ledger, receipts, and repository
  checkout were also isolated. The first answer delta arrived at 7.784 seconds
  and the turn completed at 16.182 seconds. After restarting only the Provider,
  the next turn retained the Codex UUID and ChatGPT chat ID.
- The first manual compaction returned `pinned_account_unavailable`. A repeat
  on the original thread completed in 39.244 seconds and its next turn wrote
  to a new ChatGPT chat under the original Codex UUID. The first error's
  cause remains open.
- Callback shape diagnostics recorded types and visibility flags only. The
  observed recap was marked `hide_all`. The test has no authenticated evidence
  yet of a visible reasoning summary or of a summary-only live turn.
- `codex exec -p chatgpt` ran `gpt-6-2-thinking` with `xhigh` for 288.576
  seconds, returned exit code 0, and recorded a completed turn in the
  rollout. The rollout contained zero reasoning items for that turn.
- The isolated official Codex OAuth login succeeded. The test ChatGPT account
  has no connected Orch instance. Gate B's caller attribution and Front
  delivery still need a dedicated test tunnel. Signed publication and
  production cutover remain gated on A–C.

### 2026-11-24 — Isolated authenticated Gate C

- The user accepted proceeding past Gate B after the test tunnel reported
  ready and the ChatGPT connector registration returned HTTP 424. The tunnel
  had no ChatGPT workspace binding. Natural-language connector entry and
  caller-chat attribution still have no live acceptance evidence.
- Two real Codex workers edited distinct files in separate Git worktrees.
  Their command intervals overlapped by 4.001 seconds and both declared
  edits stayed in their worktrees. A worker that created an undeclared file
  failed its scope audit. The completion endpoint now also rejects an
  outstanding undeclared edit in a retained worktree. For a shared file, a
  second interactive registration returned `worker_workspace_busy` during
  the first turn, then succeeded on resubmission and read the first edit.
- A real independent Codex verifier ran a release fixture that exited 7.
  Its normal turn settlement recorded `verifier.rejected` and moved the Work
  to `failed_verification`. The completion request was rejected.
- A loopback fault injector received a real Provider dispatch response and
  dropped it before Orch received it. Replay and relay reconciliation kept
  one original worker and one send. In a separate restart trial, both shell
  steps returned 0 and the destination ChatGPT chat received exactly one
  delivery marker. A failed Codex connection exposed intents that could hold
  a reservation without a thread UUID. The relay now settles those intents
  under the Work lock before releasing the reservation.
- The existing ChatGPT Lead rejected the second 63,795-byte prompt with
  `input_too_large`. The same input succeeded in a fresh control chat. The
  relay rolled the unscoped Work to generation 2 once and drained its replay.
  A 70 KiB request returned HTTP 400 `prompt_invalid` at the control route.
  A repeated upstream size rejection on the bounded fresh-chat handoff has
  compiled fixture coverage, with no authenticated reproduction yet.

The candidate passed seven Orch unit tests and fourteen compiled integration
tests. These results did not trigger signed publication or device activation.

### 2026-11-24 — Slice 7 redesign candidate on Android Termux

- Compiled candidate tests separated verifier verdict from turn settlement,
  rejected stale evidence, and ran build plus a fixed shell check across
  separate relay ticks. The check ran once on success and zero times after
  a failed build. Cancelling the disposable shell job produced an exit
  receipt and the tracked command PID disappeared. The loopback Provider
  fixture also recovered worker and Lead launches after dropped replies,
  rejected an unreadable Front, and recovered an unscoped rollover from the
  second chat-list page.
- On the device, a disposable official Codex daemon accepted one Orch turn
  through its control socket and returned the expected result from a local
  synthetic Responses endpoint. The Work reconciled that same thread and
  turn. The isolated daemon was stopped after the test.
- Live signed Provider trials answered plain questions without creating Works
  in the isolated ledger. Automatic local Orch selection did not pass for
  the background follow-up request. Upstream `mcpServerStatus/list` exposed all
  six Orch tools, and loopback capture showed the deferred namespace and the
  discovery text in the long Code Mode `exec` description. The candidate
  Provider preserves discovery at the existing description bound. One trial
  with explicit fixture routing instructions produced native Orch MCP call
  records. A later trial with an immutable candidate and pinned MCP paths
  used the account's deployed connector instead of the isolated tools.
- Three disposable Works reached the default ledger during those trials.
  One native MCP child used the default paths because shell environment
  variables were dropped. Two other requests used the deployed connector.
  The tests recorded successful shell exits and three Work-scoped delivery
  records or notification attempts. The exact three Work directories were
  removed from the active ledger and preserved in the private fixture roots.
  A scan for those fixture references found zero active matches. The old
  relay had dropped an unknown continuation field on a write. Candidate
  and older relay writers must not share the ledger during activation.
- Trusted caller identity for automatic Front migration and live signed
  activation of the redesign remain open.

### 2026-11-24 — Codex compaction through ChatGPT Desktop

- Official Codex `0.171.0` ran in stdio app-server mode with isolated
  `CODEX_HOME` directories and the live `chatgpt_desktop` Provider
  `0.1.0+7b`. Both a manual `thread/compact/start` and an automatic run
  produced one completed Codex compaction and one persisted compaction
  record per thread. The automatic run used a test-only threshold of
  20,000 tokens.
- Each run kept its Codex thread UUID and continued in a new ChatGPT chat
  under the same account. The post-compaction turn completed a deferred
  file change using instructions from before compaction. An independent
  file check passed. A new app-server process resumed the manual run's
  UUID, recalled the earlier completion marker, and left the file intact.
- The installed route qualifies Codex compaction and app-server resume.
  Model-initiated execution through Codex-owned tools and recovery across
  a Provider restart remain unverified. Direct ChatGPT Lead recovery is
  tracked separately below.

### 2026-11-24 — Concurrent account handoff, context rejection and upgrade checks

- **R-2.** With two authenticated `0.171.0` daemons, unsubscribe on the source
  released the disposable thread writer in 1,024 ms using its one-second
  thread-scoped unload setting. The destination resumed the same UUID and
  returned the previous answer's marker. An independent 45-second source
  command was active at destination completion and the source turn also
  completed. In the installed `cdx` PTY, a switch with another active source
  turn returned exit 1 before handoff. With an idle source, the same launcher
  switched to the destination account in place, reattached the same UUID,
  and completed a new turn through the TUI.
- **7B context rejection.** With `lead_rollover_turns=0`, the Provider returned
  `input_too_large` for a 63,795-byte prompt in the existing Lead chat.
  The identical prompt settled with the expected answer in a fresh chat, and
  a short prompt still settled in the original chat. After reproducing the
  rejection, the installed relay tick processed the Work without an error
  and left the Lead at generation 1. A compiled test separately exercised
  a recognized `context_length_exceeded` response with the threshold disabled
  and verified the new generation and exactly-once Inbox replay.
- **AppHost.** All nine signed Android shutdown cases passed again and
  preserved 40 runit process identities. With stock PRoot `5.1.107.102`,
  sending `DISTRIBUTE` without a connection blocked the isolated helper after
  stdin EOF. A client that connected and closed before `accept()` let the
  helper exit with code 0. The normal PRoot chain sends the request only
  after a successful connection.
  [Audit follow-up](docs/history/APPHOST_SHUTDOWN_AUDIT_20261123.md#6-sysv-shm-후속-검증)
- **Legacy Work upgrade.** The 7A executable wrote a pending record after
  the fixture owner received a delivery and dropped its HTTP reply. The
  installed signed 7B executable retained the same pending item as
  `legacy_delivery_receipt_unconfirmed`. The owner saw one POST in total
  and the Inbox cursor stayed at 0.

### 2026-11-23 — Android Termux force-stop recovery

- The repository installer paused at transaction phase `ready` just after
  replacing a fixture executable in a disposable prefix. The operator
  force-stopped Termux, reopened it, and ran recovery in the same kernel boot.
  Recovery exited with code 0. The saved receipt and direct readback confirmed
  that the executable matched its pre-install SHA-256 and that the installer
  lock and recovery gate were gone.

### 2026-11-23 — Signed Work project reuse and interactive launcher update

- Source `76a5d3fd32ad514fb731c476fe03a172096149c1` was pushed to
  `main`. The pinned Ed25519 key verified both signed releases and their
  artifact digests. [codex-ext sequence 6](https://github.com/humtr/codex/actions/runs/45191546037)
  and [termux-orch sequence 7](https://github.com/humtr/codex/actions/runs/45191548688)
  retain package versions `0.1.0` and `0.1.0+7b`, respectively.
- Installed `$PREFIX/bin/codex-ext` SHA-256
  `e6e9c2a3a0e444e2a13e99d1a79d734dddd7254cf149374de1d6619cfa57accd`.
  The six CLI integration cases passed against the exact installed binary
  in disposable homes. `cdx -- --version` reported upstream `0.171.0`.
- Both Orch services selected immutable generation `0.1.0+7b-76a5d3fd`.
  The running relay and installed executable matched SHA-256
  `f56c9e5cffd803df4b6461ff12759f4c356b845b386f55ec7b32889cf1d27d28`.
  The relay and tunnel passed readiness. The four other Stage 2 service
  PIDs were unchanged. `@TermuxOrch` refreshed as `ACTIVE`. Its six tool
  schemas matched the tunnel probe, including the new project and title fields.
- Two authenticated Works `work_08cf40dedf51d772` and
  `work_94415700a8f5d75f` used scope `humtr/codex` and shared the one
  `Wrapper` project. ChatGPT returned both expected worker markers.
  Provider readback confirmed the default `[W1-one] Default title receipt`
  and custom `Run-1/two Custom title receipt` in that project. Each Wait-Set
  delivered one wake-up and both Works reached `completed`.
- A third authenticated Work `work_b94fbbf67dbd242d` used the same scope.
  The configured two-turn threshold advanced its Lead to generation 2.
  Provider readback showed `Lead/2 Wrapper Lead rollover validation` in
  `Wrapper`. The old Lead mutation returned `stale_lead_generation`, the
  checkpoint survived, and the Inbox marker appeared once in the new chat.
  The replay cursor reached event 4 and the Work completed.
- An isolated Android Work `work_577dfca7d0adf9c9` ran a detached Shell
  worker with a 605-second sleep after the launching MCP process exited.
  The local build from the same source started the worker. The installed
  signed binary validated the exit receipt with the same PID and start
  ticks, observed exit 0 and the final output marker, and settled one
  participant and one Wait-Set.

### 2026-11-23 — Read-only upstream canary and R-3/R-6 checks

- **R-5.** The manual Wrapper workflow ran at `79208c1a` and discovered
  official stable Codex `0.171.0`. The downloaded evidence reports agree on
  the exact inputs and archive SHA-256
  `d8c43bee6cf11b01eb18b95f4d7961092bc40228180808859e70dabb7784c1c9`.
  The 37-step AArch64 Linux+Bionic qualification passed and confirmed
  exact-generation rollback on the same version. The release job was skipped.
  [Canary run](https://github.com/humtr/codex/actions/runs/45185943414)
  A separate 37-step run on the Android API 37 device qualified the same
  archive with the installed Wrapper in disposable directories. The fixture
  exercised Native RPC, PTY startup, daemon restart, and exact rollback. The
  Wrapper and daemon health checks passed in the account-free fixture.
- **R-3.** The installed `cdx` forwarded an entire synthetic OSC 52 payload
  through a Termux PTY. The decoded 102,400 bytes matched the UTF-8 fixture,
  including its final marker. The operator confirmed ordinary `/copy` and
  accepted this item. External client pairing remains a long-term check.
- **R-6.** All 23 isolated installer recovery cases passed on Android API 37.
  The installed Wrapper, runtime and both runtime companions passed 16 KiB
  ELF alignment checks. The observed kernel page size was 4 KiB. Actual power
  removal and 16 KiB kernel execution remain open.

### 2026-11-23 — Local in-TUI account switch repair

The account-switch build reported `0.1.0` with executable SHA-256
`55aec6babe339c1eebd536f02f906b1f63dc72d84faeec72a36703a713f55379`.
In an isolated Termux PTY, official Codex `0.171.0` accepted a `!` account
switch from `default`, and the same launcher reached a `szob` TUI on the
same thread UUID. A compiled daemon fixture confirmed that the source TUI
stays alive until its shell turn clears and that a request from another
thread still blocks while busy. The local `ai` runtime now delegates native
interactive launches through `cdx` and reads the existing profile metadata.
The already-running `ai`, TUI, and app-server process identities were preserved.

The installed `cdx` also includes deferred provider switching. It reports
`0.1.0` with SHA-256
`0cc81c7798e43ca5ad06abf720bee0d6d85283cd9bbc1af7aa407883bf0a640a`.
In an Android PTY test with loopback configurations, `!cdx provider chatgpt`
opened a new thread under the same launcher and preserved the original daemon.
Compiled tests covered the caller's shell turn, another active turn, and an
older launcher. Local replacement preserved the active launcher and daemon
identities. Deferred provider switching applies to windows opened by the new
launcher.

### 2026-11-23 — Provider and Orchestration development version restored on Termux

- Source `755f3bda404b9cb3f07bc224d8d5818d1d7bfbdd` restores package version
  `0.1.0+7b` for both components and includes the Slice 5 and Slice 7 audit
  fixes. Both active immutable directories are named `0.1.0+7b-755f3bda`.
  Independent device readback verified the exact tag commit, pinned Ed25519
  signature and archive and binary checksums.
  [Provider release run](https://github.com/humtr/codex/actions/runs/45171392317) ·
  [Orchestration release run](https://github.com/humtr/codex/actions/runs/45171392145)
- Provider signed sequence `9` runs as PID `19031` (start ticks `20512199`).
  The Provider and AppHost supervisor matched SHA-256
  `87bb368a9ba08e3b34cd61c3647751417804d4719eed6f981ad9d20b50c1dfd0`.
  The signed binary passed all nine isolated Android shutdown cases and the
  bundled account-free boot probe. All 23 recorded prior Provider tree
  identities retired at cutover. The live connector confirmed the existing
  account had returned to `completion_ready=true`.
- Orchestration signed sequence `6` runs as relay PID `20186` (start ticks
  `20513785`) and as the MCP child of tunnel runner PID `20280` (start ticks
  `20513986`). Both executables matched SHA-256
  `bc5bc2aa108bec32ff3a5ee6065d0a1b47ce1eca1dc93b6560a102ef05c132f7`.
  The signed smoke verified the detached Shell exit receipt, Wait-Set delivery
  and notification-only cursor in isolation. Live relay and tunnel readiness
  passed, and the connector returned an empty active Work list.
- The `0.1.1` generations are retained as the previous device selections.
  Three unrelated MCP tunnels and both existing managed Codex daemons kept
  their process identities. The final process scan found zero zombies.
  [Remaining operational verification](docs/ROADMAP.md#8-remaining-operational-and-physical-verification)

### 2026-11-23 — Slice 5, Slice 7 and product completion audit

- **Slice 7 delivery.** Wait-Set delivery now persists its owner and marker
  before dispatch. The lost-response regression recovers a single owner
  message after constructing a new relay, advances the Inbox cursor through
  the first 20 visible events, and reads the next page in sequence. The
  fixture adds 75 events before the wake. The report response follows its
  own receipt, and a failed notification-only delivery stays pending.
  The Codex settlement regression checks failed, missing and later successful
  turns independently. Six unit tests and three compiled E2E tests passed
  under umasks `0022` and `0077`. The locally built `termux-orch` candidate
  SHA-256 was
  `26da74e66edfff7256332db8592ec992c1c9306be5802c13a469b49b600c75aa`.
- **Slice 5 shutdown.** Chromium gets an exit window before PRoot escalation.
  The locally built candidate SHA-256
  `68021b1f104bd2e44e9ad52807046f4b7301eee201061e5e5dab8609bc5cf807`
  passed all nine isolated Android shutdown and restart cases and the
  account-free boot probe. The suite preserved 41 runit process identities.
- **Product readback.** All eight public releases passed fresh pinned-key
  signature, tag and artifact checksum readback. Among the binaries compared
  with their public archives, the installed Wrapper matched its signed
  artifact. Installed `codex-ext`, running Provider, and running Orchestration
  relay each had different bytes from their signed archives. The installed
  Wrapper doctor reported healthy, and the live Provider and relay health
  endpoints responded. The connected TermuxChatGPT status reported
  authenticated and completion-ready, and TermuxOrch returned the Work list
  and its five extension schemas through read-only calls. TermuxCodex
  reported a running daemon for the existing `janmori101` profile. The new
  Slice 5 and Slice 7 candidates were qualified separately from these
  installed components.
- **Shared tests.** Locked tests passed for the Wrapper, release builder,
  daemon client, MCP kit, codex-ext, and the three stateless MCP bridges.
  The release and installer Python suite passed 56 tests, the service and
  transport script suite passed 10, and the Provider page driver passed eight.
  [Remaining qualification](docs/ROADMAP.md#8-remaining-operational-and-physical-verification)

### 2026-11-23 — AppHost shutdown candidate on Android Termux

- The optimized local `chatgpt-provider` build, SHA-256
  `3eae69bbf5b8ff7dd1d9681fa0f80c262663590e32dc9d960e35be13fb09ebb9`,
  passed nine [device scenarios](providers/chatgpt/tests/apphost_shutdown_android.py)
  with stock PRoot `5.1.107.102` and ChatGPT Desktop `26.1117.41037`.
  The tests covered termination with an open viewer or SSE, shutdown before
  PRoot exec and during D-Bus startup, PRoot and Provider `SIGKILL`,
  same-profile restart, and startup refusal while a live Chromium singleton
  survived supervisor `SIGKILL`. The Provider-kill case also suspended the SHM
  helper to exercise fallback cleanup. The controlled shutdown cases left
  zero owned processes and zombies. The harness preserved 41 runit identities.
- The account-free manager probe booted one page target through the same
  candidate. Rust tests passed under umasks `0022` and `0077`. Formatting,
  Clippy, four Python tests, eight JavaScript tests, and local document links
  passed. These checks preceded the signed release and activation recorded above.
  [Audit and acceptance design](docs/history/APPHOST_SHUTDOWN_AUDIT_20261123.md)

### 2026-11-23 — Stage 2 work order closeout

- **R-1.** Seven stable Stage 2 component releases at `f3c9334e` passed
  publish CI and independent four-asset readback. All seven Ed25519 signatures
  verified with the pinned repository key. [Release tags, run IDs and signature hashes](docs/history/STAGE2_MILESTONES.md#r-1-signed-stage-2-releases)
- **R-2.** On `0.171.0`, `szob` resumed disposable UUID
  `01a2078c-c287-7630-a65f-0a7b24b680c9` after `janmori101` released its
  writer lock and returned the exact nonce in a live OpenAI turn.
  Rename, archive/unarchive and delete were checked through both RPC
  paths, the TUI, and `codex exec resume`. The old exec name fell back to
  a fresh disposable thread, which was removed. The original 175-row
  shared index matched byte-for-byte after deleting the test threads, and
  SQLite `quick_check` returned `ok`. [Four-path matrix](docs/history/STAGE2_MILESTONES.md#r-2-authenticated-uuid-handoff-and-session-lookup)
- **R-4.** Removed `~/.codex/packages/app-server-daemon/releases/` and
  reclaimed 514,213,376 allocated bytes (490.39 MiB). Doctor reported
  `healthy`, the runtime reported `qualified=true`, and all three
  managed daemons reported `running`. [Inventory and retained archive](docs/history/STAGE2_MILESTONES.md#r-4-legacy-disk-reclamation)

### 2026-11-23 — Stage 2 Slice 7B resilience on Android API 37 Termux

- **Source and CI.** `b51ade9e5b9a1681c51bddf226a7fa029e573304` was pushed to
  `main`. All four push workflows passed. [Termux-Orch](https://github.com/humtr/codex/actions/runs/45136421921),
  [Codex-Ext](https://github.com/humtr/codex/actions/runs/45136421952),
  [ChatGPT Provider](https://github.com/humtr/codex/actions/runs/45136421951),
  [Wrapper](https://github.com/humtr/codex/actions/runs/45136421934).
- **Deployed artifacts.** `termux-orch 0.1.0+7b` SHA-256
  `b98190cbc5da774ebe1e7d22026009ea17fb60d2c46b85e534ab1d79f6d446e9` is selected for both services. The relay is running as PID
  `31600` with start ticks `19047311`. The tunnel runner
  is `tunnel-client v0.0.21` PID `29049` with start ticks
  `19034026`. Both passed `runit` readiness.
  `chatgpt-provider 0.1.0+7b` SHA-256 `de49820262918b79f9cfedd36c6156659d3cb51f42023b999c42ef912b1eb0de` runs as PID
  `28994` with start ticks `19033625`.
  The standalone `$PREFIX/bin/codex-ext 0.1.0` SHA-256 is
  `c8322d2e6e5737466876a07f262ad02ae38f4c16e59e66ecf3d5d3a3820f9287`. `cdx` resolves to that executable.
- **Connector readback.** Refresh of `asdk_app_6b03de61dd748191bf56fef2b1029a2e`
  returned HTTP `200` and left the connector `ACTIVE`. The remote
  `tools/list` probe returned HTTP `200` and reported
  `schema_agreement: true`. Both the registered actions and the probe
  schemas matched all six installed 7B tools, including the new fields.
- **Lead rollover.** Work `work_90bb0f11a8294fff` kept project
  `g-p-6b03f0dba83081919bfdd966e51ffe11` and the verified account pin. At
  two user turns, the relay replaced `6b03f404-d618-83e8-811d-de6df5a90adc` with
  `6b03f432-03a0-83ec-93d8-36c2409dc8d4`. Provider readback confirmed the title
  `[L2] S7B_c8e153a8a9 automatic rollover` and the pinned project. The old generation
  and chat reference each returned `stale_lead_generation`. The checkpoint
  survived and Inbox event `2` appeared once in L2. The
  `lead.rolled_over` event was `4`, with
  `consumed_seq` advancing from `0` to `4`.
- **Detached Shell and Wait-Set.** In `work_b351c18c7cd4636f`, Shell `W1` PID
  `31494` and start ticks `19047199`
  matched before and after relay restart `29030 -> 31600`.
  The atomic exit receipt matched that identity and recorded exit `0` after
  `2.332 s` observed. Both output tails were present.
  The `janmori101` Codex `0.171.0` daemon (PID `3625`) completed W2 thread
  `01a20772-529a-7530-82f8-26fc332be92f`. Inbox settlements
  `4` and `5`
  fired `wait_6523e8cee52d43f5` once via event `6`
  using `notify_only`.
- **Stop wake.** `codex-ext notify emit Stop` PID `31829` advanced
  relay `wake_pushes` from `0` to `1`
  and ticks from `4` to `5`.
  The next tick was observed in `851.3 ms` and the
  relay recorded `0 ms` from acceptance to tick
  start at millisecond resolution.
- **Local gates and cleanup.** Rust formatting, Clippy `-D warnings`, and
  locked tests under both `0022` and `0077` passed for `termux-orch`
  (5 unit, 3 compiled E2E), `codex-ext` (6 unit, 5 CLI) and the Provider
  (10 Rust tests). All 8 Provider JavaScript tests passed. The two disposable
  Work directories were removed after copying their receipts to the local
  evidence directory. Live status reported `total_works: 0`.
  The root manifests, Wrapper, release builder and three stateless bridges
  had zero source diff.

### 2026-11-23 — Stage 2 Slice 7A: Durable Multi-Role Orchestration (`@TermuxOrch` / `mcp/termux-orch`, `F · L · W · V · R`) & Live E2E

- **Source and CI.** `fd3c445d183aec7f2aed57363ede0bf443e564ac` was pushed to `main`. All push checks passed, including Android packaging and the Wrapper Bionic smoke test. [Termux-Orch](https://github.com/humtr/codex/actions/runs/45131891229), [ChatGPT Provider](https://github.com/humtr/codex/actions/runs/45131891214), [Wrapper](https://github.com/humtr/codex/actions/runs/45131891356).
- **Scope & zero-Wrapper decoupling:** Extended [providers/chatgpt/src/page_driver.js](providers/chatgpt/src/page_driver.js) `observeChat` to inspect server `GET /conversation/{id}/stream_status` (reporting `remote_stream_status: "IS_STREAMING" | "COMPLETE"` when no local lane is active, preventing collisions with mobile/web streaming) with unit test in [page_driver.test.mjs](providers/chatgpt/tests/page_driver.test.mjs) (6/6 PASS). Added standalone [mcp/termux-orch](mcp/termux-orch/src/main.rs) (own `[workspace]` locking 14 crates, reusing `crates/mcp-kit` + `crates/codex-daemon-client` + loopback `chatgpt-provider /control/*`, zero `crates/wrapper` or root `Cargo.toml`/`Cargo.lock` diff) implementing `Work Ledger` (`work.json`, `checkpoint.md`), `Typed Inbox` (`events.jsonl` monotonic `seq`/`consumed_seq` & `event_id` dedup), declarative `Wait-Set` (`all_of`, `any_of`, `on_user_event`, `deadline_secs`), hard Circuit Breakers (`max_wakeups`, `max_wall_secs`, `max_concurrent_workers`, cooldown), per-work ChatGPT Project scoping (`[Work] <title>`) with pinned `(account_id, project_id)`, deterministic role session titles (`[F#]`, `[L#]`, `[W#-slug]`, `[V#-slug]`), Tier 1–3 lifecycle + `F1 -> Fn` marker/ID rebind, `inject_and_notify` + `PENDING_DELIVERY` backoff + `termux-notification`, 6 static `@TermuxOrch` tools (`orch_works`, `orch_work`, `orch_dispatch`, `orch_report`, `orch_catalog`, `orch_call`), and CI [termux-orch.yml](.github/workflows/termux-orch.yml).
- **Local static, unit & compiled E2E gates:** `providers/chatgpt` passed `node --test` (6/6), `cargo fmt --check`, `clippy -D warnings`, and `cargo test --locked` (10/10); `mcp/termux-orch` passed `cargo fmt --check`, `clippy --all-targets --locked -- -D warnings`, and dual `umask 0022`/`0077` `cargo test --locked` (4 unit including Tier 1 → `promote` → Barrier wake-up → `F2` marker rebind → `IS_STREAMING` busy backoff + 1 compiled-binary stdio MCP E2E).
- **Live `runit` deployment & programmatic `@TermuxOrch` registration (Android API 37 Termux):**
  - Activated `chatgpt-provider` `0.1.0+7a` (`8e18211f58c8d2b35c46b55f8ab39e55795a0325eddcb401ec6344148ae9d2df`, `previous -> 0.1.0+multilane`, `readiness_ok=true`, live `POST /control/observe` returned `remote_stream_status: "COMPLETE"`).
  - Installed & activated `termux-orch` `0.1.0+7a` (`fce5ea31cd407eb010ac83c3247aeaa0fcb5420ab4c8b1fcc7c920bcdba3d73b`, `previous -> 0.1.0`) across both new opt-in `runit` services: `termux-orch-relay` (`/healthz` `ok=true`) and `termux-orch-tunnel` (`tunnel_6b03ddf5129c819188e0e908ff223271`, `readyz` `ready`).
  - Programmatically registered `@TermuxOrch` (`asdk_app_6b03de61dd748191bf56fef2b1029a2e`, `ACTIVE`, `refresh_ok=true`, `probe_ok=true`, `schema_agreement=true`, `cache_invalidated=true`, 6 tools) via `POST /control/connectors`; remote ChatGPT chat `6b03ded3-884c-83ee-9038-ac3341a787d4` (`gpt-6`) invoked `orch_catalog` and `orch_works(list)` live through the OpenAI tunnel.
- **Live Slice 7A E2E Verification (3 real scenarios against live `runit` `termux-orch-relay`, `chatgpt-provider 0.1.0+7a`, `codex 0.171.0` `janmori101` daemon, and `@TermuxOrch`):**
  1. **Tier 1 `direct` → `F2` marker rebind + live relay auto-delivery (`work_76854f714e504e2a`):** Pinned `account_id=user-yoCTO7NRnB46h7CAbdtogkoc`, auto-created project `g-p-6b03e6cad2c081918b7d883796475854`, spawned `[W1-calc]` (`6b03e6ce-...`), resolved brand-new chat `F2` (`6b03e6d7-...`) via `front_marker` (`S7A_REBIND_MARKER_1795417795`) and renamed it `[F2] S7A-1 Direct Rebind 7795`, and observed live `termux-orch-relay` deliver `[ORCH_WAKEUP]` into `F2` automatically on `W1` settlement.
  2. **Tier 1 `direct` → Tier 2 in-place `promote` (`work_5b00ad2bfda737fa`):** Promoted `direct` → `orchestrated` attaching `[L1]` (`6b03e6f7-...`) inside the pinned project while preserving existing participant `W1`.
  3. **Tier 3 multi-backend parallel Barrier + `sv restart` recovery + autonomous `L1`→`V1` + `F2` completion (`work_ff1ddcc0d878543c`):** Spawned parallel `W1` (real Codex daemon `janmori101` `01a2073f-001e-7453-ade0-fa6ddeaf9c98` → `CODEX_W1_OK termux-orch=0.1.0`) and `W2` (real ChatGPT `6b03e72e-...` → `CHATGPT_W2_OK`), armed `all_of(W1,W2)` Wait-Set, restarted live `runit` `termux-orch-relay` (`PID 24604 -> 29245`), confirmed restarted relay auto-woke `[L1]` (`6b03e720-...`, event `#6`), observed `[L1]` autonomously invoke `@TermuxOrch` over the tunnel to spawn independent Codex Verifier `V1` (`01a2073f-7f65-7ce2-94a2-ec0f55e68b23` → `**V1_VERIFIED**`, event `#10`) and emit a milestone report (`#8`/`#9`), confirmed Round-2 wake-up (`#11`), rebound Front to `F2` (`#12`), and delivered final `orch_report(complete)` (`#13`/`#14`) to `F2` (`f2_received_final_report=true`). Disposable test work directories removed afterward (`total_works: 0`).

### 2026-11-23 — Multi-Lane Parallel Execution (`0.1.0+multilane`) & Multi-Worker Iterative Loop Live E2E

- **Multi-lane parallel execution (`providers/chatgpt` + `mcp/termux-chatgpt`):** Replaced single `activeTurn` in [page_driver.js](providers/chatgpt/src/page_driver.js) with bounded `lane_id`/`chat_id`-keyed `activeTurns` Map (up to 8 concurrent lanes; same-`chat_id` concurrent writes still fail closed with `generation_already_active`), wired per-request `lane_id` cancellation in [responses.rs](providers/chatgpt/src/responses.rs) so disconnecting one `/v1/responses` stream never cancels sibling lanes, added targeted `chat_id`/`lane_id`/`all` cancellation on `POST /control/stop`, and enabled optional `chat_id` on `chatgpt_send(action="stop")` in [mcp/termux-chatgpt](mcp/termux-chatgpt/src/main.rs).
- **Local gates & live activation (`0.1.0+multilane`):** Passed `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, dual `umask 0022`/`0077` `cargo test --locked` across both `providers/chatgpt` (10/10) and `mcp/termux-chatgpt` (4 unit + 3 E2E), plus `node --test providers/chatgpt/tests/page_driver.test.mjs` (5/5). Activated `chatgpt-provider` `0.1.0+multilane` (`previous -> 0.1.0+connectors`) and `termux-chatgpt-tunnel` `0.1.0+multilane` (`previous -> 0.1.0+ed3d9daf`) in `runit`, and refreshed `@TermuxChatGPT` (`asdk_app_6b03bd3144208191ad58446469b02bbf`) with `schema_agreement: true`.
- **Live multi-worker iterative loop E2E (`gpt-6-sol-wm`, chat `6b03c20b-efec-83ec-bf9d-3477c68df90e`, ~80s, 0 approval prompts):** In a disposable workspace containing random nonces (`ALPHA_060B57B4`, `GAMMA_18F6E12B`) and an intentional off-by-one bug in `beta/calc.py`, a single remote ChatGPT Orchestrator turn dispatched two `@TermuxCodex` workers in parallel (`01a206ae-7269-...` reading `alpha/data.json`, `01a206ae-1862-...` diagnosing the test failure without editing), looped Round 2 onto Worker B's existing `thread_id` to fix `beta/calc.py`, independently verified via `@TermuxShell` (`CALC_TESTS_PASS total=89` + `GAMMA_18F6E12B`), and emitted `ORCH_LOOP_VERIFIED: ALPHA_060B57B4 | GAMMA_18F6E12B | sum=89` (independently verified on disk before temp cleanup).
- **Live multi-lane parallel ChatGPT E2E (`gpt-6-sol-wm`, project `g-p-6b03043ed5888191a5c9809734d3469d`):**
  - Direct 2-lane parallel `/control/send`: observed `peak_active_turn_count: 2`, both lanes settled in `8.12s` on distinct chats (`6b03c670-443c-...` `LANE_A_OK=1517`, `6b03c670-7c40-...` `LANE_B_OK=3127`).
  - Re-entrant 3-lane parallel orchestration: live Orchestrator (`6b03c678-2014-...`, Lane 1) invoked `@TermuxChatGPT` `chatgpt_send(action="send")` twice concurrently (Worker 1 `6b03c682-c98c-...` `W1=437`, Worker 2 `6b03c688-73ec-...` `W2=899`), settling all 3 concurrent ChatGPT lanes in `25.75s` with `MULTILANE_ORCH_OK W1=437 W2=899 SUM=1336`.

### 2026-11-23 — Stage 2 Post-Audit Hardening (`ed3d9daf`), 3-MCP `runit` Tunnel Rollout, and Programmatic Linux-App Connector Registration & Live E2E

- **Post-audit remediation & CI (`ed3d9daf`):** Resolved all Tier 1–3 findings from independent Slice 6 and whole-Stage-2 audits (including `driver.rs` mutex-free `evaluate_json` + `AtomicBool` `host_running`, `runit_service.py` hyphenated flag parsing + `--runner-component`/`--runner-binary` tunnel supervision + `TimeoutExpired` rollback guard + `O_EXCL` `0o700`/`fsync`, shared Ed25519 publisher `publish_stage2_release.py`, `codex-ext`, `mcp-kit`, `termux-shell`, `termux-codex`, `termux-chatgpt`, and `chatgpt-provider` hardening). All 7/7 GitHub Actions workflows passed on `ed3d9daf` (`Wrapper CI`, `Codex-Ext CI`, `Termux-Shell MCP CI`, `Termux-Codex MCP CI`, `ChatGPT Provider & AppHost CI`, `Termux-ChatGPT MCP CI`, `Tunnel-Client & Runit Supervision CI`). Rebuilt and installed all 5 Stage 2 binaries (`0.1.0+ed3d9daf`, `previous -> 0.1.0`) and updated `~/.codex/chatgpt.config.toml` to port `4319` (`gpt-6-sol-wm`, `xhigh`, backup archived under `~/.local/state/codex/legacy-0.167.2-archive/`).
- **3 dedicated OpenAI tunnels & `runit` supervision:** Provisioned 3 workspace-scoped OpenAI tunnels under `janmori101` (`org-WyQP0qVgjjopB2qg389MwW4L`, workspace `3ebe5ee5-8050-4549-8efb-873f30a8906e`) with standalone `0600` configs under `~/.config/codex/tunnels/termux-{shell,codex,chatgpt}.yaml` (zero `tdev` dependency) and enrolled all 3 via `scripts/runit_service.py --runner-component transport/tunnel-client --runner-binary tunnel-client`:
  - `termux-shell-tunnel`: `tunnel_6b03b77cf8bc8191b405430734cc50f7` (`ready`)
  - `termux-codex-tunnel`: `tunnel_6b03b79df8d48191979de888b107f116` (`ready`)
  - `termux-chatgpt-tunnel`: `tunnel_6b03b79fb46c8191b907d4391d0a4f06` (`ready`)
- **Programmatic zero-DOM connector control (`/control/connectors*`) & live registration:** Added authenticated `GET /control/connectors`, `POST /control/connectors`, and `POST /control/connectors/{id}/refresh` to [providers/chatgpt](providers/chatgpt/src/control.rs) + [page_driver.js](providers/chatgpt/src/page_driver.js) over Electron's native `connect-app-host` `services.httpFetch` (`/accounts/check/v4-2023-04-27`, `/aip/connectors/mcp/tunnels`, `/aip/connectors/links/list_accessible`, `/aip/connectors/mcp`, `/aip/connectors/links/noauth`, `/aip/connectors/mcp/refresh_actions`, `/ecosystem/call_mcp`) and dynamic `scope.queryClient.invalidateQueries()` (zero DOM clicking, zero hardcoded chunk hashes, zero app file modification). Activated `chatgpt-provider` `0.1.0+connectors` (`previous -> 0.1.0+ed3d9daf`) and programmatically created, noauth-linked, refreshed, cache-invalidated, and verified full `schema_agreement: true` (`POST /ecosystem/call_mcp` `tools/list`) for all 3 connectors in workspace `3ebe5ee5-8050-4549-8efb-873f30a8906e`:
  - `@TermuxShell` (`asdk_app_6b03bcaf76888191b3a28a66f5f0c349`, `link_6b03bcbe50748191afa6af3142cc132c`, `ACTIVE`, 1 tool: `termux_shell`)
  - `@TermuxCodex` (`asdk_app_6b03bcfa236c81918a67c29bfa96c4d8`, `link_6b03bd09a5dc8191ba9e594bd98e620b`, `ACTIVE`, 5 tools: `codex_status`, `codex_threads`, `codex_turn`, `codex_catalog`, `codex_call`)
  - `@TermuxChatGPT` (`asdk_app_6b03bd3144208191ad58446469b02bbf`, `link_6b03bd40e518819189f317f2223900fd`, `ACTIVE`, 6 tools: `chatgpt_status`, `chatgpt_chats`, `chatgpt_projects`, `chatgpt_send`, `chatgpt_catalog`, `chatgpt_call`)
- **Live ChatGPT remote MCP conversation E2E (`gpt-6-sol-wm`, chat `6b03bf84-36e8-83ee-a3b7-d391108a50e7`, zero approval prompts):**
  - Turn 1 invoked all 3 registered remote connectors natively in one turn (`MCP_CONNECTIVITY_3_OF_3_OK`): `/TermuxShell/.../termux_shell` (`call_id=live-shell-1` -> `SHELL_MCP_E2E_OK`, `exit_code=0`, `15ms`), `/TermuxCodex/.../codex_status` (`state=ok`), and `/TermuxChatGPT/.../chatgpt_status` (`provider_reachable=true`, `authenticated`, `completion_ready=true`, 25 models).
  - Turn 2 invoked `/TermuxCodex/.../codex_status(profile="janmori101")` (`daemon_running=true`, `model=gpt-6.2-sol`, `providers=[openai, ais]`, `warnings=[]`) and `/TermuxCodex/.../codex_threads(action="list", profile="janmori101", limit=3)` (returned 3 live threads + `next_cursor`, `CODEX_PROFILE_JANMORI101_OK`).

### 2026-11-23 — Stage 2 Slice 6: Stateless `@TermuxChatGPT` (`termux-chatgpt-mcp`), Slim `BRIDGE.md`, and `chatgpt-provider` `runit` Enrollment

- **Scope & independent workspace:** Added standalone [mcp/termux-chatgpt](mcp/termux-chatgpt/src/main.rs) (`termux-chatgpt-mcp`, own `[workspace]` locking 13 crates, zero async/TLS bloat, zero local code execution privilege, zero shadow task DB, zero `crates/wrapper` coupling) over shared [crates/mcp-kit](crates/mcp-kit/src/lib.rs) and a minimal loopback-only HTTP/1.1 client ([http_client.rs](mcp/termux-chatgpt/src/http_client.rs)), exposing 6 static tools (`chatgpt_status`, `chatgpt_chats`, `chatgpt_projects`, `chatgpt_send`, `chatgpt_catalog`, `chatgpt_call`) with uniform immutable annotations (`readOnlyHint=true`, `destructiveHint=false`, `openWorldHint=false`) and 5 strictly disjoint extension actions (`project_create`, `chat_rename`, `viewer_url`, `viewer_open`, `host_restart`), normative annex [docs/spec/MCP_CHATGPT.md](docs/spec/MCP_CHATGPT.md), slimmed [docs/spec/BRIDGE.md](docs/spec/BRIDGE.md) tying together all 3 independent MCPs + `chatgpt-provider`, and path-triggered CI [termux-chatgpt.yml](.github/workflows/termux-chatgpt.yml). Root `Cargo.toml`, `Cargo.lock`, and `crates/wrapper` untouched.
- **Live `chatgpt-provider` `runit` enrollment:** Enrolled `chatgpt-provider` via `scripts/runit_service.py enroll --arg serve`; confirmed `sv status` running (PID `16502`), `GET http://127.0.0.1:4319/healthz` `ok=true` (`host_running=false` lazy boot), and `runit_service.py status --readiness-cmd curl http://127.0.0.1:4319/healthz` reporting `activation_state=active`, `readiness_ok=true`, matching installed binary `27574021...`.
- **Verification (Android API 37 Termux — unit, compiled stdio E2E, and live `runit` `chatgpt-provider`):**
  - **Static & unit/E2E gates:** `mcp/termux-chatgpt` passed `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, and `cargo test --locked` (4 unit + 2 compiled-binary stdio E2E tests covering CLI flags, uniform annotations, disjoint extensions, loopback-only guard, non-poisoning provider-down `call_id` retry, exact replay/payload conflict, mid-send `delivery_unknown` transport drop, and all 6 tools + 5 extensions) under both `umask 0022` and `0077`; `.github/scripts/check_doc_links.py` (19/19) and YAML checks (8/8) passed.
  - **Live `runit` `chatgpt-provider` + `gpt-6-sol-wm` (`xhigh`) E2E (`termux-chatgpt-mcp` default canonical URL & `control.token`):** Confirmed pre-boot `chatgpt_status` (`provider_reachable=true`, `host_running=false`), lazy boot via `chatgpt_projects(list)` (5 projects, 20.9s), warm `chatgpt_status` (`authenticated`, `completion_ready=true`, 25 models), live `chatgpt_send(action=send)` on `gpt-6-sol-wm` (`xhigh`, `23.6s`, `SLICE6_MCP_OK=2173`), exact duplicate `call_id` replay (`replayed_from_receipt=true`), payload mismatch conflict (`chatgpt_call_id_payload_conflict`), `chatgpt_chats(read)` (2 messages, project-scoped, zero private `thoughts`), `chatgpt_call(chat_rename -> "Slice6-MCP-Verified")`, `chatgpt_projects(read)`, `chatgpt_send(observe|stop)`, and `chatgpt_call(viewer_url)`.
  - **Live Termux canonical install:** Installed `~/.local/lib/codex/mcp/termux-chatgpt/generations/0.1.0/termux-chatgpt-mcp` (SHA-256 `52ae30f883eac565d25df31350d46517b55bbfa7efaea00f29458d0b1720ed87`), linked `current -> generations/0.1.0` and `$PREFIX/bin/termux-chatgpt-mcp`; verified `termux-chatgpt-mcp --version` (`0.1.0`), `--describe-tools` (6 tools), and `runit_service.py status` (`activation_state=not_enrolled`, `selected_generation.valid=true`).
- **Remaining operational / unverified boundaries:** Opt-in `runit` enrollment of the 3 MCP tunnel services (`termux-shell-tunnel`, `termux-codex-tunnel`, `termux-chatgpt-tunnel`), remote ChatGPT mobile/web connector binding, and signed GitHub Release publication for Stage 2 binaries (`workflow_dispatch` `publish_release=true`) remain separate operator steps.

### 2026-11-23 — Stage 2 Slice 5: `host/chatgpt-desktop` Generations & Slim `chatgpt-provider` (Native `local_function_signatures` & `gpt-6-sol-wm` `xhigh` Orchestration)

- **Scope & independent workspace:** Added immutable Linux ChatGPT AppHost lifecycle manager [host/chatgpt-desktop/manager.py](host/chatgpt-desktop/manager.py) (pinned official OpenAI APT GPG keyring `3BFA0E4AE8B8CC16A2D9BA684A3B4A566C4660E4`, `current`/`previous` rollback, separated `~/.local/share/codex/chatgpt-desktop/profile/{chromium,.codex}`, account-free isolated PRoot/Xvnc CDP probe `Gate 5-0`) and standalone slim [providers/chatgpt](providers/chatgpt/src/main.rs) (own `[workspace]`, single CDP `--remote-debugging-pipe` owner, allow-listed Android OAuth browser relay via `termux-am-socket`, capability-scoped noVNC viewer proxy, clean-room zero-DOM/zero-app-patching [page_driver.js](providers/chatgpt/src/page_driver.js) over Electron `connect-app-host` Cap'n Web `services.httpFetch` + dynamically discovered in-app `CompletionClient`, OpenAI `/v1/models` & `/v1/responses` with native ChatGPT `local_function_names`/`local_function_signatures` (`type: "kwargs"`) + Codex `0.171.0` `additional_tools`/`custom_tool_call`/`agent_message` (`encrypted_content`) + `thread-id`/`prompt_cache_key` delta session tracking, authenticated `/control/*`, zero DDPE/MCP/`shared_server`/Wrapper coupling), normative annex [docs/spec/CHATGPT_PROVIDER.md](docs/spec/CHATGPT_PROVIDER.md), and CI [chatgpt-provider.yml](.github/workflows/chatgpt-provider.yml). Root `Cargo.toml`, `Cargo.lock`, and `crates/wrapper` untouched.
- **Namespace unification & version-suffix cleanup:** Retired & archived conflicting `0.167.2` `codex-chatgpt-*` services/state into `~/.local/state/codex/legacy-0.167.2-archive/` (leaving `tdev*`, `sshd`, `codex 0.171.0` undisturbed); unified all Stage 2 paths into the canonical XDG `codex/` namespace (`~/.local/lib/codex/...`, `~/.local/share/codex/...`, `~/.config/codex/...`, `~/.local/state/codex/...`); removed all unnecessary internal `-v1`/`-v2`/driver version suffixes across `providers/chatgpt`, `host/chatgpt-desktop`, `mcp/termux-shell`, `mcp/termux-codex`, `crates/mcp-kit`, and `crates/codex-ext` (normalizing the 7 existing `profile.meta` headers to `codex-manager-profile\n` with `home/` credentials untouched).
- **Post-audit hardening (empirically discovered against live `chatgpt 26.1117.41037` + `codex 0.171.0`):** Switched `createProject` from deprecated `/gizmos/snorlax/upsert` to modern `POST /projects`; wired native ChatGPT `/f/conversation` `local_function_names`/`local_function_signatures` (`type: "kwargs"`) with `exec`↔`codex_exec` wire alias and `600`-char tool description bound; added `encrypted_content` extraction for Codex `0.171.0` `agent_message` subagent payloads; added `bodyTimer` cleanup, `rename_chat` path validation, randomized temp token path, `wait_ms` budget up to `300s` (`240s` `/v1/responses` default) for `xhigh` reasoning, `fileURLToPath` portability in `page_driver.test.mjs`, and `profile_dir` forwarding in `manager.py`.
- **Verification (Android API 37 Termux — source, unit, live `chatgpt 26.1117.41037` + official `codex 0.171.0`):**
  - **Static & unit gates:** `providers/chatgpt` (7/7), `crates/codex-ext` (6 unit + 4 CLI integration), `mcp/termux-shell` (4/4), `mcp/termux-codex` (2 unit + 2 E2E), `crates/mcp-kit` (4/4) all passed `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, and `cargo test --locked` under both `umask 0022` and `0077`; `node --test providers/chatgpt/tests/page_driver.test.mjs` (3/3), `host/chatgpt-desktop/test_manager.py` (4/4), `scripts/test_runit_service.py` (3/3), `.github/scripts/check_doc_links.py` (18/18), and YAML parse of all `.github/workflows/*.yml` passed.
  - **Live 9-case provider combination E2E (`chatgpt 26.1117.41037`):** Passed lazy cold-boot `/v1/responses`, live 25-model `/v1/models` (including `gpt-6-sol-wm`), `previous_response_id` multi-turn, `gpt-5-6-thinking` with zero private `thoughts` leak, SSE `stream: true` tool callbacks, async `wait_ms` + busy rejection (`generation_already_active`) + `observeChat` + programmatic `stopGeneration` + immediate recovery, complex Korean/JSON/Emoji + `readProject`, negative/auth guards (`401`/`400`/`404`/`idle`), and real `codex 0.171.0 exec`.
  - **Live `gpt-6-sol-wm` (`xhigh`) Project + Multi-Role Orchestration E2E:**
    - Created real ChatGPT Project `Stage2-Orchestration-E2E` (`g-p-6b03043ed5888191a5c9809734d3469d`, `POST /control/projects` 2.8s); verified `/control/send` Orchestrator (`6b030448-...`) → Worker (`6b030456-...`) → Orchestrator final conclusion (`FINAL_CONCLUSION: VERIFIED S=2142`) inside the project with zero private `thoughts` leak.
    - Verified unspoiled random-secret-nonce (`NONCE_752A12B2`) end-to-end with **official `codex 0.171.0 exec`** (`model = "gpt-6-sol-wm"`, `model_reasoning_effort = "xhigh"`, `history.project_id = g-p-6b03043ed5888191a5c9809734d3469d`, `73.4s`, `RC=0`): Codex Orchestrator (`6b030b5d-...`, 4 msgs) natively invoked `collaboration.spawn_agent` → spawned a real Codex Worker subagent (`6b030b7a-...`, 3 msgs) → Worker natively invoked `codex_exec`→`exec` (V8 Code Mode `tools.exec_command` reading the secret file) → Orchestrator invoked `collaboration.wait_agent` and output exact `ORCH_VERIFIED=NONCE_752A12B2`, with both sessions saved inside the project.
  - **Live Termux canonical installs:**
    - `~/.local/lib/codex/providers/chatgpt/generations/0.1.0/chatgpt-provider` (`current` + `$PREFIX/bin/chatgpt-provider`, SHA-256 `27574021c62c4040949c04591b63a623a35ae64414b45ae80f9218fe097401c1`; `runit_service.py status` = `activation_state=not_enrolled`, `selected_generation.valid=true`; `chatgpt-provider status` = `rootfs_ready=true`, `profile_exists=true`).
    - Rebuilt & refreshed `$PREFIX/bin/codex-ext` (`f7aea9c0...`), `~/.local/lib/codex/mcp/termux-shell/current/termux-shell-mcp` (`a8b015b1...`), and `~/.local/lib/codex/mcp/termux-codex/current/termux-codex-mcp` (`6e238577...`).
  - **Unverified / remaining Slice 5–6 boundaries:** Slice 6 `@TermuxChatGPT` MCP bridge (`mcp/termux-chatgpt`, remote ChatGPT mobile/web invocation of `/control/*`) is not yet built; signed GitHub Release publication for `chatgpt-provider` requires explicit `workflow_dispatch` (`publish_release=true`).

### 2026-11-22 — Stage 2 Slice 4: Stateless `@TermuxCodex` (`termux-codex-mcp`) over official `app-server daemon`

- **Scope & independent workspace:** Added standalone [mcp/termux-codex](mcp/termux-codex/src/main.rs) (`termux-codex-mcp`, zero shadow task DB, zero `crates/wrapper` coupling) reusing [crates/codex-daemon-client](crates/codex-daemon-client/src/lib.rs) and [crates/mcp-kit](crates/mcp-kit/src/lib.rs), exposing 5 static tools (`codex_status`, `codex_threads`, `codex_turn`, `codex_catalog`, `codex_call`) with uniform immutable annotations (`readOnlyHint=true`, `destructiveHint=false`, `openWorldHint=false`) and 10 strictly disjoint extension actions, normative annex [docs/spec/MCP_CODEX.md](docs/spec/MCP_CODEX.md), and path-triggered CI [termux-codex.yml](.github/workflows/termux-codex.yml). Root `Cargo.toml`/`Cargo.lock` untouched.
- **Post-audit hardening:** Verified against live `codex 0.171.0` `app-server generate-json-schema`; fixed non-poisoning pre-connect `codex_daemon_not_running` + `codex_home`-scoped `call_id` fingerprinting, immediate break with `delivery_unknown` + `poll_error` on mid-poll daemon disconnect, strict key/type/enum/range schema validation across all static tools and 10 extensions (`ReviewDelivery: ["inline"]`), consistent `wait_ms` on `steer`, `codex_status` partial-RPC `warnings` and redacted `{id, name, base_url}` provider summaries, `codex_threads` default `items` limit (`20`) and `512 KiB` response bound, intermediate profile symlink rejection, and unused direct `libc` removal.
- **Verification (Android API 37 Termux):**
  - `mcp/termux-codex` passed `cargo fmt -- --check`, `cargo clippy --all-targets --locked -- -D warnings`, and `cargo test --locked` (2 unit + 2 compiled-binary stdio E2E tests covering CLI flags, 1-shot start/replay, `steer`/`interrupt`, non-poisoning daemon-down retry, `call_id` conflict, mid-poll crash immediate exit, and strict schema self-correction) under both `umask 0022` and `0077`; `check_doc_links.py` (17/17) and YAML checks passed.
  - **Empirical real `codex 0.171.0` `app-server daemon` E2E (disposable `$TMPDIR` + local mock Responses SSE):** Confirmed stopped-daemon `codex_turn` returns `codex_daemon_not_running` without poisoning `call_id`, starting `0.171.0` daemon allows the exact same `call_id` to settle in `200ms` (`HARDENED_0171_OK`, `warnings=[]`), duplicate `call_id` replays from receipt (`llm_calls=1`), and `codex_threads(list|read|items)` + `codex_call(thread_rename|models_list|thread_archive)` succeed against live `0.171.0`.
  - **Live Termux install (`termux-codex-mcp`):** Installed `~/.local/lib/termux-codex-mcp/generations/0.1.0/termux-codex-mcp` (SHA-256 `ff21cc8c3c1408734158ddb687c935fe66c03583342abe953d43a1613b29e54f`), linked `current -> generations/0.1.0` and `$PREFIX/bin/termux-codex-mcp`; verified `runit_service.py status` (`activation_state=not_enrolled`, `selected_generation.valid=true`, zero auto-enrollment, live `0.167.2` services untouched), `termux-codex-mcp --version` (`0.1.0`), and `--describe-tools`.

### 2026-11-22 — Stage 2 Slice 3: `mcp-kit`, `@TermuxShell`, `tunnel-client v0.0.21` & opt-in `runit` 3-state supervision

- **Scope & independent workspaces:** Added shared [crates/mcp-kit](crates/mcp-kit/src/lib.rs) (`libc` + `serde_json` only; uniform immutable annotations, disjoint `catalog`/`call` registry, `O_NOFOLLOW`/`fsync` bounded `call_id` receipt store with settled-only pruning, bounded stdio JSON-RPC loop), standalone [mcp/termux-shell](mcp/termux-shell/src/main.rs) (`termux-shell-mcp`, 1 static tool `termux_shell`, zero Codex/Provider/AppHost dependency), [transport/tunnel-client](transport/tunnel-client/recipe.json) (`v0.0.21` `714f2d67...` + `termux.patch` HTTP/2 half-open PING), [scripts/build_tunnel_client.py](scripts/build_tunnel_client.py) (GitHub API 403 redirect fallback + NDK cross-compile), [scripts/runit_service.py](scripts/runit_service.py) (opt-in 3-state generation supervision with stable `current` links and auto-rollback), normative annexes [docs/spec/MCP_SHELL.md](docs/spec/MCP_SHELL.md) & [docs/spec/SUPERVISION.md](docs/spec/SUPERVISION.md), and independent CI workflows [termux-shell.yml](.github/workflows/termux-shell.yml) & [tunnel-client.yml](.github/workflows/tunnel-client.yml). Root `Cargo.toml`/`Cargo.lock` untouched.
- **Post-audit hardening:** Fixed `mcp-kit` oversized single-line newline boundary and added `prune_settled(512)` preserving unsettled receipts; fixed `termux-shell` persisted `replay_safe: true` parity, pre-receipt `cwd.is_dir()` validation, and compiled-binary stdio E2E test; fixed `runit_service.py` deferred `pending_activation` retry completion and `previous` preservation on failed-upgrade rollback; closed `HTTPError` cleanly in `build_tunnel_client.py`; matched `tunnel-client.yml` Go toolchain to `1.27.4` (`go.mod` `go 1.27`).
- **Verification (Android API 37 Termux):**
  - `crates/mcp-kit` (4/4) and `mcp/termux-shell` (4 unit + 1 compiled-binary stdio E2E) passed `fmt`/`clippy -D warnings`/`test --locked` under both `umask 0022` and `0077`; `scripts/test_build_tunnel_client.py` + `scripts/test_runit_service.py` passed 7/7 (`-W default`, 0 warnings); `check_doc_links.py` (16/16) and YAML checks passed.
  - **Native Android/arm64 `tunnel-client v0.0.21`:** `build_tunnel_client.py --version v0.0.21` passed `TestPoll(RecoversFromSilentHTTP2ConnectionLoss|PreservesHealthyHTTP2LongWait)` (`0.848s`) and all 10 Go packages, producing `0.0.21-termux.0ad5d2db16a9+714f2d67...` (`GOOS=android GOARCH=arm64 CGO_ENABLED=1`) in a disposable output directory.
  - **Live Termux install (`termux-shell-mcp`):** Installed `~/.local/lib/termux-shell-mcp/generations/0.1.0/termux-shell-mcp` (SHA-256 `437820569ca976096fe37e2d30f8ab545f370f2cbd968bbbc01d745dcc11dbf8`), activated `current -> generations/0.1.0` via `runit_service.py` (`activation_state=not_enrolled`, zero auto-enrollment, live `0.167.2` services untouched), linked `$PREFIX/bin/termux-shell-mcp`, and verified `termux-shell-mcp --version` (`0.1.0`) and `--describe-tools`.

### 2026-11-22 — Stage 2 Slice 2: `codex-daemon-client`, standalone `codex-ext` (`cdx`), §7 proof & live Termux install

- **Scope & independent workspaces:** Added reusable [crates/codex-daemon-client](crates/codex-daemon-client/src/lib.rs) (RFC 6455 WebSocket-over-UDS with `/proc/self/fd/<dirfd>/...` Termux FD 35 resolution and advisory `flock` writer-lock probe) and standalone [crates/codex-ext](crates/codex-ext/src/main.rs) (non-PTY launcher with `SIGINT` parent-ignore/child-restore & `termios` restoration, in-TUI `!cdx switch`/`!cdx provider` with `0600` session registry + fail-closed daemon busy check, `profile` CRUD compatible with existing `~/.local/share/codex/manager/profiles/`, `notify` Option A with comment/multiline-preserving `config.toml` `Stop` + `hooks.json` extras), path-triggered [codex-ext.yml](.github/workflows/codex-ext.yml), slimmed [docs/spec/USER_INTERFACES.md](docs/spec/USER_INTERFACES.md), and retired `contrib/codex-account`. Root `Cargo.toml`/`Cargo.lock` (`['codex', 'codex-release-builder', 'libc']`) and `crates/wrapper` were untouched.
- **Post-audit hardening:** Fixed `SIGINT` parent survival (`SIG_IGN` in parent, `SIG_DFL` in child `pre_exec`), fail-closed `DaemonProbe::Error` handling, external-provider -> OpenAI account switch thread-provider guard, symlink `cwd`/`CODEX_HOME` canonicalization, multiline `notify = [...]` TOML replacement, and `codex-ext.yml` publisher sequence monotonicity + post-upload Ed25519 readback.
- **Verification (Android API 37 Termux):**
  - `cargo fmt --check`, `clippy --all-targets --locked -- -D warnings`, and tests under both `umask 0022` and `0077`: `codex-daemon-client` 3/3 + `codex-ext` 6 unit + 4 E2E integration tests passed; `check_doc_links.py` (14/14), YAML parse, and `shellcheck -S info install.sh` passed.
  - **Empirical §7 proof (disposable `$TMPDIR` against live `0.171.0`):** Confirmed `thread-writer-locks/<tid>.lock` `flock` held by idle Daemon A (`B` resume rejected: `already has an active writer`), released immediately after `daemon stop` (~869 ms), followed by Account B resuming the exact same UUID and completing turn 2 with Account A's context preserved; confirmed `thread/name/set` appends to `session_index.jsonl` and updates `state_5.sqlite` `threads.name` (`thread/list` & `codex exec resume <NAME>` immediate visibility) while `thread/archive` authoritatively updates `state_5.sqlite` `archived=1`.
  - **Live Termux install:** Installed release binary `$PREFIX/bin/codex-ext` (1,227,416 B, SHA-256 `138def40cfacbda605c6e592b77d645dbd87ca77185db7cd31ba6a5b2e9def5c`) + symlink `$PREFIX/bin/cdx -> codex-ext`; verified `cdx --version` (`0.1.0`), `cdx -- --version` (`codex-cli 0.171.0`), `cdx status` (read-only detection of all 8 profiles and running `janmori101`/`szob` daemons), and `$PREFIX/bin/codex wrapper status --verify --json` (`qualified=true`) unchanged. Interactive Termux UI `/copy` clipboard paste-back remains a physical-UI observation boundary.

### 2026-11-22 — Stage 2 Slice 1: Wrapper slimming, Ed25519 `SHA256SUMS.sig` signed release & live Termux cutover

- **Scope & commits (`3a87150e..3ea207cb` on `main`):** Finalized zero-coupling Stage 2 architecture across [SPEC.md](SPEC.md), [docs/ROADMAP.md](docs/ROADMAP.md), [docs/spec/DELIVERY.md](docs/spec/DELIVERY.md), [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md), and the [retired Stage 2 work order](docs/history/STAGE2_MILESTONES.md); removed unused Phase 2.1 `system_config.rs` and `extensions.rs` hooks (`-1,905` lines in `258fe62b`, leaving `$PREFIX/bin/codex` as a pure Stage 1 runtime + `update`/`rollback`/`doctor`/`wrapper status`); implemented universal lightweight Ed25519 `SHA256SUMS.sig` signing, anti-rollback `sequence`, and key-rotation verification across [release_sign.py](.github/scripts/release_sign.py), [publish_wrapper_release.py](.github/scripts/publish_wrapper_release.py), [wrapper.yml](.github/workflows/wrapper.yml), and [install.sh](install.sh); provisioned isolated Stage 2 secret `CODEX_SIGNING_KEY` and pinned [release-public-key.pem](release/release-public-key.pem) (`0df170dae2d7234b4425fcdddaa4bd9b6e2104794d0541955b022631b1105fca`).
- **Signed CI & public release publication:** [Wrapper CI run 44997871190](https://github.com/humtr/codex/actions/runs/44997871190) on `main` (`3ea207cb`) passed all 4 jobs (`checks`, `android-package`, `android-smoke` including 23/23 `test_install_recovery.py` signed/tamper/rollback/rotation/SIGKILL tests and full upstream Bionic qualification, and `release`). `publish_wrapper_release.py` verified `RELEASE_VERIFIED repo=humtr/codex tag=wrapper source=3ea207cb77e42bd9c22a92425d42c110bc891a17 archive_sha256=68033d9cf26f83d08919c287e8827c22f58272935e68a634831def16d9eadb8f prerelease=False signed=True sequence=1 key_sha256=0df170dae2d7234b4425fcdddaa4bd9b6e2104794d0541955b022631b1105fca` across all 5 assets (`codex-wrapper-aarch64-linux-android.tar.gz`, `.sha256`, `install.sh`, `SHA256SUMS`, `SHA256SUMS.sig` 64 B).
- **Live Termux API 37 signed cutover:** Ran `sh ./install.sh --release wrapper --require-signed --skip-upstream`. Verified real Ed25519 signature (`sequence 1`), archive SHA-256, and `bin/codex` SHA-256 (`f83899d654a23baf83a5bb995c3395e3a74c07f7fca8a77b0810a7069c7d2811`); persisted `~/.local/share/codex/wrapper/release-sequence` (`1`) and `~/.local/lib/codex/wrapper/release-public-key.pem` (`0df170da...`); confirmed `codex --version` (`0.171.0`), `codex wrapper status --verify --json` (`active`, `0.171.0-d8c43b-2`, rollback `0.170.0-3f7e1a-2`, `qualified=true`), `codex doctor --json` (`termux_wrapper=healthy`, `summary=healthy`), retired `codex wrapper setup` returning `exit 2`, and zero disruption to live `0.167.2` `runit` services (`codex-chatgpt-host` PID `2605`, `codex-chatgpt-provider` PID `19984`, `codex-chatgpt-tunnel` PID `2554`, `tdev` PID `2557`, `tdev-tunnel` PID `4678`, `tdev-tunnel-conn_*` PID `3868`).

### 2026-11-22 — End-of-row atomic progress redraws and per-profile daemon tree summary

- [Wrapper CI run 44921454220](https://github.com/humtr/codex/actions/runs/44921454220)
  qualified source `ce439293` (building on `c1aeb52f` end-of-row atomic progress
  redraws and `751b863d` rate-limit fallback / ancestor deferral) with official
  `0.170.0` and `0.171.0` on the Android Bionic runner (`checks`,
  `android-package`, `android-smoke`, `release`). Published stable `wrapper` tag
  (`prerelease=false`), archive SHA-256 `2cc5410a...`, and `bin/codex` SHA-256
  `56b4adfa...` matched independent readback.
- Installed the published Wrapper (`56b4adfa...`) via
  `install.sh --skip-upstream`, ran `codex update --rollback` to leave the live
  runtime on `0.170.0-3f7e1a-2` (`previous=0.171.0-d8c43b-3`), and cleared
  `update-check.cache`. All three live managed daemons (`default` PID `21082`,
  `janmori101` PID `21219`, `szob` PID `21352`) reported `running 0.170.0` with
  matching executable `ino=650222`.

### 2026-11-21 — Flicker-free progress, concise daemon summary, ancestor deferral & GitHub API rate-limit fallback

- [Wrapper CI run 44911991173](https://github.com/humtr/codex/actions/runs/44911991173)
  qualified source `751b863d` with official `0.170.0` and `0.171.0` on the
  Android Bionic runner (`checks`, `android-package`, `android-smoke`, and
  `release`). Published stable `wrapper` tag (`prerelease=false`), archive
  SHA-256 `4e502398...`, and `bin/codex` SHA-256 `288e21d5...` matched
  independent readback.
- Isolated Android API 37 verification passed all 38/38 `qualify_wrapper.py`
  steps (`input_id: 037b592f...`), 113 Wrapper + 19 release-builder unit tests
  (umasks `0022`/`0077`), 47 `.github/scripts` Python tests, `strace` single-write
  and 4-byte in-place spinner checks (0 `\r\x1b[2K` clears during animation),
  42-column PTY prompt/resize regressions, WS-over-UDS self-daemon deferral and
  external rerun, and live GitHub API `403` rate-limit redirect fallback.
- Installed the published Wrapper binary (`288e21d5...`) via
  `install.sh --skip-upstream`, then ran `codex update --rollback` to leave the
  live runtime on `0.170.0-3f7e1a-2` (`previous=0.171.0-d8c43b-2`) with
  `update-check.cache` cleared for interactive upgrade testing. All three live
  daemons (`default` PID `30780`, `janmori101` PID `30937`, `szob` PID `31042`)
  reported `appServerVersion=0.170.0` with matching executable `ino=650222`, and
  `codex update --check --json` succeeded under live GitHub API `403` rate
  limiting (`status=available`, `upstream_version=0.171.0`).

### 2026-11-21 — Daemon synchronization release and live Wrapper cutover

- [Wrapper CI run 44871873250](https://github.com/humtr/codex/actions/runs/44871873250)
  qualified source `0cdb8eda` with official `0.171.0` and `0.170.0` on
  the Android Bionic runner. The public stable `wrapper` tag, release
  notes and four assets matched the verified CI output.
- The API 37 device qualified the existing runtime with the released
  Wrapper, installed the verified binary with `install.sh --skip-upstream`,
  then ran `codex update --upstream --version 0.171.0`. The updater returned
  already current with zero warnings. The activation-state SHA-256,
  generation slots, daemon links, PIDs and runtime inode matched the
  pre-install snapshot. Each of the three daemon control endpoints
  independently reported `backend=pid`, `status=running`, and
  `appServerVersion=0.171.0`.

### 2026-11-21 — Independent review of `c64d3e31..5994956a` (isolated Android API 37 Termux)

- **Reproduced on the baseline:** An actual `0.170.0` legacy daemon remained
  running after `update --path` activated `0.171.0`. Creating the dedicated
  link first hid its `app-server.pid` from upstream lifecycle commands.
- **Review candidate based on `5994956a`:** Qualification passed 38 steps with
  the official `0.170.0` and `0.171.0` archives on a 4 KiB kernel
  (input `885bf9e0...`, Wrapper binary SHA-256 `a789c3de...`). Both legacy
  and modern daemons changed PID and executable across rollback and restore.
  The replacements retained the SQLite binding and feature override. A stale
  PID received link alignment only, and a directory at the daemon package
  `current` path was preserved and reported while other homes completed.
  Read-only doctor checks reported a healthy Wrapper and app-server. The
  account-free upstream auth check returned exit 1. The same 38 steps passed
  with `0.169.1` as the rollback target (input `88c686b1...`).
- **Additional isolated checks:** Official already-current updates aligned a
  dedicated pin, preserved a matching live PID and reported a busy upstream
  daemon lock. After upstream `daemon update --from-cli` copied a package,
  the Wrapper re-aligned the stopped home's link and retained that copy.
  A concurrent Wrapper update was excluded while status succeeded.
  Native `command/exec` in stdio and through a restarted daemon,
  and the TUI `!` shell resolved bare `codex` to the isolated Wrapper.
  The daemon test also resolved packaged `rg` and the arg0 `apply_patch`
  alias, and both companion binaries passed qualification.
- **Startup and workspace gates:** PTY tests covered all three TTY gates,
  opt-out, a shared five-second discovery budget with delayed transport,
  invalid and future-dated cache entries, decline, EOF, update failure, and
  exec of `0.171.0` after confirmation. The cached repeat launches made zero
  discovery requests. Rust fmt, Clippy with `-D warnings`, doc links, and
  108 Wrapper plus 19 builder tests passed under both requested umasks.

### 2026-11-19 — Stage 1 Graduation: `origin/main` Promotion, Stable `wrapper` Release & Live Device Cutover (CI + actual Termux API 37)

- **Branch promotion & archival:** Archived legacy `0.167.2` `origin/main` (`fcefb8b3`) to `origin/archive/main-legacy-0.167.2`; retired ephemeral `HANDOFF.md` after archiving its WP0–WP6 summary into [STAGE1_MILESTONES.md](docs/history/STAGE1_MILESTONES.md); fixed Ubuntu CI `shellcheck` SC2015 (`1e714e8b`) and Linux CI test portability (`e76a3ae4`, `104` Wrapper + `19` builder tests = `123` passed); promoted `rewrite/wrapper` cleanly to `origin/main` at `135fccd1` (zero force-push, exact tree parity).
- **CI & stable public release publication:** [Wrapper CI run 44464940477](https://github.com/humtr/codex/actions/runs/44464940477) on `main` (`135fccd1`) passed all 4 jobs: `checks`, `android-package`, `android-smoke` (including full `qualify_wrapper.py` cross-version `0.169.1`↔`0.170.0` upstream qualification on Android Bionic), and `release` (`publish_wrapper_release.py` verified peeled tag, notes, `prerelease=False`, and exact asset readback). Published stable GitHub Release `wrapper` (`isPrerelease: false`, `codex-wrapper-aarch64-linux-android.tar.gz` 966,831 B SHA-256 `636c0a5a0cc3...`, `bin/codex` SHA-256 `dfdcbbc25be2...`, `install.sh` SHA-256 `40c57c2f2df6...`).
- **Live device cutover (actual Termux API 37):** Saved pre-cutover backup (`8701483e...`) and state snapshot to `0700` `$HOME/.local/share/codex/wrapper-recovery/20261119-stage1-graduation/`. Executed the published `https://github.com/humtr/codex/releases/download/wrapper/install.sh` cleanly: verified release archive & `bin/codex` SHA-256, installed `$PREFIX/bin/codex` (`dfdcbbc25be2...`), and retained the compatible active generation offline (`retaining the qualified active upstream generation`). Verified byte-identical `activation-state` (`35540262...`), unchanged `current=0.170.0-3f7e1a` (`ino=9717389`) & `previous=0.169.1-3de502`, undisturbed live daemons (`PID 20823, 21394, 21500`) and 5 `runit` services, and `codex doctor --json` returning `exit 0` (`termux_wrapper=healthy`, `app_server=ok`, `summary=healthy`, parity `0/0/0/3`) across `default`, `janmori101`, and `szob`.

### 2026-11-19 — Stage 1 Completion Candidate WP1–WP6 (isolated Android API 37 Termux)

Qualified on `rewrite/wrapper` (`e9d323ba` Rust/installer/CI/docs sources + `a1ee6590` `qualify_wrapper.py` read-only boundary fix; release binaries `target/release/codex` 1.6 MiB SHA-256 `dd0a9ce4ca24...` and `target/release/codex-release-builder` 824 KiB SHA-256 `68bee9bea5d5...`; `Cargo.lock` locked to `['codex', 'codex-release-builder', 'libc']`). Live `$PREFIX/bin/codex` (`addffcba`), live generations, 3 daemons, and 5 `runit` services were **not modified**:

- **Workspace, installer & CI gates:** `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked` under both umask `0022` and `0077` (**`103` Wrapper + `19` release-builder = `122` tests passed**), `sh -n` and `shellcheck` on `install.sh` and `contrib/codex-account`, Ruby YAML parse of `.github/workflows/*.yml`, `python3 -m unittest discover -s .github/scripts -p 'test_*.py'` (**`47` tests passed**, including real kernel zombie `Z` installer recovery, prerelease transitions/readback mismatch, and untracked doc-link rejection), and `check_doc_links.py` (`14/14` tracked docs).
- **Full isolated Android API 37 qualification (`qualify_wrapper.py`, kernel `6.1.162-android14-11-34343818-abS928NKSU6EZJB`, `page_size: 4096`, `android_api: 37`, no `openssl` in isolated `$PREFIX/bin`):** Passed all **29/29 steps** (`status: passed`, `input_id: fe3c859be271...`) across official `openai/codex` `0.169.1` (`3de502ad...`) and `0.170.0` (`3f7e1ae3...`):
  - Pure-Rust SHA-256 verification and `generation.meta` companion digest verification (`bin/codex-code-mode-host`, `bin/codex-exec-server`); 16 KiB ELF alignment `true` across Wrapper, `codex`, `codex-code-mode-host`, and `codex-exec-server`.
  - Managed daemon `start`/`version`/`stop` sharing standalone `current`, native `app-server` stdio RPC (`initialize`, `config/read`, `command/exec`), and PTY startup (`rendered: true`, 8 s, 12,348 bytes).
  - Read-only `codex doctor --json` (`doctor` and `shared-doctor`, both `termux_wrapper = healthy`, `app_server = ok`): verified zero writes to `$HOME`, `$PREFIX`, `$CODEX_SHARED_HOME`, Wrapper-managed daemon files, and non-`tmp/arg0-linux` profile entries; separate check with `wrapper/config/` deleted confirmed `termux_wrapper = healthy` + absent-config warning without recreating `wrapper/config/`.
  - `CODEX_SHARED_HOME` multi-profile (`CODEX_SHARED_EXTRA_FILES=custom.config.toml`): fast `--version` and `doctor` left profiles/shared home untouched; `features list` linked `custom.config.toml` and merged `session_index.jsonl` preserving both same-thread rename rows (`session_index_rows: 2`) while keeping `auth.json` isolated per profile.
  - Offline Wrapper-only upgrade (`install.sh --binary` with distinct-SHA ELF overlay `ad6ca8df...` and poisoned `curl`): retained `current=0.170.0-3f7e1a` and `previous=0.169.1-3de502` offline without invoking `curl`, followed by cross-version `--rollback` (`0.169.1-3de502` ↔ `0.170.0-3f7e1a`).
- **Explicit `--force-rebuild` & `--version` latency (same disposable root, cleaned afterward):** `codex update --version 0.170.0` skipped rebuild after Wrapper SHA change (`Codex 0.170.0 is already up to date.`), `codex update --version 0.170.0 --force-rebuild` rebuilt and activated `0.170.0-3f7e1a-2` (`previous=0.170.0-3f7e1a`), and `--rollback` restored `0.170.0-3f7e1a`. Ten-run `codex --version` latency measured `33.82 ms` median (`30.59–45.04 ms`) vs `14.25 ms` median (`12.13–26.18 ms`) raw patched `bin/codex --version` (down from ~40.2 ms pre-WP3 baseline).

### 2026-11-19 — Live Device Maintenance: Shared Daemon Unification & DB Reconciliation (actual Termux API 37)

Executed against the live Termux device following the archived maintenance runbook (`git show b7b48fca:docs/plan/STAGE1_DEVICE_MAINTENANCE_WORK_ORDER.md`):

- **Clean shutdown & backup:** Stopped all 47 Codex / `167.2` / DB-holding processes cleanly (`sv down` on `tdev-tunnel`, `tdev-tunnel-conn_*`, `codex-chatgpt-tunnel`, `codex-chatgpt-provider`, `codex-chatgpt-host`; `app-server daemon stop` on 3 homes; identity-verified `SIGTERM` on orphaned `167.2` processes; 0 `SIGKILL`; verified 5 s quiet with 0 open DB writers). Full SQLite backup API snapshots, rollout manifests, and pre/post state receipts archived under owner-only (`0700`) `$HOME/.local/state/codex-stage1-maint-20261119T0825KST/` (854 MiB).
- **Clone pre-verification:** Tested link cutover/rollback, 72-path SQLite single-transaction update, `167.2` 62-migration DB schema compatibility, and `session_index.jsonl` append-only rename semantics in an isolated disposable clone before touching live state.
- **Live DB & rollout cleanup:** Repaired 72 broken `~/.codex-profiles/*` rollout paths in one SQLite transaction (69 active, 3 archived; `PRAGMA quick_check = ok`; `wal_checkpoint(TRUNCATE)` clean); isolated duplicate rollout pair `01a0fe94-...` by moving the unreferenced 144 MiB copy outside `sessions/` while keeping the DB-referenced 417 MiB file; left 3 `/usr/tmp/*` missing-file DB rows and `session_index.jsonl` untouched.
- **Shared binary cutover & restoration:** Atomically linked `$CODEX_HOME/packages/app-server-daemon/current -> $HOME/.codex/packages/standalone/current` (`0.170.0-3f7e1a`, `dev=65098, ino=9717389`) on `default`, `janmori101`, and `szob`, restarted sequentially (PIDs `20823`, `21394`, `21500` sharing `ino=9717389` with isolated control sockets and updated FD 34), verified `codex doctor --json` across all 3 homes (`missing_active=0`, `missing_archived=0`, `dup_ids=0`, `stale_rows=3`, `termux_wrapper=healthy`), and restored all 5 `runit` services (`167.2` preserved at `ino=489102`).

### 2026-11-19 — Upstream Thread Lifetime & Rollback Recovery Candidate (isolated API 37 Termux)

Qualified on `rewrite/wrapper` above `0391865acfd9` (later included in `addffcba`). Leaves idle thread unload delay (`thread_unload_delay_secs`) to upstream while removing the historical global FD 34 `= 0` override:

- Disposable API 37 Termux homes ran managed daemons from `0.170.0-3f7e1a` against a local Responses HTTP server (closing sender immediately on `thread/queue/add` receipt): candidate completed **5/5** with fresh FD 34 and **5/5** after replacing legacy managed FD 34, whereas old FD 34 `0` and explicit user-config `0` produced **5/5** `turn_aborted`.
- Normal launch replaced legacy managed FD 34 config and preserved inode/mtime/ctime on subsequent launches; upstream `config/read` confirmed user-config and `-c` precedence (including explicit `0`).
- Release-binary fault tests verified rollback recovery after `update --path` reselection + journal-delete failure, pre/post-rename termination, state-dir sync failure, and ambiguous legacy journal recovery.
- Workspace gates passed: `cargo fmt`, `clippy -D warnings`, doc links, and `cargo test --workspace --locked` (`72` Wrapper + `9` release-builder tests under both umask `0077` and `0022`). Evidence root: `$PREFIX/tmp/wrap-idle.1tyULG/`.

### 2026-11-19 — Default Daemon Package Selection, rev 2 (isolated API 37 Termux)

Qualified on `rewrite/wrapper` above `0391865acfd9` with official `0.169.1-3de502` and `0.170.0-3f7e1a` archives in disposable Termux homes:

- Verified parity with unmodified `0.170.0` selection precedence across all 4 legacy and 4 modern PID/stderr traces and existing/broken dedicated `current` links.
- Two fresh homes started independent daemon PIDs and control sockets sharing one executable inode without creating dedicated `releases/`; explicit `daemon update --from-cli --yes` pinned a dedicated package preserved across Wrapper update and rollback.
- Help (including `-hV`), version, queries, `--no-daemon`, `--remote`, and direct `--from-cli` skipped link creation; literal `-- -hV` and attached option values containing `-hV` still launched TUI.
- Release-binary fault injection covered link rename/dir sync failures, pre/post-state-rename termination, state-dir sync failure (exit `3` + recoverable journal), direct rollback re-entry, and non-symlink preservation/rejection at `standalone/current`.
- Workspace tests passed under umasks `0077` and `0022`. Evidence root: `$PREFIX/tmp/daemon-fix-o8w22wke/`.

---

## Historical Archives

- **2026-11-22 – 2026-11-23 Stage 2 Slices 1–7, signed releases and operational verification** [docs/history/STAGE2_MILESTONES.md](docs/history/STAGE2_MILESTONES.md).
- **2026-11-16 – 2026-11-18 Stage 1 P0 cutover, installer/journal/release-provenance faults, Issue #33 FD 35 fix, Stage 2.1 snapshot engine, and retired work orders:** [docs/history/STAGE1_MILESTONES.md](docs/history/STAGE1_MILESTONES.md).
- **Pre-rewrite generation admission and legacy `rewrite/rust-core` (`0.167.x`) archives:** [docs/history/GENERATION_ADMISSION.md](docs/history/GENERATION_ADMISSION.md) · [docs/history/LEGACY_RUST_CORE.md](docs/history/LEGACY_RUST_CORE.md).
