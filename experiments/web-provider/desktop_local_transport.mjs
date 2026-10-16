// App local functions are a wire transport; all tool execution stays in Core.
import {randomBytes} from 'node:crypto';
import {acquireDesktopApp,internalModel} from './desktop_app_transport.mjs';
import {DesktopPromptContext} from './desktop_protocol.mjs';
import {DesktopToolSession,localToolSignature,localToolCatalog} from './desktop_tool_session.mjs';

const expression=(fn,...args)=>`(${fn.toString()})(${args.map(x=>JSON.stringify(x)??'undefined').join(',')},(${installLocalAppHooks.toString()}))`;
const delay=ms=>new Promise(r=>setTimeout(r,ms));
export function pendingNativeContext(messages,batch){
 if(!batch.length)return [];
 const ids=new Set(batch.map(x=>x.call.id));
 const end=messages.reduce((last,m,i)=>m.role==='toolResult'&&ids.has(m.toolCallId)?i:last,-1);
 const context=[];
 for(const [index,message] of messages.entries()){
  if(!['user','developer','agentMessage'].includes(message.role))continue;
  if(message.role==='agentMessage'||index<end||end<0)throw Error('desktop_native_pending_context_order');
  context.push(message);
 }
 return context;
}
export async function invokeLocalOperation(lease,lane,method,data){
 const code=expression(localAppOperation,lease.scopeKey,lane,method,data);
 const allowed=['desktop_internal_unavailable','desktop_native_capability_unavailable','desktop_native_declaration_unavailable','desktop_native_owner_conflict','desktop_native_owner_missing','desktop_native_operation_invalid','desktop_native_trial_budget_invalid','desktop_native_trial_budget_exhausted','desktop_native_dispatch_replayed','desktop_native_dispatch_invalid'];
 const result=await lease.client.evaluate(`(async()=>{try{return {value:await ${code}};}catch(error){return {error:${JSON.stringify(allowed)}.includes(error?.message)?error.message:'desktop_native_operation_failed'};}})()`);
 if(result.error)throw Error(result.error);return result.value;
}

