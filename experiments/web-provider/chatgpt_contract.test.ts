import { expect, test } from "bun:test";
import { defaultConfig } from "../src/config";
import { parseRequest } from "../src/responses/parser";
import { responseRequest } from "../src/server";

const input = "print('owned namespace')\n";
const tools = [{ type: "namespace", name: "mcp__python", tools: [
  { type: "custom", name: "run_script", description: "Owned fixture", format: { type: "grammar", syntax: "lark", definition: "start: /.+/" } },
]}];
const call = { type: "custom_tool_call", call_id: "call_owned", namespace: "mcp__python", name: "run_script", input };
const output = { type: "custom_tool_call_output", call_id: "call_owned", output: [{ type: "input_text", text: "owned result" }] };

function body(items: unknown[], extra = {}) {
  return { model: "chatgpt-web/luna", input: items, ...extra };
}

function declaration() {
  return { type: "additional_tools", role: "developer", tools };
}

test("custom namespace, raw input and result ID survive parser replay", () => {
  const parsed = parseRequest(body([declaration(), call, output]));
  expect(parsed.context.tools).toContainEqual(expect.objectContaining({ name: "run_script", namespace: "mcp__python", freeform: true }));
  const assistant = parsed.context.messages.find(row => row.role === "assistant");
  expect(assistant?.content).toContainEqual(expect.objectContaining({ type: "toolCall", id: "call_owned", namespace: "mcp__python", name: "run_script", arguments: { input } }));
  expect(parsed.context.messages).toContainEqual(expect.objectContaining({ role: "toolResult", toolCallId: "call_owned", toolName: "run_script", toolNamespace: "mcp__python" }));
});

for (const stream of [false, true]) {
  test(`real HTTP handler retains custom namespace/type in ${stream ? "SSE" : "JSON"}`, async () => {
    const config = defaultConfig("full");config.solAvailable = false;config.proAvailable = false;
    const turnId = `turn_owned_namespace_${stream}`;
    const req = new Request("http://127.0.0.1:17841/v1/responses", {
      method: "POST", headers: { "content-type": "application/json" },
      body: JSON.stringify(body([declaration(), { type: "message", role: "user", content: [{ type: "input_text", text: "owned fixture" }], internal_chat_message_metadata_passthrough: { turn_id: turnId } }], { stream, metadata: { turn_id: turnId, thread_id: "thread_owned_namespace" } })),
    });
    const response = await responseRequest(req, config, () => ({
      name: "owned-contract-fixture",
      async runTurn(parsed, _incoming, emit) {
        expect(parsed.context.tools).toContainEqual(expect.objectContaining({ namespace: "mcp__python", freeform: true }));
        emit({ type: "tool_call_start", id: "call_owned", name: "mcp__python__run_script" });
        emit({ type: "tool_call_delta", arguments: JSON.stringify({ input }) });
        emit({ type: "tool_call_end" });emit({ type: "done", endTurn: false });
      },
    }), { rememberState: false });
    expect(response.status).toBe(200);
    let items: Array<Record<string, unknown>>;
    if (stream) {
      const text = await response.text();
      const frames = text.split("\n").filter(row => row.startsWith("data: "));
      expect(frames.filter(row => row === "data: [DONE]")).toHaveLength(1);
      const messages = frames.filter(row => row !== "data: [DONE]").map(row => JSON.parse(row.slice(6)));
      items = messages.filter(row => row.type === "response.output_item.done").map(row => row.item);
    } else {
      items = ((await response.json()) as { output: Array<Record<string, unknown>> }).output;
    }
    expect(items.filter(row => row.type === "custom_tool_call")).toEqual([expect.objectContaining({ call_id: "call_owned", namespace: "mcp__python", name: "run_script", input })]);
    expect(items.some(row => row.type === "function_call")).toBe(false);
  });
}

test("default functions remains its established implicit alias", () => {
  const parsed = parseRequest(body([{ ...declaration(), tools: [{ ...tools[0], name: "functions" }] }, { ...call, namespace: "functions" }, output]));
  const assistant = parsed.context.messages.find(row => row.role === "assistant");
  const selected = assistant?.content.find(row => row.type === "toolCall");
  expect(selected).not.toHaveProperty("namespace");
  expect(parsed.context.tools?.[0]).not.toHaveProperty("namespace");
});

test("orphan duplicate or mismatched call/result refuses before adapter", async () => {
  const invalid = [[output], [call, call, output], [call, output, output], [call, { ...output, call_id: "other" }], [call, { ...output, type: "function_call_output" }], [{ type: "function_call", call_id: "fn", name: "exec_command", arguments: "{" }], [{ type: "function_call", call_id: "fn", name: "exec_command", arguments: "[]" }]];
  let executed = 0;
  for (const items of invalid) {
    expect(() => parseRequest(body(items))).toThrow();
    const response = await responseRequest(new Request("http://127.0.0.1/v1/responses", { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(body(items)) }), defaultConfig("full"), () => ({ name: "must-not-run", async runTurn() { executed++; } }), { rememberState: false });
    expect(response.status).toBe(400);
  }
  expect(executed).toBe(0);
});

test("completed IDs may be reused in a later exchange, not duplicated while pending", () => {
  expect(() => parseRequest(body([call, output, call, output]))).not.toThrow();
});

test("unknown and colliding tool declarations cannot be silently repaired", () => {
  for (const declared of [[{ type: "unknown", name: "bad" }], [{ type: "web_search" }], [{ type: "namespace", name: "bad", tools: [{ type: "unknown", name: "bad" }] }], [{ type: "function", name: "mcp__python__run_script" }, ...tools]]) {
    expect(() => parseRequest(body([], { tools: declared }))).toThrow();
  }
});
