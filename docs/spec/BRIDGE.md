# Stage 2 Decoupled Bridge, Provider, and 3-MCP Architecture Contract

**Status:** Normative annex of [SPEC.md](../../SPEC.md).
This annex defines the Stage 2 decoupled ownership boundary across the optional
ChatGPT Desktop Provider and the three independent MCP bridges. Detailed
per-component contracts live in their dedicated annexes:

- [CHATGPT_PROVIDER.md](CHATGPT_PROVIDER.md) — `host/chatgpt-desktop` immutable
  rootfs generations & slim `chatgpt-provider` (`/v1/responses` + `/control/*`)
- [MCP_SHELL.md](MCP_SHELL.md) — `@TermuxShell` (`termux-shell-mcp`) independent
  host shell recovery plane
- [MCP_CODEX.md](MCP_CODEX.md) — `@TermuxCodex` (`termux-codex-mcp`) stateless
  5-tool bridge over official `codex app-server daemon`
- [MCP_CHATGPT.md](MCP_CHATGPT.md) — `@TermuxChatGPT` (`termux-chatgpt-mcp`)
  stateless 6-tool bridge over `chatgpt-provider` `/control/*`
- [SUPERVISION.md](SUPERVISION.md) — opt-in `runit` 3-state generation
  supervision and `tunnel-client` transport
- [ORCHESTRATION.md](ORCHESTRATION.md) — optional Stage 2 Slice 7 durable
  multi-role orchestration (`@TermuxOrch` / `mcp/termux-orch`), strictly
  layered *above* the three stateless MCP bridges without modifying their
  stateless contracts

For active implementation ownership consult [docs/ARCHITECTURE.md](../ARCHITECTURE.md);
for verified device/CI receipts consult [CURRENT.md](../../CURRENT.md); for
retired pre-rewrite (`0.167.x`) history consult
[docs/history/LEGACY_RUST_CORE.md](../history/LEGACY_RUST_CORE.md).

---

## 1. Non-Negotiable Decoupling & Single-Owner Invariants

1. **Zero Wrapper coupling:** `$PREFIX/bin/codex` (`crates/wrapper`) has zero
   compile-time or runtime dependency on `chatgpt-provider`, Linux AppHost,
   `tunnel-client`, or any MCP server. Ordinary `codex` execution never requires
   optional Stage 2 services to be installed or running.
2. **One authoritative owner per state domain:**
   - **Official `codex app-server daemon`** owns Codex threads, turns, tool
     execution, approvals, history, and writer locks (`state_*.sqlite` /
     rollouts). No Stage 2 bridge introduces a shadow task DB, second executor,
     or `DDPE` multi-worker controller.
   - **Linux ChatGPT AppHost + `chatgpt-provider`** owns the single CDP
     `--remote-debugging-pipe`, clean-room programmatic `page_driver.js`
     (`connect-app-host` `httpFetch` + dynamically discovered `CompletionClient`
     + native `local_function_signatures` + multi-lane parallel execution up to
     8 lanes), OpenAI `/v1/responses` adaptation for `codex -p chatgpt`, and
     authenticated local `/control/*` (including programmatic `/control/connectors*`).
   - **Each MCP worker (`@TermuxShell`, `@TermuxCodex`, `@TermuxChatGPT`)** is
     an independent stateless stdio process sharing only `crates/mcp-kit`
     (uniform annotations, disjoint `*_catalog`/`*_call`, bounded `call_id`
     transport receipts) and connected via its own opt-in `runit` tunnel.
3. **Fault isolation across the 3 MCP planes:**
   - `@TermuxShell` has **zero** dependency on Codex, `chatgpt-provider`, or
     Linux AppHost — it survives as an independent recovery plane if everything
     else fails.
   - `@TermuxCodex` depends **only** on official `codex app-server daemon` and
     can be retired independently once official `codex remote-control` covers
     the same workflow.
   - `@TermuxChatGPT` depends **only** on `chatgpt-provider`'s loopback
     `/control/*` plane and holds **zero** local code/shell execution privilege.

---

## 2. Common MCP Policy Across All 3 Connectors

- **Uniform immutable annotations:** Every static tool exposed by
  `@TermuxShell`, `@TermuxCodex`, and `@TermuxChatGPT` declares
  `readOnlyHint: true`, `destructiveHint: false`, `openWorldHint: false`, plus
  an accurate per-tool `idempotentHint`.
- **Disjoint extension slots (`*_catalog` / `*_call`):** `@TermuxCodex` and
  `@TermuxChatGPT` expose allowlisted extension actions strictly disjoint from
  their static tools (enforced at startup by `mcp_kit::ExtensionRegistry`).
  Validation failures return exact `expected_schema` (or `available_actions`)
  so remote clients self-correct without connector refresh. `@TermuxShell`
  permanently exposes only its single static `termux_shell` tool.
- **Transport deduplication receipts (not task DBs):** Mutating entrypoints
  (`termux_shell`, `codex_turn`, `chatgpt_send(action="send")`) check
  downstream reachability before claiming a brand-new `call_id`, persist an
  atomic `0600` (`O_NOFOLLOW`, `fsync`) `started` intent before effect
  dispatch, replay settled outcomes on identical retry, reject payload
  mismatches with conflict errors, and return `state: "delivery_unknown"`
  (never silently replaying) when transport drops mid-flight.
- **No secret or private-thought leakage:** No bridge exposes OAuth tokens,
  cookies, `control.token` contents, or private model reasoning. The Type 1
  Responses stream carries upstream user-visible reasoning summaries as
  separate items alongside answer text.

---

## 3. Canonical XDG `codex/` Namespace

All Stage 2 components live cleanly under the unified XDG `codex/` hierarchy
(never `~/.cache` and never mixed with retired `0.167.2` directories):

- Immutable generations: `~/.local/lib/codex/{wrapper,host/chatgpt-desktop,providers/chatgpt,mcp/termux-shell,mcp/termux-codex,mcp/termux-chatgpt,mcp/termux-orch,transport/tunnel-client}`
- Persistent user/profile data: `~/.local/share/codex/{wrapper,manager/profiles,chatgpt-desktop/profile}`
- Configuration: `~/.config/codex/{providers/chatgpt.toml,ext/notify.json,tunnels/*.yaml}`
- Ephemeral runtime state, orchestration ledgers & service logs: `~/.local/state/codex/{providers/chatgpt,ext,orchestration,services/<svc>/log}`
