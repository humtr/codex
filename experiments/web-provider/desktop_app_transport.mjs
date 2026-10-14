// The signed app owns auth, request construction, streaming and cancellation.
// Only this adapter's temporary conversations are inspected. No bearer leaves it.
import {randomBytes} from 'node:crypto';
import {connectDesktop,desktopConnection} from './desktop_cdp.mjs';
import {DesktopPromptContext,desktopStages,desktopPrompt,correlatedDesktopCode} from './desktop_protocol.mjs';
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

export function linkOwnedReply(key,lane,parent,correlation){
 const s=window[key],l=s?.lanes.get(lane),m=l&&Object.values(s.scope.get(s.m.JRn,l.id)||{}).find(n=>n.message?.id===parent)?.message;
 if(!m||m.author?.role!=='assistant'||m.status!=='finished_successfully'||!m.content?.parts?.some(p=>typeof p==='string'&&p.includes(correlation)))return false;
 l.parent=parent;return true;
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
    const prompt=desktopPrompt(stage,correlation);
    attempted=true;
    const submitted=await client.evaluate(expression((key,lane,prompt,model)=>{
     const s=window[key],l=s.lanes.get(lane);l.pending=true;l.error=false;l.controller=new AbortController();
     // Omit conversationOrigin: the ordinary Chat origin is null. 'chat' is
     // not an accepted wire origin. The app constructs the complete request.
     l.submission=s.m.yRt(s.scope,{conversationId:l.id,parentMessageId:l.parent,prompt,...model,isTemporaryChat:true,requireResponseAcceptance:true,startupSignal:l.controller.signal})
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
      const replies=messages.filter(m=>m.status==='finished_successfully').map(m=>({id:m.id,text:m.content?.parts?.filter(p=>typeof p==='string').join('')||''})).filter(m=>m.text.includes(correlation));
      return {pending:l.pending,submitted:l.submitted,error:l.error,codes:replies.map(m=>m.text),parents:replies.map(m=>m.id)};
     },key,this.lane,correlation));
     if(state.missing||state.error)throw Error('desktop_internal_submit_failed');
     if(state.submitted&&!acknowledged){acknowledged=true;if(stage.phase==='request')onSubmitted();}
     const candidates=state.codes.flatMap(text=>{const blocks=[...text.matchAll(/```(?:json)?\s*([\s\S]*?)```/g)].map(m=>m[1]);return blocks.length?blocks:[text];});
     const code=correlatedDesktopCode(candidates,correlation);
     let value;try{value=JSON.parse(code);}catch{}
     if(value?.correlation===correlation){
      const parent=state.parents[state.codes.findIndex(text=>text.includes(code))];
      const linked=await client.evaluate(expression(linkOwnedReply,key,this.lane,parent,correlation));
      if(!linked)throw Error('desktop_internal_parent_missing');
      result=value;break;
     }
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
