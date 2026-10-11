// Actual noVNC browser/client proof; no screenshots or credential output.
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
 const page = await browser.newPage({viewport:{width:360,height:740},isMobile:true,hasTouch:true});let socketPathCorrect=false;
 page.on("websocket",ws=>{socketPathCorrect=new URL(ws.url()).pathname==="/"+state.capability+"/websock";});
 await page.goto(encoded.stdout.trim(),{waitUntil:"domcontentloaded",timeout:15000});
 await page.waitForFunction(()=>document.documentElement.classList.contains("noVNC_connected"),{},{timeout:15000});
 await page.waitForTimeout(1500);
 const canvasPresent = await page.locator("#noVNC_container canvas").count()===1;
 await page.locator("#owned_keyboard").click();
 const keyboardFocused=await page.evaluate(()=>document.activeElement?.id==="noVNC_keyboardinput");
 const rendering=await page.locator("#noVNC_container canvas").evaluate(canvas=>{
  const data=canvas.getContext("2d").getImageData(0,0,canvas.width,canvas.height).data;
  let nonBlack=0;const colors=new Set();
  for(let i=0;i<data.length;i+=16){if(data[i]+data[i+1]+data[i+2]>30)nonBlack++;colors.add(((data[i]>>4)<<8)|((data[i+1]>>4)<<4)|(data[i+2]>>4));}
  const box=canvas.getBoundingClientRect();
  return {nonBlackRatio:nonBlack/Math.ceil(data.length/16),colorBuckets:colors.size,visibleWidth:box.width,visibleHeight:box.height};
 });
 const size=await page.locator("#noVNC_container canvas").evaluate(canvas=>({width:canvas.width,height:canvas.height}));
 if(rendering.nonBlackRatio<0.05||rendering.colorBuckets<4)throw Error("Connected viewer did not render desktop content");
 if(!keyboardFocused||size.width>360)throw Error("Mobile keyboard/desktop proof failed");
 if(!socketPathCorrect||!canvasPresent)throw Error("Actual viewer connection proof failed");
 const invalid=await page.evaluate(async()=>{const response=await fetch('./display-proof',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({privateText:'refuse'})});return response.status;});
 if(invalid!==400)throw Error("Display proof accepted non-metric data");
 console.log(JSON.stringify({actualNoVncConnected:true,socketPathCorrect,canvasPresent,keyboardFocused,desktopWidth:size.width,desktopHeight:size.height,rendering}));
} finally {await browser.close();}
