// Actual owned Android browser proof; no account, cookies, or external URL.
import { createHash, randomBytes } from "node:crypto";
import { readFile, realpath, writeFile, stat } from "node:fs/promises";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

const [candidateArg, executableArg, rootArg] = process.argv.slice(2);
if (!candidateArg || !executableArg || !rootArg) throw new Error("candidate, browser and owned root required");
const candidate = await realpath(candidateArg);
const executablePath = await realpath(executableArg);
const root = await realpath(rootArg);
if (process.platform !== "android") throw new Error("Actual Android runtime required");
for (const [key, value] of Object.entries({ HOME: join(root, "h"), CODEX_HOME: join(root, "codex"), TMPDIR: join(root, "t") })) {
  if (process.env[key] !== value) throw new Error("Owned child environment required: " + key);
}
const dependency = join(candidate, "node_modules/playwright-core");
const metadata = JSON.parse(await readFile(join(dependency, "package.json"), "utf8"));
const dependencyHash = createHash("sha256").update(await readFile(join(dependency, "lib/coreBundle.js"))).digest("hex");
if (metadata.version !== "1.62.0" || dependencyHash !== "4952f2e7ddbfe0e8039da98a236d9a2ae66f0cee0c00cee507eb9605a3bf6c3f") throw new Error("Pinned corrected dependency required");
if (!((await stat(executablePath)).mode & 0o111)) throw new Error("Executable browser required");
const { chromium } = await import(pathToFileURL(join(dependency, "index.mjs")).href);
let browser;
let report;
try {
  browser = await chromium.launch({ executablePath, headless: true, timeout: 10000 });
  const context = await browser.newContext();
  const page = await context.newPage();
  const nonce = randomBytes(16).toString("hex");
  await page.setContent('<p id="owned">' + nonce + '</p>');
  for (let i = 0; i < 2; i++) {
    if (await page.locator("#owned").textContent() !== nonce) throw new Error("Actual DOM result mismatch");
  }
  await page.close();
  const next = await context.newPage();
  if (await next.locator("#owned").count() !== 0) throw new Error("Closed page state leaked into new page");
  report = { platform: process.platform, engine: process.version, bun: process.versions.bun ?? null,
    browser: browser.version(), browser_sha256: createHash("sha256").update(await readFile(executablePath)).digest("hex"),
    dependency_sha256: dependencyHash, dom_roundtrips: 2, page_reentry: true,
    nonce_sha256: createHash("sha256").update(nonce).digest("hex"), real_service: false, owned_environment: true };
  await context.close();
} finally {
  await browser?.close();
}
report.browser_closed = true;
await writeFile(join(root, "browser-proof.json"), JSON.stringify(report, null, 2) + "\n", { mode: 0o600 });
console.log(JSON.stringify(report));
