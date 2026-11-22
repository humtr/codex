import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import vm from "node:vm";

const DRIVER_SRC = fs.readFileSync(
  path.resolve(import.meta.dirname, "../src/page_driver.js"),
  "utf8"
);

function createMockEnv({ url = "https://chatgpt.com/c/conv-12345678", apiMap = {}, generating = false } = {}) {
  const sentPrompts = [];
  let stopVisible = generating;
  let assistantNodes = [];

  const composer = {
    tagName: "TEXTAREA",
    value: "",
    focused: false,
    focus() {
      this.focused = true;
    },
    dispatchEvent() {},
  };
  const sendBtn = {
    disabled: false,
    click() {
      sentPrompts.push(composer.value);
      assistantNodes.push({
        innerText:
          'Sure!\n```codex_tool_call\n{"call_id":"call_42","name":"exec_command","arguments":{"cmd":"ls"}}\n```',
      });
    },
  };
  const stopBtn = {
    click() {
      stopVisible = false;
    },
  };

  const sandbox = {
    location: new URL(url),
    window: { HTMLTextAreaElement: { prototype: {} } },
    Event: class Event {
      constructor(type, init) {
        this.type = type;
        this.init = init;
      }
    },
    InputEvent: class InputEvent {
      constructor(type, init) {
        this.type = type;
        this.init = init;
      }
    },
    document: {
      querySelector(sel) {
        if (sel === "#prompt-textarea") return composer;
        if (sel === 'button[data-testid="send-button"]') return sendBtn;
        if (sel === 'button[data-testid="stop-button"]') return stopVisible ? stopBtn : null;
        return null;
      },
      querySelectorAll(sel) {
        if (sel === '[data-message-author-role="assistant"]') return assistantNodes;
        return [];
      },
    },
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
    URL,
    Date,
  };
  sandbox.globalThis = sandbox;
  vm.runInNewContext(DRIVER_SRC, sandbox);
  return { driver: sandbox.__termuxChatGptDriver, sentPrompts };
}

test("page_driver status, chats, linear branch read (no private thoughts), projects, and tool callback parsing", async () => {
  const { driver, sentPrompts } = createMockEnv({
    apiMap: {
      "GET /backend-api/me": { status: 200, body: { id: "u1", name: "User", orgs: { data: [{ title: "Plus" }] } } },
      "GET /backend-api/models": { status: 200, body: { models: [{ slug: "gpt-5.4", title: "GPT-5.4", max_tokens: 128000 }] } },
      "GET /backend-api/conversations?offset=0&limit=20&order=updated&is_archived=false": {
        status: 200,
        body: { total: 2, items: [{ id: "conv-12345678", title: "Demo Chat", create_time: 100, update_time: 200 }] },
      },
      "GET /backend-api/conversation/conv-12345678": {
        status: 200,
        body: {
          title: "Demo Chat",
          current_node: "n3",
          mapping: {
            n1: { parent: null, message: { id: "m1", author: { role: "user" }, content: { content_type: "text", parts: ["Hi"] } } },
            n2: { parent: "n1", message: { id: "m2", author: { role: "assistant" }, content: { content_type: "thoughts", parts: ["PRIVATE_THOUGHT_MUST_NOT_LEAK"] } } },
            n3: {
              parent: "n2",
              message: {
                id: "m3",
                author: { role: "assistant" },
                content: {
                  content_type: "text",
                  parts: ['Sure!\n```codex_tool_call\n{"call_id":"call_42","name":"exec_command","arguments":{"cmd":"ls"}}\n```'],
                },
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
    },
  });

  const st = await driver.status();
  assert.equal(st.auth_state, "authenticated");
  assert.equal(st.models[0].slug, "gpt-5.4");

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

  const sent = await driver.sendMessage({
    chat_id: "conv-12345678",
    prompt: "List directory",
    local_function_signatures: [{ type: "function", name: "exec_command" }],
    wait_ms: 500,
  });
  assert.equal(sent.state, "settled");
  assert.equal(sent.assistant_text, "Sure!");
  assert.equal(sent.tool_calls.length, 1);
  assert.equal(sent.tool_calls[0].id, "call_42");
  assert.equal(sent.tool_calls[0].name, "exec_command");
  assert.ok(sentPrompts[0].includes("[LOCAL_FUNCTION_SIGNATURES]"));
});

test("page_driver stopGeneration clicks stop button cleanly", async () => {
  const { driver } = createMockEnv({ generating: true });
  const res = await driver.stopGeneration();
  assert.equal(res.state, "stopped");
  assert.equal(res.stopped, true);
});
