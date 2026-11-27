# Archived Local-Tools Implementation (`chatgpt-provider` @ `612d2261`)

This directory preserves the retired **Thin Option B (`local_function_names` / `local_function_signatures`)** implementation of `chatgpt-provider` verbatim from commit `612d2261`, kept outside Cargo/Node active build and CI targets for future developer reference, reuse, or evolution.

## Why It Was Archived

- ChatGPT's `/f/conversation` backend does not provide a mid-turn tool-result callback for `local_function_names`; every client-side tool call terminated the turn and required a new `POST /f/conversation` request (`N` tools = `N + 1` requests spaced ~0.1–0.2s apart), fragmenting ChatGPT's native `Worked for Ns >` UI and triggering rate limits.
- Active `chatgpt-provider` now runs **MCP Single-Turn** execution (`catalog.rs`, `session.rs`, `turn.rs`, `stream.rs`, `responses.rs`), sending one prompt-box-identical `POST /f/conversation` per user turn while OpenAI's backend invokes registered remote MCP connectors (`TermuxShell`, `TermuxChatGPT`, `TermuxCodex`, and future `TermuxOrch`) server-side inside that single turn.

## Contents (Verbatim Snapshots from `612d2261`)

- `responses_612d2261.rs` — Full Rust implementation including `NativeCallBinding`, `pending_calls`, `extract_local_function_signatures`, `resolve_parsed_tool_call`, `ParsedToolCall`, `append_tool_frames`, `function_call_output` / `custom_tool_call_output` validation, and `NativePlan`.
- `page_driver_612d2261.js` — Full `page_driver.js` including `allocateWireLocalFunctionName`, `resolveJsonSchemaType`, `normalizeParamSchema`, `buildNativeLocalFunctionSignatures`, `extractNativeToolCallFromMessage`, `recordToolCall`, `emitToolReportIfNeeded`, `buildStructuredWireMessages` (`role: "tool"`), and `min_request_interval_ms`.
- `page_driver_612d2261.test.mjs` — Node test suite covering all `local_function_names` / `local_function_signatures` wire encodings and edge cases.

## How to Reuse or Evolve Later

The active provider keeps a `TurnStrategy` seam in `providers/chatgpt/src/turn.rs` (defaulting to `McpSingleTurn`). To re-introduce selective client-side tool callbacks or another execution mode in the future, implement a new `TurnStrategy` variant and pull the relevant schema/wire helpers from these snapshots without rewriting `catalog.rs`, `session.rs`, or `stream.rs`.
