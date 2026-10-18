// Owned, temporary mobile viewer. RFB and X11 use private Unix sockets/cookies;
// the loopback HTTP/WebSocket endpoint requires a random capability on every URL.
import { timingSafeEqual, randomBytes } from "node:crypto";
import { createConnection } from "node:net";
import { resolve, sep } from "node:path";
import { appendFileSync, chmodSync, writeFileSync } from "node:fs";

const [assets, socketPath, statePath] = process.argv.slice(2);
if (!assets || !socketPath || !statePath) throw Error("assets, private RFB socket and private state path required");
const root=resolve(assets), capability=randomBytes(32).toString("hex");
const headers={"Cache-Control":"no-store", "Referrer-Policy":"no-referrer", "X-Frame-Options":"DENY", "X-Content-Type-Options":"nosniff"};
const server=Bun.serve({
 hostname:"127.0.0.1",port:0,
 async fetch(request,server){
  const url=new URL(request.url), parts=url.pathname.split("/");
  const found=parts[1]||"";
  if(!/^[a-f0-9]{64}$/.test(found)||!timingSafeEqual(Buffer.from(found),Buffer.from(capability)))return new Response("Not found",{status:404,headers});
  const path=parts.slice(2).join("/");
  if(path==="display-proof"){
   if(request.method!=="POST"||request.headers.get("origin")!==`http://127.0.0.1:${server.port}`)return new Response("Forbidden",{status:403,headers});
   const raw=await request.text();
   if(raw.length>2048)return new Response("Too large",{status:413,headers});
   let record:Record<string,unknown>;
   try{record=JSON.parse(raw);}catch{return new Response("Bad record",{status:400,headers});}
   const allowed=["connected","secureContext","canvasWidth","canvasHeight","visibleWidth","visibleHeight","nonBlackRatio","colorBuckets","viewportWidth","viewportHeight","keyboardFocused","errorCount","visualViewportHeight","visualViewportOffsetTop","keyboardAutoEnabled","toolbarHeight"];
   if(!record||typeof record!=="object"||Array.isArray(record)||Object.keys(record).some(k=>!allowed.includes(k))||Object.values(record).some(v=>typeof v!=="boolean"&&(typeof v!=="number"||!Number.isFinite(v))))return new Response("Bad record",{status:400,headers});
   // Aggregates only: never save screenshots, text, input, cookies or URLs.
   appendFileSync(statePath+".display-proof.jsonl",JSON.stringify({time:Date.now(),...record})+"\n",{mode:0o600});
   return new Response(null,{status:204,headers});
  }
  if(path==="websock"){
   if(request.headers.get("origin")!==`http://127.0.0.1:${server.port}`)return new Response("Forbidden",{status:403,headers});
   if(server.upgrade(request,{data:{stream:null as ReturnType<typeof createConnection>|null}}))return;
   return new Response("Upgrade required",{status:400,headers});
  }
  let decoded:string;
  try{decoded=decodeURIComponent(path);}catch{return new Response("Bad path",{status:400,headers});}
  if(path==="mobile_controls.js"||path==="mobile_controls.css"){
   const content=Bun.file(new URL("./"+path,import.meta.url));
   return new Response(content,{headers:{...headers,"Content-Type":path.endsWith(".js")?"text/javascript;charset=utf-8":"text/css;charset=utf-8"}});
  }
  const file=resolve(root,decoded);
  if(!file.startsWith(root+sep))return new Response("Not found",{status:404,headers});
  const content=Bun.file(file);
  if(!await content.exists()||content.size>10*1024*1024)return new Response("Not found",{status:404,headers});
  if(path==="vnc.html"){
   const mobile=`<link rel="stylesheet" href="./mobile_controls.css">
<div id="owned_mobile_controls" role="toolbar" aria-label="원격 화면 조작">
 <button id="owned_keyboard" type="button" aria-pressed="false" aria-label="키보드 열기" title="키보드 열기/닫기">
  <svg viewBox="0 0 24 24" aria-hidden="true"><rect x="2" y="5" width="20" height="14" rx="3"/><path d="M6 9h1m3 0h1m3 0h1m3 0h1M6 12h1m3 0h1m3 0h1m3 0h1M7 16h10"/></svg><span>키보드</span>
 </button>
 <button id="owned_keyboard_auto" type="button" role="switch" aria-checked="false" aria-label="터치 시 자동 키보드" title="화면을 터치할 때 키보드 열기"><span class="owned_switch" aria-hidden="true"></span><span>자동</span></button>
 <span class="owned_divider" aria-hidden="true"></span>
 <button id="owned_scroll" type="button" aria-pressed="true" aria-label="한 손가락 스크롤 모드" title="스크롤 / 마우스 드래그"><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 3v18M8 7l4-4 4 4M8 17l4 4 4-4"/></svg></button>
 <button id="owned_escape" type="button" aria-label="취소 (Esc)" title="대화상자 취소">Esc</button>
 <button id="owned_fullscreen" type="button" aria-label="전체 화면" title="전체 화면"><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M8 3H3v5m13-5h5v5M3 16v5h5m13-5v5h-5"/></svg></button>
</div><script type="module" src="./mobile_controls.js"></script>`;
   const html=(await content.text()).replace('</body>',mobile+'</body>');
   return new Response(html,{headers:{...headers,"Content-Type":"text/html;charset=utf-8"}});
  }
  return new Response(content,{headers});
 },
 websocket:{
  open(ws){
   const stream=createConnection({path:socketPath});ws.data.stream=stream;
   stream.on("data",data=>{if(ws.send(data)===-1)stream.pause();});
   stream.on("error",()=>ws.close());stream.on("close",()=>ws.close());
  },
  message(ws,message){ws.data.stream?.write(Buffer.from(message));},
  drain(ws){ws.data.stream?.resume();},
  close(ws){ws.data.stream?.destroy();},
 },
});
writeFileSync(statePath,JSON.stringify({pid:process.pid,port:server.port,capability}),{mode:0o600});chmodSync(statePath,0o600);
// Deliberately print neither capability URLs nor authentication values.
console.log("owned mobile viewer ready");
process.on("SIGTERM",()=>{server.stop(true);process.exit(0);});
