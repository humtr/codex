// Actual noVNC browser/client proof; no screenshots, pixels or credential output.
import { spawnSync } from "node:child_process";
import { dirname, isAbsolute, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { readFile } from "node:fs/promises";
const [candidate, gui] = process.argv.slice(2);
if (!candidate || !gui || !isAbsolute(candidate) || !isAbsolute(gui) || process.platform !== "android") throw Error("Owned absolute roots and native Android required");
const state = JSON.parse(await readFile(join(gui,"viewer.json"),"utf8"));
const source = dirname(fileURLToPath(import.meta.url));
const encoded = spawnSync(join(process.env.PREFIX,"bin/python3"),["-B","-c","import json,sys; from open_viewer import viewer_url; print(viewer_url(json.loads(sys.stdin.read())))"],{cwd:source,input:JSON.stringify(state),encoding:"utf8"});
if (encoded.status !== 0) throw Error("Owned URL builder refused state");
const { chromium } = await import(pathToFileURL(join(candidate,"node_modules/playwright-core/index.mjs")).href);
const browser = await chromium.launch({executablePath:join(process.env.PREFIX,"lib/chromium/chrome"),headless:true,args:["--no-sandbox","--disable-dev-shm-usage"]});
try {
 const page = await browser.newPage();let socketPathCorrect=false;
 page.on("websocket",ws=>{socketPathCorrect=new URL(ws.url()).pathname==="/"+state.capability+"/websock";});
 await page.goto(encoded.stdout.trim(),{waitUntil:"domcontentloaded",timeout:15000});
 await page.waitForFunction(()=>document.documentElement.classList.contains("noVNC_connected"),{},{timeout:15000});
 const canvasPresent = await page.locator("#noVNC_container canvas").count()===1;
 if(!socketPathCorrect||!canvasPresent)throw Error("Actual viewer connection proof failed");
 console.log(JSON.stringify({actualNoVncConnected:true,socketPathCorrect,canvasPresent}));
} finally {await browser.close();}
