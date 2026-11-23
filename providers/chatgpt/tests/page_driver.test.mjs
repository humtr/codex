import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import vm from "node:vm";

const DRIVER_SRC = fs.readFileSync(
  path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../src/page_driver.js"),
  "utf8"
);

function createMockEnv({ url = "https://chatgpt.com/c/conv-12345678", apiMap = {}, completionBehavior } = {}) {
  const startCalls = [];
  const cancelCalls = [];
  const stopCalls = [];

  const mockCompletionClient = {
    async startCompletionStream(args) {
      startCalls.push(args);
      if (completionBehavior) {
        return completionBehavior(args);
      }
      queueMicrotask(() => {
        args.onUpdate?.({
          type: "message",
          conversationId: args.request.conversation_id || "conv-new-1",
          message: {
            id: "msg-thought",
            author: { role: "assistant" },
            content: { content_type: "thoughts", parts: ["PRIVATE_THOUGHT_MUST_NOT_LEAK"] },
          },
        });
        args.onUpdate?.({
          type: "message",
          conversationId: args.request.conversation_id || "conv-new-1",
          message: {
            id: "msg-final",
            author: { role: "assistant" },
            content: {
              content_type: "text",
              parts: [
                'Sure!\n```codex_tool_call\n{"call_id":"call_42","name":"exec_command","arguments":{"cmd":"ls"}}\n```',
              ],
            },
          },
        });
        args.onComplete?.({ reason: "done" });
      });
      return { streamRequestId: "stream-1", prepareResponse: { conduit_token: "ctok-1" } };
    },
    cancelStream(reqId) {
      cancelCalls.push(reqId);
    },
    async stopCompletion(body, meta) {
      stopCalls.push({ body, meta });
    },
  };

  let uuidSeq = 0;
  const sandbox = {
    location: new URL(url),
    window: { location: new URL(url) },
    document: {
      querySelector: () => null,
      querySelectorAll: () => [],
      getElementById: () => null,
    },
    crypto: { randomUUID: () => `uuid-test-${++uuidSeq}` },
    __termuxMockCompletionClient: mockCompletionClient,
    fetch: async (reqUrl, opts = {}) => {
      const key = `${(opts.method || "GET").toUpperCase()} ${reqUrl}`;
      const hit = apiMap[key] || apiMap[reqUrl];
      if (!hit) {
        return { ok: false, status: 404, text: async () => JSON.stringify({ error: "not_found" }) };
      }
      const body = typeof hit.body === "function" ? hit.body(opts) : hit.body;
      return {
        ok: hit.status >= 200 && hit.status < 300,
        status: hit.status,
        text: async () => (typeof body === "string" ? body : JSON.stringify(body)),
      };
    },
    setTimeout,
    clearTimeout,
    AbortController,
    Intl,
    URL,
    Date,
  };
  sandbox.globalThis = sandbox;
  vm.runInNewContext(DRIVER_SRC, sandbox);
  return { driver: sandbox.__termuxChatGptDriver, startCalls, cancelCalls, stopCalls };
}

