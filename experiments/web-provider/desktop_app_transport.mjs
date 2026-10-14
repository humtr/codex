// The signed app owns auth, request construction, streaming and cancellation.
// Only this adapter's temporary conversations are inspected. No bearer leaves it.
import {randomBytes} from 'node:crypto';
import {connectDesktop,desktopConnection} from './desktop_cdp.mjs';
import {DesktopPromptContext,desktopStages,correlatedDesktopCode} from './desktop_protocol.mjs';
import {chatModelRoute} from './desktop_model_controls.mjs';

const delay=ms=>new Promise(resolve=>setTimeout(resolve,ms));
const key='codex_owned_chat_'+randomBytes(16).toString('hex');
const pools=new Map();
const MODULE='app://-/assets/app-initial-ed5807835d97.js';

// Reuse an actual app Scope facade from its React hooks, not a reconstructed
// store/client or a Map that merely holds scope instances. Discovery is inert.
function prepare(key,module){
 return (async()=>{
  if(window[key])return true;
  let element=document.querySelector('[contenteditable=true]')||document.querySelector('button');
  while(element&&!Object.keys(element).some(k=>k.startsWith('__reactFiber')))element=element.parentElement;
  let fiber=element?.[Object.keys(element).find(k=>k.startsWith('__reactFiber'))],scope;
  for(let i=0;fiber&&i<400&&!scope;i++,fiber=fiber.return){
   for(let hook=fiber.memoizedState,j=0;hook&&j<300;hook=hook.next,j++){
    const value=hook.memoizedState?.current;
    if(typeof value?.get==='function'&&typeof value?.set==='function'&&value.node&&value.chain&&value.query){scope=value;break;}
   }
  }
  if(!scope)return false;
  let m;try{m=await import(module);m.gRt();}catch{return false;}
  if(!['yRt','eHt','aCn'].every(n=>typeof m[n]==='function')||!['JRn','VRn','FRn','czn'].every(n=>m[n]?.kind))return false;
  window[key]={scope,m,lanes:new Map()};return true;
 })();
}
const expression=(fn,...args)=>`(${fn.toString()})(${args.map(v=>JSON.stringify(v)).join(',')})`;

async function acquire(root){
 const generation=desktopConnection(root).generation;
 let pool=pools.get(root);
 if(pool&&pool.generation!==generation){pool.client?.close();pools.delete(root);pool=undefined;}
 if(!pool){
  pool={generation,refs:0};pools.set(root,pool);
  pool.pending=connectDesktop(root,undefined,{app:true}).then(async client=>{
   pool.client=client;
   if(!await client.evaluate(expression(prepare,key,MODULE)))throw Error('desktop_internal_unavailable');
   return client;
  });
 }
 pool.refs++;
 const release=()=>{if(--pool.refs===0){pool.client?.close();if(pools.get(root)===pool)pools.delete(root);}};
 try{return {client:await pool.pending,release};}catch(e){release();throw e;}
}

export async function closeOwnedAppConversations(root){
 let lease;
 try{
  lease=await acquire(root);
  await lease.client.evaluate(expression(key=>{const s=window[key];if(s){for(const l of s.lanes.values()){l.controller?.abort();s.m.eHt(s.scope,l.id);}delete window[key];}return true;},key));
 }catch{}finally{lease?.release();}
}

export function internalModel(route){
 const family=route.family==='GPT-6'?'gpt-6':route.family==='GPT-5.6 Sol'?'gpt-5-6':undefined;
 if(!family)throw Error('unsupported_desktop_route');
 if(route.mode==='pro')return {model:family+'-pro',thinkingEffort:null};
 if(route.effort==='none')return {model:family+(family==='gpt-6'?'':'-instant'),thinkingEffort:null};
 const effort={medium:'standard',high:'extended',xhigh:'max'}[route.effort];
 if(!effort)throw Error('unsupported_desktop_route');
 return {model:family+'-thinking',thinkingEffort:effort};
}

