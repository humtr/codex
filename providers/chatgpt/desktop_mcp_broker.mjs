// Ephemeral result binding, never an executor or an alternate history store.
import {createHash,randomBytes} from 'node:crypto';
import {createServer,createConnection} from 'node:net';
import {chmodSync,lstatSync} from 'node:fs';
import {dirname,isAbsolute} from 'node:path';
import {validateEnvelope} from './desktop_protocol.mjs';

const LIMIT=16*1024*1024;
const clone=value=>JSON.parse(JSON.stringify(value));
const fail=code=>{throw Error(code);};
const plain=value=>value!==null&&typeof value==='object'&&!Array.isArray(value);
const stable=value=>JSON.stringify(value,(_key,v)=>plain(v)?Object.fromEntries(Object.keys(v).sort().map(k=>[k,v[k]])):v);
const safeError=error=>/^mcp_[a-z_]+$/.test(error?.message||'')?error.message:'mcp_binding_failed';

export class DesktopMcpBroker{
 constructor(){this.turns=new Map();}
 register(owner,tools){
  if(!plain(owner)||!['backend','thread','turn'].every(k=>typeof owner[k]==='string'&&owner[k]))fail('mcp_native_owner_required');
  if([...this.turns.values()].some(t=>stable(t.owner)===stable(owner)))fail('mcp_native_owner_conflict');
  const token=randomBytes(32).toString('hex');
  this.turns.set(token,{owner:clone(owner),tools:clone(tools),calls:new Map(),active:true});return token;
 }
 get(token){const turn=this.turns.get(token);if(!turn?.active)fail('mcp_turn_unavailable');return turn;}
 catalog(token){return clone(this.get(token).tools);}
 update(token,tools){const turn=this.get(token);if([...turn.calls.values()].some(e=>e.result===undefined))fail('mcp_pending_catalog_change');turn.tools=clone(tools);}
 invoke(token,args,signal){
  const turn=this.get(token);
  if(!plain(args)||typeof args.call_id!=='string'||!/^[a-zA-Z0-9_.:-]{1,160}$/.test(args.call_id)||typeof args.name!=='string'||typeof args.input!=='string'||typeof args.report!=='string')fail('mcp_call_invalid');
  const tool=turn.tools.find(t=>t.wireName===args.name);if(!tool)fail('mcp_unadvertised_tool');
  let input=args.input;if(!tool.freeform)try{input=JSON.parse(input);}catch{fail('mcp_input_invalid');}
  try{validateEnvelope({type:'tool',name:args.name,input},turn.tools);}catch{fail('mcp_input_invalid');}
  const fingerprint=stable({name:args.name,input,report:args.report});let entry=turn.calls.get(args.call_id);
  if(entry&&entry.fingerprint!==fingerprint)fail('mcp_call_changed');
  if(!entry){
   const id='call_mcp_'+createHash('sha256').update(token+'\0'+args.call_id).digest('hex');
   entry={fingerprint,call:{id,name:args.name,input,report:args.report},waiters:new Set(),offered:false,deliveries:0};turn.calls.set(args.call_id,entry);
  }
  signal?.throwIfAborted();
  if(entry.result!==undefined)return Promise.resolve(clone(entry.result));
  return new Promise((resolve,reject)=>{
   const waiter={resolve:value=>{cleanup();resolve(clone(value));},reject:error=>{cleanup();reject(error);}};
   const abort=()=>waiter.reject(signal.reason||Error('mcp_observer_detached'));
   const cleanup=()=>{entry.waiters.delete(waiter);signal?.removeEventListener('abort',abort);};
   entry.waiters.add(waiter);signal?.addEventListener('abort',abort,{once:true});
  });
 }
 pending(token){return [...this.get(token).calls.values()].filter(e=>!e.offered).map(e=>clone(e.call));}
 callId(token,logicalId){const entry=this.get(token).calls.get(logicalId);if(!entry)fail('mcp_call_invalid');return entry.call.id;}
 offered(token,ids){const turn=this.get(token),seen=new Set(),entries=[];for(const id of ids){const e=[...turn.calls.values()].find(e=>e.call.id===id);if(!e||e.offered||seen.has(id))fail('mcp_offer_invalid');seen.add(id);entries.push(e);}for(const entry of entries)entry.offered=true;}
 settle(token,batch){
  const turn=this.get(token),seen=new Set(),entries=[];
  for(const {call,result} of batch){
   const entry=[...turn.calls.values()].find(e=>e.call.id===call.id);
   if(!entry?.offered||seen.has(call.id)||entry.result!==undefined||!plain(result)||!Array.isArray(result.content)&&typeof result.content!=='string')fail('mcp_result_invalid');
   seen.add(call.id);entries.push([entry,clone(result)]);
  }
  for(const [entry,result] of entries){entry.result=result;for(const waiter of [...entry.waiters])waiter.resolve(result);}
 }
 delivered(token,id){const turn=this.get(token),entry=[...turn.calls.values()].find(e=>e.call.id===id);if(!entry||entry.result===undefined)fail('mcp_delivery_invalid');entry.deliveries++;}
 finish(token){const turn=this.get(token);if([...turn.calls.values()].some(e=>e.result===undefined))fail('mcp_pending_completion');this.revoke(token);}
 revoke(token){const turn=this.turns.get(token);if(!turn)return;turn.active=false;for(const entry of turn.calls.values())for(const waiter of [...entry.waiters])waiter.reject(Error('mcp_turn_retired'));this.turns.delete(token);}
 close(){for(const token of [...this.turns.keys()])this.revoke(token);}
}