test("page_driver status, chats, linear branch read (no private thoughts), projects, project send, and tool callback parsing", async () => {
  const { driver, startCalls } = createMockEnv({
    apiMap: {
      "GET /backend-api/me": {
        status: 200,
        body: { id: "u1", name: "User", orgs: { data: [{ id: "acc-1", title: "Plus", is_personal: true }] } },
      },
      "GET /backend-api/models": {
        status: 200,
        body: { models: [{ slug: "gpt-5-6", title: "GPT-5.6", max_tokens: 128000 }] },
      },
      "GET /backend-api/conversations?offset=0&limit=20&order=updated&is_archived=false": {
        status: 200,
        body: { total: 2, items: [{ id: "conv-12345678", title: "Demo Chat", create_time: 100, update_time: 200 }] },
      },
      "GET /backend-api/conversation/conv-12345678": {
        status: 200,
        body: {
          title: "Demo Chat",
          gizmo_id: "g-p-1",
          current_node: "n3",
          mapping: {
            n1: { parent: null, message: { id: "m1", author: { role: "user" }, content: { content_type: "text", parts: ["Hi"] } } },
            n2: { parent: "n1", message: { id: "m2", author: { role: "assistant" }, content: { content_type: "thoughts", parts: ["PRIVATE_THOUGHT_MUST_NOT_LEAK"] } } },
            n3: {
              parent: "n2",
              message: {
                id: "m3",
                author: { role: "assistant" },
                content: { content_type: "text", parts: ["Earlier reply"] },
              },
            },
          },
        },
      },
      "PATCH /backend-api/conversation/conv-12345678": { status: 200, body: { success: true } },
      "GET /backend-api/gizmos/snorlax/sidebar?conversations_per_gizmo=0": {
        status: 200,
        body: { items: [{ gizmo: { gizmo: { id: "g-p-1", display: { name: "Proj 1" } } } }] },
      },
      "POST /backend-api/projects": {
        status: 200,
        body: { resource: { gizmo: { id: "g-p-created", display: { name: "New Proj" } } } },
      },
    },
  });

  const st = await driver.status();
  assert.equal(st.auth_state, "authenticated");
  assert.equal(st.completion_ready, true);
  assert.equal(st.ui_state, "ready");
  assert.equal(st.models[0].slug, "gpt-5-6");

  const list = await driver.listChats({ limit: 20 });
  assert.equal(list.chats.length, 1);
  assert.equal(list.next_cursor, "1");

  const rd = await driver.readChat({ chat_id: "conv-12345678" });
  assert.equal(rd.messages.length, 2);
  assert.ok(!JSON.stringify(rd).includes("PRIVATE_THOUGHT_MUST_NOT_LEAK"));

  const ren = await driver.renameChat({ chat_id: "conv-12345678", title: "New Title" });
  assert.equal(ren.state, "ok");

  const projs = await driver.listProjects({});
  assert.equal(projs.projects[0].id, "g-p-1");

  const createdProj = await driver.createProject({ name: "New Proj", instructions: "Goal" });
  assert.equal(createdProj.state, "ok");
  assert.equal(createdProj.project_id, "g-p-created");
  assert.equal(createdProj.name, "New Proj");

  // Multi-turn continuation on existing chat_id inherits current_node + gizmo_id + model + mapped reasoning_effort
  const sent = await driver.sendMessage({
    chat_id: "conv-12345678",
    prompt: "List directory",
    model: "gpt-5-6-thinking",
    reasoning_effort: "low",
    local_function_signatures: [{ type: "function", name: "exec_command" }],
    wait_ms: 1000,
  });
  assert.equal(sent.state, "settled");
  assert.equal(sent.chat_id, "conv-12345678");
  assert.equal(sent.assistant_text, "Sure!");
  assert.equal(sent.tool_calls.length, 1);
  assert.equal(sent.tool_calls[0].id, "call_42");
  assert.equal(sent.tool_calls[0].name, "exec_command");

  const wire = startCalls[0].request;
  assert.equal(wire.conversation_id, "conv-12345678");
  assert.equal(wire.parent_message_id, "n3");
  assert.equal(wire.gizmo_id, "g-p-1");
  assert.equal(wire.conversation_mode.kind, "gizmo_interaction");
  assert.equal(wire.model, "gpt-5-6-thinking");
  assert.equal(wire.thinking_effort, "min");
  assert.deepEqual([...wire.local_function_names], ["exec_command"]);
  assert.equal(wire.local_function_signatures[0].type, "kwargs");
  assert.equal(wire.local_function_signatures[0].name, "exec_command");
  assert.equal(wire.messages[0].content.parts[0], "List directory");

  // Instant model must omit thinking_effort (ChatGPT returns 422 if sent on instant models);
  // continuation turns still send native local_function_signatures without repeating text preamble
  await driver.sendMessage({
    prompt: "[Tool Output call_1]\nOK",
    model: "gpt-5-6-instant",
    reasoning_effort: "medium",
    local_function_signatures: [{ type: "custom", name: "exec" }],
    is_continuation: true,
    wait_ms: 1000,
  });
  assert.equal(startCalls[1].request.thinking_effort, undefined);
  assert.deepEqual([...startCalls[1].request.local_function_names], ["codex_exec"]);
  assert.equal(startCalls[1].request.local_function_signatures[0].params[0].name, "input");
  assert.ok(!startCalls[1].request.messages[0].content.parts[0].includes("[LOCAL_FUNCTION_SIGNATURES]"));

  // Custom input & namespace tool call parsing
  const parsedCustom = driver.parseToolCallsFromText(
    '```codex_tool_call\n{"call_id":"c1","name":"exec","input":"echo 123"}\n```\n' +
    '```codex_tool_call\n{"call_id":"c2","namespace":"collaboration","name":"spawn_agent","arguments":{"task_name":"w1"}}\n```'
  );
  assert.equal(parsedCustom.calls.length, 2);
  assert.equal(parsedCustom.calls[0].arguments, "echo 123");
  assert.equal(parsedCustom.calls[1].namespace, "collaboration");
  assert.equal(parsedCustom.calls[1].name, "spawn_agent");
});