export function installLocalAppHooks(s,shared){
   const client=s.scope.get(s.m.oMn),api=shared.$wt;
   if(typeof client?.startCompletionStream!=='function'||typeof api?.streamPost!=='function')throw Error('desktop_native_capability_unavailable');
   const state=s.native={client,api,lanes:new Map(),saved:{},wrapped:{}};
   const owner=request=>{
    const lanes=[...state.lanes.values()];
    const exact=lanes.find(l=>request?.conversation_id!=null&&[l.id,l.serverId].includes(request.conversation_id));
    if(exact)return exact;
    const declared=lanes.find(l=>Array.isArray(request?.local_function_signatures)&&request.local_function_signatures.some(f=>f.name===l.signature.name));
    if(declared)return declared;
    return lanes.find(l=>!l.serverId&&Array.isArray(request?.messages)&&request.messages.some(m=>m.author?.role==='user'&&Array.isArray(m.content?.parts)&&m.content.parts.some(p=>typeof p==='string'&&p.startsWith(l.marker+'\n'))));
   };
   for(const name of ['prepareCompletionStream','startCompletionStream']){
    state.saved[name]=client[name];
    state.wrapped[name]=function(...args){
     const request=name==='startCompletionStream'?args[0]?.request:args[0],l=owner(request);
     if(!l)return state.saved[name].apply(this,args);
     // The actual host gate must still advertise its native template; never bypass it.
     const template=request.local_function_signatures?.some(f=>f.name==='handoff');
     const prepared=l.declarationVerified&&request.local_function_signatures?.length===1&&JSON.stringify(request.local_function_signatures[0])===JSON.stringify(l.signature);
     if(!s.scope.get(shared.HAt)||!template&&!prepared)throw Error('desktop_native_declaration_unavailable');
     if(template)l.declarationVerified=true;
     // This owned task exposes Core's catalog, not a second Work/Codex executor.
     const augmented={...request,local_function_signatures:[l.signature]};
     if(name==='prepareCompletionStream')return state.saved[name].call(this,augmented,args[1]);
     const opts={...args[0],request:augmented},epoch=l.epoch;
     for(const event of ['onDecodedEvent','onUpdate']){
      const original=opts[event];opts[event]=u=>{let m=u?.update?.message??u?.message;if(m?.message)m=m.message;if(l.epoch===epoch&&m?.id&&m.author&&!l.olderIds?.has(m.id))l.messages.set(m.id,m);return original?.(u);};
     }
     const complete=opts.onComplete;opts.onComplete=u=>{if(l.epoch===epoch)l.terminals.push(u.reason);return complete?.(u);};
     const error=opts.onError;opts.onError=u=>{if(l.epoch===epoch)l.error='desktop_native_stream_failed';return error?.(u);};
     return state.saved[name].call(this,opts);
    };
    client[name]=state.wrapped[name];
   }
   state.saved.streamPost=api.streamPost;
   state.wrapped.streamPost=function(path,opts){
    if(path!=='/f/conversation')return state.saved.streamPost.call(this,path,opts);
    let request;try{request=JSON.parse(opts?.body??'null');}catch{}
    const l=owner(request);if(!l)return state.saved.streamPost.call(this,path,opts);
    const ids=(request.messages||[]).map(m=>m.id).join(',');
    if(!ids||l.dispatches.has(ids))throw Error('desktop_native_dispatch_replayed');
    const roles=request.messages.map(m=>m.author?.role);
    // Mwi's extraDeveloperInstructions are Csi system messages on this app wire.
    if(roles.some(r=>!['user','tool','system','assistant'].includes(r))||!roles.includes('user')&&!roles.includes('tool')||l.requestKind==='tool'&&roles.includes('user')||l.requestKind==='mixed'&&(!roles.includes('user')||!roles.includes('tool')))throw Error('desktop_native_dispatch_invalid');
    l.dispatches.add(ids);l.roundDispatched++;
    const before=opts.chatgptSse?.onBeforeRequestStart;
    return state.saved.streamPost.call(this,path,{...opts,retry:undefined,chatgptSse:{...opts.chatgptSse,onBeforeRequestStart(r){
     if(l.starts.has(ids))throw Error('desktop_native_dispatch_replayed');
     if(s.nativeBudget&&s.nativeBudget.used>=s.nativeBudget.limit)throw Error('desktop_native_trial_budget_exhausted');
     before?.(r);if(s.nativeBudget)s.nativeBudget.used++;l.starts.add(ids);
     l.userSubmissions+=roles.filter(r=>r==='user').length;l.toolSubmissions+=roles.filter(r=>r==='tool').length;
    }}});
   };
   api.streamPost=state.wrapped.streamPost;
}

