# Linux ChatGPT AppHost Generations & Slim `chatgpt-provider`

**Status:** Normative annex of [SPEC.md](../../SPEC.md).
Implemented in [host/chatgpt-desktop/manager.py](../../host/chatgpt-desktop/manager.py)
and [providers/chatgpt/src/main.rs](../../providers/chatgpt/src/main.rs).

---

## 1. Immutable AppHost Generations (`host/chatgpt-desktop`)

- **Unified canonical XDG `codex/` namespace (escapes `~/.cache`):** Generations live under `~/.local/lib/codex/host/chatgpt-desktop/`:
  - `generations/<ver>/rootfs/` — immutable Debian 13 ARM64 rootfs with the
    official GPG-verified OpenAI `chatgpt_<ver>_arm64.deb` installed.
  - `generations/<ver>/generation.json` — provenance manifest (`version`,
    `deb_url`, `deb_sha256`, `deb_bytes`, `inrelease_date`,
    `keyring_fingerprint`).
  - `current -> generations/<ver>` and `previous -> generations/<old-ver>` —
    atomic symlinks with rollback (`activate` / `rollback`) preserving
    `previous` on re-activation of the current version.
  - `viewer/` — optional static noVNC assets served over capability-scoped
    `/viewer/{capability}/...` (provisioned/verified via
    `host/chatgpt-desktop/manager.py viewer --source <dir>`).
- **Separated persistent login profile:** User login state (both Electron's
  Chromium profile `chromium/` and its embedded Codex app-server OAuth store
  `.codex/`) lives solely under `~/.local/share/codex/chatgpt-desktop/profile/`
  (`0700`, `flock`-leased via a sibling `.lock` file by the AppHost supervisor;
  bounded boot/crash diagnostics written to `profile/host.log`) and is
  never deleted during rootfs installation, activation, or rollback.
- **Cryptographic admission:** `InRelease` is verified via `gpgv` against the
  pinned RSA-4096 keyring (`host/chatgpt-desktop/openai-apt-keyring.gpg`,
  fingerprint `3BFA0E4AE8B8CC16A2D9BA684A3B4A566C4660E4`), `Packages` SHA-256
  and size are verified against `InRelease`, and the downloaded `.deb` SHA-256
  and byte size are verified against `Packages`.
- **Canonical configuration & state:**
  - Config: `~/.config/codex/providers/chatgpt.toml`
  - Ephemeral runtime state & `0600` bearer token: `~/.local/state/codex/providers/chatgpt/`
  - Legacy `0.167.2` artifacts are never mixed into Stage 2 directories.

---

## 2. Slim `chatgpt-provider` (`providers/chatgpt`)

- **Single CDP pipe owner:** `chatgpt-provider` controls one Linux ChatGPT AppHost
  instance (`Xvnc` + `openbox` + `proot` + Chromium `--remote-debugging-pipe`
  FD 3/4 + allow-listed Android OAuth browser relay via `termux-am-socket`)
  through a private child mode of the same executable. The child mode owns
  process startup, the profile lease, and exit collection. It detects Provider
  disconnection independently of CDP and holds the lease through descendant
  reaping and X11 cleanup. Startup preserves Chromium's singleton artifacts
  and fails closed on a live ChatGPT/Electron singleton while treating dead or
  recycled non-ChatGPT PIDs in `SingletonLock` as stale. The installed PRoot
  requires no patch. Direct supervisor death is outside its cleanup guarantee.
  Shutdown gives Chromium a bounded exit window after `Browser.close` before
  escalating through PRoot. The account-free generation probe uses the same
  child mode.
- **Clean-room `page_driver.js` (zero DOM clicking / zero computer-use):**
  Minimal greenfield page driver injected via CDP `Runtime.evaluate`, using
  Electron's native `connect-app-host` Cap'n Web `services.httpFetch` service
  (`X-OpenAI-Attach-Auth: 1`) on `app://-` (and same-origin
  `fetch("/backend-api/...")` on `https://`) for chat/project/status reads, and
  dynamically discovering the app's own internal `CompletionClient`
  (`startCompletionStream`, `cancelStream`, `stopCompletion`) via React Fiber
  `scope` and Rspack's standard `__webpack_require__` export for turn
  execution, streaming observation, and cancellation — completely independent of
  onboarding screens, tutorials, modals, or DOM layout. Never modifies app
  files, never reads or exposes cookies/auth tokens, never hardcodes chunk
  hashes or minified symbol names, and never synthesizes private reasoning.
