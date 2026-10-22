// Qualification transport only: one existing host socket, bounded independent clients.
// It never launches/stops an app, reads credentials from a page or submits inference.
import {readFileSync,writeFileSync,realpathSync} from 'node:fs';
import {randomBytes,timingSafeEqual} from 'node:crypto';
import {desktopConnection} from '../providers/chatgpt/desktop_cdp.mjs';

export async function relay(descriptor,outputDirectory){
 const upstream=new WebSocket(descriptor.endpoint.replace('http:','ws:').replace(/\/$/,'')+'/cdp',{headers:{Authorization:'Bearer '+descriptor.token}});
 await new Promise((ok,no)=>{const timer=setTimeout(()=>{upstream.close();no(Error('relay_upstream_timeout'));},10000);upstream.onopen=()=>{clearTimeout(timer);ok();};upstream.onerror=()=>{clearTimeout(timer);no(Error('relay_upstream_unavailable'));};});
 const token=randomBytes(32).toString('base64url'),peers=new Set(),pending=new Map(),sessions=new Map();
 let sequence=0,closing=false;
 const fail=(ws,id,message)=>ws.send(JSON.stringify({id,error:{code:-32000,message}}));
 const methods=new Set(['Target.getTargets','Target.attachToTarget','Target.detachFromTarget','Runtime.evaluate']);
 const server=Bun.serve({hostname:'127.0.0.1',port:0,idleTimeout:0,
  fetch(req,server){
   const supplied=Buffer.from(req.headers.get('Authorization')??''),expected=Buffer.from('Bearer '+token);
   if(req.headers.has('Origin')||supplied.length!==expected.length||!timingSafeEqual(supplied,expected))return new Response('Unauthorized',{status:401});
   if(closing)return new Response('Closed',{status:503});
   if(new URL(req.url).pathname!=='/cdp')return new Response('Not found',{status:404});
   if(peers.size>=4)return new Response('Full',{status:429});
   if(server.upgrade(req))return;
   return new Response('WebSocket required',{status:426});
  },websocket:{maxPayloadLength:16*1024*1024,
   open(ws){peers.add(ws);},
   message(ws,raw){
    let value;try{value=JSON.parse(String(raw));}catch{ws.close(1008);return;}
    if(!Number.isSafeInteger(value.id)||!methods.has(value.method)){fail(ws,value.id,'relay_method_refused');return;}
    const session=value.sessionId??(value.method==='Target.detachFromTarget'?value.params?.sessionId:undefined);
    if(session&&sessions.get(session)!==ws){fail(ws,value.id,'relay_session_not_owned');return;}
    if(pending.size>=128){fail(ws,value.id,'relay_queue_full');return;}
    const id=++sequence;
    const timer=setTimeout(()=>{pending.delete(id);if(peers.has(ws))fail(ws,value.id,'relay_delivery_unknown_no_replay');},30000);
    pending.set(id,{ws,id:value.id,method:value.method,timer});
    upstream.send(JSON.stringify({...value,id}));
   },
   close(ws){
    peers.delete(ws);
    for(const [id,p]of pending)if(p.ws===ws){clearTimeout(p.timer);pending.delete(id);}
    for(const [session,owner]of sessions)if(owner===ws){sessions.delete(session);if(!closing)upstream.send(JSON.stringify({id:++sequence,method:'Target.detachFromTarget',params:{sessionId:session}}));}
   }
  }});
 upstream.onmessage=event=>{
  let value;try{value=JSON.parse(String(event.data));}catch{void close();return;}
  if(value.id!==undefined){
   const p=pending.get(value.id);if(!p)return;pending.delete(value.id);clearTimeout(p.timer);
   if(p.method==='Target.attachToTarget'&&typeof value.result?.sessionId==='string')sessions.set(value.result.sessionId,p.ws);
   if(peers.has(p.ws))p.ws.send(JSON.stringify({...value,id:p.id}));
  }else if(value.sessionId){
   const owner=sessions.get(value.sessionId);
   if(owner&&peers.has(owner))owner.send(JSON.stringify(value));
  }
 };
 upstream.onclose=()=>void close();
 async function close(){
  if(closing)return;closing=true;
  for(const p of pending.values())clearTimeout(p.timer);pending.clear();
  for(const ws of peers)ws.close();peers.clear();
  server.stop(true);upstream.close();
 }
 if(outputDirectory){
  const stat=readFileSync('/proc/self/stat','utf8'),fields=stat.slice(stat.lastIndexOf(')')+1).trim().split(/\s+/);
  const host={pid:process.pid,start:fields[19],image:realpathSync('/proc/self/exe')};
  writeFileSync(outputDirectory+'/desktop-host.json',JSON.stringify({...descriptor,endpoint:'http://127.0.0.1:'+server.port,token,host}),{mode:0o600,flag:'wx'});
 }
 return {endpoint:'ws://127.0.0.1:'+server.port+'/cdp',token,close};
}
if(import.meta.main){
 const [source,directory]=process.argv.slice(2),{descriptor}=desktopConnection(source);
 await relay(descriptor,directory);
 process.once('SIGTERM',()=>process.exit(0));process.once('SIGINT',()=>process.exit(0));
 console.log(JSON.stringify({ready:true,model_requests:0,shared_host_restarted:false}));
}