// Evaluated only inside the signed app. It observes this lane's own requests and
// preserves all unowned traffic. The last lane restores exact wrapper identities.
export async function localAppOperation(key,lane,operation,data={},installHooks){
 const s=window[key];if(!s)throw Error('desktop_internal_unavailable');
 if(operation==='create'){
  const shared=await import('app://-/assets/app-shared-2d77298f0100.js');
  if(!s.scope.get(shared.HAt)||!['bRt','yRt','aCn','eHt'].every(n=>typeof s.m[n]==='function'))throw Error('desktop_native_capability_unavailable');
  if(data.maxDispatches!==undefined){
   if(!Number.isSafeInteger(data.maxDispatches)||data.maxDispatches<1)throw Error('desktop_native_trial_budget_invalid');
   s.nativeBudget??={limit:data.maxDispatches,used:0};s.nativeBudget.limit=Math.min(s.nativeBudget.limit,data.maxDispatches);
  }
  if(s.native?.lanes.has(lane))throw Error('desktop_native_owner_conflict');
  const id=s.m.aCn(s.scope);
  if(!s.native)installHooks(s,shared);
  s.native.lanes.set(lane,{id,signature:data.signature,marker:data.marker,messages:new Map(),dispatches:new Set(),starts:new Set(),userSubmissions:0,toolSubmissions:0,terminals:[],roundDispatched:0,accepted:false});
  return true;
 }
 const state=s.native,l=state?.lanes.get(lane);if(!l)throw Error('desktop_native_owner_missing');
 if(operation==='submit'){
  l.epoch=(l.epoch||0)+1;
  l.olderIds=new Set(Object.values(s.scope.get(s.m.JRn,l.id)||{}).map(n=>n.message?.id).filter(Boolean));
  l.messages.clear();l.terminals=[];l.roundDispatched=0;l.accepted=false;l.error=undefined;
  l.requestKind=data.results?.length?(data.contextMessages?.some(m=>m.role==='user')?'mixed':'tool'):'user';
  const model=data.model,onServerThreadIdChange=id=>{l.serverId=id;};
  l.controller=new AbortController();
  const options={conversationId:l.id,isTemporaryChat:false,...model,onServerThreadIdChange};
  let pending;
  if(data.results?.length===1&&!data.developerInstructions?.length&&!data.contextMessages?.length){
   const r=data.results[0];pending=s.m.bRt(s.scope,{...options,callId:r.id,toolName:l.signature.name,result:r.result});
  }else if(data.results?.length){
   // Same Hwi wire shape; prepend all but the last result so a parallel batch
   // resumes once, after all outputs, instead of generating once per output.
   const messages=data.results.map(r=>({author:{metadata:{},name:l.signature.name,role:'tool'},channel:'commentary',content:{content_type:'code',text:JSON.stringify({call_id:r.id,result:r.result,tool:l.signature.name})},create_time:null,end_turn:null,id:crypto.randomUUID(),metadata:{is_visually_hidden_from_conversation:true},recipient:'all',status:'finished_successfully',update_time:null,weight:1}));
   const groups=[];
   for(const m of data.contextMessages||[]){
    if(!['user','developer'].includes(m.role))throw Error('desktop_native_dispatch_invalid');
    if(groups.at(-1)?.role===m.role)groups.at(-1).messages.push(m);else groups.push({role:m.role,messages:[m]});
   }
   for(const group of groups){
    messages.push({id:crypto.randomUUID(),author:{role:group.role==='user'?'user':'system'},content:{content_type:'text',parts:['Native ordered '+group.role+' context:\n'+JSON.stringify(group.messages)]},metadata:{is_visually_hidden_from_conversation:group.role!=='user'},recipient:'all',status:'finished_successfully',end_turn:null});
   }
   pending=s.m.yRt(s.scope,{...options,prompt:'',daybreakContinuationMessageId:data.results[0].id,prependMessages:messages.slice(0,-1),
    userCompletionMessages:{message:messages.at(-1),extraDeveloperInstructionMessages:(data.developerInstructions||[]).map(text=>({id:crypto.randomUUID(),author:{role:'system'},content:{content_type:'text',parts:[text]},metadata:{is_visually_hidden_from_conversation:true}}))},requireResponseAcceptance:true,startupSignal:l.controller.signal});
  }else{
   pending=s.m.yRt(s.scope,{...options,prompt:data.prompt,extraDeveloperInstructions:data.developerInstructions||[],requireResponseAcceptance:true,startupSignal:l.controller.signal});
  }
  const epoch=l.epoch;
  Promise.resolve(pending).then(v=>{if(l.epoch===epoch)l.accepted=!!v;},error=>{
   if(l.epoch!==epoch)return;
   l.error='desktop_native_submit_failed';
   const code=error?.errorCode;
   l.submitFailure={status:Number.isInteger(error?.status)?error.status:null,
    category:['network','server','client','auth'].includes(error?.errorKind)?error.errorKind:'unknown',
    code:['model_cap_exceeded','work_usage_limit_exceeded','conversation_not_found','conversation_deleted','unauthorized','invalid_request_error'].includes(code)?code:
     /^desktop_native_[a-z_]+$/.test(error?.message||'')?error.message:'unclassified'};
  });
  return true;
 }
 if(operation==='snapshot'){
  // Scope snapshots supplement decoded updates but stay within our owned branch.
  for(const node of Object.values(s.scope.get(s.m.JRn,l.id)||{})){
   const m=node.message;if(m?.id&&!l.olderIds.has(m.id)&&m.author?.role==='assistant')l.messages.set(m.id,m);
  }
  return {messages:[...l.messages.values()],error:l.error,submitFailure:l.submitFailure,accepted:l.accepted,roundDispatched:l.roundDispatched,terminals:l.terminals,
   userSubmissions:l.userSubmissions,toolSubmissions:l.toolSubmissions,dispatches:l.starts.size};
 }
 if(operation==='close'){
  l.controller?.abort();s.m.eHt(s.scope,l.id);state.lanes.delete(lane);
  if(!state.lanes.size){
   for(const name of ['prepareCompletionStream','startCompletionStream'])if(state.client[name]===state.wrapped[name])state.client[name]=state.saved[name];
   if(state.api.streamPost===state.wrapped.streamPost)state.api.streamPost=state.saved.streamPost;
   delete s.native;
  }
  return true;
 }
 throw Error('desktop_native_operation_invalid');
}

