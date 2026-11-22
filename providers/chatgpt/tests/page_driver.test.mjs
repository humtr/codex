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

  const sandbox = {
    location: new URL(url),
    window: { location: new URL(url) },
    document: {
      querySelector: () => null,
      querySelectorAll: () => [],
      getElementById: () => null,
    },
    crypto: { randomUUID: () => "uuid-test-1" },
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

test("page_driver observeChat while in_progress, busy rejection, and programmatic stopGeneration", async () => {
  const { driver, cancelCalls, stopCalls } = createMockEnv({
    apiMap: {
      "GET /backend-api/me": {
        status: 200,
        body: { id: "u1", name: "User", orgs: { data: [{ id: "acc-1", title: "Plus", is_personal: true }] } },
      },
    },
    completionBehavior(args) {
      queueMicrotask(() => {
        args.onUpdate?.({
          type: "message",
          conversationId: "conv-live-9",
          message: {
            id: "msg-partial",
            author: { role: "assistant" },
            content: { content_type: "text", parts: ["Streaming chunk..."] },
          },
        });
      });
      return { streamRequestId: "stream-live-9", prepareResponse: { conduit_token: "ctok-9" } };
    },
  });

  // Start with short wait_ms -> returns in_progress
  const first = await driver.sendMessage({ prompt: "Long essay", project_id: "g-p-9", wait_ms: 30 });
  assert.equal(first.state, "in_progress");
  assert.equal(first.generating, true);
  assert.equal(first.chat_id, "conv-live-9");
  assert.equal(first.assistant_text, "Streaming chunk...");

  // Concurrent send must fail closed with generation_already_active
  const busy = await driver.sendMessage({ prompt: "Second prompt", wait_ms: 30 });
  assert.equal(busy.state, "error");
  assert.equal(busy.error, "generation_already_active");

  // observeChat returns live partial stream
  const obs = await driver.observeChat({ chat_id: "conv-live-9" });
  assert.equal(obs.state, "in_progress");
  assert.equal(obs.assistant_text, "Streaming chunk...");

  // stopGeneration cancels stream & sends server stopCompletion
  const stopped = await driver.stopGeneration();
  assert.equal(stopped.state, "stopped");
  assert.equal(stopped.stopped, true);
  assert.deepEqual(cancelCalls, ["stream-live-9"]);
  assert.equal(stopCalls.length, 1);
  assert.equal(stopCalls[0].body.conversation_id, "conv-live-9");
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