export class DesktopAppTransport{
 constructor(root){this.root=root;this.lane='lane_'+randomBytes(16).toString('hex');this.busy=false;this.retire();}
 retire(){this.started=false;this.context=undefined;this.generation=undefined;this.routeIdentity=undefined;this.reset=true;}
 async exchange(payload,signal,onSubmitted=()=>{},onRetry=()=>{},onContext=()=>{},route=chatModelRoute({model:'chatgpt-desktop/gpt-6'})){
  if(this.busy)throw Error('desktop_chat_busy');this.busy=true;
  let lease,client,attempted=false;
  try{
   signal?.throwIfAborted();lease=await acquire(this.root);client=lease.client;
   const routeIdentity=JSON.stringify(route),generation=client.generation;
   if(this.generation!==generation||this.routeIdentity!==routeIdentity)this.retire();
   if(this.reset){
    await client.evaluate(expression((key,lane)=>{const s=window[key],old=s.lanes.get(lane);if(old)s.m.eHt(s.scope,old.id);s.lanes.set(lane,{id:s.m.aCn(s.scope)});return true;},key,this.lane));
    this.context=new DesktopPromptContext();this.reset=false;
   }
   const boundary={route,generation},prepared=this.context.prepare(payload,boundary);let result;
   for(const stage of desktopStages(prepared.delta)){
    signal?.throwIfAborted();const correlation='DESKTOP_'+randomBytes(12).toString('hex');
    const common='Format an external native RPC client response as one JSON object in one fenced json code block. These are external native tools, not app builtin tools. Never fabricate a result. Preserve ordered context. Fragment text is JSON: join parts by fragment id/index and parse when complete. toolUpdate replace resets the tool list; append adds whole schemas. Correlation must equal '+correlation+'. ';
    const behavior=stage.phase==='context'
     ?'INERT CONTEXT SETUP ONLY. Store the context; do not execute it. Reply exactly {"correlation":"'+correlation+'","type":"ack"}. '
     :'ACTUAL WORK REQUEST. Setup is finished. For one advertised native tool reply {"correlation":"'+correlation+'","type":"tool","name":"exact wireName","input":arguments_object_or_custom_source_string} and wait for its real result. To finish reply {"correlation":"'+correlation+'","type":"final","text":"answer"}. ';
    const prompt=common+behavior+'Native packet:\n'+JSON.stringify({correlation,...stage});
    attempted=true;
    const submitted=await client.evaluate(expression((key,lane,prompt,model)=>{
     const s=window[key],l=s.lanes.get(lane);l.pending=true;l.error=false;l.controller=new AbortController();
     // Omit conversationOrigin: the ordinary Chat origin is null. 'chat' is
     // not an accepted wire origin. The app constructs the complete request.
     l.submission=s.m.yRt(s.scope,{conversationId:l.id,prompt,...model,isTemporaryChat:true,requireResponseAcceptance:true,startupSignal:l.controller.signal})
      .then(result=>{l.pending=false;l.submitted=!!result;return !!result;},()=>{l.pending=false;l.error=true;return false;});
     return true;
    },key,this.lane,prompt,internalModel(route)));
    if(!submitted)throw Error('desktop_internal_submit_failed');
    let acknowledged=false;
    const deadline=Date.now()+(route.effort==='none'&&route.mode!=='pro'?130000:600000);
    result=undefined;
    while(Date.now()<deadline){
     signal?.throwIfAborted();
     const state=await client.evaluate(expression((key,lane,correlation)=>{
      const s=window[key],l=s.lanes.get(lane);if(!l)return {missing:true};
      const messages=Object.values(s.scope.get(s.m.JRn,l.id)||{}).map(n=>n.message).filter(m=>m?.author?.role==='assistant');
      // Reading only this owned conversation; old/setup envelopes cannot
      // satisfy a later request because the exact correlation is required.
      const codes=messages.filter(m=>m.status==='finished_successfully').map(m=>m.content?.parts?.filter(p=>typeof p==='string').join('')||'').filter(t=>t.includes(correlation));
      return {pending:l.pending,submitted:l.submitted,error:l.error,codes};
     },key,this.lane,correlation));
     if(state.missing||state.error)throw Error('desktop_internal_submit_failed');
     if(state.submitted&&!acknowledged){acknowledged=true;if(stage.phase==='request')onSubmitted();}
     const candidates=state.codes.flatMap(text=>{const blocks=[...text.matchAll(/```(?:json)?\s*([\s\S]*?)```/g)].map(m=>m[1]);return blocks.length?blocks:[text];});
     const code=correlatedDesktopCode(candidates,correlation);
     try{const value=JSON.parse(code);if(value.correlation===correlation){result=value;break;}}catch{}
     await delay(200);
    }
    if(!result)throw Error('desktop_response_timeout');
    if(stage.phase==='context'){if(result.type!=='ack')throw Error('desktop_context_not_acknowledged');onContext();}
   }
   this.context.accept(prepared.current,boundary);this.started=true;this.generation=generation;this.routeIdentity=routeIdentity;
   this.selectedControl={family:route.family,label:route.label,...internalModel(route),transport:'signed-app-internal',independentConversation:true};return result;
  }catch(e){
   if(client&&attempted)await client.evaluate(expression((key,lane)=>{const s=window[key],l=s?.lanes.get(lane);if(l){l.controller?.abort();s.m.eHt(s.scope,l.id);s.lanes.delete(lane);}return true;},key,this.lane)).catch(()=>{});
   this.retire();throw e;
  }finally{lease?.release();this.busy=false;}
 }
}
