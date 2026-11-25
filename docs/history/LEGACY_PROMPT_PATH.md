# Legacy Prompt-Flattening Path (Option A) — Development Reference Artifact

> **Status:** Retired from runtime in Phase 2 (2026-11-26) in favor of the
> unified Option B structured adapter (`CompletionClient.startCompletionStream`
> with `role: "developer"`, `role: "user"`, 1-turn `exclude_after_next_user_message`,
> and `role: "tool"` JSON envelopes).
>
> **Exact preservation snapshots in Git:**
> - Pre-structured baseline (pure Option A): `c9d0e245` (`Deduplicate message commentary identity and project ChatGPT DIL to Codex GFM`)
> - Dual-path baseline (Option A + Phase 1 Option B side-by-side): `c94dc121` (`Harden Phase 1 Option B native returns for cancel, compaction, dedup, and empty tool outputs`)

---

## 1. Why Option A Was Retired

Option A flattened Codex `instructions`, `developer` context, `assistant` replies,
and `function_call_output` / `custom_tool_call_output` into a single plain-text
`author.role = "user"` prompt string (`render_prompt_from_responses_request`) and
kept a fenced-code-block tool fallback (`fallbackFenceMode` / `parseToolCallsFromText`).

This caused three structural defects:
1. Every tool result created a synthetic visible `user` message bubble in ChatGPT history.
2. Role authority boundaries collapsed (`developer` instructions, `user` text, and `tool` outputs all travelled as `user` text).
3. Maintaining both Option A and Option B behind `history.structured_native_return` added ~1,700 lines of dual-branch state and maintenance complexity.

---

## 2. Retired Runtime Components (For Reference / Restoration)

| Component | Former Location (`c94dc121`) | Responsibility |
| --- | --- | --- |
| `render_prompt_from_responses_request` | `providers/chatgpt/src/responses.rs:632-737` | Flattened `instructions` + delta `input[skip_prefix..]` into `[Instructions]`, `[Developer]`, `[Tool Output <call_id>]`, `[Tool Call]`, and `[Assistant]` sections inside one `user` prompt (`MAX_DEVELOPER_BLOCK_CHARS=48_000`, `MAX_TOTAL_PROMPT_CHARS=180_000`). |
| `input_prefix_digest` + legacy `ResponseThreadMap` | `providers/chatgpt/src/responses.rs:180-185, 355-445` | Tracked raw input array length + SHA-256 without `NativeThreadBinding`; on prefix drift silently started a new ChatGPT conversation. |
| `fallbackFenceMode` | `providers/chatgpt/src/page_driver.js:1864-1878` | When `local_function_signatures` failed to convert, prepended `[LOCAL_FUNCTION_SIGNATURES]` and asked the model to emit ```` ```codex_tool_call ```` JSON blocks. |
| `parseToolCallsFromText` | `providers/chatgpt/src/page_driver.js:1012-1044` | Regex-extracted ```` ```codex_tool_call ... ``` ```` fenced blocks from plain assistant answers and converted them into `tool_calls`. |

---

## 3. How to Inspect or Restore

To inspect any of the retired implementations verbatim:
```bash
git show c94dc121:providers/chatgpt/src/responses.rs | sed -n '625,745p'
git show c94dc121:providers/chatgpt/src/page_driver.js | sed -n '1010,1050p'
```

Direct `/control/send` (`@TermuxChatGPT` and standalone control clients) continues
to accept plain `prompt` strings on `page_driver.js` independently of the retired
Responses prompt flattener.