test("page_driver multi-lane parallel execution, per-chat collision guard, isolated observeChat, and targeted stopGeneration", async () => {
  let seq = 0;
  const { driver, cancelCalls, stopCalls } = createMockEnv({
    apiMap: {
      "GET /backend-api/me": {
        status: 200,
        body: { id: "u1", name: "User", orgs: { data: [{ id: "acc-1", title: "Plus", is_personal: true }] } },
      },
      "GET /backend-api/models": {
        status: 200,
        body: { models: [{ slug: "gpt-6-sol-wm", title: "GPT-6 Sol" }] },
      },
    },
    completionBehavior(args) {
      const idx = ++seq;
      const cid = `conv-live-${idx + 8}`;
      const text = `Streaming chunk lane ${idx}...`;
      queueMicrotask(() => {
        args.onUpdate?.({
          type: "message",
          conversationId: cid,
          message: {
            id: `msg-partial-${idx}`,
            author: { role: "assistant" },
            content: { content_type: "text", parts: [text] },
          },
        });
      });
      return { streamRequestId: `stream-live-${idx}`, prepareResponse: { conduit_token: `ctok-${idx}` } };
    },
  });

  // Lane 1 starts with short wait_ms -> returns in_progress
  const first = await driver.sendMessage({ prompt: "Long essay A", project_id: "g-p-9", wait_ms: 30 });
  assert.equal(first.state, "in_progress");
  assert.equal(first.generating, true);
  assert.equal(first.chat_id, "conv-live-9");
  assert.equal(first.assistant_text, "Streaming chunk lane 1...");
  assert.ok(first.lane_id);

  // Concurrent send to the SAME chat_id MUST fail closed with generation_already_active
  const sameChatBusy = await driver.sendMessage({
    chat_id: "conv-live-9",
    prompt: "Duplicate write to same chat",
    wait_ms: 30,
  });
  assert.equal(sameChatBusy.state, "error");
  assert.equal(sameChatBusy.error, "generation_already_active");

  // Concurrent send to a NEW independent chat MUST run in parallel as Lane 2!
  const second = await driver.sendMessage({ prompt: "Long essay B", project_id: "g-p-9", wait_ms: 30 });
  assert.equal(second.state, "in_progress");
  assert.equal(second.generating, true);
  assert.equal(second.chat_id, "conv-live-10");
  assert.equal(second.assistant_text, "Streaming chunk lane 2...");
  assert.notEqual(second.lane_id, first.lane_id);

  // status() reports both active parallel lanes
  const st = await driver.status();
  assert.equal(st.generating, true);
  assert.equal(st.active_turn_count, 2);
  assert.equal(st.active_turns.length, 2);

  // observeChat isolates each lane's partial stream cleanly
  const obs1 = await driver.observeChat({ chat_id: "conv-live-9" });
  const obs2 = await driver.observeChat({ chat_id: "conv-live-10" });
  assert.equal(obs1.state, "in_progress");
  assert.equal(obs1.assistant_text, "Streaming chunk lane 1...");
  assert.equal(obs2.state, "in_progress");
  assert.equal(obs2.assistant_text, "Streaming chunk lane 2...");

  // Targeted stopGeneration({ chat_id: "conv-live-9" }) stops ONLY Lane 1, keeping Lane 2 alive!
  const stopped1 = await driver.stopGeneration({ chat_id: "conv-live-9" });
  assert.equal(stopped1.state, "stopped");
  assert.equal(stopped1.stopped, true);
  assert.equal(stopped1.chat_id, "conv-live-9");
  assert.equal(stopped1.remaining_active_count, 1);
  assert.deepEqual(cancelCalls, ["stream-live-1"]);
  assert.equal(stopCalls.length, 1);
  assert.equal(stopCalls[0].body.conversation_id, "conv-live-9");

  // Lane 2 is STILL generating undisturbed
  const obs2After = await driver.observeChat({ chat_id: "conv-live-10" });
  assert.equal(obs2After.state, "in_progress");
  assert.equal(obs2After.assistant_text, "Streaming chunk lane 2...");

  // Stop remaining lane via lane_id
  const stopped2 = await driver.stopGeneration({ lane_id: second.lane_id });
  assert.equal(stopped2.state, "stopped");
  assert.equal(stopped2.stopped, true);
  assert.equal(stopped2.remaining_active_count, 0);
  assert.deepEqual(cancelCalls, ["stream-live-1", "stream-live-2"]);
  assert.equal(stopCalls[1].body.conversation_id, "conv-live-10");
});