export class DesktopLocalTransport{
 constructor(root,{acquire=acquireDesktopApp,pause=delay,timeoutMs=600000,maxDispatches,invoke=invokeLocalOperation}={}){
  this.root=root;this.acquire=acquire;this.pause=pause;this.timeoutMs=timeoutMs;this.invoke=invoke;
  this.maxDispatches=maxDispatches;
  this.lane='local_'+randomBytes(16).toString('hex');this.gateway='codex_'+randomBytes(12).toString('hex');this.busy=false;
  this.projection=new DesktopToolSession();this.context=new DesktopPromptContext();
 }
 async close(){
  if(this.closing)return this.closing;
  const lease=this.lease;if(!lease)return;
  this.lease=undefined;
  this.closing=(async()=>{try{await this.invoke(lease,this.lane,'close');}
   finally{lease.release();this.dispatched=0;this.observedCounts=undefined;}})();
  try{await this.closing;}finally{this.closing=undefined;}
 }
 async prepare(){
  await this.retiring;if(this.lease)return;
  this.lease=await this.acquire(this.root);
  try{await this.invoke(this.lease,this.lane,'create',{signature:localToolSignature(this.gateway),marker:this.lane,maxDispatches:this.maxDispatches});}
  catch(error){await this.close().catch(()=>{});throw error;}
 }
 retire(){this.retiring=this.close().catch(()=>{});this.context=new DesktopPromptContext();this.projection=new DesktopToolSession();this.routeIdentity=undefined;}
 async exchange(payload,signal,onSubmitted=()=>{},_onRetry=()=>{},_onContext=()=>{},route,emit=()=>{}){
  if(this.busy)throw Error('desktop_chat_busy');this.busy=true;
  let sent=false,settled=false,published=false,prepared;
  try{
   signal?.throwIfAborted();if(this.blocked)throw Error('desktop_native_delivery_unknown');await this.retiring;
   if(this.rejectedContext){
    const current=payload.messages.map(({timestamp,...m})=>m),old=this.rejectedContext;
    if(current.length<=old.length||old.some((m,i)=>JSON.stringify(m)!==JSON.stringify(current[i]))||!current.slice(old.length).some(m=>m.role==='user'))throw Error('desktop_native_recovery_requires_new_input');
   }
   const model=internalModel(route),identity=JSON.stringify(route);
   if(this.routeIdentity&&this.routeIdentity!==identity){
    if(this.projection.pending().length)throw Error('desktop_native_pending_route_change');
    await this.close();this.context=new DesktopPromptContext();this.projection=new DesktopToolSession();
   }
   if(!this.lease){
    await this.prepare();
   }
   const op=(method,data)=>this.invoke(this.lease,this.lane,method,data);
   const boundary={route,generation:this.lease.client.generation};
   prepared=this.context.prepare(payload,boundary);const batch=this.projection.results(payload.messages);
   const contextMessages=pendingNativeContext(prepared.delta.messages,batch);
   const instructions=[];
   if(prepared.delta.instructions!==undefined)instructions.push(...prepared.delta.instructions);
   if(prepared.delta.tools!==undefined)instructions.push('Native Codex tool catalog (call the exact name; preserve schemas and custom input):\n'+JSON.stringify(localToolCatalog(payload.tools)));
   if(!batch.length){
    instructions.unshift('You are the model doing this native Codex development task. Give genuine user-facing initial replies, intermediate reports and a final answer. Include your brief public initial reply in the first tool call report field, and useful intermediate findings in later tool call report fields. Core displays that field as commentary before executing the tool, without another model round. Honor requested report text. Use only local.'+this.gateway+' for native tools; never imitate a tool call or fabricate its result. For JSON tools encode arguments as input JSON; custom/freeform input is exact source. For independent tools use calls: a JSON-encoded array of {name,input} pairs, omitting singular name/input. Core executes each separately and returns all results in one original tool response. Never batch dependent steps or guess results. Never hand off to Work/Codex or use desktop builtin tools instead.');
   }
   sent=true;
   await op('submit',{model,developerInstructions:instructions,...batch.length?{results:this.projection.hostResults(batch),contextMessages}:{prompt:this.lane+'\nNative ordered task context:\n'+JSON.stringify(prepared.delta.messages)}});
   this.rejectedContext=undefined;
   if(batch.length)this.projection.returned(batch);
   let acknowledged=false;const deadline=Date.now()+this.timeoutMs,roundCalls=new Map();
   while(Date.now()<deadline){
    signal?.throwIfAborted();const state=await op('snapshot');
    if(state.dispatches>(this.dispatched||0)&&!acknowledged){
     const userMessages=state.userSubmissions-(this.observedCounts?.user||0),toolMessages=state.toolSubmissions-(this.observedCounts?.tool||0);
     acknowledged=true;onSubmitted({phase:'request',source:'app_dispatched',kind:toolMessages?(userMessages?'mixed':'tool'):'user',userMessages,toolMessages,dispatchedAt:Date.now()});
     this.observedCounts={user:state.userSubmissions,tool:state.toolSubmissions};
    }
    if(state.error){this.diagnostic=state.submitFailure;throw Error(state.error);}
    if(state.terminals.length){
     if(state.roundDispatched!==1||state.terminals.length!==1||state.terminals[0]!=='done')throw Error('desktop_native_stream_incomplete');
     if(!acknowledged)throw Error('desktop_native_dispatch_unobserved');
     settled=true;
    }
    // Validate each observed batch before publishing any native call. Wait for
    // the actual host terminal so an invalid later call cannot leak partial work.
    if(settled)for(const message of state.messages){
     for(const call of this.projection.decode(message,this.gateway,payload.tools)){
     const prior=this.projection.calls.get(call.id);
     if(prior){if(JSON.stringify(prior.call)!==JSON.stringify(call))throw Error('desktop_native_call_changed');continue;}
     const old=roundCalls.get(call.id);
     if(old&&JSON.stringify(old)!==JSON.stringify(call))throw Error('desktop_native_call_changed');
     roundCalls.set(call.id,call);
     }
    }
    let final;
    for(const message of state.messages){
     this.projection.text(message,emit);
     const publicMessage=this.projection.messages.get(message.id);
     if(message.status==='finished_successfully'&&message.end_turn===true&&publicMessage?.phase==='final_answer')final=publicMessage.text;
    }
    if(settled){
     this.dispatched=state.dispatches;
     this.context.accept(prepared.current,boundary);this.routeIdentity=identity;
     this.selectedControl={...model,transport:'signed-app-local',ordinaryChat:true,independentConversation:true,userSubmissions:state.userSubmissions,toolSubmissions:state.toolSubmissions,dispatches:state.dispatches};
     if(final!==undefined){if(roundCalls.size)throw Error('desktop_native_final_with_pending_calls');return {type:'final',text:final,streamed:true};}
     if(roundCalls.size){
      for(const call of roundCalls.values()){published=true;this.projection.issue(call,payload.tools,emit);}
      return {type:'tools',calls:[...roundCalls.values()],streamed:true};
     }
     throw Error('desktop_native_empty_response');
    }
    await this.pause(100);
   }
   throw Error('desktop_response_timeout');
  }catch(error){
   this.toolDiagnostic=error.toolDiagnostic;
   if(sent)this.blocked=true;
   if(sent||!this.routeIdentity){
    let closed=false;try{await this.close();closed=true;}catch{}
    if(sent&&settled&&!published&&closed&&['unadvertised_tool','invalid_tool_input','invalid_tool_search','desktop_native_call_invalid'].includes(error.message)){
     this.blocked=false;this.rejectedContext=prepared.current.messages;
    }
    this.context=new DesktopPromptContext();this.projection=new DesktopToolSession();this.routeIdentity=undefined;
   }
   throw error;
  }finally{this.busy=false;}
 }
}
