import assert from "node:assert/strict";
import { createHash, webcrypto } from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import vm from "node:vm";

const DRIVER_SRC = fs.readFileSync(
  path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../src/page_driver.js"),
  "utf8"
);

function createMockEnv({ url = "https://chatgpt.com/c/conv-12345678", apiMap = {}, completionBehavior, localFunctionSubmit } = {}) {
  const startCalls = [];
  const cancelCalls = [];
  const stopCalls = [];
  const streamUpdates = [];

  const mockCompletionClient = {
    scope: { get: () => mockCompletionClient },
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
            recipient: "all",
            channel: "final",
            status: "finished_successfully",
            content: {
              content_type: "text",
              parts: ["Sure!"],
            },
          },
        });
        args.onUpdate?.({
          type: "message",
          conversationId: args.request.conversation_id || "conv-new-1",
          message: {
            id: "42",
            author: { role: "assistant" },
            recipient: "local.exec_command",
            status: "finished_successfully",
            content: {
              content_type: "code",
              text: '{"cmd":"ls"}',
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
    crypto: { randomUUID: () => `uuid-test-${++uuidSeq}`, subtle: webcrypto.subtle },
    __termuxMockCompletionClient: mockCompletionClient,
    __termuxMockLocalFunctionSubmit: localFunctionSubmit,
    __termuxChatGptStream: (data) => streamUpdates.push(JSON.parse(data)),
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
    TextEncoder,
  };
  sandbox.globalThis = sandbox;
  vm.runInNewContext(DRIVER_SRC, sandbox);
  return { driver: sandbox.__termuxChatGptDriver, startCalls, cancelCalls, stopCalls, streamUpdates };
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
  assert.equal(st.models[0].title, "GPT-5.6");
  assert.equal(st.models[0].description, null);
  assert.equal(st.models[0].max_tokens, 128000);
  assert.deepEqual(Array.from(st.models[0].tags), []);

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
    "GET /models": {
      status: 200,
      body: {
        models: [
          {
            slug: "gpt-5-6",
            title: "GPT-5.6",
            description: "Flagship AppHost model",
            max_tokens: 128000,
            tags: ["default"],
          },
        ],
      },
    },
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
  assert.equal(st.models[0].title, "GPT-5.6");
  assert.equal(st.models[0].description, "Flagship AppHost model");
  assert.equal(st.models[0].max_tokens, 128000);
  assert.deepEqual(Array.from(st.models[0].tags), ["default"]);
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
    description: "Termux shell MCP",
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

test("pinned send verifies the live account and the continued chat's project before dispatch", async () => {
  let account = "acct-original";
  const { driver, startCalls } = createMockEnv({
    apiMap: {
      "GET /backend-api/me": { status: 200, body: () => ({ account_id: account }) },
      "GET /backend-api/conversation/conv-other-project": {
        status: 200, body: { gizmo_id: "g-p-other", current_node: "node" },
      },
    },
  });
  const params = {
    expected_account_id: "acct-original",
    project_id: "g-p-pinned",
    prompt: "Resume", wait_ms: 50,
  };
  assert.equal((await driver.sendMessage(params)).state, "settled");
  assert.equal(startCalls[0].expectedAccountId, "acct-original");
  assert.equal(startCalls[0].request.gizmo_id, "g-p-pinned");
  assert.equal((await driver.sendMessage({ ...params, chat_id: "conv-other-project" })).error, "pinned_project_mismatch");
  account = "acct-replaced";
  assert.equal((await driver.sendMessage(params)).error, "pinned_account_mismatch");
  assert.equal(startCalls.length, 1);
});

test("legacy user pin verifies the native AppHost account instead of using membership order", async () => {
  let ambiguous = false;
  const accounts = {
    "personal": { account: { account_id: "personal", structure: "personal" } },
    "workspace": { account: { account_id: "workspace", structure: "workspace" } },
  };
  const { driver, startCalls } = createMockEnv({
    apiMap: {
      "GET /backend-api/me": {
        status: 200,
        body: (opts) => {
          const expected = opts.headers?.["X-OpenAI-Expected-Account-Id"];
          if (!ambiguous && expected === "personal") return { error: "account_changed" };
          return { id: "user-pinned" };
        },
      },
      "GET /backend-api/accounts/check/v4-2023-04-27": {
        status: 200, body: { accounts },
      },
    },
  });
  const params = {
    expected_account_id: "user-pinned", project_id: "g-p-pinned",
    prompt: "Resume", wait_ms: 50,
  };
  assert.equal((await driver.sendMessage(params)).state, "settled");
  assert.equal(startCalls[0].expectedAccountId, "workspace");
  ambiguous = true;
  assert.equal((await driver.sendMessage(params)).error, "pinned_account_unavailable");
  assert.equal(startCalls.length, 1);
});

test("streaming publishes AppHost thought headings, titles, recaps, commentary, revisions, and answers cleanly", async () => {
  let callbacks;
  let ready;
  const started = new Promise((resolve) => { ready = resolve; });
  const { driver, streamUpdates } = createMockEnv({
    apiMap: { "GET /backend-api/me": { status: 200, body: { account_id: "acc-1" } } },
    completionBehavior(args) { callbacks = args; ready(); return { streamRequestId: "stream-1" }; },
  });
  const result = driver.sendMessage({
    lane_id: "lane",
    prompt: "test",
    push_events: true,
    verify_identity: true,
    model: "gpt-6-2-thinking",
    reasoning_effort: "max",
  });
  await started;
  const message = (id, content, metadata, extra = {}) => ({
    id, author: { role: "assistant" }, content, metadata, ...extra,
  });
  const decoded = (msg, isInputMessage = false) =>
    callbacks.onDecodedEvent({
      isInputMessage,
      update: { type: "message", conversationId: "conv-test", message: msg },
    });
  const thought = (summary, finished = false) => ({
    summary, content: "PRIVATE_THOUGHT_BODY", finished,
  });

  // 1. Input messages never leak; full reasoning (thoughts parts/summary+content, raw_cot, hidden, hide_all recap, analysis) is opened cleanly.
  decoded(message("user-echo", { content_type: "text", parts: ["ECHO"] }), true);
  decoded(message("direct-parts", { content_type: "thoughts", parts: ["Direct thought line"] }, undefined, { status: "finished_successfully" }));
  decoded(message("raw", { content_type: "thoughts", thoughts: [thought("RAW_COT_HEADING", true)] },
    { summary_type: "raw_cot" }));
  decoded(message("hidden", { content_type: "thoughts", thoughts: [thought("HIDDEN_HEADING", true)] },
    { is_visually_hidden_reasoning_group: true }));
  decoded(message("recap-hide", { content_type: "reasoning_recap", content: "Thought for 3 seconds" },
    { reasoning_recap_type: "hide_all" }, { status: "finished_successfully" }));
  decoded({ ...message("analysis", { content_type: "text", parts: ["Full analysis reasoning"] },
    undefined, { status: "finished_successfully" }), channel: "analysis" });

  // 2. Multi-step thoughts with full content: mutable update -> finish, fallback update after onDecodedEvent ignored.
  decoded(message("think", {
    content_type: "thoughts",
    thoughts: [thought("Step 1 heading"), thought("Checking")],
  }, { is_visually_hidden_from_conversation: true }));
  callbacks.onUpdate({
    type: "message",
    conversationId: "conv-test",
    message: message("think", { content_type: "thoughts", thoughts: [thought("DUPLICATE_ON_UPDATE", true)] }),
  });
  decoded(message("think", {
    content_type: "thoughts",
    thoughts: [thought("Step 1 heading"), thought("Checking files", true)],
  }));
  decoded(message("think", {
    content_type: "thoughts",
    thoughts: [
      thought("Step 1 heading"),
      thought("Checking files", true),
      { summary: "Reviewing changes", chunks: ["Line A\n", "Line B"], finished: true },
    ],
  }));
  decoded(message("titled", { content_type: "thoughts", thoughts: [{ summary: "", content: "" }] },
    { rich_reasoning_titles: ["Reading metadata title"] }, { status: "finished_successfully" }));
  decoded(message("recap", { content_type: "reasoning_recap", content: "Visible recap" },
    undefined, { status: "finished_successfully" }));

  // 3. Commentary preamble kept distinct from final answer + Unicode/whitespace answer revisions.
  decoded(message("pre-1", { content_type: "text", parts: ["Step 1 preamble"] },
    { cot_version: "v5", is_thinking_preamble_message: true }, { channel: "commentary", status: "finished_successfully" }));
  decoded(message("ans-1", { content_type: "text", parts: ["Draft A"] },
    undefined, { channel: "final", status: "in_progress" }));
  decoded(message("ans-1", { content_type: "text", parts: ["Replaced 한국어  text"] },
    undefined, { channel: "final", status: "in_progress" }));
  decoded(message("ans-1", { content_type: "text", parts: ["  Final 한국어 🚀 answer\nLine 2"] },
    undefined, { channel: "final", status: "finished_successfully", end_turn: true }));

  const itemEvents = streamUpdates
    .filter((x) => x.kind === "item_upsert")
    .map((x) => [x.lane_id, x.item_id, x.item_kind, x.text, x.committed]);
  assert.deepEqual(itemEvents, [
    ["lane", "thought:direct-parts", "reasoning_summary", "Direct thought line", true],
    ["lane", "thought:raw:1", "reasoning_summary", "**RAW_COT_HEADING**\nPRIVATE_THOUGHT_BODY", true],
    ["lane", "thought:hidden:1", "reasoning_summary", "**HIDDEN_HEADING**\nPRIVATE_THOUGHT_BODY", true],
    ["lane", "recap:recap-hide", "reasoning_summary", "Thought for 3 seconds", true],
    ["lane", "analysis:analysis", "reasoning_summary", "Full analysis reasoning", true],
    ["lane", "thought:think:1", "reasoning_summary", "**Step 1 heading**\nPRIVATE_THOUGHT_BODY", true],
    ["lane", "thought:think:2", "reasoning_summary", "**Checking**\nPRIVATE_THOUGHT_BODY", false],
    ["lane", "thought:think:2", "reasoning_summary", "**Checking files**\nPRIVATE_THOUGHT_BODY", true],
    ["lane", "thought:think:3", "reasoning_summary", "**Reviewing changes**\nLine A\nLine B", true],
    ["lane", "title:titled:0", "reasoning_summary", "Reading metadata title", true],
    ["lane", "recap:recap", "reasoning_summary", "Visible recap", true],
    ["lane", "commentary:pre-1", "commentary", "Step 1 preamble", true],
    ["lane", "answer:ans-1", "answer", "Draft A", false],
    ["lane", "answer:ans-1", "answer", "Replaced 한국어  text", false],
    ["lane", "answer:ans-1", "answer", "  Final 한국어 🚀 answer\nLine 2", true],
  ]);

  callbacks.onComplete();
  const settled = await result;
  assert.equal(settled.state, "settled");
  assert.equal(settled.account_id, "acc-1");
  assert.ok(!JSON.stringify(settled).includes("ECHO"));
  assert.ok(!JSON.stringify(settled).includes("DUPLICATE_ON_UPDATE"));
  assert.equal(settled.assistant_text, "  Final 한국어 🚀 answer\nLine 2");
  assert.deepEqual(
    JSON.parse(JSON.stringify(settled.items)).map((x) => [x.kind, x.text]),
    [
      ["reasoning_summary", "Direct thought line"],
      ["reasoning_summary", "**RAW_COT_HEADING**\nPRIVATE_THOUGHT_BODY"],
      ["reasoning_summary", "**HIDDEN_HEADING**\nPRIVATE_THOUGHT_BODY"],
      ["reasoning_summary", "Thought for 3 seconds"],
      ["reasoning_summary", "Full analysis reasoning"],
      ["reasoning_summary", "**Step 1 heading**\nPRIVATE_THOUGHT_BODY"],
      ["reasoning_summary", "**Checking files**\nPRIVATE_THOUGHT_BODY"],
      ["reasoning_summary", "**Reviewing changes**\nLine A\nLine B"],
      ["reasoning_summary", "Reading metadata title"],
      ["reasoning_summary", "Visible recap"],
      ["commentary", "Step 1 preamble"],
      ["answer", "  Final 한국어 🚀 answer\nLine 2"],
    ]
  );

  // Non-streaming push_events:false yields identical normalized items and tests native tool commit + dedup.
  let nonStreamCallbacks;
  const nonStreamEnv = createMockEnv({
    apiMap: { "GET /backend-api/me": { status: 200, body: { account_id: "acc-1" } } },
    completionBehavior(args) {
      nonStreamCallbacks = args;
      assert.deepEqual(JSON.parse(JSON.stringify(args.request.supported_encodings)), ["v1"]);
      return { streamRequestId: "stream-2" };
    },
  });
  const nsPromise = nonStreamEnv.driver.sendMessage({
    lane_id: "lane-ns",
    prompt: "test",
    push_events: false,
    wait_ms: 5000,
    local_function_signatures: [{ type: "function", name: "check_tool", parameters: { type: "object" } }],
  });
  await new Promise((r) => setTimeout(r, 10));
  nonStreamCallbacks.onDecodedEvent({
    update: { type: "message", conversationId: "conv-ns", message: message("t1", { content_type: "thoughts", thoughts: [thought("Thinking step", true)] }) },
  });
  nonStreamCallbacks.onDecodedEvent({
    update: { type: "message", conversationId: "conv-ns", message: message("a1", { content_type: "text", parts: ["Final NS"] }, undefined, { status: "finished_successfully" }) },
  });
  nonStreamCallbacks.onDecodedEvent({
    update: {
      type: "message",
      conversationId: "conv-ns",
      message: { id: "tc1", author: { role: "assistant" }, recipient: "check_tool", content: { text: "{\"x\":1}" }, status: "finished_successfully" },
    },
  });
  // Duplicate delivery of the same message ID must not duplicate normalized items.
  nonStreamCallbacks.onDecodedEvent({
    update: {
      type: "message",
      conversationId: "conv-ns",
      message: { id: "tc1", author: { role: "assistant" }, recipient: "check_tool", content: { text: "{\"x\":1}" }, status: "finished_successfully" },
    },
  });
  nonStreamCallbacks.onComplete();
  const nsSettled = await nsPromise;
  assert.deepEqual(JSON.parse(JSON.stringify(nsSettled.items)), [
    { id: "thought:t1:1", kind: "reasoning_summary", text: "**Thinking step**\nPRIVATE_THOUGHT_BODY" },
    { id: "answer:a1", kind: "answer", text: "Final NS" },
    { id: "tool:call_tc1", kind: "tool_call", call_id: "call_tc1", name: "check_tool", arguments: "{\"x\":1}" },
  ]);
  assert.equal(nonStreamEnv.streamUpdates.length, 0);
});

test("page_driver omits temporary chat flags, deduplicates reasoning titles, commits prior thoughts before tools, and ignores undeclared recipients", async () => {
  let callbacks;
  let ready;
  let resolveStarted = () => {};
  const { driver, startCalls, streamUpdates } = createMockEnv({
    apiMap: {
      "GET /backend-api/me": { status: 200, body: { account_id: "acc-tools" } },
      "GET /backend-api/conversation/conv-existing": {
        status: 200,
        body: { current_node: "node-1", mapping: {} },
      },
    },
    completionBehavior(args) {
      callbacks = args;
      resolveStarted();
      return { streamRequestId: `stream-${startCalls.length}` };
    },
  });

  // 1. Standard turn with local_function_signatures omits temporary chat & disabled_plugin_ids.
  ready = new Promise((resolve) => { resolveStarted = resolve; });
  const p1 = driver.sendMessage({
    lane_id: "lane-t1",
    prompt: "Run pwd",
    push_events: true,
    model: "gpt-6-2-thinking",
    local_function_signatures: [{ type: "function", name: "exec_command", parameters: { type: "object" } }],
  });
  await ready;
  assert.equal(startCalls[0].request.is_do_not_remember, undefined);
  assert.equal(startCalls[0].request.history_and_training_disabled, undefined);
  assert.equal(startCalls[0].request.temporary_chat_requests_personalization, undefined);
  assert.equal(startCalls[0].request.disabled_plugin_ids, undefined);

  // Uncommitted thought commits before new tool-role title / tool_call; duplicate heading is suppressed; tool body never leaks.
  callbacks.onDecodedEvent({
    update: {
      type: "message",
      conversationId: "conv-existing",
      message: {
        id: "m-thought-open",
        author: { role: "assistant" },
        status: "in_progress",
        content: {
          content_type: "thoughts",
          thoughts: [{ summary: "Planning shell command", content: "Need git branch from local repo.", finished: false }],
        },
      },
    },
  });
  callbacks.onDecodedEvent({
    update: {
      type: "message",
      conversationId: "conv-existing",
      message: {
        id: "m-tool-dup-title",
        author: { role: "tool" },
        status: "finished_successfully",
        content: { content_type: "text", parts: ["TOOL_BODY_MUST_NOT_LEAK"] },
        metadata: { rich_reasoning_titles: ["Planning shell command", "Checked remote status"] },
      },
    },
  });
  callbacks.onDecodedEvent({
    update: {
      type: "message",
      conversationId: "conv-existing",
      message: {
        id: "m-call-1",
        author: { role: "assistant" },
        recipient: "exec_command",
        status: "finished_successfully",
        content: { text: "{\"cmd\":\"pwd\"}" },
      },
    },
  });
  callbacks.onComplete();
  const s1 = await p1;
  assert.equal(s1.state, "settled");
  assert.ok(!JSON.stringify(streamUpdates).includes("TOOL_BODY_MUST_NOT_LEAK"));
  const upsertsAndCalls = streamUpdates
    .filter((x) => x.kind === "item_upsert" || x.kind === "tool_call")
    .map((x) => x.kind === "tool_call" ? [x.kind, x.name] : [x.kind, x.item_id, x.text, x.committed]);
  assert.deepEqual(upsertsAndCalls, [
    ["item_upsert", "thought:m-thought-open:1", "**Planning shell command**\nNeed git branch from local repo.", false],
    ["item_upsert", "thought:m-thought-open:1", "**Planning shell command**\nNeed git branch from local repo.", true],
    ["item_upsert", "title:m-tool-dup-title:1", "Checked remote status", true],
    ["tool_call", "exec_command"],
  ]);

  // 2. Continuation turn preserves conversation_id without temporary chat flags.
  ready = new Promise((resolve) => { resolveStarted = resolve; });
  const p2 = driver.sendMessage({
    lane_id: "lane-t2",
    chat_id: "conv-existing",
    prompt: "[Tool Output call_1]\n/repo",
    model: "gpt-6-2-thinking",
    local_function_signatures: [{ type: "function", name: "exec_command", parameters: { type: "object" } }],
  });
  await ready;
  assert.equal(startCalls[1].request.conversation_id, "conv-existing");
  assert.equal(startCalls[1].request.is_do_not_remember, undefined);
  assert.equal(startCalls[1].request.history_and_training_disabled, undefined);
  assert.equal(startCalls[1].request.temporary_chat_requests_personalization, undefined);
  callbacks.onComplete();
  await p2;

  // 3. Control send without local tools ignores internal recipients like `python`.
  ready = new Promise((resolve) => { resolveStarted = resolve; });
  const p3 = driver.sendMessage({
    lane_id: "lane-t3",
    prompt: "Plain chat without local tools",
    model: "gpt-6-2-thinking",
  });
  await ready;
  assert.equal(startCalls[2].request.is_do_not_remember, undefined);
  assert.equal(startCalls[2].request.history_and_training_disabled, undefined);
  callbacks.onDecodedEvent({
    update: {
      type: "message",
      conversationId: "conv-plain",
      message: {
        id: "m-python-internal",
        author: { role: "assistant" },
        recipient: "python",
        status: "finished_successfully",
        content: { text: "print(1+1)" },
      },
    },
  });
  callbacks.onDecodedEvent({
    update: {
      type: "message",
      conversationId: "conv-plain",
      message: {
        id: "m-final-plain",
        author: { role: "assistant" },
        status: "finished_successfully",
        content: { content_type: "text", parts: ["2"] },
      },
    },
  });
  callbacks.onComplete();
  const s3 = await p3;
  assert.equal(s3.tool_calls.length, 0);
  assert.equal(s3.assistant_text, "2");
});

test("page_driver projects interim pre-tool messages and stripped tool reports as commentary alongside reasoning", async () => {
  let callbacks;
  let ready;
  let resolveStarted = () => {};
  const { driver, startCalls, streamUpdates } = createMockEnv({
    apiMap: {
      "GET /backend-api/me": { status: 200, body: { account_id: "acc-commentary" } },
    },
    completionBehavior(args) {
      callbacks = args;
      resolveStarted();
      return { streamRequestId: `stream-c-${startCalls.length}` };
    },
  });

  ready = new Promise((resolve) => { resolveStarted = resolve; });
  const p = driver.sendMessage({
    lane_id: "lane-c1",
    prompt: "Inspect repo",
    push_events: true,
    model: "gpt-6-2-thinking",
    local_function_signatures: [
      {
        type: "function",
        name: "exec_command",
        parameters: { type: "object", properties: { cmd: { type: "string" } }, required: ["cmd"] },
      },
      {
        type: "function",
        name: "already_has_report",
        parameters: { type: "object", properties: { report: { type: "string" } }, required: ["report"] },
      },
      {
        type: "custom",
        name: "apply_patch",
      },
    ],
  });
  await ready;

  const sigs = startCalls[0].request.local_function_signatures;
  const execSig = sigs.find((s) => s.name === "exec_command");
  const keepSig = sigs.find((s) => s.name === "already_has_report");
  const customSig = sigs.find((s) => s.name === "apply_patch");
  assert.ok(execSig.params.some((param) => param.name === "report" && param.required === false));
  assert.equal(keepSig.params.filter((param) => param.name === "report").length, 1);
  assert.ok(!customSig.params.some((param) => param.name === "report"));

  // Interim message with channel="final" and end_turn=false becomes commentary (not final_answer).
  callbacks.onDecodedEvent({
    update: {
      type: "message",
      conversationId: "conv-c1",
      message: {
        id: "m-interim",
        author: { role: "assistant" },
        channel: "final",
        status: "in_progress",
        end_turn: false,
        content: { content_type: "text", parts: ["저장소 상태를 먼저 확인하겠습니다."] },
      },
    },
  });
  callbacks.onDecodedEvent({
    update: {
      type: "message",
      conversationId: "conv-c1",
      message: {
        id: "m-interim",
        author: { role: "assistant" },
        channel: "final",
        status: "finished_successfully",
        end_turn: false,
        content: { content_type: "text", parts: ["저장소 상태를 먼저 확인하겠습니다."] },
      },
    },
  });
  // Simultaneous reasoning thought arrives.
  callbacks.onDecodedEvent({
    update: {
      type: "message",
      conversationId: "conv-c1",
      message: {
        id: "m-thought",
        author: { role: "assistant" },
        status: "finished_successfully",
        content: {
          content_type: "thoughts",
          thoughts: [{ summary: "Checking git status", content: "Will run git status -sb.", finished: true }],
        },
      },
    },
  });
  // Tool call with injected `report`: stripped from forwarded arguments, emitted as commentary before tool_call.
  callbacks.onDecodedEvent({
    update: {
      type: "message",
      conversationId: "conv-c1",
      message: {
        id: "m-exec",
        author: { role: "assistant" },
        recipient: "exec_command",
        status: "finished_successfully",
        content: { text: JSON.stringify({ cmd: "git status -sb", report: "현재 브랜치와 변경 파일을 조회합니다." }) },
      },
    },
  });
  // Native tool that already declared `report` preserves its `report` parameter untouched.
  callbacks.onDecodedEvent({
    update: {
      type: "message",
      conversationId: "conv-c1",
      message: {
        id: "m-keep",
        author: { role: "assistant" },
        recipient: "already_has_report",
        status: "finished_successfully",
        content: { text: JSON.stringify({ report: "KEEP_ME" }) },
      },
    },
  });
  callbacks.onComplete();
  const settled = await p;

  assert.equal(settled.state, "settled");
  assert.equal(settled.assistant_text, "");
  assert.deepEqual(
    JSON.parse(JSON.stringify(settled.tool_calls.map((c) => [c.name, c.arguments]))),
    [
      ["exec_command", "{\"cmd\":\"git status -sb\"}"],
      ["already_has_report", "{\"report\":\"KEEP_ME\"}"],
    ]
  );
  const committedSeq = JSON.parse(
    JSON.stringify(
      streamUpdates
        .filter((x) => (x.kind === "item_upsert" && x.committed) || x.kind === "tool_call")
        .map((x) => (x.kind === "tool_call" ? ["tool_call", x.name, x.arguments] : [x.item_kind, x.text]))
    )
  );
  assert.deepEqual(committedSeq, [
    ["commentary", "저장소 상태를 먼저 확인하겠습니다."],
    ["reasoning_summary", "**Checking git status**\nWill run git status -sb."],
    ["commentary", "현재 브랜치와 변경 파일을 조회합니다."],
    ["tool_call", "exec_command", "{\"cmd\":\"git status -sb\"}"],
    ["tool_call", "already_has_report", "{\"report\":\"KEEP_ME\"}"],
  ]);
});

test("page_driver enforces single item per msgKey so promoted pre-tool commentary is never duplicated on late close", async () => {
  let callbacks;
  let ready;
  let resolveStarted = () => {};
  const { driver, streamUpdates } = createMockEnv({
    apiMap: {
      "GET /backend-api/me": { status: 200, body: { account_id: "acc-dedup" } },
    },
    completionBehavior(args) {
      callbacks = args;
      resolveStarted();
      return { streamRequestId: "stream-dedup-1" };
    },
  });

  ready = new Promise((resolve) => { resolveStarted = resolve; });
  const p = driver.sendMessage({
    lane_id: "lane-dedup",
    prompt: "저장소 확인",
    push_events: true,
    model: "gpt-6-thinking",
    local_function_signatures: [
      {
        type: "function",
        name: "exec_command",
        parameters: { type: "object", properties: { cmd: { type: "string" } }, required: ["cmd"] },
      },
    ],
  });
  await ready;

  const tableMsg = "## Codex 저장소 확인 결과\n\n| 항목 | 결과 |\n|---|---|\n| 브랜치 | main |";
  // 1. Message M1 starts in_progress as channel="final".
  callbacks.onDecodedEvent({
    update: {
      type: "message",
      conversationId: "conv-dedup",
      message: {
        id: "m-table",
        author: { role: "assistant" },
        channel: "final",
        status: "in_progress",
        content: { content_type: "text", parts: [tableMsg] },
      },
    },
  });
  // 2. Tool call M2 arrives, promoting M1 to committed commentary before tool_call.
  callbacks.onDecodedEvent({
    update: {
      type: "message",
      conversationId: "conv-dedup",
      message: {
        id: "m-call-git",
        author: { role: "assistant" },
        recipient: "exec_command",
        status: "finished_successfully",
        content: { text: JSON.stringify({ cmd: "git show -1 --stat" }) },
      },
    },
  });
  // 3. Reasoning recap arrives.
  callbacks.onDecodedEvent({
    update: {
      type: "message",
      conversationId: "conv-dedup",
      message: {
        id: "m-recap",
        author: { role: "assistant" },
        status: "finished_successfully",
        content: { content_type: "reasoning_recap", content: "Worked for 5s" },
      },
    },
  });
  // 4. Late closing snapshot of the exact same M1 with end_turn=false arrives (reproduces resp_b7053492bc608f8e).
  callbacks.onDecodedEvent({
    update: {
      type: "message",
      conversationId: "conv-dedup",
      message: {
        id: "m-table",
        author: { role: "assistant" },
        channel: "final",
        status: "finished_successfully",
        end_turn: false,
        content: { content_type: "text", parts: [tableMsg] },
      },
    },
  });
  // 5. Genuine strict suffix expansion on M1 only emits the new suffix, never repeats tableMsg.
  callbacks.onDecodedEvent({
    update: {
      type: "message",
      conversationId: "conv-dedup",
      message: {
        id: "m-table",
        author: { role: "assistant" },
        channel: "final",
        status: "finished_successfully",
        end_turn: false,
        content: { content_type: "text", parts: [`${tableMsg}\n\n추가 메모: 검증 완료.`] },
      },
    },
  });
  callbacks.onComplete();
  const settled = await p;

  const committedSeq = JSON.parse(
    JSON.stringify(
      streamUpdates
        .filter((x) => (x.kind === "item_upsert" && x.committed) || x.kind === "tool_call")
        .map((x) => (x.kind === "tool_call" ? ["tool_call", x.name] : [x.item_kind, x.text]))
    )
  );
  assert.deepEqual(committedSeq, [
    ["commentary", tableMsg],
    ["tool_call", "exec_command"],
    ["reasoning_summary", "Worked for 5s"],
    ["commentary", "추가 메모: 검증 완료."],
  ]);
  assert.equal(settled.items.filter((it) => it.text === tableMsg).length, 1);
});

// Structured Option B fixtures emulate the AppHost CompletionClient wire shape.
function structuredFixture({ secondRound = true, toolRounds = 1, parallelCalls = 1, streamMutation, apiMap = {}, developer = false, oneTurnDeveloper = false, hiddenUserPreamble = false, liveResultCommentary = false, zeroArgCall = false } = {}) {
  const chat = 'conv-structured';
  const callId = 'original-call-123456789012345678901234567890';
  const original = {
    id: callId,
    author: { role: 'assistant' },
    recipient: 'local.fixture',
    status: 'finished_successfully',
    content: zeroArgCall ? { content_type: 'code', text: '' } : { content_type: 'code', text: '{"x":1}' },
  };
  const commentary = {
    id: 'live-result-commentary', author: { role: 'assistant' }, recipient: 'all',
    channel: 'commentary', status: 'finished_successfully',
    content: { content_type: 'text', parts: ['Checking the returned value.'] },
  };
  const apiHits = [];
  let round = 0;
  const env = createMockEnv({
    apiMap: {
      'GET /backend-api/me': { status: 200, body: () => { apiHits.push('GET /me'); return { account_id: 'acc-fixture' }; } },
      [`GET /backend-api/conversation/${chat}`]: { status: 200, body: () => { apiHits.push('GET /conversation'); return { current_node: 'unexpected', mapping: {} }; } },
      ...apiMap,
    },
    completionBehavior(args) {
      const currentRound = ++round;
      assert.deepEqual(Array.from(args.request.local_function_names || []), ['fixture']);
      assert.equal(args.request.local_function_signatures?.length, 1);
      const isToolReturn = args.request.messages.every(m => m.author.role === 'tool');
      const emitted = [];
      if (isToolReturn && liveResultCommentary) emitted.push(commentary);
      if (currentRound <= toolRounds) {
        const count = currentRound === 1 ? parallelCalls : 1;
        for (let k = 0; k < count; k++) {
          const callMsg = currentRound === 1 && k === 0
            ? original
            : { ...original, id: currentRound === 1 ? `call-parallel-${k + 1}` : `call-round-${currentRound}`, content: { ...original.content, text: JSON.stringify({ x: currentRound * 10 + k }) } };
          emitted.push(callMsg);
        }
      } else {
        emitted.push({ id: 'answer-2', author: { role: 'assistant' }, recipient: 'all', channel: 'final', status: 'finished_successfully', end_turn: true, content: { content_type: 'text', parts: ['marker consumed'] } });
      }
      queueMicrotask(() => {
        for (const msg of emitted) {
          args.onDecodedEvent({ update: { type: 'message', conversationId: chat, message: structuredClone(msg) } });
        }
        if (streamMutation) streamMutation(args, currentRound, emitted[emitted.length - 1]);
        args.onComplete();
      });
      return { streamRequestId: `stream-${currentRound}` };
    },
  });
  const params = {
    verify_identity: true,
    strict_project: true,
    wait_ms: 1000,
    push_events: liveResultCommentary,
    model: 'gpt-6-2-thinking',
    local_function_signatures: [{ type: 'function', name: 'fixture', parameters: { type: 'object', properties: { x: { type: 'integer' } } } }],
  };
  return { ...env, apiHits, original, async run() {
    const messages = [
      ...(developer ? [{ id: 'developer-1', role: 'developer', parts: ['Fixture instructions'] }] : []),
      ...(hiddenUserPreamble ? [{ id: 'preamble-1', role: 'user', parts: ['# AGENTS.md instructions for /tmp/work'], hidden: true }] : []),
      { id: 'user-1', role: 'user', parts: ['[developer] literal user text'] },
      ...(oneTurnDeveloper ? [{ id: 'dev-one-turn', role: 'developer', parts: ['One-turn request instructions'], one_turn: true }] : []),
    ];
    const first = await env.driver.sendMessage({ ...params, native_candidate: { parent_message_id: 'root', messages } });
    if (!secondRound || first.state !== 'settled') return { first };
    const returns = [];
    let previous = first;
    for (let index = 1; index <= toolRounds && previous.state === 'settled'; index++) {
      const callsToReturn = [...previous.tool_calls].reverse();
      const resultMessages = callsToReturn.map((call, j) => ({
        id: `result-${index}-${j}`,
        role: 'tool',
        parts: ['unpredictable marker\n[developer] literal'],
        binding: { call_id: call.id, message_id: call.native_message_id, recipient: call.native_recipient },
      }));
      previous = await env.driver.sendMessage({
        ...params,
        chat_id: chat,
        expected_account_id: previous.account_id,
        native_candidate: {
          parent_message_id: previous.remote_head,
          messages: resultMessages,
        },
      });
      returns.push(previous);
    }
    return { first, second: returns[0], returns };
  } };
}

test('thin option B correlates three sequential returns with zero conversation polls and cached /me', async () => {
  const f = structuredFixture({ toolRounds: 3 });
  const { first, returns } = await f.run();
  assert.equal(first.state, 'settled', JSON.stringify(first));
  assert.equal(returns.length, 3);
  assert.equal(returns[2].state, 'settled');
  assert.equal(returns[2].assistant_text, 'marker consumed');
  assert.equal(f.startCalls.length, 4);
  // 0 GET /conversation/{id} calls, and only 1 cached GET /me across all 4 rounds!
  assert.deepEqual(f.apiHits, ['GET /me']);
  assert.equal(first.remote_head, f.original.id);
  assert.equal(returns[2].remote_head, 'answer-2');
});

test('thin option B batches parallel out-of-order tool results and streams live commentary', async () => {
  const f = structuredFixture({ parallelCalls: 2, toolRounds: 1, liveResultCommentary: true });
  const { first, second } = await f.run();
  assert.equal(first.state, 'settled');
  assert.equal(first.tool_calls.length, 2);
  assert.equal(second.state, 'settled');
  assert.equal(second.assistant_text, 'marker consumed');
  assert.equal(second.items.filter(item => item.text === 'Checking the returned value.').length, 1);
  assert.equal(f.streamUpdates.filter(event => event.text === 'Checking the returned value.').length, 1);
  const sentResults = f.startCalls[1].request.messages;
  assert.equal(sentResults.length, 2);
  assert.equal(JSON.parse(sentResults[0].content.text).call_id, 'call-parallel-2');
  assert.equal(JSON.parse(sentResults[1].content.text).call_id, f.original.id);
  assert.equal(f.startCalls[1].expectedAccountId, 'acc-fixture');
});

test('thin option B preserves developer role, one-turn metadata, hidden user preamble, and zero-arg calls', async () => {
  const f = structuredFixture({ developer: true, oneTurnDeveloper: true, hiddenUserPreamble: true, zeroArgCall: true });
  const { first, second } = await f.run();
  assert.equal(first.state, 'settled');
  assert.equal(first.tool_calls.length, 1);
  assert.equal(first.tool_calls[0].arguments, '{}');
  assert.equal(second.state, 'settled');
  const [dev, hiddenPreamble, user, oneTurn] = f.startCalls[0].request.messages;
  assert.equal(dev.author.role, 'developer');
  assert.equal(dev.metadata.is_visually_hidden_from_conversation, true);
  assert.equal(dev.metadata.exclude_after_next_user_message, undefined);
  assert.equal(hiddenPreamble.author.role, 'user');
  assert.equal(hiddenPreamble.metadata.is_visually_hidden_from_conversation, true);
  assert.equal(user.author.role, 'user');
  assert.equal(user.metadata?.is_visually_hidden_from_conversation, undefined);
  assert.equal(oneTurn.author.role, 'developer');
  assert.equal(oneTurn.metadata.exclude_after_next_user_message, true);
});

test('thin option B supports temporary chats and project-only memory_scope on createProject', async () => {
  let createdBody = null;
  const env = createMockEnv({
    apiMap: {
      'GET /backend-api/me': { status: 200, body: { account_id: 'acc-temp' } },
      'POST /backend-api/projects': { status: 200, body: (opts) => {
        createdBody = JSON.parse(opts.body);
        return { gizmo: { id: 'g-p-util', memory_scope: createdBody.memory_scope, display: { name: createdBody.name } } };
      } },
    },
    completionBehavior(args) {
      queueMicrotask(() => {
        args.onDecodedEvent({
          update: {
            type: 'message',
            conversationId: 'conv-temp-1',
            message: { id: 'ans-temp', author: { role: 'assistant' }, recipient: 'all', channel: 'final', status: 'finished_successfully', end_turn: true, content: { content_type: 'text', parts: ['Short Title'] } },
          },
        });
        args.onComplete();
      });
      return { streamRequestId: 'stream-temp' };
    },
  });
  const res = await env.driver.sendMessage({
    model: 'gpt-6-2-instant',
    temporary: true,
    project_id: 'g-p-should-be-omitted',
    wait_ms: 1000,
    native_candidate: {
      parent_message_id: 'root',
      messages: [{ id: 'u-temp', role: 'user', parts: ['Generate a title'] }],
    },
  });
  assert.equal(res.state, 'settled');
  assert.equal(res.assistant_text, 'Short Title');
  const req = env.startCalls[0].request;
  assert.equal(req.history_and_training_disabled, true);
  assert.equal(req.is_do_not_remember, true);
  assert.equal(req.gizmo_id, undefined);

  const proj = await env.driver.createProject({ name: 'Codex Utility', memory_scope: 'project_v2' });
  assert.equal(proj.state, 'ok');
  assert.equal(proj.project_id, 'g-p-util');
  assert.equal(proj.memory_scope, 'project_v2');
  assert.equal(createdBody.memory_scope, 'project_v2');
});

test('thin option B resolves native account pin, deduplicates signatures, and rejects chat identity switch', async () => {
  for (const verifiedAccounts of [['workspace'], [], ['personal', 'workspace']]) {
    const f = structuredFixture({ apiMap: {
      'GET /backend-api/me': { status: 200, body: opts => {
        const expected = opts.headers?.['X-OpenAI-Expected-Account-Id'];
        return !expected || verifiedAccounts.includes(expected) ? { id: 'authenticated-user' } : { error: 'account_changed' };
      } },
      'GET /backend-api/accounts/check/v4-2023-04-27': { status: 200, body: {
        account_ordering: ['personal', 'workspace'],
        accounts: {
          personal: { account: { account_id: 'personal' } },
          workspace: { account: { account_id: 'workspace' } },
        },
      } },
    } });
    const { first, second } = await f.run();
    if (verifiedAccounts.length === 1) {
      assert.equal(first.state, 'settled');
      assert.equal(second.state, 'settled');
      assert.equal(first.account_id, 'workspace');
      assert.deepEqual(f.startCalls.map(call => call.expectedAccountId), ['workspace', 'workspace']);
    } else {
      assert.equal(first.error, 'pinned_account_unavailable');
      assert.equal(f.startCalls.length, 0);
    }
  }

  const switched = structuredFixture({
    secondRound: false,
    streamMutation(args) { args.onDecodedEvent({ update: { type: 'message', conversationId: 'wrong-chat' } }); },
  });
  assert.equal((await switched.run()).first.error, 'native_chat_changed');

  // Duplicate tool signatures deduplicated cleanly and branching from prior remote_head works natively.
  const f = structuredFixture({ secondRound: false });
  const dupSig = { type: 'function', name: 'fixture', parameters: { type: 'object', properties: { x: { type: 'integer' } } } };
  const first = await f.driver.sendMessage({
    verify_identity: true,
    wait_ms: 1000,
    model: 'gpt-6-2-thinking',
    local_function_signatures: [dupSig, dupSig],
    native_candidate: { parent_message_id: 'root', messages: [{ id: 'u-1', role: 'user', parts: ['hello'] }] },
  });
  assert.equal(first.state, 'settled');
  const retry = await f.driver.sendMessage({
    verify_identity: true,
    wait_ms: 1000,
    chat_id: first.chat_id,
    expected_account_id: first.account_id,
    model: 'gpt-6-2-thinking',
    local_function_signatures: [dupSig],
    native_candidate: { parent_message_id: first.remote_head, messages: [{ id: 'u-2', role: 'user', parts: ['retry after cancel'] }] },
  });
  assert.equal(retry.state, 'settled');
  assert.equal(f.startCalls[1].request.parent_message_id, first.remote_head);
});

test('thin option B propagates Retry-After from completion stream errors', async () => {
  const env = createMockEnv({
    apiMap: {
      'GET /backend-api/me': { status: 200, body: { account_id: 'acc-rl' } },
    },
    completionBehavior(args) {
      queueMicrotask(() => {
        args.onError({
          responseStatus: 429,
          error: 'Too Many Requests',
          responseHeaders: { 'Retry-After': '37' },
        });
      });
      return { streamRequestId: 'stream-rl' };
    },
  });
  const res = await env.driver.sendMessage({
    model: 'gpt-6-2-thinking',
    wait_ms: 1000,
    native_candidate: { parent_message_id: 'root', messages: [{ id: 'u-rl', role: 'user', parts: ['hi'] }] },
  });
  assert.equal(res.state, 'error');
  assert.equal(res.error, 'backend_http_429');
  assert.equal(res.retry_after, '37');
});
