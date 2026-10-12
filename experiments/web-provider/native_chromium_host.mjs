// Real owned Chromium over private CDP pipes. No authentication/debug port is public.
import {spawn} from 'node:child_process';
import {readFileSync,writeFileSync,lstatSync,realpathSync,mkdirSync,unlinkSync,mkdtempSync,chmodSync,rmSync} from 'node:fs';
import {isAbsolute,join,dirname} from 'node:path';
import {fileURLToPath} from 'node:url';
import {randomBytes,timingSafeEqual} from 'node:crypto';

const [profilePath,descriptorPath,restoreSession]=process.argv.slice(2);
if(restoreSession&&restoreSession!=="--restore-session")throw Error("Invalid native startup mode");
if(process.platform!=='android'||![profilePath,descriptorPath].every(p=>p&&isAbsolute(p)))throw Error('Native Android and absolute owned paths required');
mkdirSync(profilePath,{recursive:true,mode:0o700});
const profile=lstatSync(profilePath);
if(!profile.isDirectory()||profile.isSymbolicLink()||profile.uid!==process.getuid()||(profile.mode&0o777)!==0o700)throw Error('Private owned browser profile required');
const browserImage=realpathSync(join(process.env.PREFIX,'lib/chromium/chrome'));
const token=randomBytes(32).toString('base64url');
// Android sun_path is bounded: Chrome appends its singleton socket suffix to TMPDIR.
const browserTemporary=mkdtempSync(join(process.env.PREFIX,'tmp','cp-'));
chmodSync(browserTemporary,0o700);
const host=spawn(browserImage,['--user-data-dir='+profilePath,'--remote-debugging-pipe','--no-sandbox','--disable-dev-shm-usage','--no-first-run','--no-default-browser-check','--disable-background-mode','--window-position=0,0',...(restoreSession?['--restore-last-session']:['about:blank'])],{env:{...process.env,TMPDIR:browserTemporary},stdio:['ignore','ignore','ignore','pipe','pipe']});
if(!host.stdio[3]?.writable||!host.stdio[4]?.readable)throw Error('Chromium private FD3/FD4 transport unavailable');
let sequence=0,buffer=Buffer.alloc(0),peer,upgrading=false,closing=false,server;
const pending=new Map();
const sessions=new Set();
const networkMetadata=[];
let connectionCleanup=Promise.resolve();
function send(message){host.stdio[3].write(JSON.stringify(message)+'\0');}
function request(method,params={},timeout=5000){
 const id=++sequence;
 return new Promise((resolve,reject)=>{
  const timer=setTimeout(()=>{pending.delete(id);reject(Error('Owned Chromium command timed out'));},timeout);
  pending.set(id,{resolve,reject,timer});send({id,method,params});
 });
}
host.stdio[4].on('data',chunk=>{
 buffer=Buffer.concat([buffer,chunk]);
 if(buffer.length>16*1024*1024){void shutdown();return;}
 for(;;){
  const end=buffer.indexOf(0);if(end<0)return;
  const frame=buffer.subarray(0,end);buffer=buffer.subarray(end+1);if(!frame.length)continue;
  let message;try{message=JSON.parse(frame.toString('utf8'));}catch{console.error('Chromium pipe invalid-frame bytes:',frame.length);void shutdown();return;}
  if(message.method==='Network.responseReceived'){
   const response=message.params?.response;
   if(response){
    let kind=message.params.type==='Document'?'document':undefined;
    if(!kind){try{const path=new URL(response.url).pathname;if(path.includes('/conversation'))kind='conversation';else if(path.includes('/plugins')||path.includes('/apps'))kind='connector';}catch{}}
    if(kind){networkMetadata.push({kind,status:response.status});if(networkMetadata.length>24)networkMetadata.shift();writeFileSync(descriptorPath+'.network',JSON.stringify(networkMetadata),{mode:0o600});}
   }
  }
  if(message.method==='Target.attachedToTarget'&&message.params?.sessionId)sessions.add(message.params.sessionId);
  if(message.method==='Target.detachedFromTarget')sessions.delete(message.params?.sessionId);
  const owner=message.id!==undefined?pending.get(message.id):undefined;
  if(owner){
   pending.delete(message.id);clearTimeout(owner.timer);
   if(owner.peer&&message.result?.sessionId)sessions.add(message.result.sessionId);
   if(owner.peer){if(owner.peer===peer)owner.peer.send(JSON.stringify({...message,id:owner.clientId}));}
   else if(message.error)owner.reject(Error('Owned Chromium rejected its bootstrap command'));
   else owner.resolve(message.result);
  }else if(message.id===undefined&&peer){peer.send(JSON.stringify(message));}
 }
});
function processIdentity(pid){
 const directory='/proc/'+pid;
 const fields=readFileSync(directory+'/stat','utf8').slice(readFileSync(directory+'/stat','utf8').lastIndexOf(')')+1).trim().split(/\s+/);
 if(fields[0]==='Z')throw Error('Owned process exited');
 return {pid,start:fields[19],image:realpathSync(directory+'/exe')};
}
function authorized(req){
 if(req.headers.has('origin'))return false;
 const supplied=Buffer.from(req.headers.get('authorization')??'');
 const expected=Buffer.from('Bearer '+token);
 return supplied.length===expected.length&&timingSafeEqual(supplied,expected);
}
async function shutdown(){
 if(closing)return;closing=true;
 if(peer){peer.close();peer=undefined;}
 try{await request('Browser.close',{},3000);}catch{}
 if(host.exitCode===null&&host.signalCode===null){host.kill('SIGTERM');await new Promise(resolve=>{const timer=setTimeout(resolve,3000);host.once('exit',()=>{clearTimeout(timer);resolve();});});}
 if(host.exitCode===null&&host.signalCode===null){host.kill('SIGKILL');await new Promise(resolve=>host.once('exit',resolve));}
 for(const owner of pending.values()){clearTimeout(owner.timer);if(!owner.peer)owner.reject(Error('Owned Chromium closed'));}pending.clear();
 await server?.stop(true);
 rmSync(browserTemporary,{recursive:true,force:true});
 try{
  const current=JSON.parse(readFileSync(descriptorPath,'utf8'));
  if(current.host.pid===process.pid&&current.token===token)unlinkSync(descriptorPath);
 }catch{}
}
process.once('SIGTERM',()=>void shutdown());process.once('SIGINT',()=>void shutdown());
host.once('exit',()=>{if(!closing)void shutdown();});
let bootstrapStage="browser-version";
try{
 const version=await request('Browser.getVersion',{},15000);
 bootstrapStage="relay-server";
 server=Bun.serve({hostname:'127.0.0.1',port:0,idleTimeout:0,websocket:{
  maxPayloadLength:8*1024*1024,
  open(ws){upgrading=false;if(closing){ws.close();return;}peer=ws;},
  message(ws,data){
   if(ws!==peer)return;
   let message;try{message=JSON.parse(typeof data==='string'?data:Buffer.from(data).toString('utf8'));}catch{ws.close(1008);return;}
   if(!Number.isSafeInteger(message.id)||message.id<0||typeof message.method!=='string'||message.method.length>128){ws.close(1008);return;}
   const id=++sequence;
   pending.set(id,{peer:ws,clientId:message.id});send({...message,id});
  },
  close(ws){
   if(ws!==peer)return;peer=undefined;
   for(const [id,owner]of pending)if(owner.peer===ws)pending.delete(id);
   const retiredSessions=[...sessions];sessions.clear();
   connectionCleanup=(async()=>{
    // A pipe has one long-lived root session; each WebSocket must release its
    // auto-attach state, including paused service workers, before replacement.
    await request('Target.setAutoAttach',{autoAttach:false,waitForDebuggerOnStart:false,flatten:true}).catch(()=>{});
    await request('Target.setDiscoverTargets',{discover:false}).catch(()=>{});
    await Promise.allSettled(retiredSessions.map(sessionId=>request('Target.detachFromTarget',{sessionId})));
   })();
  },
 },async fetch(req,s){
  if(!authorized(req))return new Response('Unauthorized',{status:401});
  if(closing)return new Response('Closed',{status:503});
  const path=new URL(req.url).pathname.replace(/\/$/,'');
  if(path==='/json/version')return Response.json({'Browser':version.product,'Protocol-Version':version.protocolVersion,'webSocketDebuggerUrl':'ws://127.0.0.1:'+s.port+'/cdp'});
  if(path==='/cdp'){
   await connectionCleanup;
   if(peer||upgrading)return new Response('Owned connection in use',{status:409});
   upgrading=true;
   if(s.upgrade(req,{data:{}}))return;
   upgrading=false;
   return new Response('WebSocket required',{status:426});
  }
  return new Response('Not found',{status:404});
 }});
 bootstrapStage="process-identity";
 const descriptor={version:1,kind:'termux-chromium-pipe',endpoint:'http://127.0.0.1:'+server.port,token,
  host:processIdentity(process.pid),browser:processIdentity(host.pid),profile:realpathSync(profilePath)};
 bootstrapStage="descriptor-publication";
 writeFileSync(descriptorPath,JSON.stringify(descriptor),{mode:0o600,flag:'wx'});
}catch(error){console.error('Native bootstrap failure:',bootstrapStage,error?.code??'unknown',String(error?.message??'').replaceAll(token,'[token]').replace(/\/[^ \n,;]+/g,'[path]').slice(0,200));await shutdown();throw Error('Native owned Chromium host bootstrap failed');}