- **Type 1 (`codex -p chatgpt`) surface (MCP Single-Turn):** Localhost-only
  `GET /v1/models` and `POST /v1/responses` (monotonic OpenAI Responses SSE or
  JSON) dispatching **one prompt-box-identical `POST /f/conversation` per user
  turn** with **zero `local_function_names` / `local_function_signatures`**.
  Host execution and multi-step tool loops are performed server-side inside
  that single `/f/conversation` turn through registered ChatGPT remote MCP
  connectors (`TermuxShell`, `TermuxCodex`, `TermuxChatGPT`, and
  `TermuxOrch`). The retired client-side `local_function_names` /
  `pending_calls` implementation is archived for developer reference in
  [providers/chatgpt/archive/local-tools](../../providers/chatgpt/archive/local-tools/README.md),
  and `turn.rs` preserves a clean `TurnStrategy` extension seam for future
  execution modes. The
  Provider also maintains a Codex-compatible catalog file at
  `<state_dir>/model-catalog.json` (`0600`, atomic; seeded from configured
  fallback models at startup and refreshed whenever `GET /v1/models`,
  `GET /control/status`, or eager startup receives live AppHost `/models`
  metadata) and emits `model_catalog_json = "<state_dir>/model-catalog.json"`
  in `print-codex-profile` so Codex `/model` displays ChatGPT models instead
  of default OpenAI slugs. The visible catalog excludes Codex-only `*-wm` /
  `*-codex` / `hidden`-tagged slugs, collapses `-instant` / `-thinking` /
  `-t-mini` variants into a single latest-first entry per model family with
  selectable reasoning efforts and `"supports_reasoning_summary_parameter": true`,
  and routes `/v1/responses` dispatches to the
  corresponding ChatGPT variant slug according to the requested reasoning
  effort.