test("page_driver on app://- Electron origin uses connect-app-host Cap'n Web httpFetch", async () => {
  const seenRequests = [];
  let serverPort = null;
  let serverExportId = 1;
  let nextClientExportId = 1;

  class FakeMessagePort {
    constructor() {
      this.peer = null;
      this.onmessage = null;
      this.onmessageerror = null;
    }
    start() {}
    close() {}
    postMessage(data) {
      const target = this.peer;
      queueMicrotask(() => {
        if (target && target.onmessage) target.onmessage({ data });
      });
    }
  }
  class FakeMessageChannel {
    constructor() {
      this.port1 = new FakeMessagePort();
      this.port2 = new FakeMessagePort();
      this.port1.peer = this.port2;
      this.port2.peer = this.port1;
    }
  }

  const routes = {
    "GET /me": {
      status: 200,
      body: { id: "u-electron", name: "Desktop User", orgs: { data: [{ id: "acc-e1", title: "Plus", is_personal: true }] } },
    },
    "GET /models": { status: 200, body: { models: [{ slug: "gpt-5-6", title: "GPT-5.6", max_tokens: 128000 }] } },
    "GET /conversations?offset=0&limit=5&order=updated&is_archived=false": {
      status: 200,
      body: { total: 1, items: [{ id: "c-el-1", title: "App Chat", create_time: 10, update_time: 20 }] },
    },
  };

  const sandbox = {
    location: new URL("app://-/index.html"),
    MessageChannel: FakeMessageChannel,
    TextEncoder,
    TextDecoder,
    Uint8Array,
    ArrayBuffer,
    crypto: { randomUUID: () => "uuid-1" },
    window: {
      location: new URL("app://-/index.html"),
      postMessage(msg) {
        if (msg && msg.type === "connect-app-host" && msg.port) {
          serverPort = msg.port;
          serverPort.onmessage = ({ data }) => {
            if (!Array.isArray(data)) return;
            if (data[0] === "push") {
              const impId = serverExportId++;
              const [, , , args] = data[1];
              const req = args[1];
              seenRequests.push(req);
              const hit = routes[`${req.method} ${req.url}`];
              const pipeId = nextClientExportId++;
              serverPort.postMessage(["pipe"]);
              const payload = new TextEncoder().encode(JSON.stringify(hit ? hit.body : { error: "not_found" }));
              const status = hit ? hit.status : 404;
              const respMeta = status === 200 ? { headers: [] } : { status, headers: [] };
              serverPort.postMessage([
                "resolve",
                impId,
                { response: ["response", ["readable", pipeId], respMeta] },
              ]);
              nextClientExportId++;
              serverPort.postMessage(["stream", ["pipeline", pipeId, ["write"], [["bytes", payload]]]]);
              nextClientExportId++;
              serverPort.postMessage(["stream", ["pipeline", pipeId, ["close"], []]]);
            }
          };
        }
      },
    },
    document: {
      querySelector: () => null,
      querySelectorAll: () => [],
      getElementById: () => null,
    },
    fetch: async () => {
      throw new Error("raw fetch must not be called on app://- origin");
    },
    setTimeout,
    clearTimeout,
    Intl,
    URL,
    Date,
  };
  sandbox.globalThis = sandbox;
  vm.runInNewContext(DRIVER_SRC, sandbox);

  const driver = sandbox.__termuxChatGptDriver;
  const st = await driver.status();
  assert.equal(st.transport, "app_host_http_fetch");
  assert.equal(st.auth_state, "authenticated");
  assert.equal(st.account.id, "u-electron");
  assert.equal(st.models[0].slug, "gpt-5-6");
  assert.equal(seenRequests[0].headers["X-OpenAI-Attach-Auth"], "1");

  const chats = await driver.listChats({ limit: 5 });
  assert.equal(chats.state, "ok");
  assert.equal(chats.chats[0].id, "c-el-1");
});

