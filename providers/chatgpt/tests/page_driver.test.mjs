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
  // and omits retired local_function_names/local_function_signatures.
  const sent = await driver.sendMessage({
    chat_id: "conv-12345678",
    prompt: "List directory",
    model: "gpt-5-6-thinking",
    reasoning_effort: "low",
    wait_ms: 1000,
  });
  assert.equal(sent.state, "settled");
  assert.equal(sent.chat_id, "conv-12345678");
  assert.equal(sent.assistant_text, "Sure!");
  assert.deepEqual(Array.from(sent.tool_calls), []);

  const wire = startCalls[0].request;
  assert.equal(wire.conversation_id, "conv-12345678");
  assert.equal(wire.parent_message_id, "n3");
  assert.equal(wire.gizmo_id, "g-p-1");
  assert.equal(wire.conversation_mode.kind, "gizmo_interaction");
  assert.equal(wire.model, "gpt-5-6-thinking");
  assert.equal(wire.thinking_effort, "min");
  assert.equal(wire.local_function_names, undefined);
  assert.equal(wire.local_function_signatures, undefined);
  assert.equal(wire.messages[0].content.parts[0], "List directory");

  // Instant model must omit thinking_effort (ChatGPT returns 422 if sent on instant models).
  await driver.sendMessage({
    prompt: "Quick reply",
    model: "gpt-5-6-instant",
    reasoning_effort: "medium",
    is_continuation: true,
    wait_ms: 1000,
  });
  assert.equal(startCalls[1].request.thinking_effort, undefined);
  assert.equal(startCalls[1].request.local_function_names, undefined);
  assert.equal(startCalls[1].request.local_function_signatures, undefined);
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
              const respMeta = status === 200 ? { headers: hit?.headers || [] } : { status, headers: hit?.headers || [] };
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

  routes["GET /me"] = { status: 429, body: { error: "rate_limited" }, headers: [["Retry-After", "37"]] };
  const limited = await driver.status();
  assert.equal(limited.retry_after, "37");

  routes["GET /me"] = { status: 200, body: "x".repeat(8 * 1024 * 1024 + 1) };
  const oversized = await driver.status();
  assert.match(oversized.backend_error || "", /response_too_large/);
});

