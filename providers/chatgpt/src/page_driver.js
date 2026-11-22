// Clean-room greenfield page driver for the official Linux ChatGPT Desktop app.
//
// Injected into the ChatGPT renderer page via CDP Runtime.evaluate.
// Zero legacy app_bridge.js / DDPE / MCP coupling.
// Uses same-origin authenticated backend APIs for reads/metadata and minimal
// composer/stop selectors for turn driving. Never reads or returns cookies/tokens.
(() => {
  if (globalThis.__termuxChatGptDriver && globalThis.__termuxChatGptDriver.version === 1) {
    return { installed: true, version: 1 };
  }

  const API_BASE =
    location && location.protocol === "https:"
      ? "/backend-api"
      : "https://chatgpt.com/backend-api";
  const DEFAULT_LIMIT = 20;
  const MAX_TEXT_CHARS = 65536;

  function truncate(str, limit = MAX_TEXT_CHARS) {
    if (typeof str !== "string") return "";
    return str.length > limit ? str.slice(0, limit) : str;
  }

  async function apiFetch(path, options = {}, timeoutMs = 5000) {
    const controller = typeof AbortController === "function" ? new AbortController() : null;
    const timer = controller ? setTimeout(() => controller.abort(), timeoutMs) : null;
    try {
      const resp = await fetch(`${API_BASE}${path}`, {
        credentials: "include",
        headers: {
          "Content-Type": "application/json",
          ...(options.headers || {}),
        },
        signal: controller ? controller.signal : undefined,
        ...options,
      });
      const text = await resp.text();
      let json = null;
      try {
        json = text ? JSON.parse(text) : null;
      } catch (_) {
        json = null;
      }
      return { ok: resp.ok, status: resp.status, json, text: truncate(text, 4096) };
    } catch (err) {
      return {
        ok: false,
        status: 0,
        json: null,
        text: truncate(String(err && err.message ? err.message : err), 512),
      };
    } finally {
      if (timer) clearTimeout(timer);
    }
  }

  function currentChatIdFromUrl() {
    const m = location.pathname.match(/\/c\/([0-9a-fA-F-]{8,})/);
    return m ? m[1] : null;
  }

  function findComposer() {
    return (
      document.querySelector("#prompt-textarea") ||
      document.querySelector('[contenteditable="true"][data-id="root"]') ||
      document.querySelector('textarea[placeholder*="Message"], textarea[data-id]')
    );
  }

  function findSendButton() {
    return (
      document.querySelector('button[data-testid="send-button"]') ||
      document.querySelector('button[aria-label*="Send"]')
    );
  }

  function findStopButton() {
    return (
      document.querySelector('button[data-testid="stop-button"]') ||
      document.querySelector('button[aria-label*="Stop"]')
    );
  }

  function isLoginRequired() {
    const url = location.href || "";
    if (/auth\.openai\.com|\/auth\/login/.test(url)) return true;
    const loginBtn = document.querySelector('[data-testid="login-button"]');
    return Boolean(loginBtn && !findComposer());
  }

  function extractLinearMessages(conv) {
    const mapping = conv && conv.mapping ? conv.mapping : {};
    const chain = [];
    let nodeId = conv && conv.current_node ? conv.current_node : null;
    const seen = new Set();
    while (nodeId && mapping[nodeId] && !seen.has(nodeId)) {
      seen.add(nodeId);
      const node = mapping[nodeId];
      if (node.message) {
        const msg = node.message;
        const role = msg.author && msg.author.role ? msg.author.role : "unknown";
        const contentType = msg.content && msg.content.content_type ? msg.content.content_type : "";
        const parts = Array.isArray(msg.content && msg.content.parts) ? msg.content.parts : [];
        const text = parts
          .filter((p) => typeof p === "string")
          .join("\n")
          .trim();
        if (
          (role === "user" || role === "assistant") &&
          text.length > 0 &&
          contentType !== "thoughts" &&
          contentType !== "reasoning_recap"
        ) {
          chain.push({
            id: msg.id || nodeId,
            role,
            text: truncate(text),
            create_time: msg.create_time || null,
            model: (msg.metadata && msg.metadata.model_slug) || null,
          });
        }
      }
      nodeId = node.parent || null;
    }
    chain.reverse();
    return chain;
  }

  function parseToolCallsFromText(text) {
    // Single-path in-app tool callback extraction for Type 1 (`codex -p chatgpt`)
    // when `local_function_signatures` are supplied.
    const calls = [];
    const fenceRe = /```codex_tool_call\s*([\s\S]*?)```/g;
    let match;
    let stripped = text;
    while ((match = fenceRe.exec(text)) !== null) {
      try {
        const parsed = JSON.parse(match[1].trim());
        if (parsed && typeof parsed.name === "string" && parsed.name.length > 0) {
          calls.push({
            id:
              typeof parsed.call_id === "string" && parsed.call_id
                ? parsed.call_id
                : `call_${calls.length + 1}`,
            name: parsed.name,
            arguments:
              typeof parsed.arguments === "string"
                ? parsed.arguments
                : JSON.stringify(parsed.arguments || {}),
          });
          stripped = stripped.replace(match[0], "").trim();
        }
      } catch (_) {
        // Ignore malformed fenced blocks so user text is preserved.
      }
    }
    return { calls, text: stripped };
  }

  function setComposerText(el, value) {
    el.focus();
    if (el.tagName === "TEXTAREA" || el.tagName === "INPUT") {
      const setter = Object.getOwnPropertyDescriptor(
        window.HTMLTextAreaElement.prototype,
        "value"
      )?.set;
      if (setter) setter.call(el, value);
      else el.value = value;
      el.dispatchEvent(new Event("input", { bubbles: true }));
    } else {
      el.textContent = value;
      el.dispatchEvent(new InputEvent("input", { bubbles: true, data: value }));
    }
  }

  function wait(ms) {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }

  async function status() {
    const loginReq = isLoginRequired();
    let authState = loginReq ? "login_required" : "loading";
    let account = null;
    let models = [];

    if (!loginReq) {
      const me = await apiFetch("/me");
      if (me.ok && me.json) {
        authState = "authenticated";
        account = {
          id: me.json.id || null,
          name: me.json.name || null,
          plan:
            (me.json.orgs && me.json.orgs.data && me.json.orgs.data[0]?.title) ||
            null,
        };
      } else if (me.status === 401 || me.status === 403) {
        authState = "login_required";
      }

      if (authState === "authenticated") {
        const mRes = await apiFetch("/models");
        if (mRes.ok && mRes.json && Array.isArray(mRes.json.models)) {
          models = mRes.json.models.map((m) => ({
            slug: m.slug,
            title: m.title || m.slug,
            max_tokens: m.max_tokens || null,
          }));
        }
      }
    }

    return {
      state: "ok",
      driver_version: 1,
      page_url: `${location.origin}${location.pathname}`,
      auth_state: authState,
      account,
      active_chat_id: currentChatIdFromUrl(),
      generating: Boolean(findStopButton()),
      composer_ready: Boolean(findComposer()),
      models,
    };
  }

  async function listChats(opts = {}) {
    const limit = Math.max(1, Math.min(100, Number(opts.limit) || DEFAULT_LIMIT));
    const offset = Math.max(0, Number(opts.cursor) || 0);
    const archived = opts.archived ? "true" : "false";
    const res = await apiFetch(
      `/conversations?offset=${offset}&limit=${limit}&order=updated&is_archived=${archived}`
    );
    if (!res.ok || !res.json) {
      return { state: "error", error: `backend_http_${res.status}`, detail: res.text };
    }
    const items = Array.isArray(res.json.items) ? res.json.items : [];
    const total = Number(res.json.total || items.length);
    const nextOffset = offset + items.length;
    return {
      state: "ok",
      chats: items.map((c) => ({
        id: c.id,
        title: c.title || "",
        create_time: c.create_time || null,
        update_time: c.update_time || null,
        gizmo_id: c.gizmo_id || null,
      })),
      total,
      next_cursor: nextOffset < total ? String(nextOffset) : null,
    };
  }

  async function readChat(opts = {}) {
    const chatId = opts.chat_id || currentChatIdFromUrl();
    if (!chatId) {
      return { state: "error", error: "chat_id_required", detail: "no chat_id provided or active" };
    }
    const res = await apiFetch(`/conversation/${encodeURIComponent(chatId)}`);
    if (!res.ok || !res.json) {
      return { state: "error", error: `backend_http_${res.status}`, detail: res.text };
    }
    let messages = extractLinearMessages(res.json);
    const limit = Math.max(1, Math.min(200, Number(opts.limit) || 50));
    if (messages.length > limit) {
      messages = messages.slice(messages.length - limit);
    }
    return {
      state: "ok",
      chat_id: chatId,
      title: res.json.title || "",
      gizmo_id: res.json.gizmo_id || null,
      messages,
    };
  }

  async function renameChat(opts = {}) {
    const { chat_id, title } = opts;
    if (!chat_id || !title) {
      return { state: "error", error: "invalid_arguments", detail: "chat_id and title are required" };
    }
    const res = await apiFetch(`/conversation/${encodeURIComponent(chat_id)}`, {
      method: "PATCH",
      body: JSON.stringify({ title }),
    });
    if (!res.ok) {
      return { state: "error", error: `backend_http_${res.status}`, detail: res.text };
    }
    return { state: "ok", chat_id, title };
  }

  async function listProjects(opts = {}) {
    const limit = Math.max(1, Math.min(100, Number(opts.limit) || DEFAULT_LIMIT));
    const res = await apiFetch(`/gizmos/snorlax/sidebar?conversations_per_gizmo=0`);
    if (!res.ok || !res.json) {
      return { state: "error", error: `backend_http_${res.status}`, detail: res.text };
    }
    const rawItems = Array.isArray(res.json.items) ? res.json.items : [];
    const projects = rawItems.slice(0, limit).map((item) => {
      const g = (item.gizmo && item.gizmo.gizmo) || item.gizmo || item;
      return {
        id: g.id || null,
        name: (g.display && g.display.name) || g.name || "",
        updated_at: g.updated_at || null,
      };
    });
    return { state: "ok", projects, next_cursor: res.json.cursor || null };
  }

  async function readProject(opts = {}) {
    const { project_id } = opts;
    if (!project_id) {
      return { state: "error", error: "project_id_required", detail: "project_id is required" };
    }
    const cursor = opts.cursor ? `&cursor=${encodeURIComponent(opts.cursor)}` : "";
    const res = await apiFetch(
      `/gizmos/${encodeURIComponent(project_id)}/conversations?limit=${Math.max(1, Math.min(100, Number(opts.limit) || DEFAULT_LIMIT))}${cursor}`
    );
    if (!res.ok || !res.json) {
      return { state: "error", error: `backend_http_${res.status}`, detail: res.text };
    }
    const items = Array.isArray(res.json.items) ? res.json.items : [];
    return {
      state: "ok",
      project_id,
      chats: items.map((c) => ({
        id: c.id,
        title: c.title || "",
        update_time: c.update_time || null,
      })),
      next_cursor: res.json.cursor || null,
    };
  }

  async function createProject(opts = {}) {
    const name = typeof opts.name === "string" ? opts.name.trim() : "";
    if (!name) {
      return { state: "error", error: "project_name_required", detail: "non-empty name required" };
    }
    const res = await apiFetch("/gizmos/snorlax/upsert", {
      method: "POST",
      body: JSON.stringify({
        display: { name },
        instructions: typeof opts.instructions === "string" ? opts.instructions : "",
      }),
    });
    if (!res.ok || !res.json) {
      return { state: "error", error: `backend_http_${res.status}`, detail: res.text };
    }
    const g = res.json.gizmo || res.json;
    return {
      state: "ok",
      project_id: g.id || null,
      name: (g.display && g.display.name) || name,
    };
  }

  async function observeChat(opts = {}) {
    const chatId = opts.chat_id || currentChatIdFromUrl();
    const generating = Boolean(findStopButton());
    let latestAssistant = "";
    let latestMessageId = null;
    if (chatId) {
      const rd = await readChat({ chat_id: chatId, limit: 10 });
      if (rd.state === "ok" && Array.isArray(rd.messages)) {
        const lastA = [...rd.messages].reverse().find((m) => m.role === "assistant");
        if (lastA) {
          latestAssistant = lastA.text;
          latestMessageId = lastA.id;
        }
      }
    } else {
      const nodes = document.querySelectorAll('[data-message-author-role="assistant"]');
      if (nodes.length > 0) {
        latestAssistant = truncate((nodes[nodes.length - 1].innerText || "").trim());
      }
    }
    return {
      state: generating ? "in_progress" : "settled",
      chat_id: chatId,
      generating,
      message_id: latestMessageId,
      assistant_text: latestAssistant,
    };
  }

  async function stopGeneration() {
    const btn = findStopButton();
    if (!btn) {
      return { state: "idle", stopped: false, chat_id: currentChatIdFromUrl() };
    }
    btn.click();
    const deadline = Date.now() + 5000;
    while (Date.now() < deadline && findStopButton()) {
      await wait(120);
    }
    return {
      state: findStopButton() ? "in_progress" : "stopped",
      stopped: !findStopButton(),
      chat_id: currentChatIdFromUrl(),
    };
  }

  async function sendMessage(opts = {}) {
    if (isLoginRequired()) {
      return { state: "error", error: "login_required", detail: "ChatGPT account login required" };
    }
    const prompt = typeof opts.prompt === "string" ? opts.prompt : "";
    if (!prompt.trim()) {
      return { state: "error", error: "prompt_required", detail: "non-empty prompt required" };
    }
    if (findStopButton()) {
      return { state: "error", error: "generation_already_active", detail: "stop or wait for active response" };
    }

    let wirePrompt = prompt;
    if (Array.isArray(opts.local_function_signatures) && opts.local_function_signatures.length > 0) {
      const sigBlock = JSON.stringify(opts.local_function_signatures);
      wirePrompt =
        `[LOCAL_FUNCTION_SIGNATURES]\n${sigBlock}\n` +
        `If you need a local tool, emit one fenced block:\n` +
        `\`\`\`codex_tool_call\n{"call_id":"call_1","name":"<tool>","arguments":{}}\n\`\`\`\n` +
        `Otherwise answer directly.\n\n${prompt}`;
    }

    const composer = findComposer();
    if (!composer) {
      return { state: "error", error: "composer_unavailable", detail: "ChatGPT composer input not ready" };
    }

    const prevAssistantNodes = document.querySelectorAll('[data-message-author-role="assistant"]').length;
    setComposerText(composer, wirePrompt);
    await wait(80);
    const sendBtn = findSendButton();
    if (!sendBtn || sendBtn.disabled) {
      return { state: "error", error: "send_button_unavailable", detail: "send button disabled or missing" };
    }
    sendBtn.click();

    const waitMs = Math.max(0, Math.min(120000, Number(opts.wait_ms ?? 30000)));
    const deadline = Date.now() + waitMs;
    let sawGenerating = false;
    while (Date.now() < deadline) {
      const stopVisible = Boolean(findStopButton());
      if (stopVisible) sawGenerating = true;
      const curNodes = document.querySelectorAll('[data-message-author-role="assistant"]').length;
      if ((sawGenerating && !stopVisible) || (!stopVisible && curNodes > prevAssistantNodes)) {
        break;
      }
      await wait(180);
    }

    const obs = await observeChat({ chat_id: opts.chat_id || currentChatIdFromUrl() });
    const parsed = parseToolCallsFromText(obs.assistant_text || "");
    return {
      state: obs.state,
      chat_id: obs.chat_id,
      message_id: obs.message_id,
      assistant_text: parsed.text,
      tool_calls: parsed.calls,
      generating: obs.generating,
    };
  }

  globalThis.__termuxChatGptDriver = {
    version: 1,
    status,
    listChats,
    readChat,
    renameChat,
    listProjects,
    readProject,
    createProject,
    sendMessage,
    observeChat,
    stopGeneration,
    parseToolCallsFromText,
  };
  return { installed: true, version: 1 };
})();