test("page_driver tool-only turn does not leak stale text, observeChat reports async error, schema types/collisions/zero-arg & listProjects cursor work", async () => {
  let streamErrCb = null;
  const { driver, startCalls } = createMockEnv({
    apiMap: {
      "GET /backend-api/me": {
        status: 200,
        body: { id: "u1", name: "User", orgs: { data: [{ id: "acc-1", title: "Plus", is_personal: true }] } },
      },
      "GET /backend-api/gizmos/snorlax/sidebar?conversations_per_gizmo=0&cursor=page2": {
        status: 200,
        body: { items: [{ gizmo: { gizmo: { id: "g-p-2", display: { name: "Proj 2" } } } }], cursor: null },
      },
      "GET /backend-api/conversation/conv-tool-only": {
        status: 200,
        body: {
          title: "Tool Only",
          current_node: "n2",
          mapping: {
            n1: { parent: null, message: { id: "m1", author: { role: "user" }, content: { content_type: "text", parts: ["Turn 1"] } } },
            n2: { parent: "n1", message: { id: "m2", author: { role: "assistant" }, content: { content_type: "text", parts: ["STALE_TURN_1_TEXT"] } } },
          },
        },
      },
    },
    completionBehavior(args) {
      if (args.request.conversation_id === "conv-tool-only") {
        queueMicrotask(() => {
          args.onUpdate?.({
            type: "message",
            conversationId: "conv-tool-only",
            message: {
              id: "msg-zero-arg",
              author: { role: "assistant" },
              recipient: "ns_b_status",
              status: "finished_successfully",
              content: { content_type: "code", text: "" },
            },
          });
          args.onComplete?.({ reason: "done" });
        });
      } else {
        streamErrCb = args.onError;
      }
      return { streamRequestId: "stream-x", prepareResponse: { conduit_token: "ctok-x" } };
    },
  });

  // 1. listProjects passes cursor query parameter
  const page2 = await driver.listProjects({ cursor: "page2" });
  assert.equal(page2.state, "ok");
  assert.equal(page2.projects[0].id, "g-p-2");

  // 2. Tool-only turn with array/object/nullable schema, cross-namespace collision, and zero-arg call
  const res = await driver.sendMessage({
    chat_id: "conv-tool-only",
    prompt: "Check status",
    local_function_signatures: [
      {
        type: "function",
        name: "status",
        namespace: "ns_a",
        parameters: {
          type: "object",
          properties: {
            ids: { type: ["array", "null"], items: { type: "string" } },
            meta: { type: "object", properties: { ok: { type: "boolean" } } },
          },
          required: ["ids"],
        },
      },
      {
        type: "function",
        name: "status",
        namespace: "ns_b",
        parameters: { type: "object", properties: {} },
      },
    ],
    wait_ms: 1000,
  });
  assert.equal(res.state, "settled");
  assert.equal(res.assistant_text, "");
  assert.ok(!res.assistant_text.includes("STALE_TURN_1_TEXT"));
  assert.equal(res.tool_calls.length, 1);
  assert.equal(res.tool_calls[0].name, "status");
  assert.equal(res.tool_calls[0].namespace, "ns_b");
  assert.equal(res.tool_calls[0].arguments, "{}");

  const sig0 = startCalls[0].request.local_function_signatures[0];
  assert.equal(sig0.params[0].type.type, "array");
  assert.equal(sig0.params[0].type.items.type, "string");
  assert.equal(sig0.params[1].type.type, "object");
  assert.equal(startCalls[0].request.local_function_names.length, 2);

  // 3. Async onError after in_progress MUST be surfaced by observeChat (not swallowed as settled!)
  const pending = await driver.sendMessage({ prompt: "Will fail async", wait_ms: 20 });
  assert.equal(pending.state, "in_progress");
  streamErrCb({ responseStatus: 429, message: "rate limited" });
  const obsErr = await driver.observeChat({});
  assert.equal(obsErr.state, "error");
  assert.equal(obsErr.error, "backend_http_429");
});