- **Single-adapter streaming lifecycle (`Page Driver -> LaneMailbox -> TurnAssembly -> ResponsesEncoder`):**
  The Provider normalizes ChatGPT AppHost completions through one lifecycle
  pipeline (`catalog.rs`, `session.rs`, `turn.rs`, `stream.rs`, `responses.rs`)
  shared by both streaming (`stream=true`) and non-streaming (`stream=false`)
  `/v1/responses` execution:
  - `page_driver.js` requests `supported_encodings: ["v1"]` and observes
    messages through `onDecodedEvent` as the single authoritative stream path
    (with an `onUpdate` fallback only when `onDecodedEvent` has not fired).
  - Messages are normalized per item ID and semantic channel (`reasoning_summary`,
    `commentary`, and `answer`), distinguishing uncommitted `Mutable` snapshots
    (`in_progress`) from append-only `Streaming` prefixes and `Committed`
    outputs (`status !== "in_progress"`, `end_turn === true`, finished
    `thoughts` entries, or turn settlement). Uncommitted snapshot revisions,
    insertions, or deletions update in-place without failing settled streams.
  - Simultaneous `commentary` + `reasoning` + MCP progress passthrough:
    - Public interim progress messages (`channel === "commentary"`,
      `is_thinking_preamble_message === true`, `status === "finished_partial_completion"`,
      or `end_turn === false`) emit as `type: "message", phase: "commentary"`
      so Codex TUI displays step-by-step progress inline.
      Each ChatGPT message `msgKey` owns at most one canonical text item ID so
      late status updates never duplicate an already-committed message (only
      genuine strict-suffix extensions emit incremental follow-up items), and
      `TurnAssembly` suppresses exact duplicate commentary items and repeated
      commentary prefixes in `final_answer` without discarding unique content.
    - Remote MCP tool calls and completions observed in the `onDecodedEvent`
      stream (`author.role === "tool"` or assistant calls targeting a remote
      MCP recipient) emit concise live progress summaries (`reasoning_summary`)
      and reset the upstream idle timer (`activity`) so long single-turn
      builds/tests remain visibly active in Codex TUI.
    - Native-first DIL-to-GFM presentation projection (`presentation.rs`):
      user-visible `commentary` and `final_answer` text uses one source-aware
      semantic projection for stable streaming prefixes and settled responses.
      Ordinary Markdown, code, literal and unknown ordinary markup remain
      intact. Recognized display components retain supplied labels, values,
      units, links and ordering in native Markdown or explicit readable
      fallback; unresolved values and unsupported capabilities are identified,
      not fabricated or silently dropped. Only deterministic local literals
      and explicitly supported references/operations may be interpreted; no
      arbitrary script, network access, callbacks, tools, or UI actions.
      Controls are static unless an actual upstream-owned interaction exists.
      Raw AppHost history, reasoning and tool payloads are unchanged.
    - Full AppHost reasoning emitted by the model (`thoughts[*]` entries with
      `summary` + `content` / `chunks`, formatted as `**{summary}**\n{content}`
      when both are present; `thoughts` `parts`/`content`; `channel === "analysis"`
      text; `reasoning_recap` such as `Worked for Ns`; deduplicated
      `metadata.{rich_reasoning_titles, reasoning_titles, reasoning_title}`)
      emits simultaneously as `type: "reasoning"` (`summary_text`) for Codex's
      live working status line and `Ctrl+T` transcript. The Provider never
      synthesizes private reasoning, never leaks user echo messages or raw tool
      payloads into reasoning, and keeps `readChat` focused on visible
      `user`/`assistant` conversation history.
  - Background utility isolation: Codex background utility calls (identified via
    `x-codex-turn-metadata` `request_kind != "turn"` or `memory_consolidation`,
    or tool-less title/memory prompts) bypass session leasing, project pinning,
    and `response_threads.json` persistence, and map unknown non-ChatGPT utility
    model slugs (such as Codex-only utility slugs) to an available ChatGPT slug so they
    never trigger `503 response_thread_busy` against the active user turn.
  - `cdp.rs` delivers structured `StreamEvent` items through a per-lane
    non-blocking `LaneMailbox` that coalesces rapid mutable snapshots per item
    without stalling sibling lanes and queues committed/terminal events
    losslessly.
  - `stream.rs` maintains one authoritative `TurnAssembly` (`Mutable`,
    `Streaming`, `Committed`) and one `ResponsesEncoder` that emits immutable
    Responses frames for committed items and explicitly stable append-only
    prefixes. Arbitrary mutable snapshots remain buffered; a violation of an
    already emitted prefix freezes the published prefix rather than corrupting
    SSE. For presentation, buffer only incomplete render units; subsequent
    ordinary prose must stream once ambiguity is resolved. A settled strict
    extension must retain every projected byte already emitted and add the
    remaining suffix. A projection inconsistency at settlement fails explicitly
    instead of silently committing truncated output. Background utility text
    bypasses presentation. SSE and JSON share the encoder; the committed Codex
    prefix uses exactly its emitted text, not a second projection. If a turn
    supplies only a
    user-visible summary with no answer, the same text is returned as the final
    answer. The stream emits 12-second SSE keepalives with a configurable
    upstream inactivity limit (`[server] sse_idle_secs`, default 660s / 11
    minutes, valid range `30..=7200`) and a two-hour total limit.
  - Completed Responses include upstream token usage when explicitly reported;
    otherwise a deterministic estimate based on the UTF-8 JSON size of logical
    input, instructions, and emitted output (four bytes per token, rounded up)
    is marked as estimated. This is not a billing measurement or proof of
    backend context capacity. The catalog retains the 1M context, 95% effective
    window, and 800k auto-compact baseline.
  - Server `429` / `Retry-After` deadlines are enforced centrally at
    `page_driver.js`'s `apiFetch()` gate (default minimum `60s` cooldown when
    upstream omits `Retry-After`): subsequent backend reads, observations,
    status refreshes, and dispatches fail fast with `backend_http_429` and
    `retry_after` until expiry without automatic replay. `status()` caches
    `/me` and `/models` with a bounded TTL so repeated local health/account
    checks do not amplify backend traffic. Legacy `[server]
    min_request_interval_ms` and `[history] structured_native_return` config
    keys are accepted as ignored compatibility fields so existing
    `chatgpt.toml` files load cleanly.
  - Client disconnect before remote settlement cancels only that lane via
    `stop_generation` without recording continuation. Remote settlement
    disarms cancellation, reconciles the final assembly, and persists up to 256
    scoped continuation entries in `response_threads.json` (`0600`, atomic under
    `${XDG_STATE_HOME:-$HOME/.local/state}/codex/providers/chatgpt/`, or the
    configured state directory) **before** final SSE frame delivery so a local
    client disconnect after remote completion preserves the conversation pin and
    rejects duplicate replay inputs (both array and string inputs) with
    `response_input_already_sent`.
  - Account and project are checked before using a stored conversation. An
    incompatible Codex input prefix starts a new ChatGPT conversation under the
    same account pin.
