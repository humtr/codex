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
   const allowed=["connected","secureContext","canvasWidth","canvasHeight","visibleWidth","visibleHeight","nonBlackRatio","colorBuckets","viewportWidth","viewportHeight","keyboardFocused","errorCount"];
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
  const file=resolve(root,decoded);
  if(!file.startsWith(root+sep))return new Response("Not found",{status:404,headers});
  const content=Bun.file(file);
  if(!await content.exists()||content.size>10*1024*1024)return new Response("Not found",{status:404,headers});
  if(path==="vnc.html"){
   const mobile=`<style>
#noVNC_container{height:calc(100dvh - 60px)!important}
#owned_mobile_controls{position:fixed;bottom:0;left:0;right:0;min-height:54px;padding:3px;display:flex;align-items:center;justify-content:center;gap:8px;background:#202124;color:white;z-index:9999;font:14px system-ui}
#owned_mobile_controls button{min-height:44px;padding:8px 14px;background:#e8eaed;color:#202124;border:0;border-radius:8px;font:16px system-ui}
</style><div id="owned_mobile_controls"><button id="owned_keyboard" aria-pressed="true">키보드 자동: 켬</button><button id="owned_fullscreen">전체 화면</button></div>
<script type="module">
import UI from './app/ui.js';
let keyboardAutoEnabled=true;let touchStart=null;let touchProofCount=0;
const keyboardButton=document.getElementById('owned_keyboard');
function focusKeyboard(){
 const input=document.getElementById('noVNC_keyboardinput');
 // Android Back may dismiss IME while DOM focus stays on this textarea.
 // Re-focus inside the trusted gesture even when noVNC thinks it is focused.
 input.blur();UI.showVirtualKeyboard();
}
function manualKeyboard(){
 keyboardAutoEnabled=!keyboardAutoEnabled;
 keyboardButton.setAttribute('aria-pressed',String(keyboardAutoEnabled));
 keyboardButton.textContent=keyboardAutoEnabled?'키보드 자동: 켬':'키보드 자동: 끔';
 if(keyboardAutoEnabled)focusKeyboard();else UI.hideVirtualKeyboard();
}
keyboardButton.onclick=manualKeyboard;
const originalKeyboardButton=document.getElementById('noVNC_keyboard_button');
originalKeyboardButton.removeEventListener('click',UI.toggleVirtualKeyboard);
originalKeyboardButton.addEventListener('click',manualKeyboard);
const container=document.getElementById('noVNC_container');
container.addEventListener('pointerdown',event=>{
 if(keyboardAutoEnabled&&event.pointerType==='touch'&&event.isPrimary&&event.target.tagName==='CANVAS')touchStart={id:event.pointerId,x:event.clientX,y:event.clientY};
 else touchStart=null;
},true);
container.addEventListener('pointercancel',()=>{touchStart=null;},true);
container.addEventListener('pointerup',event=>{
 const start=touchStart;touchStart=null;
 if(!keyboardAutoEnabled||!start||start.id!==event.pointerId||Math.hypot(event.clientX-start.x,event.clientY-start.y)>12||event.target.tagName!=='CANVAS'||!document.documentElement.classList.contains('noVNC_connected'))return;
 // Preserve noVNC's remote tap; mouse, scroll and multi-touch do not open IME.
 focusKeyboard();if(touchProofCount++<3)setTimeout(displayProof,500);
},true);
let errorCount=0;window.addEventListener('error',()=>errorCount++);window.addEventListener('unhandledrejection',()=>errorCount++);
async function displayProof(){
 const canvas=document.querySelector('#noVNC_container canvas');
 const record={connected:document.documentElement.classList.contains('noVNC_connected'),secureContext:window.isSecureContext,viewportWidth:window.innerWidth,viewportHeight:window.innerHeight,keyboardFocused:document.activeElement?.id==='noVNC_keyboardinput',errorCount};
 if(canvas){
  const box=canvas.getBoundingClientRect();Object.assign(record,{canvasWidth:canvas.width,canvasHeight:canvas.height,visibleWidth:box.width,visibleHeight:box.height});
  try{const data=canvas.getContext('2d').getImageData(0,0,canvas.width,canvas.height).data;let lit=0;const colors=new Set();
   for(let i=0;i<data.length;i+=16){if(data[i]+data[i+1]+data[i+2]>30)lit++;colors.add(((data[i]>>4)<<8)|((data[i+1]>>4)<<4)|(data[i+2]>>4));}
   Object.assign(record,{nonBlackRatio:lit/Math.max(1,Math.ceil(data.length/16)),colorBuckets:colors.size});
  }catch{record.errorCount++;}
 }
 await fetch('./display-proof',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(record)}).catch(()=>{});
}
for(const delay of [1500,5000,12000])setTimeout(displayProof,delay);
document.getElementById('owned_keyboard').addEventListener('click',()=>setTimeout(displayProof,500));
document.getElementById('owned_fullscreen').onclick=()=>{if(!document.fullscreenElement)document.documentElement.requestFullscreen?.();else document.exitFullscreen?.();};
</script>`;
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