test("page_driver listConnectors, registerConnector (create+noauth+refresh+probe), idempotency, name conflict, and refreshConnector", async () => {
  const tunnelId = "tunnel_6b03b77cf8bc8191b405430734cc50f7";
  const otherTunnelId = "tunnel_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
  const shellSchema = {
    type: "object",
    properties: { command: { type: "string" } },
    required: ["command"],
  };
  let created = false;
  let linked = false;
  let refreshed = false;
  const seenHeaders = [];

  const { driver } = createMockEnv({
    apiMap: {
      "GET /backend-api/accounts/check/v4-2023-04-27": {
        status: 200,
        body: {
          account_ordering: ["ws-3ebe5ee5"],
          accounts: {
            "ws-3ebe5ee5": {
              account: {
                account_id: "ws-3ebe5ee5",
                name: "Workspace",
                structure: "workspace",
                plan_type: "team",
              },
            },
          },
        },
      },
      "GET /backend-api/aip/connectors/mcp/tunnels": {
        status: 200,
        body: (opts) => {
          seenHeaders.push(opts.headers);
          return { tunnels: [{ id: tunnelId, name: "TermuxShell" }] };
        },
      },
      "POST /backend-api/aip/connectors/links/list_accessible": {
        status: 200,
        body: () => ({
          links: created
            ? [
                {
                  id: "link_1",
                  connector_id: "connector_shell_1",
                  name: "TermuxShell",
                  auth_status: "OK",
                  auth_type: "NONE",
                },
              ]
            : [],
        }),
      },
      "POST /backend-api/aip/connectors/mcp": {
        status: 200,
        body: (opts) => {
          const body = JSON.parse(opts.body);
          assert.equal(body.tunnel_id, tunnelId);
          assert.equal(body.auth_request.supported_auth[0].type, "NONE");
          created = true;
          return { connector: { id: "connector_shell_1", name: body.name, tunnel_id: body.tunnel_id } };
        },
      },
      "GET /backend-api/aip/connectors/connector_shell_1/link": {
        status: 200,
        body: () => ({
          link: linked ? { id: "link_1", connector_id: "connector_shell_1", auth_status: "OK" } : null,
        }),
      },
      "POST /backend-api/aip/connectors/links/noauth": {
        status: 200,
        body: (opts) => {
          const body = JSON.parse(opts.body);
          assert.equal(body.connector_id, "connector_shell_1");
          linked = true;
          return { id: "link_1" };
        },
      },
      "POST /backend-api/aip/connectors/mcp/refresh_actions": {
        status: 200,
        body: (opts) => {
          const body = JSON.parse(opts.body);
          assert.equal(body.link_id, "link_1");
          refreshed = true;
          return { ok: true };
        },
      },
      "POST /backend-api/ecosystem/call_mcp": {
        status: 200,
        body: {
          tools: [{ name: "termux_shell", inputSchema: shellSchema }],
        },
      },
      "GET /backend-api/aip/connectors/connector_shell_1?include_actions=true": {
        status: 200,
        body: () => ({
          id: "connector_shell_1",
          name: "TermuxShell",
          tunnel_id: tunnelId,
          actions: refreshed ? [{ name: "termux_shell", params: shellSchema }] : [],
        }),
      },
    },
  });

  const listed = await driver.listConnectors({});
  assert.equal(listed.state, "ok");
  assert.equal(listed.account_id, "ws-3ebe5ee5");
  assert.equal(listed.tunnels.length, 1);
  assert.equal(seenHeaders[0]["ChatGPT-Account-Id"], "ws-3ebe5ee5");
  assert.equal(seenHeaders[0]["OAI-Product-Sku"], "CONNECTOR_SETTING");

  const reg = await driver.registerConnector({
    name: "TermuxShell",
    description: "Stage 2 shell MCP",
    tunnel_id: tunnelId,
  });
  assert.equal(reg.state, "ok");
  assert.equal(reg.created, true);
  assert.equal(reg.linked, true);
  assert.equal(reg.connector_id, "connector_shell_1");
  assert.equal(reg.link_id, "link_1");
  assert.equal(reg.schema_agreement, true);
  assert.equal(reg.actions[0].name, "termux_shell");

  // Re-registering the same tunnel_id is idempotent (no duplicate creation)
  const again = await driver.registerConnector({
    name: "TermuxShell",
    tunnel_id: tunnelId,
  });
  assert.equal(again.state, "ok");
  assert.equal(again.created, false);
  assert.equal(again.already_existed, true);

  // Same name with a DIFFERENT tunnel_id must fail closed with name_conflict
  const conflict = await driver.registerConnector({
    name: "TermuxShell",
    tunnel_id: otherTunnelId,
  });
  assert.equal(conflict.state, "error");
  assert.equal(conflict.error, "name_conflict");

  // Explicit refreshConnector succeeds with schema_agreement
  const ref = await driver.refreshConnector({ connector_id: "connector_shell_1" });
  assert.equal(ref.state, "ok");
  assert.equal(ref.schema_agreement, true);
});

