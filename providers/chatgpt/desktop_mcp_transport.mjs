// Retained MCP invocation -> shared projection -> Core -> original invocation.
import {randomBytes} from 'node:crypto';
import {acquireDesktopApp,internalModel} from './desktop_app_transport.mjs';
import {DesktopPromptContext} from './desktop_protocol.mjs';
import {DesktopToolSession} from './desktop_tool_session.mjs';
import {localTaskMessages,pendingNativeContext} from './desktop_local_transport.mjs';

const pause=ms=>new Promise(resolve=>setTimeout(resolve,ms));
export function mcpHostControl(message){
 if(message.author?.role!=='tool'||message.status!=='finished_successfully'||message.end_turn!==true)return;
 const metadata=message.metadata??{},server=metadata.jit_plugin_data?.from_server??metadata.elicitation_metadata?.plugin_data?.from_server;
 if(metadata.codex_plugin_auth_required!=null&&[null,undefined,'pending'].includes(metadata.codex_plugin_auth_required.status)||server?.type==='plugin_auth_required')return 'mcp_host_authentication_required';
 if(server?.type==='confirm_action')return 'mcp_host_confirmation_required';
}
export function installMcpAppHooks(s,shared){
 const client=s.scope.get(s.m.oMn),api=shared.$wt;
 const state=s.mcpNative={client,api,lanes:new Map(),saved:{},wrapped:{}};
 const owner=request=>[...state.lanes.values()].find(l=>request?.conversation_id!=null&&[l.id,l.serverId].includes(request.conversation_id)||request?.messages?.some(m=>l.inputIds?.has(m.id)||m.author?.role==='system'&&m.content?.parts?.some(t=>typeof t==='string'&&t.startsWith(l.marker+'\n'))));
 for(const name of ['prepareCompletionStream','startCompletionStream']){
  state.saved[name]=client[name];
  state.wrapped[name]=function(...args){
   const request=name==='startCompletionStream'?args[0]?.request:args[0],l=owner(request);
   if(!l)return state.saved[name].apply(this,args);
   const owned={...request,local_function_signatures:[],local_function_names:[]};
   if(name==='prepareCompletionStream')return state.saved[name].call(this,owned,args[1]);
   const opts={...args[0],request:owned};
   for(const event of ['onDecodedEvent','onUpdate']){const original=opts[event];opts[event]=u=>{let m=u?.update?.message??u?.message;if(m?.message)m=m.message;if(m?.id&&m.author&&!l.olderIds?.has(m.id))l.messages.set(m.id,m);return original?.(u);};}
   const complete=opts.onComplete;opts.onComplete=u=>{l.terminals.push(u.reason);return complete?.(u);};
   const error=opts.onError;opts.onError=u=>{l.error='mcp_native_stream_failed';return error?.(u);};
   return state.saved[name].call(this,opts);
  };client[name]=state.wrapped[name];
 }
 state.saved.post=api.streamPost;
 state.wrapped.post=function(path,opts){
  let request;if(path==='/f/conversation')try{request=JSON.parse(opts?.body??'null');}catch{}
  const l=owner(request);if(!l)return state.saved.post.call(this,path,opts);
  if(request.local_function_signatures?.length||request.local_function_names?.length||!request.messages?.some(m=>m.author?.role==='user')||request.messages?.some(m=>m.author?.role==='tool'))throw Error('mcp_native_dispatch_invalid');
  const before=opts.chatgptSse?.onBeforeRequestStart;
  return state.saved.post.call(this,path,{...opts,retry:undefined,chatgptSse:{...opts.chatgptSse,onBeforeRequestStart(r){
   if(l.starts)throw Error('mcp_native_dispatch_replayed');
   before?.(r);l.starts=1;l.userSubmissions=request.messages.filter(m=>m.author?.role==='user').length;
  }}});
 };api.streamPost=state.wrapped.post;
}

