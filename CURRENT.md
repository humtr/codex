# Current product state

Updated: 2026-11-11 KST. This is the **single verified-state record**,
not an implementation plan, authorization, or incident chronology.
[Product contracts](SPEC.md) and [functional annexes](docs/spec/INDEX.md)
define required behavior; [HANDOFF](HANDOFF.md) lists only remaining work.
Distinguish this conversation's actual readbacks, original Native receipts,
the local Termux AI's reported operations, signed publication, and physical
device activation. Do not print or copy credentials into this public file.

## Source and documentation boundaries

- Remote development source `rewrite/rust-core` remains
  `7b98296a411a2c435bb9f34ee51f286f33f3491b` (read back on
  2026-11-11). PR [#29](https://github.com/humtr/codex/pull/29)
  was merged as `afdfb8bd7e57`; the Core-derived `current`
  compatibility alias is part of source 0.167.1+, not old signed Full163.
- Separate release/publication `main` last read at
  `b2567063d2a7a333a718d2495e2c9f8c928d822c`. Do not treat
  independent Git histories as one automatically mergeable lineage.
- Primary **local Termux checkout** is at `rewrite/rust-core`
  `e4f9ab5dc3045cbde8317774da95eb98db7646dd`, the committed
  Native continuation cleanup. It is **not** the remote development HEAD.
  Untracked `ais.config.toml`, `chatgpt.config.toml`, and
  `providers/chatgpt/src/accounts.rs` were observed intact; no ownership
  or permission to incorporate those files is implied.
- Independently developed single-tool MCP shell code was committed
  locally on `feat/termux-mcp-shell-20261111` as
  `c0a8189815c70f0c6678bc9374bcd352423dca57`. The running
  worker is installed from that local development generation, but
  this source change is **not** in remote `rewrite/rust-core`.
- The separately prepared `plan/release-framework-20261111`
  at `3ef675ad721c0a08a50891394cf03df3abb66b2d`
  has 13 documentary changes (+499/-6) in a clean local worktree,
  including the detailed W0–W14 implementation plan and candidate
  `QUALIFICATION.md`/`UPDATE_DISPLAY.md` annexes. It has **not**
  been integrated into the remote development source.
- The previously consolidated product documentation remains in place:
  stable `SPEC.md`, 29 normative annexes, `INDEX`, agent-specific
  `AGENTS.md`, `docs/DOCUMENTATION.md`, declared
  `docs/TEST_DEPENDENCIES.toml` and executable document regression
  checks. A separate documentation-review branch does not become
  product source merely because its commit was pushed.

## Signed releases, update surface and qualification

- `release/upstream.json` at the development source selects Codex
  upstream **0.167.1**, source
  `0a94398fa60c4fac0e7659e4213a0a36e613dd46`.
- Signed [Full163 0.167.0](https://github.com/humtr/codex/releases/tag/full-v0.167.0-c19b61b-s163)
  and signed [Core S164 0.167.1](https://github.com/humtr/codex/releases/tag/core-v0.167.1-7b98296-s164)
  are distinct published artifacts. Core S164 is not a new Full image
  and its publication does not prove activation on this handset.
  Previously preserved Full159/Full153 and rollback references must not
  be silently replaced. The derived `core/current` alias is not an
  activation or signing authority.
- The user's observed `codex update` printed installed Codex
  **0.167.0**, Full #163 / Base #161 and a Core/Full qualification
  mismatch for 0.167.1. Source review showed redundant
  `[1] Keep current` / `[q] Keep current` and a local build guard
  that requires a same-version enhanced Codex TUI when the current
  edition is Full. This has **not** been fixed; see HANDOFF for
  Core/Runtime/Full separation and Core-only selection work.

| Exact source / gate | Verified result | Does not establish |
| --- | --- | --- |
| `7b98296a` [Source Acceptance](https://github.com/humtr/codex/actions/runs/43012469220) | **success** | Installed model or device |
| `7b98296a` [Core qualification](https://github.com/humtr/codex/actions/runs/43013017085) | **success** | Full/Android acceptance |
| `7b98296a` [TUI Native qualification](https://github.com/humtr/codex/actions/runs/43013363622) | **completed / success** | Actual two GPT-6 turns on the same handset thread |
| Older `f9dfc3f` [TUI Native qualification](https://github.com/humtr/codex/actions/runs/42945712023) | **failure** on positive `pin_action_updates_shared_section_state` | Not the result of the newer `7b98296a` run |

Source, signed publication, selected runtime, original TUI conversation,
and physical Android screen proof remain independent acceptance levels.

## Native shared resource recovery — operationally accepted, not remotely merged

The confirmed local root cause is in
`providers/chatgpt/src/native_claims.rs`: a
`functions.write_stdin` continuation without its preceding reservation
was persisted as `Opaque` with `session: None`; after the actual PTY
process exited with code 0, `observed()` returned without removing
the guard's own reservation. The shared ledger
`$PREFIX/var/tmp/codex-native-effects/shared-native-v1/claims.json`
then denied unrelated `functions.exec_command` with
`native_resource_busy` despite work-level `active=[]`.

- The first isolated occurrence was reconciled with owner/result
  evidence, a private backup, an exclusive `flock` and atomic rename.
  Its local patch was **committed** as `e4f9ab5d`, preserving the
  existing continuation session and clearing its original identity
  upon observed exit. The focused test is
  `continuation_without_prior_claim_cleans_up_on_exit`.
- The later recurrence on 2026-11-11 was caused by the **operating
  runit Provider still executing its pre-fix 2026-11-09 binary**.
  The terminating PTY session (58308) left one confirmed completed,
  stale `Opaque` claim; the local Termux repair AI reports backup at
  `$PREFIX/var/tmp/codex-native-effects/shared-native-v1/claims.json.bak-20261111-2305`,
  a specific locked/atomic ledger settlement and service-only
  `sv restart codex-chatgpt-provider`. No unrelated process kill,
  history reset or SD-card access was reported.
- Later independent readbacks confirm the Provider runit service
  running as **PID 4193**, and `/proc/4193/exe` matching the installed
  Provider inode and SHA256. The current development debug binary
  and installed Provider are **not byte/ELF LOAD-segment identical**;
  a further exact production-build provenance proof is still needed.
  Do not infer the patch is absent from that difference alone.
- Original Native work history was independently read: a first
  new work executed `NATIVE_RESOURCE_RECOVERED` and then
  `SEQUENTIAL_EXEC_OK_2`, both exit **0**; after finishing,
  a second independent work executed `NATIVE_REUSE_OK`, exit **0**,
  and finished normally. Both have completed status without
  recorded active/unconfirmed effects. The original AI's direct
  shell and locked repair trace were supplied as an operating report,
  not each independently re-executed here.
- `cargo test --manifest-path providers/chatgpt/Cargo.toml --offline --lib native_claims`
  was subsequently run against local `e4f9ab5d`: **15 passed, 0
  failed, exit 0** (including continuation cleanup and conflicting
  effects). In this conversation, the independent `termux_shell`
  probe `DIRECT_TERMUX_MCP_OK` also returned **exit 0**, and
  an idle-time ledger read returned **0 claims**.
- This establishes actual recovered Native tool admission, same-work
  continuity and independent-work reuse. It does **not** establish
  every future Owner-loss/ACK-unknown/concurrent-writer state, nor
  remote-source integration of `e4f9ab5d`.

## Independent Termux MCP worker and app connector — live in this Chat

- Single MCP tool **`termux_shell`**, not separate shell/receipt
  tools. It executes through the Termux app UID independently of
  Codex Core, running Provider service and shared Native claims.
  Recovery is its current priority, not an exclusive permitted use.
  It does **not** replace the Native ledger's ownership of model
  instructions, sessions, approvals, effects and results.
  Repeated identical `call_id` arguments return the original shell
  receipt rather than blindly re-running the effect. The MCP tool
  uses the existing shared annotation policy without a new exception.
- Local source: `feat/termux-mcp-shell-20261111` / `c0a81898`.
  Installed binary:
  `~/.local/lib/codex/termux-mcp-worker/generations/c0a8189815c7/codex-termux-mcp-worker`.
  Active stdio entrance:
  `~/.local/state/codex/providers/chatgpt/mcp-stdio`, with the
  previous wrapper kept as a private backup.
- The Termux repair AI reports an authorized scoped restart of
  `codex-chatgpt-tunnel` and a successful Linux ChatGPT app
  `refresh_actions` reply (`ok: true`) with app query-cache
  invalidation: **17 → 18 actions**. Its reported connector/tunnel
  belongs to the `6ad4...` line; this is not automatically the
  separate `janmori101` historical `6ae9...` line.
- Independently verified live processes: tunnel supervisor **PID
  2076**, independent MCP worker **PID 2117**, Provider **PID 4193**.
  The *running stdio wrapper* returned exactly **18 tools**:
  `termux_shell` first, no `termux_shell_receipt`, original
  `codex_sessions` retained. **This same ChatGPT conversation**
  advertises `@TermuxBridge.termux_shell` and directly executed
  `DIRECT_TERMUX_MCP_OK` with exit **0**. Remote app tool
  registration and actual end-to-end shell execution are therefore
  independently observed, not merely a local tool-list claim.
- No account login/logout or profile migration was performed in
  this verification. **Which Linux app account currently owns the
  successful connector refresh is not independently established
  as `janmori101`**. A previous `janmori101` desktop session
  descriptor referenced a departed PID. Separate profile directories
  and connector names do not prove the live authenticated account.

## Outstanding independent proofs

Real installed `/provider` GPT-6 two-turn same-thread response and
original Chat return; SSE/HTTP 413 uncertain result recovery without
duplicate execution; independent safe parallel Native work;
authenticated Linux AppHost account/workspace mapping; notification
tap to exact Android terminal/activity and opt-in headless display;
Core-only 0.167.1 update/Runtime compatibility; signed release
admission/device activation; and remote integration of local
`e4f9ab5d` and `c0a81898` remain separately open.

The dated [R01–R10/U1–U7 review queue](docs/development/REVIEW_ACTIONS_20261110.md)
is historical source triage with a continuing decision matrix, not
proof that every earlier issue still reproduces. No changes to user
sessions, SQLite/JSONL records, profile credentials or signed
generations were made during this documentation verification.