test("page_driver observeChat inspects server stream_status for remote mobile/web turns", async () => {
  let remoteStatus = "IS_STREAMING";
  const { driver } = createMockEnv({
    apiMap: {
      "GET /backend-api/conversation/conv-mobile-1/stream_status": {
        status: 200,
        body: () => ({ status: remoteStatus }),
      },
      "GET /backend-api/conversation/conv-mobile-1": {
        status: 200,
        body: {
          title: "[F1] Mobile Work",
          current_node: "n2",
          mapping: {
            n1: { parent: null, message: { id: "m1", author: { role: "user" }, content: { content_type: "text", parts: ["Status?"] } } },
            n2: { parent: "n1", message: { id: "m2", author: { role: "assistant" }, content: { content_type: "text", parts: ["Done!"] } } },
          },
        },
      },
    },
  });

  // While remote mobile turn is streaming, observeChat MUST report in_progress (no collision!)
  const busy = await driver.observeChat({ chat_id: "conv-mobile-1" });
  assert.equal(busy.state, "in_progress");
  assert.equal(busy.generating, true);
  assert.equal(busy.remote_stream_status, "IS_STREAMING");

  // Once server reports COMPLETE, observeChat returns settled with final assistant text
  remoteStatus = "COMPLETE";
  const settled = await driver.observeChat({ chat_id: "conv-mobile-1" });
  assert.equal(settled.state, "settled");
  assert.equal(settled.generating, false);
  assert.equal(settled.remote_stream_status, "COMPLETE");
  assert.equal(settled.assistant_text, "Done!");
});
