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
  (`0700`, `flock`-leased via sibling `.lock` file by `chatgpt-provider`;
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

- **Single CDP pipe owner:** `chatgpt-provider` owns one Linux ChatGPT AppHost
  instance (`Xvnc` + `openbox` + `proot` + Chromium `--remote-debugging-pipe`
  FD 3/4 + allow-listed Android OAuth browser relay via `termux-am-socket`)
  directly, retiring any redundant `codex-chatgpt-host` shell keepalive
  service, DDPE orchestrator, embedded MCP server, and `shared_server`.
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
  single-path in-app tool callbacks via `local_function_signatures`.
- **Local authenticated `/control/*` plane (for Slice 6 `@TermuxChatGPT`):**
  Every `/control/*` endpoint requires constant-time `Authorization: Bearer`
  authentication against `<state_dir>/control.token` (`0600`):
  - `GET /control/status`, `GET /control/viewer`, `POST /control/viewer/open`,
    `POST /control/host/restart`
  - `GET /control/chats`, `GET /control/chats/{id}`, `POST /control/chats/{id}/rename`
  - `GET /control/projects`, `GET /control/projects/{id}`, `POST /control/projects`
  - `POST /control/send`, `POST /control/observe`, `POST /control/stop`
  - `GET /control/connectors`, `POST /control/connectors`,
    `POST /control/connectors/{id}/refresh` (programmatic zero-DOM custom MCP
    tunnel connector discovery, creation, noauth linking, action refresh,
    in-app query cache invalidation, and `/ecosystem/call_mcp` schema-agreement
    verification over native `connect-app-host` `services.httpFetch`)
- **Zero Wrapper coupling:** Runs as an independent workspace and binary;
  `crates/wrapper` and root `Cargo.lock` have zero compile-time or runtime
  dependency on `chatgpt-provider`.