test("page_driver does not leak stale text, surfaces MCP progress, observeChat reports async error, and listProjects cursor works", async () => {
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
          title: "MCP Turn",
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
              id: "msg-mcp-call",
              author: { role: "assistant" },
              recipient: "TermuxShell.termux_shell",
              status: "finished_successfully",
              content: { content_type: "code", text: "{\"command\":\"SECRET_ARG\"}" },
            },
          });
          args.onUpdate?.({
            type: "message",
            conversationId: "conv-tool-only",
            message: {
              id: "msg-mcp-done",
              author: { role: "tool", name: "TermuxShell.termux_shell" },
              status: "finished_successfully",
              content: { content_type: "text", parts: ["SECRET_STDOUT"] },
              metadata: { tool_response_metadata: { mcp_tool_result: { isError: false } } },
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

  // 2. MCP progress emits secret-free reasoning summaries without leaking stale turn-1 text
  const res = await driver.sendMessage({
    chat_id: "conv-tool-only",
    prompt: "Check status",
    wait_ms: 1000,
  });
  assert.equal(res.state, "settled");
  assert.equal(res.assistant_text, "");
  assert.ok(!res.assistant_text.includes("STALE_TURN_1_TEXT"));
  assert.ok(res.reasoning_summary.includes("Running TermuxShell.termux_shell"));
  assert.ok(res.reasoning_summary.includes("Completed TermuxShell.termux_shell"));
  assert.ok(!res.reasoning_summary.includes("SECRET_ARG"));
  assert.ok(!res.reasoning_summary.includes("SECRET_STDOUT"));
  assert.equal(startCalls[0].request.local_function_names, undefined);

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

  // Non-streaming push_events:false yields identical normalized items and deduplicates MCP progress.
  let nonStreamCallbacks;
  const nonStreamEnv = createMockEnv({
    apiMap: { "GET /backend-api/me": { status: 200, body: { account_id: "acc-1" } } },
    completionBehavior(args) {
      nonStreamCallbacks = args;
      assert.deepEqual(JSON.parse(JSON.stringify(args.request.supported_encodings)), ["v1"]);
      assert.equal(args.request.local_function_names, undefined);
      return { streamRequestId: "stream-2" };
    },
  });
  const nsPromise = nonStreamEnv.driver.sendMessage({
    lane_id: "lane-ns",
    prompt: "test",
    push_events: false,
    wait_ms: 5000,
  });
  await new Promise((r) => setTimeout(r, 10));
  nonStreamCallbacks.onDecodedEvent({
    update: { type: "message", conversationId: "conv-ns", message: message("t1", { content_type: "thoughts", thoughts: [thought("Thinking step", true)] }) },
  });
  nonStreamCallbacks.onDecodedEvent({
    update: {
      type: "message",
      conversationId: "conv-ns",
      message: { id: "mcp1", author: { role: "assistant" }, recipient: "TermuxShell.termux_shell", content: { text: "{\"command\":\"ls\"}" }, status: "finished_successfully" },
    },
  });
  nonStreamCallbacks.onDecodedEvent({
    update: {
      type: "message",
      conversationId: "conv-ns",
      message: { id: "mcp1", author: { role: "assistant" }, recipient: "TermuxShell.termux_shell", content: { text: "{\"command\":\"ls\"}" }, status: "finished_successfully" },
    },
  });
  nonStreamCallbacks.onDecodedEvent({
    update: { type: "message", conversationId: "conv-ns", message: message("a1", { content_type: "text", parts: ["Final NS"] }, undefined, { status: "finished_successfully" }) },
  });
  nonStreamCallbacks.onComplete();
  const nsSettled = await nsPromise;
  assert.deepEqual(JSON.parse(JSON.stringify(nsSettled.items)), [
    { id: "thought:t1:1", kind: "reasoning_summary", text: "**Thinking step**\nPRIVATE_THOUGHT_BODY" },
    { id: "mcp:mcp1", kind: "reasoning_summary", text: "Running TermuxShell.termux_shell" },
    { id: "answer:a1", kind: "answer", text: "Final NS" },
  ]);
  assert.equal(nonStreamEnv.streamUpdates.length, 0);
});

test("MCP single-turn preserves developer role, one-turn metadata, hidden user preamble, and rejects legacy tool role", async () => {
  const apiHits = [];
  const env = createMockEnv({
    apiMap: {
      "GET /backend-api/me": { status: 200, body: () => { apiHits.push("GET /me"); return { account_id: "acc-mcp" }; } },
    },
    completionBehavior(args) {
      queueMicrotask(() => {
        args.onDecodedEvent({
          update: {
            type: "message",
            conversationId: "conv-mcp-1",
            message: {
              id: "ans-mcp-1",
              author: { role: "assistant" },
              recipient: "all",
              channel: "final",
              status: "finished_successfully",
              end_turn: true,
              content: { content_type: "text", parts: ["Single turn complete"] },
            },
          },
        });
        args.onComplete();
      });
      return { streamRequestId: "stream-mcp-1" };
    },
  });

  const first = await env.driver.sendMessage({
    model: "gpt-6-2-thinking",
    reasoning_effort: "high",
    wait_ms: 1000,
    native_candidate: {
      parent_message_id: "root-1",
      messages: [
        { id: "dev-1", role: "developer", parts: ["Developer rule"] },
        { id: "pre-1", role: "user", hidden: true, parts: ["<environment_context><cwd>/repo</cwd></environment_context>"] },
        { id: "usr-1", role: "user", parts: ["Run git status"] },
        { id: "inst-1", role: "developer", one_turn: true, parts: ["[Termux MCP Single-Turn Execution Mode]"] },
      ],
    },
  });
  assert.equal(first.state, "settled");
  assert.equal(first.chat_id, "conv-mcp-1");
  assert.equal(first.remote_head, "ans-mcp-1");
  assert.equal(first.assistant_text, "Single turn complete");

  const req = env.startCalls[0].request;
  assert.equal(req.local_function_names, undefined);
  assert.equal(req.local_function_signatures, undefined);
  const [dev, hiddenPreamble, user, oneTurn] = req.messages;
  assert.equal(dev.author.role, "developer");
  assert.equal(dev.metadata.is_visually_hidden_from_conversation, true);
  assert.equal(dev.metadata.exclude_after_next_user_message, undefined);
  assert.equal(hiddenPreamble.author.role, "user");
  assert.equal(hiddenPreamble.metadata.is_visually_hidden_from_conversation, true);
  assert.equal(user.author.role, "user");
  assert.equal(user.metadata?.is_visually_hidden_from_conversation, undefined);
  assert.equal(oneTurn.author.role, "developer");
  assert.equal(oneTurn.metadata.exclude_after_next_user_message, true);

  // Follow-up turn reuses cached /me (zero extra GET /me) and continues from remote_head.
  const second = await env.driver.sendMessage({
    chat_id: first.chat_id,
    expected_account_id: first.account_id,
    model: "gpt-6-2-thinking",
    wait_ms: 1000,
    native_candidate: {
      parent_message_id: first.remote_head,
      messages: [{ id: "usr-2", role: "user", parts: ["Follow up"] }],
    },
  });
  assert.equal(second.state, "settled");
  assert.deepEqual(apiHits, ["GET /me"]);
  assert.equal(env.startCalls[1].request.parent_message_id, "ans-mcp-1");

  // Legacy role:"tool" wire messages are rejected cleanly in MCP single-turn mode.
  const bad = await env.driver.sendMessage({
    model: "gpt-6-2-thinking",
    wait_ms: 1000,
    native_candidate: {
      parent_message_id: "root-2",
      messages: [{ id: "t-1", role: "tool", parts: ["out"], binding: { message_id: "m1", recipient: "local.exec" } }],
    },
  });
  assert.equal(bad.state, "error");
  assert.equal(bad.error, "native_input_unsupported");
});

test("MCP single-turn resolves native account pin and rejects chat identity switch", async () => {
  for (const verifiedAccounts of [["workspace"], [], ["personal", "workspace"]]) {
    const env = createMockEnv({
      apiMap: {
        "GET /backend-api/me": {
          status: 200,
          body: (opts) => {
            const expected = opts.headers?.["X-OpenAI-Expected-Account-Id"];
            return !expected || verifiedAccounts.includes(expected)
              ? { id: "authenticated-user" }
              : { error: "account_changed" };
          },
        },
        "GET /backend-api/accounts/check/v4-2023-04-27": {
          status: 200,
          body: {
            account_ordering: ["personal", "workspace"],
            accounts: {
              personal: { account: { account_id: "personal" } },
              workspace: { account: { account_id: "workspace" } },
            },
          },
        },
      },
      completionBehavior(args) {
        queueMicrotask(() => {
          args.onDecodedEvent({
            update: {
              type: "message",
              conversationId: "conv-pin-1",
              message: {
                id: "ans-pin-1",
                author: { role: "assistant" },
                recipient: "all",
                channel: "final",
                status: "finished_successfully",
                end_turn: true,
                content: { content_type: "text", parts: ["Pinned OK"] },
              },
            },
          });
          args.onComplete();
        });
        return { streamRequestId: "stream-pin" };
      },
    });
    const res = await env.driver.sendMessage({
      verify_identity: true,
      model: "gpt-6-2-thinking",
      wait_ms: 1000,
      native_candidate: {
        parent_message_id: "root",
        messages: [{ id: "u-pin", role: "user", parts: ["hello"] }],
      },
    });
    if (verifiedAccounts.length === 1) {
      assert.equal(res.state, "settled");
      assert.equal(res.account_id, "workspace");
      assert.equal(env.startCalls[0].expectedAccountId, "workspace");
    } else {
      assert.equal(res.error, "pinned_account_unavailable");
      assert.equal(env.startCalls.length, 0);
    }
  }

  const switched = createMockEnv({
    apiMap: { "GET /backend-api/me": { status: 200, body: { account_id: "acc-1" } } },
    completionBehavior(args) {
      queueMicrotask(() => {
        args.onDecodedEvent({ update: { type: "message", conversationId: "wrong-chat" } });
        args.onComplete();
      });
      return { streamRequestId: "stream-switch" };
    },
  });
  const res = await switched.driver.sendMessage({
    chat_id: "conv-expected",
    expected_account_id: "acc-1",
    model: "gpt-6-2-thinking",
    wait_ms: 1000,
    native_candidate: {
      parent_message_id: "p-1",
      messages: [{ id: "u-sw", role: "user", parts: ["hi"] }],
    },
  });
  assert.equal(res.error, "native_chat_changed");
});

test("temporary chats and project-only memory_scope work on createProject", async () => {
  let createdBody = null;
  const env = createMockEnv({
    apiMap: {
      "GET /backend-api/me": { status: 200, body: { account_id: "acc-temp" } },
      "POST /backend-api/projects": {
        status: 200,
        body: (opts) => {
          createdBody = JSON.parse(opts.body);
          return { gizmo: { id: "g-p-util", memory_scope: createdBody.memory_scope, display: { name: createdBody.name } } };
        },
      },
    },
    completionBehavior(args) {
      queueMicrotask(() => {
        args.onDecodedEvent({
          update: {
            type: "message",
            conversationId: "conv-temp-1",
            message: {
              id: "ans-temp",
              author: { role: "assistant" },
              recipient: "all",
              channel: "final",
              status: "finished_successfully",
              end_turn: true,
              content: { content_type: "text", parts: ["Short Title"] },
            },
          },
        });
        args.onComplete();
      });
      return { streamRequestId: "stream-temp" };
    },
  });
  const res = await env.driver.sendMessage({
    model: "gpt-6-2-instant",
    temporary: true,
    project_id: "g-p-should-be-omitted",
    wait_ms: 1000,
    native_candidate: {
      parent_message_id: "root",
      messages: [{ id: "u-temp", role: "user", parts: ["Generate a title"] }],
    },
  });
  assert.equal(res.state, "settled");
  assert.equal(res.assistant_text, "Short Title");
  const req = env.startCalls[0].request;
  assert.equal(req.history_and_training_disabled, true);
  assert.equal(req.is_do_not_remember, true);
  assert.equal(req.gizmo_id, undefined);

  const proj = await env.driver.createProject({ name: "Codex Utility", memory_scope: "project_v2" });
  assert.equal(proj.state, "ok");
  assert.equal(proj.project_id, "g-p-util");
  assert.equal(proj.memory_scope, "project_v2");
  assert.equal(createdBody.memory_scope, "project_v2");
});

test("live eligible snapshots grow before completion and freeze on rewrite", async () => {
  let callbacks, ready;
  const started = new Promise((resolve) => { ready = resolve; });
  const { driver, streamUpdates } = createMockEnv({
    apiMap: { "GET /backend-api/me": { status: 200, body: { account_id: "acc-1" } } },
    completionBehavior(args) { callbacks = args; ready(); return { streamRequestId: "live" }; },
  });
  const pending = driver.sendMessage({ lane_id: "live", prompt: "test", push_events: true });
  await started;
  const update = (id, channel, text, status = "in_progress") => callbacks.onDecodedEvent({
    update: {
      type: "message",
      conversationId: "live-chat",
      message: {
        id,
        author: { role: "assistant" },
        channel,
        status,
        content: { content_type: "text", parts: [text] },
      },
    },
  });
  for (const text of ["Look", "Looking", "Looking now"]) update("r", "analysis", text);
  update("r", "analysis", "Looking now", "finished_successfully");
  for (const text of ["한", "한국어", "한국어 🚀"]) update("a", "final", text);
  const live = streamUpdates.filter((e) => e.streamable && !e.committed);
  assert.deepEqual(live.map((e) => e.text), ["Looking", "Looking now", "한국어", "한국어 🚀"]);
  update("a", "final", "rewritten");
  callbacks.onComplete({ usage: { input_tokens: 12, output_tokens: 3 } });
  const settled = await pending;
  assert.equal(settled.assistant_text, "한국어 🚀");
  assert.equal(settled.usage.total_tokens, 15);
});

test("Retry-After propagates and rejects subsequent dispatches without replay until expiry", async () => {
  let count = 0;
  const { driver, startCalls } = createMockEnv({
    apiMap: { "GET /backend-api/me": { status: 200, body: { account_id: "acc-1" } } },
    completionBehavior(args) {
      count += 1;
      if (count === 1) {
        args.onError({ responseStatus: 429, error: "Too Many Requests", responseHeaders: { "Retry-After": "30" } });
      } else {
        args.onComplete();
      }
      return { streamRequestId: `rl-${count}` };
    },
  });
  const failed = await driver.sendMessage({ prompt: "test", lane_id: "p1", push_events: true });
  assert.equal(failed.state, "error");
  assert.equal(failed.error, "backend_http_429");
  assert.equal(failed.retry_after, "30");
  const blocked = await driver.sendMessage({ prompt: "test", lane_id: "p2", push_events: true });
  assert.equal(blocked.state, "error");
  assert.equal(blocked.error, "backend_http_429");
  assert.ok(Number(blocked.retry_after) > 0);
  assert.equal(startCalls.length, 1);
});

test("resumed extension after early commit reuses one ext item instead of duplicating chunks", async () => {
  const { driver } = createMockEnv({
    apiMap: { "GET /backend-api/me": { status: 200, body: { account_id: "acc-1" } } },
    completionBehavior({ onDecodedEvent, onComplete }) {
      const emit = (msg) => onDecodedEvent({
        update: { type: "message", conversationId: "ext-chat", message: msg },
      });
      // Initial chunk of msg-1, early-committed by an intervening commentary message (promoteInterimAnswer=true).
      emit({
        id: "msg-1",
        author: { role: "assistant" },
        channel: "final",
        status: "in_progress",
        content: { content_type: "text", parts: ["Part 1."] },
      });
      emit({
        id: "commentary-1",
        author: { role: "assistant" },
        channel: "commentary",
        status: "finished_successfully",
        end_turn: false,
        content: { content_type: "text", parts: ["Checking details..."] },
      });
      // msg-1 resumes streaming across multiple in_progress snapshots then finishes.
      for (const tail of ["Part 2", "Part 2 grows", "Part 2 grows completely."]) {
        emit({
          id: "msg-1",
          author: { role: "assistant" },
          channel: "final",
          status: "in_progress",
          content: { content_type: "text", parts: [`Part 1.\n${tail}`] },
        });
      }
      emit({
        id: "msg-1",
        author: { role: "assistant" },
        channel: "final",
        status: "finished_successfully",
        end_turn: true,
        content: { content_type: "text", parts: ["Part 1.\nPart 2 grows completely."] },
      });
      onComplete({});
      return { streamRequestId: "ext-1" };
    },
  });
  const settled = await driver.sendMessage({ lane_id: "ext-lane", prompt: "go", push_events: true });
  assert.equal(settled.state, "settled");
  const visible = settled.items.filter((i) => i.kind === "answer" || i.kind === "commentary");
  assert.equal(visible.length, 3, `expected 3 items (initial + commentary + single resumed ext), got ${visible.length}`);
  assert.equal(visible[0].text, "Part 1.");
  assert.equal(visible[1].text, "Checking details...");
  assert.equal(visible[2].text, "Part 2 grows completely.");
});

test("continuation syncs advanced ChatGPT App current_node and surfaces missed turns as commentary", async () => {
  const env = createMockEnv({
    apiMap: {
      "GET /backend-api/me": { status: 200, body: { account_id: "acc-1" } },
      "GET /backend-api/conversation/chat-sync": {
        status: 200,
        body: {
          current_node: "app-asst-3",
          gizmo_id: "g-p-codex",
          mapping: {
            "codex-asst-2": {
              id: "codex-asst-2",
              parent: "root",
              message: { id: "codex-asst-2", author: { role: "assistant" }, content: { content_type: "text", parts: ["Turn 2 reply"] } },
            },
            "app-user-3": {
              id: "app-user-3",
              parent: "codex-asst-2",
              message: { id: "app-user-3", author: { role: "user" }, content: { content_type: "text", parts: ["Continued from phone"] } },
            },
            "app-asst-3": {
              id: "app-asst-3",
              parent: "app-user-3",
              message: { id: "app-asst-3", author: { role: "assistant" }, content: { content_type: "text", parts: ["Phone reply done"] } },
            },
          },
        },
      },
    },
    completionBehavior({ onDecodedEvent, onComplete }) {
      onDecodedEvent({
        update: {
          type: "message",
          conversationId: "chat-sync",
          message: {
            id: "codex-asst-4",
            author: { role: "assistant" },
            channel: "final",
            status: "finished_successfully",
            end_turn: true,
            content: { content_type: "text", parts: ["Turn 4 using phone context"] },
          },
        },
      });
      onComplete({});
      return { streamRequestId: "sync-4" };
    },
  });
  const res = await env.driver.sendMessage({
    lane_id: "sync-lane",
    chat_id: "chat-sync",
    project_id: "g-p-codex",
    push_events: true,
    native_candidate: {
      parent_message_id: "codex-asst-2",
      messages: [{ id: "u4", role: "user", parts: ["Now continue in TUI"] }],
    },
  });
  assert.equal(res.state, "settled");
  // Must attach Turn 4 onto app-asst-3 (NOT fork off codex-asst-2!)
  assert.equal(env.startCalls[0].request.parent_message_id, "app-asst-3");
  assert.equal(res.remote_head, "codex-asst-4");
  const syncItem = res.items.find((i) => i.kind === "commentary" && i.id.startsWith("sync:"));
  assert.ok(syncItem, "expected commentary sync item for missed ChatGPT App turn");
  assert.match(syncItem.text, /Continued from phone/);
  assert.match(syncItem.text, /Phone reply done/);
});
