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
  and fails closed on a live or unverified singleton. The installed PRoot
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
- **Type 1 (`codex -p chatgpt`) surface:** Localhost-only `GET /v1/models` and
  `POST /v1/responses` (monotonic OpenAI Responses SSE or JSON) supporting
  single-path in-app tool callbacks via `local_function_signatures`. The
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
  effort. When
  Codex supplies deferred nested tools through Code Mode, the description
  sent to the AppHost retains the advertised `ALL_TOOLS` discovery instruction
  within the description bound.
- **Single-adapter streaming lifecycle (`Page Driver -> LaneMailbox -> TurnAssembly -> ResponsesEncoder`):**
  The Provider normalizes ChatGPT AppHost completions through one lifecycle
  pipeline shared by both streaming (`stream=true`) and non-streaming
  (`stream=false`) `/v1/responses` execution:
  - `page_driver.js` requests `supported_encodings: ["v1"]` and observes
    messages through `onDecodedEvent` as the single authoritative stream path
    (with an `onUpdate` fallback only when `onDecodedEvent` has not fired).
  - Messages are normalized per item ID and semantic channel (`reasoning_summary`,
    `commentary`, `answer`, and native `tool_call`), distinguishing uncommitted
    `Mutable` snapshots (`in_progress`) from `Committed` outputs (`status !==
    "in_progress"`, `end_turn === true`, finished `thoughts` entries, or turn
    settlement). Uncommitted snapshot revisions, insertions, or deletions update
    in-place without failing settled streams.
  - Simultaneous `commentary` + `reasoning` passthrough:
    - Public interim progress messages (`channel === "commentary"`,
      `is_thinking_preamble_message === true`, `status === "finished_partial_completion"`,
      `end_turn === false`, or uncommitted pre-tool assistant messages) and
      optional per-tool `report` progress text (injected only on `type: "function"`
      tools without an existing `report` property and strictly stripped before
      forwarding `arguments` to Codex) emit as `type: "message", phase: "commentary"`
      so Codex TUI displays step-by-step commentary inline alongside tool calls.
      Each ChatGPT message `msgKey` owns at most one canonical text item ID so
      late status updates never duplicate an already-committed message (only
      genuine strict-suffix extensions emit incremental follow-up items), and
      `TurnAssembly` suppresses exact duplicate commentary items and repeated
      commentary prefixes in `final_answer` without discarding unique content.
    - Narrow DIL-to-GFM presentation projection (`presentation.rs`): committed
      `commentary` and `final_answer` messages translate allowlisted ChatGPT DIL
      UI tags (`<Link>`, `<badge>`, `<row>`, `<grid>`/`<grid-item>`/`<caption>`,
      `<title>`, `<text>`, `<box>`, `<card>`, `<divider>`, `<svg>` labels,
      `<checkbox>`, `<MemoryCite>`, and `{@body}`/`{#if}` directives) outside
      fenced/inline code into Codex-native GFM Markdown while preserving code
      blocks, inline code, unknown HTML/generics, tool arguments, and `readChat`
      host history verbatim.
    - Full AppHost reasoning emitted by the model (`thoughts[*]` entries with
      `summary` + `content` / `chunks`, formatted as `**{summary}**\n{content}`
      when both are present; `thoughts` `parts`/`content`; `channel === "analysis"`
      text; `reasoning_recap`; deduplicated `metadata.{rich_reasoning_titles,
      reasoning_titles, reasoning_title}`) emits simultaneously as
      `type: "reasoning"` (`summary_text`) for Codex's live working status line
      and `Ctrl+T` transcript. The Provider never synthesizes private reasoning,
      never leaks user echo messages or non-reasoning tool payloads into
      reasoning, and keeps `readChat` focused on visible `user`/`assistant`
      conversation history.
  - Background utility isolation: Codex background utility calls (identified via
    `x-codex-turn-metadata` `request_kind != "turn"` or `memory_consolidation`,
    or tool-less title/memory prompts) bypass session leasing, project pinning,
    and `response_threads.json` persistence, and map unknown non-ChatGPT utility
    model slugs (such as `gpt-6-luna`) to an available ChatGPT slug so they
    never trigger `503 response_thread_busy` against the active user turn.
  - `cdp.rs` delivers structured `StreamEvent` items through a per-lane
    non-blocking `LaneMailbox` that coalesces rapid mutable snapshots per item
    without stalling sibling lanes and queues committed/terminal events
    losslessly.
  - `responses.rs` maintains one authoritative `TurnAssembly` (`Mutable` vs
    `Committed`) and one `ResponsesEncoder` that emits immutable Responses
    frames only for committed items (streaming committed reasoning summary
    sections live while thinking, then preambles/answers/deduplicated tool calls).
    If a turn supplies only a user-visible summary with no answer or tool call,
    the same text is returned as the final answer. The stream has a five-minute
    upstream inactivity limit and a two-hour total limit.
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
- **Conditional structured Native-return candidate:** The experimental
  `history.structured_native_return` mode uses the same Responses lifecycle
  and requires an explicit `server.state_dir`. Native continuation state
  lives in that directory's `structured-native-return` subdirectory. Keep
  the existing control token, catalog and legacy continuation map at their
  original paths. Local opt-in can exercise this mode before qualified
  production adoption. User content, original native
  call bindings, typed results and authorized instruction context must remain
  distinct. Reject unsupported content and instruction scope explicitly.
  Require original result roles, preserved authority, declared native calls,
  full supported content and the verified continuation thread.
  Supported Codex developer text travels as an AppHost developer message
  in input order. Check its role and content if AppHost retains the
  message in its graph. AppHost can omit developer input from that graph.
  Preserve the submitted developer context in the Codex prefix. Request-level
  `instructions` travel on user turns as a trailing hidden `developer` message
  with `exclude_after_next_user_message: true` (matching AppHost's own
  one-turn developer placement so it remains active across the turn's tool
  rounds and expires on the next user message); unchanged `instructions` stay
  implicit on tool-return rounds while mid-round changes fail closed with
  `native_instruction_scope_changed`. A developer update accompanies genuine
  user input.
  Because AppHost's internal `submitChatGPTLocalFunctionResult` wrapper
  overwrites declared `local_function_signatures` with AppHost built-ins on
  round 2+ and accepts only one result per call, the candidate selects Option
  B: submitting user/developer inputs and structured `role: "tool"` result
  envelopes (`author.role = "tool"`, `channel = "commentary"`, `content_type
  = "code"`, JSON `{ call_id, result, tool }`, `is_visually_hidden_from_conversation = true`)
  directly through AppHost's unified `CompletionClient.startCompletionStream`
  with Codex's declared `local_function_names` / `local_function_signatures`
  preserved on every round and exact Provider-generated `submitted_message_ids`.
  Single and batched/parallel tool returns (including out-of-order arrivals)
  travel in one atomic completion request (`parallel_tool_calls: true`).
  Verify any retained tool-result node on readback; if AppHost omits a tool
  node from durable history, require exact parent-branch advancement, original
  call/argument-digest verification, and zero synthetic user turns or
  conflicting off-branch correlations. Trailing non-visible reasoning nodes
  (`thoughts`, `reasoning_recap`, `analysis`, or hidden assistant nodes) on
  the committed head are tolerated while any unstreamed visible assistant
  message or unexpected tool call fails closed.
  Stream committed visible output in order up to the first pending tool
  call. Expose the call and subsequent output after verified settlement.
  The candidate records a versioned dispatch intent before sending, verifies
  the original account/project/remote head and the submitted message chain,
  and persists settled call bindings before emitting tool-call output items.
  If AppHost exposes only the authenticated user ID, resolve the dispatch
  account through its read-only account guard and require a unique match.
  Bind the verified account ID to the continuation.
  Readback must match the original declared call, arguments, recipient, exact
  branch head and exact `submitted_message_ids`, including duplicate
  correlation on other branches. The original AppHost argument source is
  bound by a digest and checked again before returning a result and at result
  settlement. Durable dispatch receipts retain the original parent, prior
  response ID, per-message digests, and active request-instruction digest.
  The Provider may record observed chat and account identities before final
  settlement. Preserve unresolved receipts for explicit reconciliation and
  retain the operating generation and state for rollback. Physical UI,
  interactive approval denial, compaction, and signed publication remain
  explicit qualification gates before production adoption.
  If the Page Driver confirms rejection before invoking the AppHost
  completion stream, restore the pre-request binding or remove the new
  intent. A timeout or ambiguous dispatch keeps the intent for reconciliation.
- **Local authenticated `/control/*` plane (for Slice 6 `@TermuxChatGPT`):**
  Every `/control/*` endpoint requires constant-time `Authorization: Bearer`
  authentication against `<state_dir>/control.token` (`0600`):
  - `GET /control/status`, `GET /control/viewer`, `POST /control/viewer/open`,
    `POST /control/host/restart`
  - `GET /control/chats`, `GET /control/chats/{id}`, `POST /control/chats/{id}/rename`
  - `GET /control/projects`, `GET /control/projects/{id}`, `POST /control/projects`
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
