# Linux ChatGPT AppHost Generations & Slim `chatgpt-provider`

**Status:** Normative annex of [SPEC.md](../../SPEC.md).
Implemented in [host/chatgpt-desktop/manager.py](../../host/chatgpt-desktop/manager.py)
and [providers/chatgpt/src/main.rs](../../providers/chatgpt/src/main.rs).

---

## 1. Immutable AppHost Generations (`host/chatgpt-desktop`)

- **Escapes `~/.cache`:** Generations live under `~/.local/lib/chatgpt-desktop/`:
  - `generations/<ver>/rootfs/` — immutable Debian 13 ARM64 rootfs with the
    official GPG-verified OpenAI `chatgpt_<ver>_arm64.deb` installed.
  - `generations/<ver>/generation.json` — provenance manifest (`version`,
    `deb_url`, `deb_sha256`, `deb_bytes`, `inrelease_date`,
    `keyring_fingerprint`).
  - `current -> generations/<ver>` and `previous -> generations/<old-ver>` —
    atomic symlinks with rollback (`activate` / `rollback`) preserving
    `previous` on re-activation of the current version.
- **Separated persistent login profile:** User login state lives solely under
  `~/.local/share/chatgpt-desktop/profile/` (`0700`, `flock`-leased by
  `chatgpt-provider`) and is never deleted during rootfs installation,
  activation, or rollback.
- **Cryptographic admission:** `InRelease` is verified via `gpgv` against the
  pinned RSA-4096 keyring (`host/chatgpt-desktop/openai-apt-keyring.gpg`,
  fingerprint `3BFA0E4AE8B8CC16A2D9BA684A3B4A566C4660E4`), `Packages` SHA-256
  and size are verified against `InRelease`, and the downloaded `.deb` SHA-256
  and byte size are verified against `Packages`.
- **Live `0.167.2` isolation:** Legacy `~/.cache/codex-web-experiment/` and
  `~/.config/codex/providers/chatgpt.toml` are never modified.

---

## 2. Slim `chatgpt-provider` (`providers/chatgpt`)

- **Single CDP pipe owner:** `chatgpt-provider` owns one Linux ChatGPT AppHost
  instance (`Xvnc` + `openbox` + `proot` + Chromium `--remote-debugging-pipe`
  FD 3/4) directly, retiring any redundant `codex-chatgpt-host` shell keepalive
  service, DDPE orchestrator, embedded MCP server, and `shared_server`.
- **Clean-room `page_driver.js`:** Minimal greenfield page driver injected via
  CDP `Runtime.evaluate`, using same-origin authenticated backend APIs for
  chat/project/status reads and composer/stop selectors for turn execution.
  Never reads or exposes cookies/auth tokens; never synthesizes private
  reasoning.
- **Type 1 (`codex -p chatgpt`) surface:** Localhost-only `GET /v1/models` and
  `POST /v1/responses` (monotonic OpenAI Responses SSE or JSON) supporting
  single-path in-app tool callbacks via `local_function_signatures`.
- **Local authenticated `/control/*` plane (for Slice 6 `@TermuxChatGPT`):**
  Every `/control/*` endpoint requires constant-time `Authorization: Bearer`
  authentication against `<state_dir>/control.token` (`0600`):
  - `GET /control/status`, `GET /control/viewer`, `POST /control/host/restart`
  - `GET /control/chats`, `GET /control/chats/{id}`, `POST /control/chats/{id}/rename`
  - `GET /control/projects`, `GET /control/projects/{id}`, `POST /control/projects`
  - `POST /control/send`, `POST /control/observe`, `POST /control/stop`
- **Zero Wrapper coupling:** Runs as an independent workspace and binary;
  `crates/wrapper` and root `Cargo.lock` have zero compile-time or runtime
  dependency on `chatgpt-provider`.