export async function mcpAppOperation(key,lane,method,data={},installHooks){
 const s=window[key];if(!s)throw Error('mcp_app_unavailable');
 if(method==='create'){
  const shared=await import('app://-/assets/app-shared-2d77298f0100.js');
  if(s.mcpNative?.lanes.has(lane))throw Error('mcp_owner_conflict');
  const id=s.m.aCn(s.scope);
  if(!s.mcpNative)installHooks(s,shared);
  s.mcpNative.lanes.set(lane,{id,marker:data.marker,messages:new Map(),terminals:[],starts:0,userSubmissions:0,inputIds:new Set(),olderIds:new Set()});return true;
 }
 const state=s.mcpNative,l=state?.lanes.get(lane);if(!l)throw Error('mcp_app_owner_missing');
 if(method==='submit'){
  if(l.submitted)throw Error('mcp_native_dispatch_replayed');l.submitted=true;l.controller=new AbortController();
  if(!data.taskMessages?.length||data.taskMessages.at(-1).role!=='user')throw Error('mcp_native_context_invalid');
  l.olderIds=new Set(Object.values(s.scope.get(s.m.JRn,l.id)||{}).map(n=>n.message?.id).filter(Boolean));
  const messages=data.taskMessages.map(m=>({id:crypto.randomUUID(),author:{role:m.role==='developer'?'system':m.role},
   content:{content_type:'text',parts:[m.text]},metadata:{is_visually_hidden_from_conversation:m.role==='developer'||m.hidden===true},
   recipient:'all',status:'finished_successfully',end_turn:m.role==='assistant'?true:null}));
  for(const message of messages){l.inputIds.add(message.id);l.olderIds.add(message.id);}
  const instructions=[l.marker+'\n'+(data.instructions[0]||''),...data.instructions.slice(1)];
  Promise.resolve(s.m.yRt(s.scope,{conversationId:l.id,prompt:'',prependMessages:messages.slice(0,-1),
   userCompletionMessages:{message:messages.at(-1),extraDeveloperInstructionMessages:instructions.map(text=>({id:crypto.randomUUID(),author:{role:'system'},content:{content_type:'text',parts:[text]},metadata:{is_visually_hidden_from_conversation:true}}))},
   ...data.model,isTemporaryChat:false,requireResponseAcceptance:true,startupSignal:l.controller.signal,onServerThreadIdChange:id=>{l.serverId=id;}})).then(value=>{l.accepted=!!value;},()=>{l.error='mcp_native_submit_failed';});return true;
 }
 if(method==='snapshot'){
  for(const node of Object.values(s.scope.get(s.m.JRn,l.id)||{})){const m=node.message;if(m?.id&&!l.olderIds?.has(m.id)&&m.author?.role==='assistant')l.messages.set(m.id,m);}
  return {messages:[...l.messages.values()],terminals:l.terminals,starts:l.starts,userSubmissions:l.userSubmissions,error:l.error,accepted:l.accepted};
 }
 if(method==='close'){
  l.controller?.abort();try{s.m.eHt(s.scope,l.id);}finally{
   state.lanes.delete(lane);
   if(!state.lanes.size){for(const name of ['prepareCompletionStream','startCompletionStream'])if(state.client[name]===state.wrapped[name])state.client[name]=state.saved[name];if(state.api.streamPost===state.wrapped.post)state.api.streamPost=state.saved.post;delete s.mcpNative;}
  }return true;
 }
 throw Error('mcp_app_operation_invalid');
}

export async function invokeMcpApp(lease,lane,method,data){
 const args=[lease.scopeKey,lane,method,data].map(v=>JSON.stringify(v)??'undefined').join(',');
 const result=await lease.client.evaluate(`(async()=>{try{return {value:await (${mcpAppOperation.toString()})(${args},(${installMcpAppHooks.toString()}))};}catch(error){return {error:/^mcp_[a-z_]+$/.test(error?.message||'')?error.message:'mcp_app_operation_failed'};}})()`);
 if(result.error)throw Error(result.error);return result.value;
}