// One request per private Unix connection. Losing a peer cancels only its waiter.
export async function serveMcpBroker(broker,socketPath){
 const parent=lstatSync(dirname(socketPath));
 if(!isAbsolute(socketPath)||!parent.isDirectory()||parent.isSymbolicLink()||parent.uid!==process.getuid()||parent.mode&0o077||Buffer.byteLength(socketPath)>107)fail('mcp_private_socket_required');
 const peers=new Set();
 const server=createServer(socket=>{
  peers.add(socket);socket.on('error',()=>{});const abort=new AbortController();let chunks=[],size=0,received=false;
  socket.once('close',()=>{peers.delete(socket);abort.abort(Error('mcp_observer_detached'));});
  socket.on('data',chunk=>{
   if(received)return;size+=chunk.length;if(size>LIMIT){socket.destroy();return;}chunks.push(chunk);
   if(!chunk.includes(10))return;received=true;
   void(async()=>{
    let response,token,callId;
    try{
     const row=JSON.parse(new TextDecoder('utf-8',{fatal:true}).decode(Buffer.concat(chunks)));chunks=[];token=row.token;
     if(row.method==='catalog')response={value:broker.catalog(token)};
     else if(row.method==='invoke'){
      const promise=broker.invoke(token,row.args,abort.signal);
      callId=broker.callId(token,row.args.call_id);
      response={value:await promise};
     }else fail('mcp_operation_invalid');
    }catch(error){response={error:safeError(error)};}
    if(!socket.destroyed)socket.end(JSON.stringify(response)+'\n',()=>{if(callId&&!response.error)try{broker.delivered(token,callId);}catch{}});
   })();
  });
 });
 await new Promise((resolve,reject)=>{server.once('error',reject);server.listen(socketPath,resolve);});chmodSync(socketPath,0o600);
 return async()=>{broker.close();for(const peer of peers)peer.destroy();await new Promise(resolve=>server.close(resolve));};
}

export function callMcpBroker(socketPath,row,signal){
 signal?.throwIfAborted();
 return new Promise((resolve,reject)=>{
  const socket=createConnection(socketPath);let chunks=[],size=0,settled=false;
  const finish=(error,value)=>{if(settled)return;settled=true;signal?.removeEventListener('abort',abort);socket.destroy();error?reject(error):resolve(value);};
  const abort=()=>finish(signal.reason||Error('mcp_observer_detached'));
  signal?.addEventListener('abort',abort,{once:true});
  socket.once('connect',()=>socket.write(JSON.stringify(row)+'\n'));
  socket.on('data',chunk=>{size+=chunk.length;if(size>LIMIT)return finish(Error('mcp_result_too_large'));chunks.push(chunk);if(chunk.includes(10))try{const result=JSON.parse(new TextDecoder('utf-8',{fatal:true}).decode(Buffer.concat(chunks)));finish(result.error?Error(safeError(Error(result.error))):null,result.value);}catch{finish(Error('mcp_result_invalid'));}});
  socket.once('error',()=>finish(Error('mcp_connection_unavailable')));
  socket.once('end',()=>{if(!settled)finish(Error('mcp_delivery_unknown'));});
 });
}
