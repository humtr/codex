// Clean-room greenfield page driver for the official Linux ChatGPT Desktop app.
//
// Injected into the ChatGPT renderer page via CDP Runtime.evaluate.
// Zero app file modification; zero legacy app_bridge.js / DDPE / MCP coupling;
// zero hardcoded chunk hashes or minified symbol names; zero DOM clicking or
// computer-use automation.
//
// Architecture:
// - Authenticated REST reads/mutations:
//     * On `app://-` (Electron ChatGPT Desktop): native `connect-app-host`
//       MessageChannel (`services.httpFetch` with `X-OpenAI-Attach-Auth: 1`).
//     * On `https://` (web / unit tests): same-origin `fetch("/backend-api/...")`.
// - Programmatic turn execution / streaming / cancellation:
//     * Dynamically discovers the app's own internal CompletionClient
//       (`startCompletionStream`, `cancelStream`, `stopCompletion`) via the live
//       React Fiber `scope` and standard Rspack `__webpack_require__` export —
//       operating directly in-process independent of onboarding screens, modals,
//       tutorials, or active UI view.
// Never reads or returns cookies/auth tokens; never synthesizes private reasoning.
(() => {
  if (globalThis.__termuxChatGptDriver) {
    return { installed: true };
  }

  const DEFAULT_LIMIT = 20;
  const MAX_TEXT_CHARS = 65536;
  const MAX_RESPONSE_BYTES = 8 * 1024 * 1024;

  function truncate(str, limit = MAX_TEXT_CHARS) {
    if (typeof str !== "string") return "";
    return str.length > limit ? str.slice(0, limit) : str;
  }

  function newUuid() {
    if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
      return crypto.randomUUID();
    }
    return "xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx".replace(/[xy]/g, (c) => {
      const r = (Math.random() * 16) | 0;
      return (c === "x" ? r : (r & 0x3) | 0x8).toString(16);
    });
  }

  function decodeUtf8Chunks(chunks) {
    let total = 0;
    for (const c of chunks) total += c.byteLength;
    const merged = new Uint8Array(total);
    let offset = 0;
    for (const c of chunks) {
      merged.set(c, offset);
      offset += c.byteLength;
    }
    return new TextDecoder("utf-8", { fatal: false }).decode(merged);
  }

  function toUint8Array(val) {
    if (Array.isArray(val) && val[0] === "bytes") {
      return toUint8Array(val[1]);
    }
    if (val instanceof Uint8Array) return val;
    if (ArrayBuffer.isView(val)) {
      return new Uint8Array(val.buffer, val.byteOffset, val.byteLength);
    }
    if (val instanceof ArrayBuffer) return new Uint8Array(val);
    if (typeof val === "string") return new TextEncoder().encode(val);
    return null;
  }

  // Minimal Cap'n Web client for Electron's `connect-app-host` MessagePort
  // `services.httpFetch` service.
  class AppHostHttpClient {
    constructor() {
      this.port = null;
      this.nextImportId = 1;
      this.nextExportId = 1;
      this.pendingPulls = new Map();
      this.pipes = new Map();
      this.closed = false;
      this.everReceived = false;
    }

    ensureConnected() {
      if (this.port && !this.closed) return;
      if (typeof MessageChannel !== "function" || typeof window.postMessage !== "function") {
        throw new Error("connect-app-host MessageChannel unavailable");
      }
      const { port1, port2 } = new MessageChannel();
      this.port = port1;
      this.closed = false;
      this.everReceived = false;
      this.nextImportId = 1;
      this.nextExportId = 1;
      this.pendingPulls.clear();
      this.pipes.clear();

      port1.onmessage = (ev) => this._onMessage(ev.data);
      port1.onmessageerror = () => this._failAll("app_host_port_message_error");
      port1.start();

      const targetOrigin =
        window.location && window.location.origin && window.location.origin !== "null"
          ? window.location.origin
          : "*";
      window.postMessage({ type: "connect-app-host", port: port2 }, targetOrigin, [port2]);
    }

    _failAll(reason) {
      this.closed = true;
      try {
        if (this.port) this.port.close();
      } catch (_) {}
      this.port = null;
      for (const [, p] of this.pendingPulls) p.reject(new Error(reason));
      this.pendingPulls.clear();
      for (const [, pipe] of this.pipes) pipe.reject(new Error(reason));
      this.pipes.clear();
    }

    _allocPipe() {
      const id = this.nextExportId++;
      let resolve;
      let reject;
      const done = new Promise((res, rej) => {
        resolve = res;
        reject = rej;
      });
      this.pipes.set(id, {
        chunks: [],
        bytes: 0,
        settled: false,
        done,
        resolve: (v) => {
          const p = this.pipes.get(id);
          if (p && !p.settled) {
            p.settled = true;
            resolve(v);
          }
        },
        reject: (e) => {
          const p = this.pipes.get(id);
          if (p && !p.settled) {
            p.settled = true;
            reject(e);
          }
        },
      });
      return id;
    }

    _onMessage(msg) {
      if (msg == null) {
        this._failAll("app_host_port_closed");
        return;
      }
      this.everReceived = true;
      if (!Array.isArray(msg) || msg.length === 0) return;
      const tag = msg[0];
      if (tag === "pipe") {
        this._allocPipe();
        return;
      }
      if (tag === "stream" || tag === "push") {
        const expId = this.nextExportId++;
        const expr = msg[1];
        if (
          Array.isArray(expr) &&
          expr[0] === "pipeline" &&
          typeof expr[1] === "number" &&
          Array.isArray(expr[2])
        ) {
          const pipe = this.pipes.get(expr[1]);
          const method = expr[2][0];
          const args = Array.isArray(expr[3]) ? expr[3] : [];
          if (pipe) {
            if (method === "write") {
              const bytes = toUint8Array(args[0]);
              if (bytes && pipe.bytes + bytes.byteLength <= MAX_RESPONSE_BYTES) {
                pipe.chunks.push(bytes);
                pipe.bytes += bytes.byteLength;
              }
            } else if (method === "close" || method === "abort") {
              pipe.resolve(decodeUtf8Chunks(pipe.chunks));
              pipe.chunks = [];
            }
          }
        }
        if (tag === "stream" && this.port) {
          this.port.postMessage(["resolve", expId, undefined]);
        }
        return;
      }
      if (tag === "pull") {
        const expId = msg[1];
        if (typeof expId === "number" && this.port) {
          this.port.postMessage(["resolve", expId, undefined]);
        }
        return;
      }
      if (tag === "resolve" || tag === "reject") {
        const impId = msg[1];
        const pending = this.pendingPulls.get(impId);
        if (!pending) return;
        this.pendingPulls.delete(impId);
        if (tag === "resolve") {
          pending.resolve(msg[2]);
        } else {
          const errVal = msg[2];
          const errMsg =
            Array.isArray(errVal) && errVal[0] === "error"
              ? String(errVal[2] || errVal[1] || "rpc_rejected")
              : String(errVal || "rpc_rejected");
          pending.reject(new Error(errMsg));
        }
        return;
      }
      if (tag === "abort") {
        this._failAll("app_host_rpc_aborted");
      }
    }

    async fetch(path, options = {}, timeoutMs = 15000) {
      this.ensureConnected();
      const reqId = newUuid();
      const method = (options.method || "GET").toUpperCase();
      const headers = {
        "X-OpenAI-Attach-Auth": "1",
        Accept: "application/json",
        ...(options.body != null ? { "Content-Type": "application/json" } : {}),
        ...(options.headers || {}),
      };
      const init = {
        url: path,
        method,
        headers,
        returnErrorResponse: true,
      };
      if (options.body != null) {
        init.body = typeof options.body === "string" ? options.body : JSON.stringify(options.body);
      }

      const impId = this.nextImportId++;
      let timer = null;
      const rpcPromise = new Promise((resolve, reject) => {
        this.pendingPulls.set(impId, { resolve, reject });
        timer = setTimeout(() => {
          this.pendingPulls.delete(impId);
          if (!this.everReceived) {
            this._failAll("app_host_fetch_timeout");
          }
          reject(new Error("app_host_fetch_timeout"));
        }, timeoutMs);
      });

      this.port.postMessage([
        "push",
        ["pipeline", 0, ["services", "httpFetch", "fetch"], [reqId, init]],
      ]);
      this.port.postMessage(["pull", impId]);

      try {
        const resVal = await rpcPromise;
        if (this.port) {
          this.port.postMessage(["release", impId, 1]);
        }
        if (resVal && typeof resVal === "object" && "response" in resVal) {
          const respWire = resVal.response;
          // Standard Cap'n Web Response encoding: ["response", bodyExpr, { status?, statusText?, headers? }]
          // Cap'n Web omits `status` when HTTP status is 200 (default).
          if (Array.isArray(respWire) && respWire[0] === "response") {
            const bodyExpr = respWire[1];
            const meta = respWire[2] || {};
            const status = meta.status == null ? 200 : Number(meta.status);
            let text = "";
            if (Array.isArray(bodyExpr) && bodyExpr[0] === "readable") {
              const pipeId = bodyExpr[1];
              const pipe = this.pipes.get(pipeId);
              if (pipe) {
                let bodyTimer = null;
                try {
                  text = await Promise.race([
                    pipe.done,
                    new Promise((_, rej) => {
                      bodyTimer = setTimeout(
                        () => rej(new Error("app_host_body_timeout")),
                        timeoutMs
                      );
                    }),
                  ]);
                } finally {
                  if (bodyTimer) clearTimeout(bodyTimer);
                  this.pipes.delete(pipeId);
                }
              }
            } else if (typeof bodyExpr === "string") {
              text = bodyExpr;
            }
            let json = null;
            try {
              json = text ? JSON.parse(text) : null;
            } catch (_) {
              json = null;
            }
            return {
              ok: status >= 200 && status < 300,
              status,
              json,
              text: truncate(text, 4096),
            };
          }
          if (respWire && typeof respWire.text === "function") {
            const text = await respWire.text();
            let json = null;
            try {
              json = text ? JSON.parse(text) : null;
            } catch (_) {
              json = null;
            }
            const status = Number(respWire.status || 0);
            return { ok: status >= 200 && status < 300, status, json, text: truncate(text, 4096) };
          }
        }
        if (resVal && typeof resVal === "object" && resVal.responseType === "error") {
          const status = Number(resVal.responseStatus ?? resVal.status ?? 0);
          return {
            ok: false,
            status,
            json: null,
            text: truncate(String(resVal.error || resVal.errorCode || "fetch_error"), 1024),
          };
        }
        return {
          ok: false,
          status: 0,
          json: null,
          text: truncate(`unexpected_app_host_response: ${JSON.stringify(resVal)}`, 512),
        };
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
  }

  const appHostClient = new AppHostHttpClient();
  let cachedCompletionClient = null;
  let cachedAccountId = null;
  const activeTurns = new Map(); // laneId -> turn state (supports multi-lane parallel execution)
  const cancelledLanes = new Map();
  let lastTurn = null;
  const MAX_TRACKED_TURNS = 32;
  const MAX_CONCURRENT_GENERATING = 8;

  function generatingTurns() {
    const out = [];
    for (const t of activeTurns.values()) {
      if (t && t.generating) out.push(t);
    }
    return out;
  }

  function pruneSettledTurns() {
    if (activeTurns.size <= MAX_TRACKED_TURNS) return;
    for (const [key, turn] of activeTurns.entries()) {
      if (activeTurns.size <= MAX_TRACKED_TURNS) break;
      if (!turn.generating && turn !== lastTurn) {
        activeTurns.delete(key);
      }
    }
  }

  function findTrackedTurn({ laneId, chatId, preferGenerating = true } = {}) {
    if (laneId && activeTurns.has(laneId)) {
      return activeTurns.get(laneId);
    }
    if (chatId) {
      let latestMatch = null;
      for (const t of activeTurns.values()) {
        if (t.chatId === chatId) {
          if (preferGenerating && t.generating) return t;
          latestMatch = t;
        }
      }
      return latestMatch;
    }
    if (preferGenerating) {
      const active = generatingTurns();
      if (active.length > 0) return active[active.length - 1];
    }
    return lastTurn;
  }

  function isElectronAppOrigin() {
    return Boolean(location && location.protocol === "app:");
  }

  async function apiFetch(path, options = {}, timeoutMs = 15000) {
    if (isElectronAppOrigin()) {
      return appHostClient.fetch(path, options, timeoutMs);
    }

    const apiBase =
      location && location.protocol === "https:"
        ? "/backend-api"
        : "https://chatgpt.com/backend-api";
    const controller = typeof AbortController === "function" ? new AbortController() : null;
    const timer = controller ? setTimeout(() => controller.abort(), timeoutMs) : null;
    try {
      const resp = await fetch(`${apiBase}${path}`, {
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

  // Dynamically discover the app's React Fiber state scope (zero minified symbol names).
  function findAppScope() {
    if (!document) return null;
    const roots = [
      document.getElementById("root"),
      document.body,
      document.querySelector("button"),
      document.querySelector("div"),
    ].filter(Boolean);

    const visited = new Set();
    const queue = [];
    for (const domNode of roots) {
      const fk = Object.keys(domNode).find(
        (k) => k.startsWith("__reactFiber") || k.startsWith("__reactContainer")
      );
      if (fk && domNode[fk]) queue.push(domNode[fk]);
    }

    let steps = 0;
    while (queue.length > 0 && steps < 1200) {
      steps++;
      const fiber = queue.shift();
      if (!fiber || visited.has(fiber)) continue;
      visited.add(fiber);

      for (let hook = fiber.memoizedState, j = 0; hook && j < 100; hook = hook.next, j++) {
        const cand = hook.memoizedState?.current ?? hook.memoizedState;
        if (
          cand &&
          typeof cand.get === "function" &&
          typeof cand.set === "function" &&
          cand.node &&
          cand.chain
        ) {
          return cand;
        }
      }
      const mp = fiber.memoizedProps?.value;
      if (mp && typeof mp.get === "function" && typeof mp.set === "function" && mp.node) {
        return mp;
      }
      if (fiber.child) queue.push(fiber.child);
      if (fiber.sibling) queue.push(fiber.sibling);
      if (fiber.return) queue.push(fiber.return);
    }
    return null;
  }

  // Dynamically obtain Rspack's standard `__webpack_require__` export from the
  // page's own module entry `<script type="module" src="...">` (zero hardcoded hashes).
  async function findWebpackRequire() {
    if (typeof globalThis.__termuxWebpackRequire === "function") {
      return globalThis.__termuxWebpackRequire;
    }
    if (!document) return null;
    const scripts = [...document.querySelectorAll('script[type="module"][src]')];
    for (const s of scripts) {
      try {
        const mod = await import(s.src);
        if (mod && typeof mod.__webpack_require__ === "function") {
          globalThis.__termuxWebpackRequire = mod.__webpack_require__;
          return mod.__webpack_require__;
        }
      } catch (_) {}
    }
    return null;
  }

  function isCompletionClient(obj) {
    return Boolean(
      obj &&
        typeof obj.startCompletionStream === "function" &&
        typeof obj.stopCompletion === "function" &&
        typeof obj.cancelStream === "function"
    );
  }

  // Dynamically discover the app's own CompletionClient by capability signature.
  // Works independently of onboarding screens, modals, or DOM layout.
  // When `waitMs > 0` (used by `sendMessage` on lazy cold boot), polls until the
  // Electron renderer finishes mounting React/Rspack.
  async function ensureCompletionClient(waitMs = 0) {
    if (isCompletionClient(globalThis.__termuxMockCompletionClient)) {
      return globalThis.__termuxMockCompletionClient;
    }
    if (isCompletionClient(cachedCompletionClient)) {
      return cachedCompletionClient;
    }

    const tryOnce = async () => {
      const scope = findAppScope();
      if (!scope) return null;
      const wpReq = await findWebpackRequire();
      if (!wpReq || !wpReq.m) return null;

      const searchLoadedModules = () => {
        for (const [modId, factory] of Object.entries(wpReq.m)) {
          if (typeof factory !== "function") continue;
          let src = "";
          try {
            src = Function.prototype.toString.call(factory);
          } catch (_) {
            continue;
          }
          if (
            !src.includes("startCompletionStream") ||
            !src.includes("stopCompletion") ||
            !src.includes("cancelStream")
          ) {
            continue;
          }
          let mod;
          try {
            mod = wpReq(modId);
          } catch (_) {
            continue;
          }
          if (!mod || typeof mod !== "object") continue;
          for (const val of Object.values(mod)) {
            if (isCompletionClient(val)) {
              cachedCompletionClient = val;
              return val;
            }
            try {
              const inst = scope.get(val);
              if (isCompletionClient(inst)) {
                cachedCompletionClient = inst;
                return inst;
              }
            } catch (_) {}
          }
        }
        return null;
      };

      let found = searchLoadedModules();
      if (found) return found;

      if (typeof wpReq.u === "function" && typeof wpReq.e === "function") {
        try {
          const entrySrc = Function.prototype.toString.call(wpReq.u);
          const chunkMatches = [...entrySrc.matchAll(/(\d+):"([0-9a-f]{8,})"/g)];
          const loadedUrls = new Set(
            typeof performance !== "undefined" && performance.getEntriesByType
              ? performance.getEntriesByType("resource").map((r) => r.name)
              : []
          );
          const orderedIds = chunkMatches
            .map((m) => Number(m[1]))
            .sort((a, b) => {
              const au = [...loadedUrls].some((u) => u.includes(`/${a}.`));
              const bu = [...loadedUrls].some((u) => u.includes(`/${b}.`));
              return Number(bu) - Number(au);
            });
          for (const cid of orderedIds.slice(0, 20)) {
            if (![...loadedUrls].some((u) => u.includes(`/${cid}.`))) break;
            try {
              await wpReq.e(cid);
            } catch (_) {}
            found = searchLoadedModules();
            if (found) return found;
          }
        } catch (_) {}
      }
      return null;
    };

    const first = await tryOnce();
    if (first || waitMs <= 0) return first;
    const deadline = Date.now() + waitMs;
    while (Date.now() < deadline) {
      await new Promise((r) => setTimeout(r, 400));
      const hit = await tryOnce();
      if (hit) return hit;
    }
    return null;
  }

  function normalizeThinkingEffort(modelSlug, rawEffort) {
    if (!rawEffort || typeof rawEffort !== "string") return undefined;
    const slug = String(modelSlug || "").toLowerCase();
    if (slug.includes("instant")) return undefined;
    const key = rawEffort.trim().toLowerCase();
    const map = {
      none: "zero",
      zero: "zero",
      minimal: "min",
      min: "min",
      low: "min",
      medium: "standard",
      standard: "standard",
      high: "extended",
      extended: "extended",
      xhigh: "xhigh",
      max: "max",
      ultra: "ultra",
    };
    return map[key];
  }

  function currentChatIdFromUrl() {
    const combined = `${location?.pathname || ""} ${location?.hash || ""}`;
    const m = combined.match(/\/c\/([0-9a-fA-F-]{8,})/);
    return m ? m[1] : null;
  }

  function isOnboardingVisible() {
    return Boolean(document && document.querySelector('[data-testid="onboarding-progress-bar"]'));
  }

  function isLoginRequired() {
    const url = location?.href || "";
    if (/auth\.openai\.com|\/auth\/login/.test(url)) return true;
    return false;
  }

  function isVisibleAssistantMessage(msg) {
    const meta = msg.metadata;
    return msg.author?.role === "assistant" &&
      (!msg.recipient || msg.recipient === "all") &&
      !meta?.is_visually_hidden_from_conversation &&
      !meta?.is_visually_hidden_reasoning_group &&
      meta?.summary_type !== "raw_cot";
  }

  function canExposeReasoning(msg) {
    const role = msg && msg.author && msg.author.role;
    return Boolean(
      msg &&
      (role === "assistant" || role === "tool") &&
      (!msg.recipient || msg.recipient === "all")
    );
  }

  function formatThoughtEntry(thought) {
    if (!thought || typeof thought !== "object") return "";
    const heading = typeof thought.summary === "string" ? thought.summary.trim() : "";
    let body = "";
    if (typeof thought.content === "string" && thought.content.trim()) {
      body = thought.content.trim();
    } else if (Array.isArray(thought.chunks)) {
      body = thought.chunks
        .map((c) => (typeof c === "string" ? c : typeof c?.text === "string" ? c.text : ""))
        .join("")
        .trim();
    }
    if (heading && body) {
      if (body === heading) return heading;
      if (body.startsWith(heading)) {
        const rest = body.slice(heading.length).trim();
        return rest ? `**${heading}**\n${rest}` : heading;
      }
      return `**${heading}**\n${body}`;
    }
    return body || heading;
  }

  function extractVisibleReasoningTitles(msg) {
    const meta = msg && msg.metadata;
    if (!meta || typeof meta !== "object") return [];
    const list = Array.isArray(meta.rich_reasoning_titles)
      ? meta.rich_reasoning_titles
      : Array.isArray(meta.reasoning_titles)
        ? meta.reasoning_titles
        : null;
    const out = [];
    if (list) {
      for (const entry of list) {
        if (typeof entry === "string" && entry.trim()) {
          out.push(truncate(entry.trim()));
        }
      }
    }
    if (out.length === 0) {
      const single =
        (typeof meta.reasoning_title === "string" && meta.reasoning_title.trim()) ||
        (typeof meta.work_activity?.reasoning_title === "string" && meta.work_activity.reasoning_title.trim()) ||
        null;
      if (single) out.push(truncate(single));
    }
    return out;
  }

  function extractRawPartsText(msg) {
    const parts = Array.isArray(msg?.content?.parts) ? msg.content.parts : null;
    if (!parts) return null;
    const strs = parts.filter((p) => typeof p === "string");
    return strs.length > 0 ? strs.join("\n") : null;
  }

  function extractTextFromMessage(msg) {
    if (!msg || typeof msg !== "object") return null;
    const role = msg.author && msg.author.role ? msg.author.role : "unknown";
    const contentType = msg.content && msg.content.content_type ? msg.content.content_type : "";
    if (contentType === "thoughts" || contentType === "reasoning_recap") return null;
    if (role === "assistant" && (msg.channel === "analysis" ||
      !isVisibleAssistantMessage(msg))) return null;
    const parts = Array.isArray(msg.content && msg.content.parts) ? msg.content.parts : [];
    const text = parts
      .filter((p) => typeof p === "string")
      .join("\n")
      .trim();
    if (!text) return null;
    return {
      id: msg.id || null,
      role,
      text: truncate(text),
      create_time: msg.create_time || null,
      model: (msg.metadata && msg.metadata.model_slug) || null,
    };
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
        const extracted = extractTextFromMessage(node.message);
        if (extracted && (extracted.role === "user" || extracted.role === "assistant")) {
          chain.push({ ...extracted, id: extracted.id || nodeId });
        }
      }
      nodeId = node.parent || null;
    }
    chain.reverse();
    return chain;
  }

  // Avoid ChatGPT backend name collisions with built-in container/runtime `exec`
  // and disambiguate same-named tools across namespaces without dropping any tool.
  function allocateWireLocalFunctionName(rawName, namespace, usedWireNames, reservedRawNames) {
    const sanitize = (s) => s.replace(/[^A-Za-z0-9_-]/g, "_").slice(0, 64);
    let base = rawName;
    if (rawName === "exec" && !namespace) {
      base = reservedRawNames.has("codex_exec") ? "codex_local_exec" : "codex_exec";
    } else if (usedWireNames.has(rawName) && namespace) {
      base = sanitize(`${namespace}_${rawName}`);
    }
    let candidate = sanitize(base);
    let suffix = 2;
    while (!candidate || usedWireNames.has(candidate)) {
      const tag = `_${suffix++}`;
      candidate = `${sanitize(base).slice(0, Math.max(1, 64 - tag.length))}${tag}`;
    }
    usedWireNames.add(candidate);
    return candidate;
  }

  function resolveJsonSchemaType(pdef) {
    if (!pdef || typeof pdef !== "object") return "string";
    if (typeof pdef.type === "string") return pdef.type;
    if (Array.isArray(pdef.type)) {
      const nonNull = pdef.type.find((t) => typeof t === "string" && t !== "null");
      if (nonNull) return nonNull;
    }
    if (Array.isArray(pdef.anyOf)) {
      for (const variant of pdef.anyOf) {
        const vt = resolveJsonSchemaType(variant);
        if (vt && vt !== "null") return vt;
      }
    }
    if (pdef.properties && typeof pdef.properties === "object") return "object";
    if (pdef.items && typeof pdef.items === "object") return "array";
    return "string";
  }

  function normalizeParamSchema(pdef, fallbackDesc, depth = 0) {
    const rawType = resolveJsonSchemaType(pdef);
    const desc = truncate(
      pdef && typeof pdef.description === "string" && pdef.description.trim()
        ? pdef.description.trim()
        : fallbackDesc,
      160
    );
    if (rawType === "integer" || rawType === "number" || rawType === "boolean" || rawType === "string") {
      const out = { type: rawType, description: desc };
      if (pdef && Array.isArray(pdef.enum) && pdef.enum.length > 0 && pdef.enum.length <= 32) {
        out.enum = pdef.enum;
      }
      return out;
    }
    if (rawType === "array" && depth < 3) {
      const itemSchema =
        pdef && pdef.items && typeof pdef.items === "object"
          ? normalizeParamSchema(pdef.items, "item", depth + 1)
          : { type: "string", description: "item" };
      return { type: "array", description: desc, items: itemSchema };
    }
    if (rawType === "object" && depth < 3) {
      const out = { type: "object", description: desc };
      if (pdef && pdef.properties && typeof pdef.properties === "object") {
        const nested = {};
        for (const [k, v] of Object.entries(pdef.properties)) {
          nested[k] = normalizeParamSchema(v, k, depth + 1);
        }
        out.properties = nested;
        if (Array.isArray(pdef.required)) {
          out.required = pdef.required.filter((r) => typeof r === "string");
        }
      }
      return out;
    }
    return { type: "string", description: desc };
  }

  function buildNativeLocalFunctionSignatures(sigs) {
    if (!Array.isArray(sigs) || sigs.length === 0) {
      return {
        names: [],
        signatures: [],
        wireToOriginal: new Map(),
        injectedReportWireNames: new Set(),
      };
    }
    const names = [];
    const signatures = [];
    const wireToOriginal = new Map();
    const injectedReportWireNames = new Set();
    const seenKeys = new Set();
    const usedWireNames = new Set();
    const reservedRawNames = new Set();
    for (const s of sigs) {
      if (s && typeof s.name === "string" && s.name.trim()) {
        reservedRawNames.add(s.name.trim());
      }
    }
    for (const s of sigs) {
      if (!s || typeof s.name !== "string") continue;
      const rawName = s.name.trim();
      const ns =
        typeof s.namespace === "string" && s.namespace.trim() && s.namespace.trim() !== "functions"
          ? s.namespace.trim()
          : null;
      const dedupKey = `${ns || ""}::${rawName}`;
      if (!rawName || !/^[A-Za-z0-9_-]{1,64}$/.test(rawName) || seenKeys.has(dedupKey)) continue;
      seenKeys.add(dedupKey);
      const wireName = allocateWireLocalFunctionName(rawName, ns, usedWireNames, reservedRawNames);
      wireToOriginal.set(wireName, { name: rawName, namespace: ns });
      names.push(wireName);
      const params = [];
      let hasReportParam = false;
      if (s.type === "custom") {
        params.push({
          name: "input",
          required: true,
          type: { type: "string", description: "Raw tool input string" },
        });
      } else {
        if (s.parameters && typeof s.parameters === "object" && s.parameters.properties) {
          const reqSet = new Set(Array.isArray(s.parameters.required) ? s.parameters.required : []);
          for (const [pname, pdef] of Object.entries(s.parameters.properties)) {
            if (pname === "report") hasReportParam = true;
            params.push({
              name: pname,
              required: reqSet.has(pname),
              type: normalizeParamSchema(pdef, pname, 0),
            });
          }
        }
        if (!hasReportParam) {
          injectedReportWireNames.add(wireName);
          params.push({
            name: "report",
            required: false,
            type: {
              type: "string",
              description:
                "Optional concise public progress update in the user's language before running this tool (never private reasoning).",
            },
          });
        }
      }
      signatures.push({
        name: wireName,
        description: truncate(
          typeof s.description === "string" && s.description.trim()
            ? s.description.trim()
            : `Invoke local Codex tool ${ns ? `${ns}.${rawName}` : rawName}`,
          2400
        ),
        type: "kwargs",
        params,
      });
    }
    return { names, signatures, wireToOriginal, injectedReportWireNames };
  }

  function extractNativeToolCallFromMessage(
    msg,
    declaredNames,
    wireToOriginal,
    injectedReportWireNames
  ) {
    if (!msg || typeof msg !== "object") return null;
    if (msg.author?.role !== "assistant") return null;
    const recipient = typeof msg.recipient === "string" ? msg.recipient.trim() : "";
    if (!recipient || recipient === "all") return null;
    const stripped = recipient.includes(".") ? recipient.split(".").pop() : recipient;
    if (declaredNames.size === 0 || (!declaredNames.has(recipient) && !declaredNames.has(stripped))) {
      return null;
    }
    const content = msg.content;
    if (!content || typeof content !== "object") return null;
    let rawArgs = null;
    if (typeof content.text === "string") {
      rawArgs = content.text.trim();
    } else if (Array.isArray(content.parts)) {
      rawArgs = content.parts
        .filter((p) => typeof p === "string")
        .join("\n")
        .trim();
    }
    if (rawArgs === null) return null;
    if (!rawArgs) {
      // Support legitimate zero-argument native tool calls once finalized (or with empty object content).
      if (msg.status === "finished_successfully" || msg.end_turn === true) {
        rawArgs = "{}";
      } else {
        return null;
      }
    }
    const matchedWire = declaredNames.has(recipient) ? recipient : stripped;
    const mapped = wireToOriginal && wireToOriginal.get(matchedWire);
    const resolvedName = mapped ? mapped.name : matchedWire;
    const resolvedNs = mapped ? mapped.namespace : null;
    let report = null;
    if (injectedReportWireNames && injectedReportWireNames.has(matchedWire)) {
      try {
        const parsed = JSON.parse(rawArgs);
        if (parsed && typeof parsed === "object" && !Array.isArray(parsed) && "report" in parsed) {
          if (typeof parsed.report === "string" && parsed.report.trim()) {
            report = parsed.report.trim();
          }
          delete parsed.report;
          rawArgs = JSON.stringify(parsed);
        }
      } catch (_) {}
    }
    return {
      id: msg.id ? `call_${String(msg.id).replace(/[^A-Za-z0-9_-]/g, "").slice(0, 24)}` : `call_${newUuid().slice(0, 8)}`,
      name: resolvedName,
      ...(resolvedNs ? { namespace: resolvedNs } : {}),
      arguments: rawArgs,
      native_recipient: recipient,
      ...(report ? { report } : {}),
    };
  }

  function parseToolCallsFromText(text) {
    const calls = [];
    const fenceRe = /```codex_tool_call\s*([\s\S]*?)```/g;
    let match;
    let stripped = text;
    while ((match = fenceRe.exec(text)) !== null) {
      try {
        const parsed = JSON.parse(match[1].trim());
        if (parsed && typeof parsed.name === "string" && parsed.name.length > 0) {
          const payload =
            typeof parsed.input === "string"
              ? parsed.input
              : typeof parsed.arguments === "string"
                ? parsed.arguments
                : JSON.stringify(parsed.arguments || {});
          const entry = {
            id:
              typeof parsed.call_id === "string" && parsed.call_id
                ? parsed.call_id
                : `call_${calls.length + 1}`,
            name: parsed.name,
            arguments: payload,
          };
          if (typeof parsed.namespace === "string" && parsed.namespace.trim()) {
            entry.namespace = parsed.namespace.trim();
          }
          calls.push(entry);
          stripped = stripped.replace(match[0], "").trim();
        }
      } catch (_) {}
    }
    return { calls, text: stripped };
  }

  async function status() {
    const loginReq = isLoginRequired();
    let authState = loginReq ? "login_required" : "loading";
    let account = null;
    let models = [];
    let backendError = null;

    if (!loginReq) {
      const me = await apiFetch("/me");
      if (me.ok && me.json) {
        authState = "authenticated";
        const orgs = me.json.orgs && Array.isArray(me.json.orgs.data) ? me.json.orgs.data : [];
        const personalOrg = orgs.find((o) => o.is_personal) || orgs[0] || null;
        cachedAccountId = personalOrg?.id || me.json.account_id || null;
        account = {
          id: me.json.id || null,
          account_id: cachedAccountId || null,
          name: me.json.name || null,
          plan: personalOrg?.title || null,
        };
      } else if (me.status === 401 || me.status === 403 || me.status === 432) {
        authState = "login_required";
      } else {
        backendError = `backend_http_${me.status}: ${truncate(me.text, 200)}`;
      }

      if (authState === "authenticated") {
        const mRes = await apiFetch("/models");
        if (mRes.ok && mRes.json && Array.isArray(mRes.json.models)) {
          models = mRes.json.models.map((m) => ({
            slug: m.slug,
            title: m.title || m.slug,
            description:
              typeof m.description === "string" && m.description.trim()
                ? m.description.trim()
                : null,
            max_tokens:
              typeof m.max_tokens === "number" && m.max_tokens > 0
                ? m.max_tokens
                : null,
            tags: Array.isArray(m.tags)
              ? m.tags.filter((t) => typeof t === "string")
              : [],
          }));
        }
      }
    }

    const completionClient = await ensureCompletionClient();
    const completionReady = Boolean(completionClient);
    const onboarding = isOnboardingVisible();
    const uiState =
      authState === "login_required"
        ? "login_required"
        : completionReady
          ? "ready"
          : onboarding
            ? "onboarding"
            : "loading";

    const liveTurns = generatingTurns();
    const primaryTurn = liveTurns.length > 0 ? liveTurns[liveTurns.length - 1] : lastTurn;

    return {
      state: "ok",
      transport: isElectronAppOrigin() ? "app_host_http_fetch" : "same_origin_fetch",
      page_url:
        location && location.origin && location.origin !== "null"
          ? `${location.origin}${location.pathname}`
          : String(location && location.href ? location.href : "").split("?", 1)[0],
      auth_state: authState,
      ui_state: uiState,
      completion_ready: completionReady,
      pinned_send: true,
      onboarding_visible: onboarding,
      account,
      active_chat_id: (primaryTurn && primaryTurn.chatId) || currentChatIdFromUrl(),
      generating: liveTurns.length > 0,
      active_turn_count: liveTurns.length,
      active_turns: liveTurns.map((t) => ({
        lane_id: t.laneId,
        chat_id: t.chatId,
        project_id: t.projectId,
        model: t.model,
        started_at: t.startedAt,
      })),
      models,
      ...(backendError ? { backend_error: backendError } : {}),
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
    const tracked = findTrackedTurn({ laneId: opts.lane_id, chatId: opts.chat_id });
    const chatId = opts.chat_id || (tracked && tracked.chatId) || currentChatIdFromUrl();
    if (!chatId) {
      return { state: "error", error: "chat_id_required", detail: "no chat_id provided or active" };
    }
    const res = await apiFetch(`/conversation/${encodeURIComponent(chatId)}`);
    if (!res.ok || !res.json) {
      return { state: "error", error: `backend_http_${res.status}`, detail: res.text };
    }
    let messages = extractLinearMessages(res.json);
    const userTurnCount = messages.filter((message) => message.role === "user").length;
    const limit = Math.max(1, Math.min(200, Number(opts.limit) || 50));
    if (messages.length > limit) {
      messages = messages.slice(messages.length - limit);
    }
    return {
      state: "ok",
      chat_id: chatId,
      title: res.json.title || "",
      gizmo_id: res.json.gizmo_id || null,
      current_node: res.json.current_node || null,
      user_turn_count: userTurnCount,
      ...(tracked?.chatId === chatId && tracked.error ? { local_turn_error: tracked.error.code } : {}),
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
    const cursor = opts.cursor ? `&cursor=${encodeURIComponent(opts.cursor)}` : "";
    const res = await apiFetch(`/gizmos/snorlax/sidebar?conversations_per_gizmo=0${cursor}`);
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
    const payload = {
      name,
      instructions: typeof opts.instructions === "string" ? opts.instructions : "",
    };
    const res = await apiFetch("/projects", {
      method: "POST",
      body: JSON.stringify(payload),
    });
    if (!res.ok || !res.json) {
      return { state: "error", error: `backend_http_${res.status}`, detail: res.text };
    }
    if (res.json.error) {
      return {
        state: "error",
        error: "project_create_failed",
        detail: truncate(String(res.json.error), 512),
      };
    }
    const g =
      (res.json.resource && res.json.resource.gizmo) ||
      res.json.gizmo ||
      res.json.resource ||
      res.json;
    return {
      state: "ok",
      project_id: g.id || null,
      name: (g.display && g.display.name) || g.name || name,
    };
  }

  async function observeChat(opts = {}) {
    const requestedLaneId =
      typeof opts.lane_id === "string" && opts.lane_id.trim() ? opts.lane_id.trim() : null;
    const requestedChatId =
      typeof opts.chat_id === "string" && opts.chat_id.trim() ? opts.chat_id.trim() : null;
    const turn = findTrackedTurn({
      laneId: requestedLaneId,
      chatId: requestedChatId,
      preferGenerating: true,
    });
    const matchedTurn =
      turn &&
      (!requestedLaneId || turn.laneId === requestedLaneId) &&
      (!requestedChatId || turn.chatId === requestedChatId)
        ? turn
        : null;

    if (matchedTurn && matchedTurn.generating) {
      return {
        state: "in_progress",
        lane_id: matchedTurn.laneId,
        chat_id: matchedTurn.chatId || requestedChatId,
        generating: true,
        message_id: matchedTurn.messageId,
        assistant_text: matchedTurn.assistantText,
      };
    }

    if (matchedTurn && !matchedTurn.generating && matchedTurn.error) {
      return {
        state: "error",
        lane_id: matchedTurn.laneId,
        error: matchedTurn.error.code,
        detail: matchedTurn.error.detail,
        chat_id: matchedTurn.chatId || requestedChatId,
        generating: false,
      };
    }

    const chatId = requestedChatId || (matchedTurn && matchedTurn.chatId) || currentChatIdFromUrl();
    if (!chatId) {
      return {
        state: "settled",
        ...(matchedTurn ? { lane_id: matchedTurn.laneId } : {}),
        chat_id: null,
        generating: false,
        message_id: (matchedTurn && matchedTurn.messageId) || null,
        assistant_text: (matchedTurn && matchedTurn.assistantText) || "",
      };
    }

    // Check server-side stream_status so remote mobile/web turns on this chat_id
    // are observed as in_progress before any automated wake-up or report injection.
    let remoteStreamStatus = null;
    const ssRes = await apiFetch(
      `/conversation/${encodeURIComponent(chatId)}/stream_status`,
      { method: "GET" },
      8000
    );
    if (ssRes.ok && ssRes.json && typeof ssRes.json.status === "string") {
      remoteStreamStatus = ssRes.json.status;
      if (remoteStreamStatus === "IS_STREAMING") {
        return {
          state: "in_progress",
          ...(matchedTurn ? { lane_id: matchedTurn.laneId } : {}),
          chat_id: chatId,
          generating: true,
          remote_stream_status: remoteStreamStatus,
          message_id: (matchedTurn && matchedTurn.messageId) || null,
          assistant_text: (matchedTurn && matchedTurn.assistantText) || "",
        };
      }
    }

    const rd = await readChat({ chat_id: chatId, limit: 10 });
    if (rd.state === "ok" && Array.isArray(rd.messages)) {
      const lastA = [...rd.messages].reverse().find((m) => m.role === "assistant");
      return {
        state: "settled",
        ...(matchedTurn ? { lane_id: matchedTurn.laneId } : {}),
        chat_id: chatId,
        generating: false,
        ...(remoteStreamStatus ? { remote_stream_status: remoteStreamStatus } : {}),
        message_id: (lastA && lastA.id) || (matchedTurn && matchedTurn.messageId) || null,
        assistant_text: (lastA && lastA.text) || (matchedTurn && matchedTurn.assistantText) || "",
      };
    }
    return {
      state: "settled",
      ...(matchedTurn ? { lane_id: matchedTurn.laneId } : {}),
      chat_id: chatId,
      generating: false,
      ...(remoteStreamStatus ? { remote_stream_status: remoteStreamStatus } : {}),
      message_id: (matchedTurn && matchedTurn.messageId) || null,
      assistant_text: (matchedTurn && matchedTurn.assistantText) || "",
    };
  }

  async function stopSingleTurn(turn, client) {
    turn.stopped = true;
    turn.generating = false;
    if (client && turn.requestId) {
      try {
        client.cancelStream(turn.requestId);
      } catch (_) {}
      if (turn.chatId) {
        try {
          await client.stopCompletion(
            { conversation_id: turn.chatId },
            { conduitToken: turn.conduitToken || undefined }
          );
        } catch (_) {}
      }
    }
    turn.resolveDone();
  }

  async function stopGeneration(opts = {}) {
    const requestedLaneId =
      typeof opts.lane_id === "string" && opts.lane_id.trim() ? opts.lane_id.trim() : null;
    const requestedChatId =
      typeof opts.chat_id === "string" && opts.chat_id.trim() ? opts.chat_id.trim() : null;
    const stopAll = opts.all === true;
    if (requestedLaneId && !requestedChatId && !activeTurns.has(requestedLaneId)) {
      cancelledLanes.set(requestedLaneId, Date.now() + 300000);
      while (cancelledLanes.size > MAX_TRACKED_TURNS * 2) {
        cancelledLanes.delete(cancelledLanes.keys().next().value);
      }
    }

    const client = await ensureCompletionClient();

    if (stopAll && !requestedLaneId && !requestedChatId) {
      const targets = generatingTurns();
      if (targets.length === 0) {
        return {
          state: "idle",
          stopped: false,
          stopped_count: 0,
          stopped_chat_ids: [],
          chat_id: (lastTurn && lastTurn.chatId) || currentChatIdFromUrl(),
        };
      }
      const stoppedChatIds = [];
      for (const t of targets) {
        await stopSingleTurn(t, client);
        if (t.chatId) stoppedChatIds.push(t.chatId);
      }
      return {
        state: "stopped",
        stopped: true,
        stopped_count: targets.length,
        stopped_chat_ids: stoppedChatIds,
        remaining_active_count: generatingTurns().length,
      };
    }

    const turn = findTrackedTurn({
      laneId: requestedLaneId,
      chatId: requestedChatId,
      preferGenerating: true,
    });
    if (
      !turn ||
      !turn.generating ||
      (requestedLaneId && turn.laneId !== requestedLaneId) ||
      (requestedChatId && turn.chatId !== requestedChatId)
    ) {
      return {
        state: "idle",
        stopped: false,
        chat_id: requestedChatId || (turn && turn.chatId) || currentChatIdFromUrl(),
        ...(requestedLaneId ? { lane_id: requestedLaneId } : {}),
        remaining_active_count: generatingTurns().length,
      };
    }

    await stopSingleTurn(turn, client);
    return {
      state: "stopped",
      stopped: true,
      lane_id: turn.laneId,
      chat_id: turn.chatId,
      remaining_active_count: generatingTurns().length,
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

    let chatId = typeof opts.chat_id === "string" && opts.chat_id.trim() ? opts.chat_id.trim() : null;
    const laneId =
      typeof opts.lane_id === "string" && opts.lane_id.trim() ? opts.lane_id.trim() : newUuid();

    const blockedLane = () => {
      const cancelledUntil = cancelledLanes.get(laneId);
      if (cancelledUntil > Date.now()) return { state: "stopped", lane_id: laneId };
      const liveTurns = generatingTurns();
      if (activeTurns.has(laneId) && activeTurns.get(laneId).generating) {
        return {
          state: "error",
          error: "generation_already_active",
          detail: "lane already has an active response",
          lane_id: laneId,
          chat_id: activeTurns.get(laneId).chatId || chatId,
        };
      }
      if (chatId) {
        const sameChat = liveTurns.find((t) => t.chatId === chatId);
        if (sameChat) {
          return {
            state: "error",
            error: "generation_already_active",
            detail: "conversation already has an active response",
            lane_id: sameChat.laneId,
            chat_id: chatId,
          };
        }
      }
      if (liveTurns.length >= MAX_CONCURRENT_GENERATING) {
        return {
          state: "error",
          error: "too_many_active_generations",
          detail: `at most ${MAX_CONCURRENT_GENERATING} concurrent lanes are allowed`,
          active_turn_count: liveTurns.length,
        };
      }

      return null;
    };
    const initialBlock = blockedLane();
    if (initialBlock) return initialBlock;

    const shouldWaitForBoot =
      isElectronAppOrigin() || Boolean(location && location.protocol === "about:");
    const client = await ensureCompletionClient(shouldWaitForBoot ? 30000 : 0);
    if (!client) {
      return {
        state: "error",
        error: "completion_client_unavailable",
        detail: "in-app CompletionClient not initialized yet",
      };
    }

    // Ensure we have current accountId and verify session auth (briefly retrying on cold boot).
    const expectedAccountId =
      typeof opts.expected_account_id === "string" && opts.expected_account_id.trim()
        ? opts.expected_account_id.trim()
        : null;
    let dispatchAccountId = cachedAccountId;
    let observedIdentity = null;
    if (expectedAccountId || opts.verify_identity || !cachedAccountId) {
      let me = await apiFetch("/me");
      for (let attempt = 0; attempt < 4 && !me.ok && me.status === 0 && isElectronAppOrigin(); attempt++) {
        await new Promise((r) => setTimeout(r, 1500));
        me = await apiFetch("/me");
      }
      if (!me.ok || !me.json) {
        if (me.status === 401 || me.status === 403 || me.status === 432) {
          return {
            state: "error",
            error: "login_required",
            detail: "ChatGPT account login required",
          };
        }
        return { state: "error", error: `backend_http_${me.status}`, detail: me.text };
      }
      const orgs = me.json.orgs && Array.isArray(me.json.orgs.data) ? me.json.orgs.data : [];
      const personalOrg = orgs.find((o) => o.is_personal) || orgs[0] || null;
      cachedAccountId = personalOrg?.id || me.json.account_id || null;
      dispatchAccountId = cachedAccountId;
      observedIdentity = dispatchAccountId || me.json.id || null;
      if (opts.verify_identity && !observedIdentity) {
        return { state: "error", error: "pinned_account_unavailable" };
      }
      if (expectedAccountId) {
        if (observedIdentity !== expectedAccountId) {
          return { state: "error", error: "pinned_account_mismatch" };
        }
        if (!dispatchAccountId) {
          // /me can expose only the authenticated user ID. Resolve the
          // native scope by testing the AppHost's account guard on read-only
          // /me requests before dispatch. Membership or ordering alone
          // does not identify the account bound to the AppHost.
          const { accounts } = await resolveChatGptAccounts();
          const candidates = accounts.filter((account) => !account.is_deactivated);
          const verified = (await Promise.all(candidates.map(async ({ id }) => {
            const scoped = await apiFetch("/me", {
              headers: { "X-OpenAI-Expected-Account-Id": id },
            });
            return scoped.ok && scoped.json?.id === expectedAccountId ? id : null;
          }))).filter(Boolean);
          if (verified.length !== 1) {
            return { state: "error", error: "pinned_account_unavailable" };
          }
          dispatchAccountId = verified[0];
        }
      }
    }

    let projectId =
      typeof opts.project_id === "string" && opts.project_id.trim() ? opts.project_id.trim() : null;
    let parentMessageId = newUuid();

    // If continuing an existing conversation, resolve its exact current_node leaf & project.
    if (chatId) {
      const convRes = await apiFetch(`/conversation/${encodeURIComponent(chatId)}`,
        opts.strict_project && dispatchAccountId
          ? { headers: { "X-OpenAI-Expected-Account-Id": dispatchAccountId } }
          : undefined);
      if (!convRes.ok || !convRes.json) {
        return {
          state: "error",
          error: `backend_http_${convRes.status}`,
          detail: convRes.text || "failed to load target conversation",
        };
      }
      if (typeof convRes.json.current_node === "string" && convRes.json.current_node) {
        parentMessageId = convRes.json.current_node;
      }
      if ((opts.strict_project || (expectedAccountId && projectId)) &&
          (convRes.json.gizmo_id || null) !== projectId) {
        return { state: "error", error: "pinned_project_mismatch" };
      }
      if (!projectId && typeof convRes.json.gizmo_id === "string" && convRes.json.gizmo_id) {
        projectId = convRes.json.gizmo_id;
      }
    }

    const nativeTools = buildNativeLocalFunctionSignatures(opts.local_function_signatures);
    const declaredToolNames = new Set(nativeTools.names);
    if (Array.isArray(opts.tool_names)) {
      for (const n of opts.tool_names) {
        if (typeof n === "string" && n.trim()) {
          declaredToolNames.add(n.trim());
          if (n.includes(".")) declaredToolNames.add(n.split(".").pop().trim());
        }
      }
    }

    const fallbackFenceMode =
      !opts.is_continuation &&
      Array.isArray(opts.local_function_signatures) &&
      opts.local_function_signatures.length > 0 &&
      nativeTools.signatures.length === 0;

    let wirePrompt = prompt;
    if (fallbackFenceMode) {
      const sigBlock = JSON.stringify(opts.local_function_signatures);
      wirePrompt =
        `[LOCAL_FUNCTION_SIGNATURES]\n${sigBlock}\n` +
        `If you need a local tool, emit one fenced block:\n` +
        `\`\`\`codex_tool_call\n{"call_id":"call_1","name":"<tool>","arguments":{}}\n\`\`\`\n` +
        `Otherwise answer directly.\n\n${prompt}`;
    }

    const modelSlug =
      typeof opts.model === "string" && opts.model.trim() ? opts.model.trim() : "auto";
    const thinkingEffort = normalizeThinkingEffort(modelSlug, opts.reasoning_effort);

    const userMsgId = newUuid();
    const requestPayload = {
      action: "next",
      ...(chatId ? { conversation_id: chatId } : {}),
      ...(projectId ? { gizmo_id: projectId } : {}),
      conversation_mode: projectId
        ? { kind: "gizmo_interaction", gizmo_id: projectId }
        : { kind: "primary_assistant" },
      supported_encodings: ["v1"],
      ...(nativeTools.signatures.length > 0
        ? {
            local_function_names: nativeTools.names,
            local_function_signatures: nativeTools.signatures,
          }
        : {}),
      messages: [
        {
          id: userMsgId,
          author: { role: "user" },
          create_time: Date.now() / 1000,
          content: { content_type: "text", parts: [wirePrompt] },
        },
      ],
      model: modelSlug,
      parent_message_id: parentMessageId,
      ...(thinkingEffort ? { thinking_effort: thinkingEffort } : {}),
      timezone:
        (typeof Intl !== "undefined" &&
          Intl.DateTimeFormat &&
          Intl.DateTimeFormat().resolvedOptions().timeZone) ||
        "UTC",
      timezone_offset_min: new Date().getTimezoneOffset(),
    };

    let resolveDone;
    const donePromise = new Promise((resolve) => {
      resolveDone = resolve;
    });

    const turn = {
      laneId,
      startedAt: new Date().toISOString(),
      requestId: null,
      conduitToken: null,
      chatId,
      projectId,
      model: modelSlug,
      generating: true,
      stopped: false,
      error: null,
      messageId: null,
      assistantText: "",
      items: new Map(),
      nativeToolCalls: new Map(),
      donePromise,
      resolveDone: () => {
        turn.generating = false;
        resolveDone();
      },
    };
    // Auth, boot, and conversation reads above yield to other callers.
    const finalBlock = blockedLane();
    if (finalBlock) return finalBlock;
    activeTurns.set(laneId, turn);
    lastTurn = turn;
    pruneSettledTurns();

    const emitStream = opts.push_events === true && typeof globalThis.__termuxChatGptStream === "function"
      ? (payload) => globalThis.__termuxChatGptStream(JSON.stringify({ lane_id: laneId, ...payload }))
      : () => {};

    let lastActivity = 0;
    const activity = () => {
      const now = Date.now();
      if (now - lastActivity >= 1000) {
        lastActivity = now;
        emitStream({ kind: "activity" });
      }
    };

    const committedReasoningTexts = new Set();
    const committedReasoningHeadings = new Set();
    const extractReasoningHeading = (text) => {
      if (typeof text !== "string") return "";
      const firstLine = text.split("\n", 1)[0].trim();
      const bold = firstLine.match(/^\*\*(.+?)\*\*$/);
      return (bold ? bold[1] : firstLine).trim();
    };
    const noteCommittedReasoning = (text, headingHint = null) => {
      const norm = typeof text === "string" ? text.trim() : "";
      if (norm) {
        committedReasoningTexts.add(norm);
        const h = extractReasoningHeading(norm);
        if (h) committedReasoningHeadings.add(h);
      }
      if (typeof headingHint === "string" && headingHint.trim()) {
        committedReasoningHeadings.add(headingHint.trim());
      }
    };
    const isDuplicateReasoning = (text, headingHint = null) => {
      const norm = typeof text === "string" ? text.trim() : "";
      if (!norm) return true;
      if (committedReasoningTexts.has(norm)) return true;
      if (!norm.includes("\n") && committedReasoningHeadings.has(extractReasoningHeading(norm))) {
        return true;
      }
      if (headingHint && !norm.includes("\n") && committedReasoningHeadings.has(headingHint.trim())) {
        return true;
      }
      return false;
    };

    let lastCommittedCommentary = "";
    const commitPriorItemsForNewMessage = (currentMsgKey, promoteInterimAnswer = false) => {
      let demotedAnswer = false;
      for (const item of turn.items.values()) {
        if (item.committed || item.msgKey === currentMsgKey) continue;
        if (item.kind === "reasoning_summary") {
          item.committed = true;
          noteCommittedReasoning(item.text);
          emitStream({
            kind: "item_upsert",
            item_id: item.id,
            item_kind: item.kind,
            text: item.text,
            committed: true,
          });
        } else if (item.kind === "commentary") {
          item.committed = true;
          lastCommittedCommentary = item.text.trim();
          emitStream({
            kind: "item_upsert",
            item_id: item.id,
            item_kind: item.kind,
            text: item.text,
            committed: true,
          });
        } else if (promoteInterimAnswer && item.kind === "answer") {
          item.kind = "commentary";
          item.committed = true;
          lastCommittedCommentary = item.text.trim();
          demotedAnswer = true;
          emitStream({
            kind: "item_upsert",
            item_id: item.id,
            item_kind: "commentary",
            text: item.text,
            committed: true,
          });
        }
      }
      if (demotedAnswer) {
        syncDerivedAssistantText();
      }
    };

    const upsertTextItem = (
      itemId,
      itemKind,
      rawText,
      committed,
      msgKey = null,
      promoteInterimAnswer = false
    ) => {
      const text = truncate(rawText);
      const existing = turn.items.get(itemId);
      if (!text) {
        if (existing && !existing.committed) {
          turn.items.delete(itemId);
          emitStream({ kind: "item_remove", item_id: itemId });
        }
        return;
      }
      if (msgKey !== null) {
        commitPriorItemsForNewMessage(msgKey, promoteInterimAnswer);
      }
      if (itemKind === "reasoning_summary" && !existing && isDuplicateReasoning(text)) {
        return;
      }
      if (existing) {
        if (existing.committed) return;
        const nextCommitted = Boolean(committed);
        if (existing.kind === itemKind && existing.text === text && existing.committed === nextCommitted) {
          return;
        }
        existing.kind = itemKind;
        existing.text = text;
        existing.committed = nextCommitted;
        if (msgKey !== null) existing.msgKey = msgKey;
      } else {
        turn.items.set(itemId, {
          id: itemId,
          kind: itemKind,
          text,
          committed: Boolean(committed),
          msgKey,
        });
      }
      if (itemKind === "reasoning_summary" && committed) {
        noteCommittedReasoning(text);
      }
      if (itemKind === "commentary" && committed) {
        lastCommittedCommentary = text.trim();
      }
      emitStream({
        kind: "item_upsert",
        item_id: itemId,
        item_kind: itemKind,
        text,
        committed: Boolean(committed),
      });
    };

    const emittedToolIds = new Set();
    const emittedToolReportIds = new Set();
    const emitToolReportIfNeeded = (call, msgKey = null) => {
      const report = typeof call?.report === "string" ? call.report.trim() : "";
      if (!report || emittedToolReportIds.has(call.id)) return;
      emittedToolReportIds.add(call.id);
      if (report === lastCommittedCommentary) return;
      upsertTextItem(`report:${call.id}`, "commentary", report, true, msgKey);
    };
    const recordToolCall = (call, committed, msgKey = null) => {
      if (msgKey !== null) {
        commitPriorItemsForNewMessage(msgKey, true);
      }
      turn.nativeToolCalls.set(call.id, call);
      const itemId = `tool:${call.id}`;
      if (committed) {
        emitToolReportIfNeeded(call, msgKey);
      }
      const existing = turn.items.get(itemId);
      if (!existing || !existing.committed) {
        if (committed && existing) {
          // Ensure tool item sits after any newly inserted `report:<call_id>` item.
          turn.items.delete(itemId);
        }
        turn.items.set(itemId, {
          id: itemId,
          kind: "tool_call",
          committed: Boolean(committed),
          call,
          msgKey,
        });
      }
      if (committed && !emittedToolIds.has(call.id)) {
        emittedToolIds.add(call.id);
        emitStream({
          kind: "tool_call",
          call_id: call.id,
          name: call.name,
          ...(call.namespace ? { namespace: call.namespace } : {}),
          arguments: call.arguments,
        });
      }
    };

    const syncDerivedAssistantText = () => {
      const answers = [];
      for (const item of turn.items.values()) {
        if (item.kind === "answer" && item.text) answers.push(item.text);
      }
      turn.assistantText = truncate(answers.join("\n\n"));
    };

    const isMessageCommitted = (msg) => Boolean(
      (typeof msg.status === "string" && msg.status !== "in_progress") ||
      msg.end_turn === true
    );

    const recordMessage = (msg) => {
      if (!msg || typeof msg !== "object" || turn.stopped) return;
      const content = msg.content;
      const contentType = content?.content_type;
      const msgKey = String(msg.id || turn.messageId || `anon_${turn.items.size}`);
      const msgCommitted = isMessageCommitted(msg);

      const nativeCall = extractNativeToolCallFromMessage(
        msg,
        declaredToolNames,
        nativeTools.wireToOriginal,
        nativeTools.injectedReportWireNames
      );
      if (nativeCall) {
        if (msg.id) turn.messageId = msg.id;
        recordToolCall(nativeCall, msgCommitted, msgKey);
        return;
      }

      if (contentType === "thoughts") {
        if (!canExposeReasoning(msg)) return;
        let emittedThought = false;
        if (Array.isArray(content.thoughts)) {
          const len = content.thoughts.length;
          for (let i = 0; i < len; i++) {
            const entry = content.thoughts[i];
            const formatted = formatThoughtEntry(entry);
            if (!formatted) continue;
            emittedThought = true;
            const thoughtCommitted = Boolean(
              i + 1 < len || entry?.finished === true || msgCommitted
            );
            upsertTextItem(`thought:${msgKey}:${i + 1}`, "reasoning_summary", formatted, thoughtCommitted, msgKey);
          }
        } else {
          const direct = formatThoughtEntry({
            summary: content.summary,
            content: content.content,
            chunks: content.chunks,
          });
          if (direct) {
            emittedThought = true;
            upsertTextItem(`thought:${msgKey}`, "reasoning_summary", direct, msgCommitted, msgKey);
          } else {
            const rawParts = extractRawPartsText(msg);
            if (rawParts && rawParts.trim()) {
              emittedThought = true;
              upsertTextItem(`thought:${msgKey}`, "reasoning_summary", rawParts.trim(), msgCommitted, msgKey);
            }
          }
        }
        if (!emittedThought) {
          const titles = extractVisibleReasoningTitles(msg);
          for (let i = 0; i < titles.length; i++) {
            const titleCommitted = Boolean(i + 1 < titles.length || msgCommitted);
            upsertTextItem(`title:${msgKey}:${i}`, "reasoning_summary", titles[i], titleCommitted, msgKey);
          }
        }
        return;
      }

      if (contentType === "reasoning_recap") {
        if (!canExposeReasoning(msg)) return;
        const recap = typeof content.content === "string"
          ? content.content
          : extractRawPartsText(msg);
        if (typeof recap === "string" && recap.trim()) {
          upsertTextItem(`recap:${msgKey}`, "reasoning_summary", recap.trim(), msgCommitted, msgKey);
        }
        return;
      }

      if (msg.channel === "analysis") {
        if (msg.author?.role !== "assistant" || (msg.recipient && msg.recipient !== "all")) return;
        const analysisText = extractRawPartsText(msg);
        if (analysisText && analysisText.trim()) {
          upsertTextItem(`analysis:${msgKey}`, "reasoning_summary", analysisText.trim(), msgCommitted, msgKey);
        }
        return;
      }

      if (canExposeReasoning(msg)) {
        const titles = extractVisibleReasoningTitles(msg);
        for (let i = 0; i < titles.length; i++) {
          const titleCommitted = Boolean(i + 1 < titles.length || msgCommitted);
          upsertTextItem(`title:${msgKey}:${i}`, "reasoning_summary", titles[i], titleCommitted, msgKey);
        }
      }

      if (!isVisibleAssistantMessage(msg)) return;
      const raw = extractRawPartsText(msg);
      if (raw === null) return;
      if (msg.id) turn.messageId = msg.id;
      const isCommentary =
        msg.channel === "commentary" ||
        msg.metadata?.is_thinking_preamble_message === true ||
        msg.status === "finished_partial_completion" ||
        (msg.status === "finished_successfully" && msg.end_turn === false);
      if (isCommentary) {
        const prevAnswerId = `answer:${msgKey}`;
        const prevAnswer = turn.items.get(prevAnswerId);
        if (prevAnswer && !prevAnswer.committed) {
          turn.items.delete(prevAnswerId);
          emitStream({ kind: "item_remove", item_id: prevAnswerId });
        }
        upsertTextItem(`commentary:${msgKey}`, "commentary", raw, msgCommitted, msgKey, true);
        syncDerivedAssistantText();
        return;
      }
      if (fallbackFenceMode && raw.includes("```codex_tool_call")) {
        // Hold mutable fallback fence text uncommitted until turn/message settles.
        upsertTextItem(`answer:${msgKey}`, "answer", raw, false, msgKey, true);
      } else {
        upsertTextItem(`answer:${msgKey}`, "answer", raw, msgCommitted, msgKey, true);
      }
      syncDerivedAssistantText();
    };

    let sawDecodedEvent = false;
    const handleStreamPayload = (cid, msg, isInput) => {
      if (typeof cid === "string" && cid) {
        turn.chatId = cid;
      }
      if (!isInput && msg) {
        recordMessage(msg);
      }
    };

    const settleItems = () => {
      const pending = Array.from(turn.items.values());
      for (const item of pending) {
        if (!item.committed) {
          if (item.kind === "tool_call" && item.call) {
            emitToolReportIfNeeded(item.call, item.msgKey);
            turn.items.delete(item.id);
            item.committed = true;
            turn.items.set(item.id, item);
            if (!emittedToolIds.has(item.call.id)) {
              emittedToolIds.add(item.call.id);
              emitStream({
                kind: "tool_call",
                call_id: item.call.id,
                name: item.call.name,
                ...(item.call.namespace ? { namespace: item.call.namespace } : {}),
                arguments: item.call.arguments,
              });
            }
          } else {
            emitStream({
              kind: "item_upsert",
              item_id: item.id,
              item_kind: item.kind,
              text: item.text,
              committed: true,
            });
          }
        }
      }
      emitStream({
        kind: "turn_completed",
        chat_id: turn.chatId,
        message_id: turn.messageId,
      });
    };

    try {
      const handle = await client.startCompletionStream({
        request: requestPayload,
        expectedAccountId: dispatchAccountId || undefined,
        turnTraceId: newUuid(),
        onDecodedEvent: (ev) => {
          sawDecodedEvent = true;
          if (!ev || typeof ev !== "object") return;
          activity();
          const cid = ev?.update?.conversationId || ev?.payload?.conversation_id;
          const msg = ev?.update?.type === "message" ? ev.update.message : ev?.payload?.message;
          handleStreamPayload(cid, msg, ev?.isInputMessage === true);
        },
        onUpdate: (update) => {
          if (sawDecodedEvent || !update || typeof update !== "object") return;
          activity();
          if (update.type === "message" && update.message) {
            handleStreamPayload(update.conversationId, update.message, false);
          } else if (typeof update.conversationId === "string" && update.conversationId) {
            turn.chatId = update.conversationId;
          }
        },
        onComplete: () => {
          if (!turn.stopped) settleItems();
          turn.resolveDone();
        },
        onError: (err) => {
          if (!turn.stopped) {
            const code = err?.errorCode || (err?.responseStatus ? `backend_http_${err.responseStatus}` : "completion_stream_error");
            const detail = err?.error || err?.message || String(err || "completion stream failed");
            turn.error = { code: String(code), detail: truncate(String(detail), 512) };
          }
          turn.resolveDone();
        },
      });
      if (handle && typeof handle === "object") {
        turn.requestId = handle.streamRequestId || null;
        turn.conduitToken = handle.prepareResponse?.conduit_token || null;
        if (turn.stopped) await stopSingleTurn(turn, client);
      }
    } catch (err) {
      turn.generating = false;
      turn.error = {
        code: "completion_dispatch_failed",
        detail: truncate(String(err && err.message ? err.message : err), 512),
      };
      turn.resolveDone();
    }

    const waitMs = Math.max(0, Math.min(300000, Number(opts.wait_ms ?? 30000)));
    if (opts.push_events) {
      await turn.donePromise;
    } else if (waitMs > 0 && turn.generating) {
      let timer;
      try {
        await Promise.race([
          turn.donePromise,
          new Promise((resolve) => { timer = setTimeout(resolve, waitMs); }),
        ]);
      } finally {
        clearTimeout(timer);
      }
    }

    if (turn.error) {
      return {
        state: "error",
        lane_id: turn.laneId,
        error: turn.error.code,
        detail: turn.error.detail,
        chat_id: turn.chatId,
      };
    }

    if (turn.generating) {
      return {
        state: "in_progress",
        lane_id: turn.laneId,
        chat_id: turn.chatId,
        message_id: turn.messageId,
        assistant_text: turn.assistantText,
        tool_calls: [],
        generating: true,
      };
    }

    // If settled with a server conversation_id, zero native tool calls, and empty stream answers,
    // fetch only an assistant message strictly AFTER this turn's userMsgId so previous-turn text
    // can never leak into a tool-only or empty turn.
    const hasAnswerItem = [...turn.items.values()].some((item) => item.kind === "answer" && item.text);
    if (!hasAnswerItem && turn.nativeToolCalls.size === 0 && turn.chatId) {
      const rd = await readChat({ chat_id: turn.chatId, limit: 10 });
      if (rd.state === "ok" && Array.isArray(rd.messages)) {
        const userIdx = rd.messages.findIndex((m) => m.id === userMsgId);
        if (userIdx >= 0) {
          const lastA = [...rd.messages.slice(userIdx + 1)]
            .reverse()
            .find((m) => m.role === "assistant");
          if (lastA && lastA.text) {
            const fid = `answer:${lastA.id || "fallback"}`;
            turn.items.set(fid, { id: fid, kind: "answer", text: truncate(lastA.text), committed: true });
            turn.messageId = lastA.id;
            syncDerivedAssistantText();
          }
        }
      }
    }

    const normalizedItems = [];
    const summaryParts = [];
    const answerParts = [];
    const seenToolIds = new Set();
    const settledToolCalls = [];
    for (const item of turn.items.values()) {
      if (item.kind === "tool_call" && item.call) {
        if (!seenToolIds.has(item.call.id)) {
          seenToolIds.add(item.call.id);
          const tc = {
            id: item.call.id,
            name: item.call.name,
            ...(item.call.namespace ? { namespace: item.call.namespace } : {}),
            arguments: item.call.arguments,
          };
          settledToolCalls.push(tc);
          normalizedItems.push({ id: item.id, kind: "tool_call", call_id: tc.id, name: tc.name, ...(tc.namespace ? { namespace: tc.namespace } : {}), arguments: tc.arguments });
        }
        continue;
      }
      if (!item.text) continue;
      if (item.kind === "reasoning_summary") {
        summaryParts.push(item.text);
        normalizedItems.push({ id: item.id, kind: "reasoning_summary", text: item.text });
      } else if (item.kind === "commentary") {
        normalizedItems.push({ id: item.id, kind: "commentary", text: item.text });
      } else if (item.kind === "answer") {
        if (turn.nativeToolCalls.size === 0 && (fallbackFenceMode || item.text.includes("```codex_tool_call"))) {
          const parsed = parseToolCallsFromText(item.text);
          if (parsed.text) {
            answerParts.push(parsed.text);
            normalizedItems.push({ id: item.id, kind: "answer", text: parsed.text });
          }
          for (const fc of parsed.calls) {
            if (!seenToolIds.has(fc.id)) {
              seenToolIds.add(fc.id);
              settledToolCalls.push(fc);
              normalizedItems.push({ id: `tool:${fc.id}`, kind: "tool_call", call_id: fc.id, name: fc.name, ...(fc.namespace ? { namespace: fc.namespace } : {}), arguments: fc.arguments });
            }
          }
        } else {
          answerParts.push(item.text);
          normalizedItems.push({ id: item.id, kind: "answer", text: item.text });
        }
      }
    }

    const finalReasoningSummary = truncate(summaryParts.join("\n\n"));
    const finalAssistantText = truncate(answerParts.join("\n\n"));
    turn.assistantText = finalAssistantText;
    return {
      state: turn.stopped ? "stopped" : "settled",
      lane_id: turn.laneId,
      chat_id: turn.chatId,
      message_id: turn.messageId,
      ...(opts.verify_identity ? { account_id: observedIdentity } : {}),
      items: normalizedItems,
      reasoning_summary: finalReasoningSummary,
      assistant_text: finalAssistantText,
      tool_calls: settledToolCalls,
      generating: false,
    };
  }

  function canonicalJsonValue(val) {
    if (Array.isArray(val)) return val.map(canonicalJsonValue);
    if (val && typeof val === "object") {
      const out = {};
      for (const k of Object.keys(val).sort()) {
        out[k] = canonicalJsonValue(val[k]);
      }
      return out;
    }
    return val;
  }

  function catalogAgreement(actions, tools) {
    if (!Array.isArray(actions) || !Array.isArray(tools) || actions.length !== tools.length) {
      return false;
    }
    return tools.every((tool) => {
      const matches = actions.filter((a) => a && a.name === tool.name);
      return (
        matches.length === 1 &&
        matches[0].params &&
        matches[0].params.type === "object" &&
        tool.inputSchema &&
        tool.inputSchema.type === "object" &&
        JSON.stringify(canonicalJsonValue(matches[0].params)) ===
          JSON.stringify(canonicalJsonValue(tool.inputSchema))
      );
    });
  }

  async function resolveChatGptAccounts(preferredAccountId) {
    const accounts = [];
    let ordering = [];
    const check = await apiFetch("/accounts/check/v4-2023-04-27");
    if (check.ok && check.json && typeof check.json === "object") {
      if (Array.isArray(check.json.account_ordering)) {
        ordering = check.json.account_ordering.filter((x) => typeof x === "string" && x);
      }
      const map = check.json.accounts && typeof check.json.accounts === "object" ? check.json.accounts : {};
      for (const [key, entry] of Object.entries(map)) {
        if (key === "default" || !entry || typeof entry !== "object") continue;
        const acc = entry.account && typeof entry.account === "object" ? entry.account : {};
        const id = acc.account_id || key;
        if (!id || typeof id !== "string") continue;
        accounts.push({
          id,
          name: acc.name || null,
          structure: acc.structure || null,
          plan_type: acc.plan_type || (entry.entitlement && entry.entitlement.subscription_plan) || null,
          is_deactivated: Boolean(acc.is_deactivated),
        });
      }
    }
    if (!cachedAccountId) {
      const me = await apiFetch("/me");
      if (me.ok && me.json) {
        const orgs = me.json.orgs && Array.isArray(me.json.orgs.data) ? me.json.orgs.data : [];
        const personalOrg = orgs.find((o) => o.is_personal) || orgs[0] || null;
        cachedAccountId = personalOrg?.id || me.json.account_id || cachedAccountId;
      }
    }
    const explicit =
      typeof preferredAccountId === "string" && preferredAccountId.trim()
        ? preferredAccountId.trim()
        : null;
    const selected =
      explicit ||
      ordering.find((id) => accounts.some((a) => a.id === id && !a.is_deactivated)) ||
      (accounts.find((a) => !a.is_deactivated) && accounts.find((a) => !a.is_deactivated).id) ||
      cachedAccountId ||
      null;
    return { accountId: selected, accounts, ordering };
  }

  function connectorHeaders(accountId, sku) {
    const headers = {
      "OAI-Product-Sku": typeof sku === "string" && sku.trim() ? sku.trim() : "CONNECTOR_SETTING",
    };
    if (accountId) {
      headers["ChatGPT-Account-Id"] = accountId;
      headers["ChatGPT-Account-ID"] = accountId;
    }
    return headers;
  }

  async function invalidateConnectorQueries() {
    try {
      const scope = findAppScope();
      if (scope && scope.queryClient && typeof scope.queryClient.invalidateQueries === "function") {
        await scope.queryClient.invalidateQueries();
        return true;
      }
    } catch (_) {}
    return false;
  }

  function isVisibleConnectorLink(link) {
    return Boolean(
      link &&
      link.visibility !== "HIDDEN" &&
      !(typeof link.id === "string" && link.id.startsWith("implicit_link"))
    );
  }

  async function fetchAccessibleConnectorDetails(headers, includeDetails) {
    const linksRes = await apiFetch(
      "/aip/connectors/links/list_accessible",
      {
        method: "POST",
        headers,
        body: JSON.stringify({ principals: [], link_refresh_strategy: "BLOCKING" }),
      },
      20000
    );
    const rawLinks =
      linksRes.ok && linksRes.json && Array.isArray(linksRes.json.links) ? linksRes.json.links : [];
    const visibleLinks = rawLinks.filter(isVisibleConnectorLink);
    const connectors = [];
    if (includeDetails) {
      const seen = new Set();
      for (const link of visibleLinks.slice(0, 40)) {
        const cid = link.connector_id;
        if (!cid || seen.has(cid)) continue;
        seen.add(cid);
        const det = await apiFetch(
          `/aip/connectors/${encodeURIComponent(cid)}?include_actions=true`,
          { method: "GET", headers },
          15000
        );
        if (det.ok && det.json) {
          const c = det.json;
          connectors.push({
            id: c.id || cid,
            name: c.name || link.name || "",
            description: c.description || "",
            connector_type: c.connector_type || null,
            tunnel_id: c.tunnel_id || null,
            mcp_url: c.mcp_url || null,
            link_id: link.id || null,
            auth_status: link.auth_status || null,
            actions: Array.isArray(c.actions)
              ? c.actions.map((a) => ({ name: a.name, params: a.params || null }))
              : [],
          });
        } else {
          connectors.push({
            id: cid,
            name: link.name || "",
            link_id: link.id || null,
            auth_status: link.auth_status || null,
            detail_status: det.status,
          });
        }
      }
    }
    return {
      linksStatus: linksRes.status,
      linksOk: linksRes.ok,
      linksError: linksRes.ok ? null : linksRes.text,
      links: visibleLinks.map((l) => ({
        id: l.id || null,
        connector_id: l.connector_id || null,
        name: l.name || null,
        auth_status: l.auth_status || null,
        auth_type: l.auth_type || null,
      })),
      connectors,
    };
  }

  async function listConnectors(opts = {}) {
    const { accountId, accounts } = await resolveChatGptAccounts(opts.account_id);
    const sku = typeof opts.sku === "string" && opts.sku.trim() ? opts.sku.trim() : "CONNECTOR_SETTING";
    const headers = connectorHeaders(accountId, sku);

    const tunnelsRes = await apiFetch("/aip/connectors/mcp/tunnels", { method: "GET", headers }, 15000);
    const tunnels =
      tunnelsRes.ok && tunnelsRes.json && Array.isArray(tunnelsRes.json.tunnels)
        ? tunnelsRes.json.tunnels
        : [];
    const includeDetails = opts.include_details !== false;
    const accessible = await fetchAccessibleConnectorDetails(headers, includeDetails);

    return {
      state: "ok",
      account_id: accountId,
      sku,
      accounts,
      tunnels_status: tunnelsRes.status,
      tunnels,
      ...(tunnelsRes.ok ? {} : { tunnels_error: tunnelsRes.text }),
      links_status: accessible.linksStatus,
      ...(accessible.linksError ? { links_error: accessible.linksError } : {}),
      links: accessible.links,
      connectors: accessible.connectors,
    };
  }

  async function refreshAndProbeConnector({ connectorId, linkId, headers, expectedName }) {
    const refreshRes = await apiFetch(
      "/aip/connectors/mcp/refresh_actions",
      {
        method: "POST",
        headers,
        body: JSON.stringify({ link_id: linkId }),
      },
      30000
    );
    const cacheInvalidated = await invalidateConnectorQueries();

    const probeRes = await apiFetch(
      "/ecosystem/call_mcp",
      {
        method: "POST",
        headers,
        body: JSON.stringify({
          app_uri: `connectors://${connectorId}`,
          launcher_host: "chatgpt",
          method: "tools/list",
          params: {},
        }),
      },
      30000
    );
    const liveTools =
      probeRes.ok && probeRes.json && Array.isArray(probeRes.json.tools)
        ? probeRes.json.tools
        : null;

    let after = null;
    for (let i = 0; i < 4; i++) {
      const det = await apiFetch(
        `/aip/connectors/${encodeURIComponent(connectorId)}?include_actions=true`,
        { method: "GET", headers },
        15000
      );
      if (det.ok && det.json) {
        after = det.json;
        if (
          liveTools &&
          Array.isArray(after.actions) &&
          catalogAgreement(after.actions, liveTools)
        ) {
          break;
        }
      }
      if (i < 3) {
        await new Promise((r) => setTimeout(r, 500 * (i + 1)));
      }
    }

    const actions = after && Array.isArray(after.actions) ? after.actions : [];
    const agreement = liveTools ? catalogAgreement(actions, liveTools) : false;
    return {
      refresh_ok: refreshRes.ok,
      refresh_status: refreshRes.status,
      ...(refreshRes.ok ? {} : { refresh_error: refreshRes.text }),
      cache_invalidated: cacheInvalidated,
      probe_ok: probeRes.ok && Array.isArray(liveTools),
      probe_status: probeRes.status,
      ...(probeRes.ok ? {} : { probe_error: probeRes.text }),
      name: (after && after.name) || expectedName || null,
      tunnel_id: (after && after.tunnel_id) || null,
      actions: actions.map((a) => ({ name: a.name, params: a.params || null })),
      live_tools: liveTools
        ? liveTools.map((t) => ({ name: t.name, inputSchema: t.inputSchema || null }))
        : [],
      schema_agreement: agreement,
    };
  }

  async function registerConnector(opts = {}) {
    const name = typeof opts.name === "string" ? opts.name.trim() : "";
    const tunnelId = typeof opts.tunnel_id === "string" ? opts.tunnel_id.trim() : "";
    const description =
      typeof opts.description === "string" ? opts.description.trim() : "";
    if (!name || name.length > 120) {
      return { state: "error", error: "invalid_connector_name", detail: "name must be 1..=120 chars" };
    }
    if (!/^tunnel_(?:[a-z0-9]{4}_)?[a-z0-9]{32}$/.test(tunnelId)) {
      return { state: "error", error: "invalid_tunnel_id", detail: "tunnel_id must match tunnel_[xxxx_]<32-alnum>" };
    }

    const { accountId } = await resolveChatGptAccounts(opts.account_id);
    const sku = typeof opts.sku === "string" && opts.sku.trim() ? opts.sku.trim() : "CONNECTOR_SETTING";
    const headers = connectorHeaders(accountId, sku);

    const accessible = await fetchAccessibleConnectorDetails(headers, true);
    let existing = accessible.connectors.find((c) => c.tunnel_id === tunnelId) || null;
    if (!existing) {
      const nameConflict = accessible.connectors.find(
        (c) => typeof c.name === "string" && c.name.trim() === name && c.tunnel_id !== tunnelId
      );
      if (nameConflict) {
        return {
          state: "error",
          error: "name_conflict",
          detail: `connector named '${name}' already exists for a different target`,
          existing_connector_id: nameConflict.id,
          existing_tunnel_id: nameConflict.tunnel_id || null,
        };
      }
    }

    let connectorId = existing ? existing.id : null;
    let created = false;
    let alreadyExisted = Boolean(existing);
    if (!connectorId) {
      const createRes = await apiFetch(
        "/aip/connectors/mcp",
        {
          method: "POST",
          headers,
          body: JSON.stringify({
            name,
            description,
            tunnel_id: tunnelId,
            logo_url: null,
            auth_request: {
              supported_auth: [{ type: "NONE" }],
            },
          }),
        },
        30000
      );
      if (!createRes.ok || !createRes.json || !createRes.json.connector || !createRes.json.connector.id) {
        return {
          state: "error",
          error: `connector_create_http_${createRes.status}`,
          status: createRes.status,
          account_id: accountId,
          detail: createRes.text,
        };
      }
      connectorId = createRes.json.connector.id;
      created = true;
    }

    let linkId = existing ? existing.link_id : null;
    let authStatus = existing ? existing.auth_status : null;
    let linked = false;
    const linkCheck = await apiFetch(
      `/aip/connectors/${encodeURIComponent(connectorId)}/link`,
      { method: "GET", headers },
      15000
    );
    if (linkCheck.ok && linkCheck.json && linkCheck.json.link && linkCheck.json.link.id) {
      linkId = linkCheck.json.link.id;
      authStatus = linkCheck.json.link.auth_status || authStatus;
    } else {
      const noauthRes = await apiFetch(
        "/aip/connectors/links/noauth",
        {
          method: "POST",
          headers,
          body: JSON.stringify({
            connector_id: connectorId,
            name,
            action_names: [],
          }),
        },
        20000
      );
      if (!noauthRes.ok) {
        return {
          state: "error",
          error: `connector_link_http_${noauthRes.status}`,
          status: noauthRes.status,
          account_id: accountId,
          connector_id: connectorId,
          created,
          detail: noauthRes.text,
        };
      }
      linked = true;
      linkId = (noauthRes.json && (noauthRes.json.id || (noauthRes.json.link && noauthRes.json.link.id))) || null;
      const recheck = await apiFetch(
        `/aip/connectors/${encodeURIComponent(connectorId)}/link`,
        { method: "GET", headers },
        15000
      );
      if (recheck.ok && recheck.json && recheck.json.link) {
        linkId = recheck.json.link.id || linkId;
        authStatus = recheck.json.link.auth_status || authStatus;
      }
    }

    if (opts.refresh === false || !linkId) {
      const cacheInvalidated = await invalidateConnectorQueries();
      return {
        state: "ok",
        account_id: accountId,
        created,
        already_existed: alreadyExisted,
        linked,
        connector_id: connectorId,
        name,
        tunnel_id: tunnelId,
        link_id: linkId,
        auth_status: authStatus,
        cache_invalidated: cacheInvalidated,
      };
    }

    const verified = await refreshAndProbeConnector({
      connectorId,
      linkId,
      headers,
      expectedName: name,
    });
    return {
      state: verified.refresh_ok && verified.probe_ok && verified.schema_agreement ? "ok" : "degraded",
      account_id: accountId,
      created,
      already_existed: alreadyExisted,
      linked,
      connector_id: connectorId,
      name: verified.name || name,
      tunnel_id: verified.tunnel_id || tunnelId,
      link_id: linkId,
      auth_status: authStatus,
      ...verified,
    };
  }

  async function refreshConnector(opts = {}) {
    const connectorId = typeof opts.connector_id === "string" ? opts.connector_id.trim() : "";
    if (!connectorId) {
      return { state: "error", error: "connector_id_required", detail: "connector_id is required" };
    }
    const { accountId } = await resolveChatGptAccounts(opts.account_id);
    const sku = typeof opts.sku === "string" && opts.sku.trim() ? opts.sku.trim() : "CONNECTOR_SETTING";
    const headers = connectorHeaders(accountId, sku);

    const beforeRes = await apiFetch(
      `/aip/connectors/${encodeURIComponent(connectorId)}?include_actions=true`,
      { method: "GET", headers },
      15000
    );
    if (!beforeRes.ok || !beforeRes.json) {
      return {
        state: "error",
        error: `connector_get_http_${beforeRes.status}`,
        status: beforeRes.status,
        account_id: accountId,
        connector_id: connectorId,
        detail: beforeRes.text,
      };
    }

    let linkId = null;
    let authStatus = null;
    const linkRes = await apiFetch(
      `/aip/connectors/${encodeURIComponent(connectorId)}/link`,
      { method: "GET", headers },
      15000
    );
    if (linkRes.ok && linkRes.json && linkRes.json.link && linkRes.json.link.id) {
      linkId = linkRes.json.link.id;
      authStatus = linkRes.json.link.auth_status || null;
    } else if (opts.link_if_missing === true) {
      const noauthRes = await apiFetch(
        "/aip/connectors/links/noauth",
        {
          method: "POST",
          headers,
          body: JSON.stringify({
            connector_id: connectorId,
            name: beforeRes.json.name || connectorId,
            action_names: [],
          }),
        },
        20000
      );
      if (noauthRes.ok && noauthRes.json) {
        linkId = noauthRes.json.id || (noauthRes.json.link && noauthRes.json.link.id) || null;
      }
    }
    if (!linkId) {
      return {
        state: "error",
        error: "connector_not_linked",
        account_id: accountId,
        connector_id: connectorId,
        detail: linkRes.text || "connector has no active link",
      };
    }

    const verified = await refreshAndProbeConnector({
      connectorId,
      linkId,
      headers,
      expectedName: beforeRes.json.name || null,
    });
    return {
      state: verified.refresh_ok && verified.probe_ok && verified.schema_agreement ? "ok" : "degraded",
      account_id: accountId,
      connector_id: connectorId,
      link_id: linkId,
      auth_status: authStatus,
      ...verified,
    };
  }

  globalThis.__termuxChatGptDriver = {
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
    listConnectors,
    registerConnector,
    refreshConnector,
    parseToolCallsFromText,
  };
  return { installed: true };
})();