- **Single-Turn MCP Execution & Context Bridge (`turn.rs` + `session.rs`):**
  All `/v1/responses` user turns dispatch as a single native ChatGPT
  `CompletionClient.startCompletionStream` request (identical to typing in the
  regular ChatGPT prompt box, preserving `Worked for Ns >` and expand/collapse
  in the ChatGPT App UI):
  - User content, `agent_message` items, and authorized `developer` context
    remain distinct. Unsupported content types (such as `input_image`) fail
    closed with HTTP 400 `native_content_unsupported`.
  - Supported Codex `developer` messages travel as hidden AppHost `developer`
    messages in input order. Codex's automatic machine-generated `role: "user"`
    preambles (`# AGENTS.md instructions`, `<environment_context>`,
    `<collaboration_mode>`, `<turn_aborted>`, `<subagent_notification>`) remain
    `author.role = "user"` (zero privilege escalation) and are marked
    `is_visually_hidden_from_conversation: true` so ChatGPT reads them while
    showing only genuine human prompts in the conversation UI.
  - Request-level `instructions` travel on user turns as a trailing hidden
    `developer` message with `exclude_after_next_user_message: true` so they
    apply to the turn and expire on the next user message. When `[mcp]
    bridge_guidance = true` (default), `turn.rs` appends a concise one-turn
    MCP execution bridge directive carrying the resolved workspace `cwd` and
    the active Stage 2 MCP connector roles (`TermuxShell` for host shell/file
    work with required `call_id`, `TermuxChatGPT` for other ChatGPT
    chats/projects, `TermuxCodex` for independent Codex daemon threads) so
    ChatGPT does not attempt retired local CLI function tools.
  - Per-`chat_id` single-flight in `page_driver.js` prevents self-chat and
    cyclic re-entry (`generation_already_active`, returning a clear recovery
    error to the MCP call without interrupting the active turn) while allowing
    unbounded parallel sibling lanes up to `MAX_CONCURRENT_GENERATING`. Native
    `codex app-server` (`TermuxCodex`) runs on `~/.codex/config.toml` rather
    than `~/.codex/chatgpt.config.toml`, keeping the daemon leaf physically
    separate from the `:4319` Provider loop.
  - The Page Driver trusts AppHost's natural `onDecodedEvent` / `onComplete` /
    `onError` lifecycle without redundant `GET /conversation/{id}` polling and
    caches verified account identity (`GET /me`) and active connector summaries
    with bounded TTLs.
  - On client disconnect (`Ctrl+C`), `CancelTurnGuard` cancels only the active
    lane without locking the session. When Codex compacts or resets history
    without `previous_response_id`, the Provider hands off cleanly to a fresh
    ChatGPT conversation.
  - **Background utility isolation:** Codex background utility sub-sessions
    (`title_gen` including 4-item `"concise, single-line task title"` requests,
    `memories`, `auto_review`/`guardian`) bypass the session lease, strip heavy
    Codex developer/AGENTS.md boilerplate, omit MCP bridge guidance, route to
    `-instant` with no reasoning effort, and default to ChatGPT temporary chats
    (`history_and_training_disabled: true`, `is_do_not_remember: true`) so zero
    permanent sidebar chats or `response_threads.json` entries are created.
    Operators may optionally configure `history.utility_project_id` or
    `history.utility_project_name` (auto-created with
    `memory_scope: "project_v2"` for project-only memory isolation).
  - **3-tier automatic project routing:** Continuing chats keep their bound
    `project_id`. Fresh chats resolve in priority order: (1) explicit
    `turn_metadata.project_id` / `history.project_id` / `history.project_name`,
    (2) cached `listProjects()` match derived from Codex `<cwd>` (collapsing
    `/tmp`, `$PREFIX/tmp`, `$HOME`, `/` to shared project `"Codex"`), or
    (3) `createProject` for the derived name when `history.auto_project = true`
    (default).
- **Local authenticated `/control/*` plane (for Slice 6 `@TermuxChatGPT`):**
  Every `/control/*` endpoint requires constant-time `Authorization: Bearer`
  authentication against `<state_dir>/control.token` (`0600`):
  - `GET /control/status`, `GET /control/viewer`, `POST /control/viewer/open`,
    `POST /control/host/restart`
  - `GET /control/chats`, `GET /control/chats/{id}`, `POST /control/chats/{id}/rename`
  - `GET /control/projects`, `GET /control/projects/{id}`, `POST /control/projects`
    (optional `memory_scope`: `"unset"` or `"project_v2"` for project-only memory)
  - `POST /control/send`, `POST /control/observe`, `POST /control/stop`
    (multi-lane parallel execution keyed by `lane_id` / `chat_id`, up to 8
    concurrent lanes; concurrent writes to the *same* `chat_id` still fail
    closed with `generation_already_active`, while independent chats and
    `/v1/responses` streams run and cancel in isolation without disturbing
    sibling lanes)
  - `GET /control/connectors`, `POST /control/connectors`,
    `POST /control/connectors/{id}/refresh` (programmatic zero-DOM custom MCP
    tunnel connector discovery, creation, noauth linking, action refresh,
    in-app query cache invalidation, and `/ecosystem/call_mcp` schema-agreement
    verification over native `connect-app-host` `services.httpFetch`)
- **Zero Wrapper coupling:** Runs as an independent workspace and binary;
  `crates/wrapper` and root `Cargo.lock` have zero compile-time or runtime
  dependency on `chatgpt-provider`.