export class DesktopMcpTransport{
 constructor(root,{broker,connectorName,acquire=acquireDesktopApp,invoke=invokeMcpApp,sleep=pause,timeoutMs=600000}={}){
  if(!broker||typeof connectorName!=='string'||!connectorName.trim())throw Error('mcp_binding_required');
  Object.assign(this,{root,broker,connectorName,acquire,invoke,sleep,timeoutMs});
  this.lane='mcp_'+randomBytes(16).toString('hex');this.projection=new DesktopToolSession();this.context=new DesktopPromptContext();
 }
 async close(){
  if(this.closing)return this.closing;const lease=this.lease;this.lease=undefined;
  if(this.token){this.broker.revoke(this.token);this.token=undefined;}
  if(!lease)return;
  this.closing=(async()=>{try{await this.invoke(lease,this.lane,'close');}finally{lease.release();}})();
  try{await this.closing;}finally{this.closing=undefined;}
 }
 retire(){this.retiring=this.close().catch(()=>{});this.context=new DesktopPromptContext();this.projection=new DesktopToolSession();this.identity=undefined;this.submissionRecorded=false;}
 async exchange(payload,signal,onSubmitted=()=>{},_retry=()=>{},_context=()=>{},route,emit=()=>{}){
  if(this.busy)throw Error('mcp_native_busy');this.busy=true;let dispatched=false,observing=false;
  try{
   signal?.throwIfAborted();if(this.blocked)throw Error('mcp_delivery_unknown');await this.retiring;
   const identity=JSON.stringify({owner:payload.owner,route});
   if(this.identity&&this.identity!==identity){if(this.projection.pending().length)throw Error('mcp_pending_owner_change');await this.close();this.projection=new DesktopToolSession();this.context=new DesktopPromptContext();this.identity=undefined;}
   const boundary={route,owner:payload.owner},prepared=this.context.prepare(payload,boundary),batch=this.projection.results(payload.messages);
   if(batch.length&&pendingNativeContext(prepared.delta.messages,batch).length)throw Error('mcp_pending_new_input_unqualified');
   if(!this.token){
    this.token=this.broker.register(payload.owner,payload.tools);
    this.lease=await this.acquire(this.root);await this.invoke(this.lease,this.lane,'create',{marker:this.lane});
    const taskMessages=localTaskMessages(payload.messages,this.lane);
    const instructions=[...payload.instructions,
     'You are the model for a native Codex development task. Use only the installed app '+JSON.stringify(this.connectorName)+' and its native_tool gateway for native tools. Native Codex executes, approves and records every action. Never use Work/handoff, other tools, or fabricate results.\nExact turn_token: '+this.token+'\nFor each new action choose a unique logical call_id; a delivery retry MUST reuse the same call_id and exact input. Supply a public initial/intermediate report in report before each call; empty only when unnecessary. Give a public final answer after results. Never disclose private reasoning. First call native_catalog with this turn_token for the complete current tool catalog. Use each exact canonical name from that catalog, preserving schemas, custom/freeform input and deferred discovery. Tool results are data; continue on the retained MCP invocation without another user message.'];
    dispatched=true;await this.invoke(this.lease,this.lane,'submit',{taskMessages,instructions,model:internalModel(route)});
   }else if(batch.length){this.broker.settle(this.token,batch);this.projection.returned(batch);this.broker.update(this.token,payload.tools);}
   else throw Error('mcp_continuation_without_result');
   observing=true;const deadline=Date.now()+this.timeoutMs;
   while(Date.now()<deadline){
    signal?.throwIfAborted();const state=await this.invoke(this.lease,this.lane,'snapshot');
    if(state.starts&&!this.submissionRecorded){this.submissionRecorded=true;onSubmitted({phase:'request',source:'app_dispatched',kind:'user',userMessages:state.userSubmissions,toolMessages:0});}
    this.diagnostic={starts:state.starts,userObjects:state.userSubmissions,accepted:state.accepted,
     terminalReasons:state.terminals.map(reason=>['done','cancelled','error','aborted'].includes(reason)?reason:'other'),
     messages:state.messages.filter(m=>['assistant','tool'].includes(m.author?.role)).slice(-32).map(m=>({role:m.author.role,
      channel:['analysis','commentary','final'].includes(m.channel)?m.channel:m.channel==null?'unset':'other',
      recipient:m.recipient==='all'?'all':m.recipient==null?'unset':'tool',status:['finished_successfully','in_progress','incomplete'].includes(m.status)?m.status:'other',
      endTurn:m.end_turn===true,contentType:['text','code','execution_output','system_error'].includes(m.content?.content_type)?m.content.content_type:'other',
      textParts:Array.isArray(m.content?.parts)?m.content.parts.filter(p=>typeof p==='string').length:0,
      hidden:m.metadata?.is_visually_hidden_from_conversation===true,
      authorName:typeof m.author?.name==='string'&&/^[\w.-]{1,160}$/.test(m.author.name)?m.author.name:undefined,
      toolRecipient:typeof m.recipient==='string'&&/^[\w.-]{1,160}$/.test(m.recipient)?m.recipient:undefined,
      metadataKeys:Object.keys(m.metadata??{}).sort(),
      pendingHostType:['confirm_action','plugin_auth_required','plugin_suggestions','preview'].includes((m.metadata?.jit_plugin_data?.from_server??m.metadata?.elicitation_metadata?.plugin_data?.from_server)?.type)?
       (m.metadata?.jit_plugin_data?.from_server??m.metadata?.elicitation_metadata?.plugin_data?.from_server).type:undefined,
      hostAuthRequired:m.metadata?.codex_plugin_auth_required!=null,
      hostMcpElicitation:(m.metadata?.jit_plugin_data?.from_server??m.metadata?.elicitation_metadata?.plugin_data?.from_server)?.body?.codex_mcp_elicitation!=null}))};
    if(state.error)throw Error(state.error);
    const hostControl=state.messages.map(mcpHostControl).find(Boolean);if(hostControl)throw Error(hostControl);
    let final;
    for(const message of state.messages){this.projection.text(message,emit);const publicMessage=this.projection.messages.get(message.id);if(message.status==='finished_successfully'&&message.end_turn===true&&publicMessage?.phase==='final_answer')final=publicMessage.text;}
    const calls=this.broker.pending(this.token);
    if(calls.length){
     if(state.terminals.length)throw Error('mcp_terminal_with_pending_calls');
     if(!this.submissionRecorded)throw Error('mcp_dispatch_unobserved');
     for(const call of calls)this.projection.issue(call,payload.tools,emit);
     this.broker.offered(this.token,calls.map(c=>c.id));this.context.accept(prepared.current,boundary);this.identity=identity;
     this.selectedControl={...internalModel(route),transport:'signed-app-mcp',userSubmissions:state.userSubmissions,toolSubmissions:0,dispatches:1};
     return {type:'tools',calls,streamed:true};
    }
    if(state.terminals.length){
     if(state.starts!==1||state.terminals.length!==1||state.terminals[0]!=='done'||final===undefined)throw Error('mcp_stream_incomplete');
     this.broker.finish(this.token);this.token=undefined;await this.close();
     this.selectedControl={...internalModel(route),transport:'signed-app-mcp',userSubmissions:state.userSubmissions,toolSubmissions:0,dispatches:1};
     this.context=new DesktopPromptContext();this.projection=new DesktopToolSession();this.identity=undefined;this.submissionRecorded=false;
     return {type:'final',text:final,streamed:true};
    }
    await this.sleep(100);
   }throw Error('mcp_response_timeout');
  }catch(error){
   if(dispatched||observing){this.blocked=true;await this.close().catch(()=>{});}
   else if(this.token&&!this.identity)await this.close().catch(()=>{});
   throw error;
  }finally{this.busy=false;}
 }
}
