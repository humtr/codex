// Owned, temporary mobile viewer. RFB and X11 use private Unix sockets/cookies;
// the loopback HTTP/WebSocket endpoint requires a random capability on every URL.
import { timingSafeEqual, randomBytes } from "node:crypto";
import { createConnection } from "node:net";
import { resolve, sep } from "node:path";
import { chmodSync, writeFileSync } from "node:fs";

const [assets, socketPath, statePath] = process.argv.slice(2);
if (!assets || !socketPath || !statePath) throw Error("assets, private RFB socket and private state path required");
const root=resolve(assets), capability=randomBytes(32).toString("hex");
const headers={"Cache-Control":"no-store", "Referrer-Policy":"no-referrer", "X-Frame-Options":"DENY", "X-Content-Type-Options":"nosniff"};
const server=Bun.serve({
 hostname:"127.0.0.1",port:0,
 async fetch(request,server){
  const url=new URL(request.url), parts=url.pathname.split("/");
  const found=parts[1]||"";
  if(found.length!==capability.length||!timingSafeEqual(Buffer.from(found),Buffer.from(capability)))return new Response("Not found",{status:404,headers});
  const path=parts.slice(2).join("/");
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
