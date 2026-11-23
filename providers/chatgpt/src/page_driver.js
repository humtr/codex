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
  let activeTurn = null;

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

  function extractTextFromMessage(msg) {
    if (!msg || typeof msg !== "object") return null;
    const role = msg.author && msg.author.role ? msg.author.role : "unknown";
    const contentType = msg.content && msg.content.content_type ? msg.content.content_type : "";
    if (contentType === "thoughts" || contentType === "reasoning_recap") return null;
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
      return { names: [], signatures: [], wireToOriginal: new Map() };
    }
    const names = [];
    const signatures = [];
    const wireToOriginal = new Map();
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
      if (s.type === "custom") {
        params.push({
          name: "input",
          required: true,
          type: { type: "string", description: "Raw tool input string" },
        });
      } else if (s.parameters && typeof s.parameters === "object" && s.parameters.properties) {
        const reqSet = new Set(Array.isArray(s.parameters.required) ? s.parameters.required : []);
        for (const [pname, pdef] of Object.entries(s.parameters.properties)) {
          params.push({
            name: pname,
            required: reqSet.has(pname),
            type: normalizeParamSchema(pdef, pname, 0),
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
    return { names, signatures, wireToOriginal };
  }

  function extractNativeToolCallFromMessage(msg, declaredNames, wireToOriginal) {
    if (!msg || typeof msg !== "object") return null;
    if (msg.author?.role !== "assistant") return null;
    const recipient = typeof msg.recipient === "string" ? msg.recipient.trim() : "";
    if (!recipient || recipient === "all") return null;
    const stripped = recipient.includes(".") ? recipient.split(".").pop() : recipient;
    if (declaredNames.size > 0 && !declaredNames.has(recipient) && !declaredNames.has(stripped)) {
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
    return {
      id: msg.id ? `call_${String(msg.id).replace(/[^A-Za-z0-9_-]/g, "").slice(0, 24)}` : `call_${newUuid().slice(0, 8)}`,
      name: resolvedName,
      ...(resolvedNs ? { namespace: resolvedNs } : {}),
      arguments: rawArgs,
      native_recipient: recipient,
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
        cachedAccountId = personalOrg?.id || me.json.account_id || cachedAccountId;
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
            max_tokens: m.max_tokens || null,
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
      onboarding_visible: onboarding,
      account,
      active_chat_id: (activeTurn && activeTurn.chatId) || currentChatIdFromUrl(),
      generating: Boolean(activeTurn && activeTurn.generating),
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
    const chatId = opts.chat_id || (activeTurn && activeTurn.chatId) || currentChatIdFromUrl();
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
      current_node: res.json.current_node || null,
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
    const requestedId = opts.chat_id || null;
    if (
      activeTurn &&
      activeTurn.generating &&
      (!requestedId || requestedId === activeTurn.chatId)
    ) {
      return {
        state: "in_progress",
        chat_id: activeTurn.chatId || requestedId,
        generating: true,
        message_id: activeTurn.messageId,
        assistant_text: activeTurn.assistantText,
      };
    }

    if (
      activeTurn &&
      !activeTurn.generating &&
      activeTurn.error &&
      (!requestedId || requestedId === activeTurn.chatId)
    ) {
      return {
        state: "error",
        error: activeTurn.error.code,
        detail: activeTurn.error.detail,
        chat_id: activeTurn.chatId || requestedId,
        generating: false,
      };
    }

    const chatId = requestedId || (activeTurn && activeTurn.chatId) || currentChatIdFromUrl();
    if (!chatId) {
      return {
        state: "settled",
        chat_id: null,
        generating: false,
        message_id: (activeTurn && activeTurn.messageId) || null,
        assistant_text: (activeTurn && activeTurn.assistantText) || "",
      };
    }

    const rd = await readChat({ chat_id: chatId, limit: 10 });
    if (rd.state === "ok" && Array.isArray(rd.messages)) {
      const lastA = [...rd.messages].reverse().find((m) => m.role === "assistant");
      return {
        state: "settled",
        chat_id: chatId,
        generating: false,
        message_id: (lastA && lastA.id) || (activeTurn && activeTurn.messageId) || null,
        assistant_text: (lastA && lastA.text) || (activeTurn && activeTurn.assistantText) || "",
      };
    }
    return {
      state: "settled",
      chat_id: chatId,
      generating: false,
      message_id: (activeTurn && activeTurn.messageId) || null,
      assistant_text: (activeTurn && activeTurn.assistantText) || "",
    };
  }

  async function stopGeneration() {
    if (!activeTurn || !activeTurn.generating) {
      return {
        state: "idle",
        stopped: false,
        chat_id: (activeTurn && activeTurn.chatId) || currentChatIdFromUrl(),
      };
    }
    const turn = activeTurn;
    turn.stopped = true;
    turn.generating = false;
    const client = await ensureCompletionClient();
    if (client) {
      if (turn.requestId) {
        try {
          client.cancelStream(turn.requestId);
        } catch (_) {}
      }
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
    return {
      state: "stopped",
      stopped: true,
      chat_id: turn.chatId,
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
    if (activeTurn && activeTurn.generating) {
      return {
        state: "error",
        error: "generation_already_active",
        detail: "stop or wait for active response",
      };
    }

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
    if (!cachedAccountId) {
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
    }

    let chatId = typeof opts.chat_id === "string" && opts.chat_id.trim() ? opts.chat_id.trim() : null;
    let projectId =
      typeof opts.project_id === "string" && opts.project_id.trim() ? opts.project_id.trim() : null;
    let parentMessageId = newUuid();

    // If continuing an existing conversation, resolve its exact current_node leaf & project.
    if (chatId) {
      const convRes = await apiFetch(`/conversation/${encodeURIComponent(chatId)}`);
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

    let wirePrompt = prompt;
    if (
      !opts.is_continuation &&
      Array.isArray(opts.local_function_signatures) &&
      opts.local_function_signatures.length > 0 &&
      nativeTools.signatures.length === 0
    ) {
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
      nativeToolCalls: new Map(),
      donePromise,
      resolveDone: () => {
        turn.generating = false;
        resolveDone();
      },
    };
    activeTurn = turn;

    const recordMessage = (msg) => {
      if (!msg || typeof msg !== "object") return;
      const nativeCall = extractNativeToolCallFromMessage(
        msg,
        declaredToolNames,
        nativeTools.wireToOriginal
      );
      if (nativeCall) {
        turn.nativeToolCalls.set(nativeCall.id, nativeCall);
        if (msg.id) turn.messageId = msg.id;
        return;
      }
      const extracted = extractTextFromMessage(msg);
      if (extracted && extracted.role === "assistant") {
        turn.assistantText = extracted.text;
        if (extracted.id) turn.messageId = extracted.id;
      }
    };

    try {
      const handle = await client.startCompletionStream({
        request: requestPayload,
        expectedAccountId: cachedAccountId || undefined,
        turnTraceId: newUuid(),
        onUpdate: (update) => {
          if (!update || typeof update !== "object") return;
          if (typeof update.conversationId === "string" && update.conversationId) {
            turn.chatId = update.conversationId;
          }
          if (update.type === "message" && update.message) {
            recordMessage(update.message);
          }
        },
        onDecodedEvent: (ev) => {
          const cid = ev?.update?.conversationId || ev?.payload?.conversation_id;
          if (typeof cid === "string" && cid) {
            turn.chatId = cid;
          }
          const msg = ev?.update?.type === "message" ? ev.update.message : ev?.payload?.message;
          if (msg) {
            recordMessage(msg);
          }
        },
        onComplete: () => {
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
    if (waitMs > 0 && turn.generating) {
      await Promise.race([
        turn.donePromise,
        new Promise((resolve) => setTimeout(resolve, waitMs)),
      ]);
    }

    if (turn.error) {
      return {
        state: "error",
        error: turn.error.code,
        detail: turn.error.detail,
        chat_id: turn.chatId,
      };
    }

    if (turn.generating) {
      return {
        state: "in_progress",
        chat_id: turn.chatId,
        message_id: turn.messageId,
        assistant_text: turn.assistantText,
        tool_calls: [],
        generating: true,
      };
    }

    // If settled with a server conversation_id, zero native tool calls, and empty stream text,
    // fetch only an assistant message strictly AFTER this turn's userMsgId so previous-turn text
    // can never leak into a tool-only or empty turn.
    if (!turn.assistantText && turn.nativeToolCalls.size === 0 && turn.chatId) {
      const rd = await readChat({ chat_id: turn.chatId, limit: 10 });
      if (rd.state === "ok" && Array.isArray(rd.messages)) {
        const userIdx = rd.messages.findIndex((m) => m.id === userMsgId);
        if (userIdx >= 0) {
          const lastA = [...rd.messages.slice(userIdx + 1)]
            .reverse()
            .find((m) => m.role === "assistant");
          if (lastA) {
            turn.assistantText = lastA.text;
            turn.messageId = lastA.id;
          }
        }
      }
    }

    const parsed = parseToolCallsFromText(turn.assistantText || "");
    const nativeCalls = [...turn.nativeToolCalls.values()].map((c) => ({
      id: c.id,
      name: c.name,
      ...(c.namespace ? { namespace: c.namespace } : {}),
      arguments: c.arguments,
    }));
    const allCalls = nativeCalls.length > 0 ? nativeCalls : parsed.calls;
    return {
      state: turn.stopped ? "stopped" : "settled",
      chat_id: turn.chatId,
      message_id: turn.messageId,
      assistant_text: parsed.text,
      tool_calls: allCalls,
      generating: false,
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
    parseToolCallsFromText,
  };
  return { installed: true };
})();
