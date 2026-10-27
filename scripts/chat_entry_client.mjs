// Qualification input client only. The isolated native Provider has no app host.
import {readFileSync, writeFileSync} from 'node:fs';
import {createHash,randomUUID} from 'node:crypto';
import {createInterface} from 'node:readline';
import {connectDesktop} from '../providers/chatgpt/desktop_cdp.mjs';

const root=process.argv[2],plan=JSON.parse(readFileSync(root+'/plan.json','utf8'));
const bridge=readFileSync(root+'/candidate/app_bridge.js','utf8');
const owner=plan.provider_owner??'chat-entry-client:'+root,revision=createHash('sha256').update(JSON.stringify(bridge)).digest('hex');
if(plan.shared_bridge&&(!plan.provider_owner||!/^[0-9a-f]{48}$/.test(owner)))throw Error('explicit_shared_bridge_owner_required');
const c=await connectDesktop(plan.desktop_session,undefined,{app:true});
const save=(name,value)=>{
 const payload=JSON.stringify(value,null,2);
 // Cleanup errors must not destroy the original initialization/submission error.
 writeFileSync(root+'/'+name.replace(/\.json$/,'-'+randomUUID()+'.json'),payload,{mode:0o600,flag:'wx'});
 writeFileSync(root+'/'+name,payload,{mode:0o600});
};
async function evaluate(action,args={}) {
 const expression=`(${async function(action,args,bridge,owner,revision,plan) {
  const b=eval('('+bridge+')'),slot=Symbol.for('termux.codex.chat-entry.client.v1');
  function inputRequestScope(lanes,sourceLane,request) {
   const entry=[...lanes].find(([,l])=>request.conversation_id!=null&&[l.localId,l.serverId,l.uiLocalId].includes(request.conversation_id)||request.messages?.some(m=>m.metadata?.termux_codex_lane===l.marker)||request.local_function_signatures?.some(f=>f.name===l.signature?.name));
   return entry?{lane_id:entry[0],scope:entry[0]===sourceLane?'source':'provider'}:null;
  }
  function admitObservedRequest(requests,scope,plan) {
   if(requests.length>=(plan.max_model_responses??plan.max_source_requests))throw Error('chat_entry_total_request_budget_exhausted');
   if(scope==='source'&&requests.filter(r=>r.scope==='source').length>=(plan.max_source_lane_requests??plan.max_source_requests))throw Error('chat_entry_source_request_budget_exhausted');
  }
  function ownedLane(lanes,args) {
   if(args.lane_id!==undefined){
    if(args.conversation_id!==undefined||!/^[0-9a-f]{48}$/.test(args.lane_id??''))throw Error('explicit_owned_lane_required');
    const lane=lanes.get(args.lane_id);
    if(!lane)throw Error('owned_conversation_unavailable_or_ambiguous');
    return lane;
   }
   if(typeof args.conversation_id!=='string')throw Error('explicit_owned_conversation_required');
   const matches=[...lanes.values()].filter(l=>l.serverId===args.conversation_id);
   if(matches.length!==1)throw Error('owned_conversation_unavailable_or_ambiguous');
   return matches[0];
  }
  if(action==='inspect') {
   const s=globalThis[Symbol.for('termux.codex.app.bridge.v1')];
   return {bridge_present:!!s,lanes:[...(s?.lanes??[])].map(([id,l])=>({id,pending:l.pending,streamActive:l.streamActive,stopped:l.stopRequested})),model_requests:0};
  }
  if(action==='init') {
   if(globalThis[slot])throw Error('input_client_already_initialized');
   const previous=globalThis[Symbol.for('termux.codex.app.bridge.v1')];
   if(previous) {
    if(plan.shared_bridge){if(previous.owner!==owner||previous.revision!==revision)throw Error('shared_app_owner_or_revision_changed');}
    else {if(previous.lanes.size)throw Error('prior_app_lanes_must_be_reconciled');await b({op:'dispose',owner:previous.owner});}
   }
   const v=await b({op:'init',owner,bridgeRevision:revision,returnPath:'mcp',bindings:plan.bindings});
   if(!v.ready)throw Error('input_client_not_ready');
   const s=globalThis[Symbol.for('termux.codex.app.bridge.v1')];
   const identity=await b({op:'identity',owner});
   let projects=await b({op:'project_list',owner,identity});
   projects=projects.filter(p=>p.name===plan.project_name);
   if(projects.length>1)throw Error('ambiguous_project');
   const project=projects[0]??await b({op:'project_create',owner,identity,name:plan.project_name});
   const confirmed=await b({op:'project_read',owner,identity,project:project.id});
   if(!confirmed.compatible)throw Error('project_not_owned_compatible');
   const state={owner,identity,project:confirmed,lane:'chat-entry-source-'+(plan.input_client_id??owner),sourceCount:1,submissions:[],requests:[],initialUTC:new Date().toISOString(),originalWire:s.saved.streamPost};
   // Passive qualification diagnostics stay in private local artifacts. Preserve
   // the original rejection; never retry a request or change its arguments.
   state.originalGet=s.wire.safeGet;state.readErrors=[];
   state.getObserver=async function(path,...args){
    try{return await state.originalGet.call(this,path,...args);}
    catch(error){
     if(['/conversation/{conversation_id}','/conversations/{conversation_id}/messages','/conversations'].includes(path)){
      if(state.readErrors.length===64)state.readErrors.shift();
      state.readErrors.push({UTC:new Date().toISOString(),path,name:error?.name,message:error?.message,status:error?.responseStatus,stack:error?.stack});
     }
     throw error;
    }
   };
   state.deadlineUnixMs=plan.deadline_unix_ms??Date.now()+(plan.max_elapsed_seconds??1200)*1000;
   if(!Number.isSafeInteger(state.deadlineUnixMs)||state.deadlineUnixMs<=Date.now())throw Error('input_client_budget_expired');
   state.wireWrapper=function(path,options) {
    if(path!=='/f/conversation')return state.originalWire.call(this,path,options);
    const request=JSON.parse(options?.body??'{}'),before=options.chatgptSse?.onBeforeRequestStart;
    const observed=inputRequestScope(s.lanes,state.lane,request);
    if(!observed)return state.originalWire.call(this,path,options);
    return state.originalWire.call(this,path,{...options,chatgptSse:{...options.chatgptSse,onBeforeRequestStart(v){
     admitObservedRequest(state.requests,observed.scope,plan);
     state.requests.push({UTC:new Date().toISOString(),monotonic:performance.now(),body_bytes:new TextEncoder().encode(options.body).length,
      model:request.model,effort:request.thinking_effort,conversation_id:request.conversation_id??null,parent_message_id:request.parent_message_id??null,
      message_ids:request.messages?.map(m=>m.id),roles:request.messages?.map(m=>m.author?.role),...observed});
     return before?.(v);
    }}});
   };
   await b({op:'open',owner,lane:state.lane,returnPath:'mcp',project:project.id,signature:{name:'no_local_tools',description:'Use the installed MCP connector',parameters:{type:'object',properties:{}}},...(plan.shared_bridge?{nativeThread:'input-client:'+(plan.input_client_id??owner),limits:{maxDispatches:plan.max_source_lane_requests??plan.max_source_requests,deadlineUnixMs:state.deadlineUnixMs}}:{})});
   // Open against the original hooks before installing the passive observer.
   // A released bridge may currently expose its saved wire function.
   s.saved.streamPost=state.wireWrapper;
   s.wire.safeGet=state.getObserver;
   globalThis[slot]=state;
   return {initialized:true,project:confirmed,model_requests:0};
  }
  const state=globalThis[slot];
  if(state?.owner!==owner)throw Error('input_client_owner_changed');
  const s=globalThis[Symbol.for('termux.codex.app.bridge.v1')];
  const identity=await b({op:'identity',owner});
  if(['account','user','structure'].some(k=>identity[k]!==state.identity[k]))throw Error('input_account_changed');
  const common={owner,lane:state.lane};
  if(action==='read_diagnostics')return {model_requests:0,read_errors:state.readErrors};
  if(action==='owned_lane_status'){
   if(!plan.shared_bridge)throw Error('shared_source_lifecycle_required');
   const l=ownedLane(s.lanes,args);
   return {conversation_id:l.serverId,streamActive:l.streamActive===true,pending:l.pending===true,
    stopRequested:l.stopRequested===true,serverStopState:l.serverStopState??'not_observed',model_requests:0};
  }
  if(action==='server_status'){
   if(!plan.shared_bridge||typeof args.conversation_id!=='string'||![...s.lanes.values()].some(l=>l.serverId===args.conversation_id))throw Error('explicit_owned_conversation_required');
   const v=await s.wire.safeGet('/conversation/{conversation_id}/stream_status',{parameters:{path:{conversation_id:args.conversation_id}},additionalHeaders:{'ChatGPT-Account-ID':state.identity.account},retry:'never'});
   const after=await b({op:'identity',owner});
   if(['account','user','structure'].some(k=>after[k]!==state.identity[k]))throw Error('input_account_changed');
   return {conversation_id:args.conversation_id,status:(v?.data??v)?.status??null,model_requests:0};
  }
  if(action==='new_source'){
   if(!plan.shared_bridge||state.lane!==null)throw Error('prior_source_must_be_released');
   state.lane='chat-entry-source-'+(++state.sourceCount);
   await b({op:'open',owner,lane:state.lane,returnPath:'mcp',project:state.project.id,nativeThread:'input-client:'+(plan.input_client_id??owner),limits:{maxDispatches:plan.max_source_lane_requests??plan.max_source_requests,deadlineUnixMs:state.deadlineUnixMs},signature:{name:'no_local_tools',description:'Use the installed MCP connector',parameters:{type:'object',properties:{}}}});
   return {opened:true,model_requests:0};
  }
  if(action==='submit') {
   if(state.submissions.length>=plan.max_user_turns)throw Error('input_turn_budget_exhausted');
   if(state.submissions.some(v=>v.step===args.step))throw Error('input_turn_already_submitted');
   const before=await b({op:'snapshot',...common});
   if(before.pending||before.streamActive||before.error)throw Error('previous_input_unsettled');
   state.submissions.push({step:args.step,UTC:new Date().toISOString(),monotonic:performance.now()});
   const submitted=await b({op:'submit',...common,round:crypto.randomUUID(),model:plan.host_model,thinkingEffort:plan.thinking_effort,messages:[{role:'user',text:args.text}],instructions:[],results:[]});
   return {submitted,step:args.step,model_requests:1};
  }
  if(action==='snapshot')return {...(state.lane===null?{source_released:true}:await b({op:'snapshot',...common})),submissions:state.submissions,requests:state.requests,project:state.project};
  if(action==='approve'||action==='approve_ui')return b({op:action,...common,identity:state.identity,connector:plan.connector_name??'Termux Bridge'});
  if(action==='stop_inputs'){
   if(!plan.shared_bridge)throw Error('shared_source_lifecycle_required');
   const observations=[];
   for(const [lane]of s.lanes)if(lane.startsWith('chat-entry-source')||lane.startsWith('saved-input-')){
    const value=await b({op:'stop',owner,lane});observations.push({lane,stopped:value.stopped===true});
   }
   return {observations,model_requests:0};
  }
  if(action==='approve_saved'||action==='saved_snapshot'){
   if(!plan.shared_bridge||!/^[0-9a-f]{64}$/.test(args.input_id??''))throw Error('explicit_saved_input_required');
   const lane='saved-input-'+args.input_id;
   if(action==='saved_snapshot'&&!s.lanes.has(lane))return {available:false,model_requests:0};
   if(!s.lanes.has(lane))throw Error('owned_saved_input_unavailable');
   if(action==='saved_snapshot')return b({op:'snapshot',owner,lane});
   return b({op:'approve',owner,lane,identity:state.identity,connector:plan.connector_name??'Termux Bridge'});
  }
  if(action==='stop')return b({op:'stop',...common});
  if(action==='saved'||action==='saved_existing') {
   const id=action==='saved'?s.lanes.get(state.lane)?.serverId:args.conversation_id;
   if(!id)throw Error('saved_chat_not_available');
   if(action==='saved_existing'&&!plan.permitted_saved_conversations?.includes(id))throw Error('explicit_saved_conversation_required');
   const options={parameters:{path:{conversation_id:id}}};
   const raw=await s.wire.safeGet('/conversation/{conversation_id}',options),conv=raw.data??raw;
   if(conv.is_temporary_chat!==false||conv.gizmo_id!==state.project.id)throw Error('wrong_saved_chat');
   const messages=Object.values(conv.mapping??{}).map(n=>n.message).filter(Boolean);
   const publicMessages=messages.filter(m=>m.author?.role==='user'||m.author?.role==='tool'||m.author?.role==='assistant'&&
    !m.metadata?.is_visually_hidden_from_conversation&&[null,undefined,'final','commentary'].includes(m.channel)&&['text','code','tool_use'].includes(m.content?.content_type));
   // Only message update_time is volatile on title PATCH. Retain every other
   // graph/value, including hidden records; raw failure evidence stays intact.
   const canonical=(v,path=[])=>Array.isArray(v)?v.map((x,i)=>canonical(x,[...path,i])):v&&typeof v==='object'?Object.fromEntries(Object.keys(v).sort().filter(k=>!(path.length===2&&path[1]==='message'&&k==='update_time')).map(k=>[k,canonical(v[k],[...path,k])])):v;
   const digest=async v=>[...new Uint8Array(await crypto.subtle.digest('SHA-256',new TextEncoder().encode(JSON.stringify(canonical(v)))))].map(x=>x.toString(16).padStart(2,'0')).join('');
   const contentHash=await digest(conv.mapping);
   let titleReadback=null;
   if(args.title&&conv.title!==args.title) {
    await s.wire.safePatch('/conversation/{conversation_id}',{...options,requestBody:{title:args.title},retry:'never'});
    const again=await s.wire.safeGet('/conversation/{conversation_id}',options),after=again.data??again;
    titleReadback={before:conv,after};
    if(after.title!==args.title||await digest(after.mapping)!==contentHash){const error=Error('title_preservation_unconfirmed');error.evidence={before:conv,after};throw error;}
   }
   return {conversation_id:id,project:state.project.id,is_temporary:false,title:args.title??conv.title,
    public_messages:publicMessages,content_sha256:contentHash,title_readback:titleReadback,submissions:state.submissions,requests:state.requests,model_requests:0};
  }
  if(action==='dispose'||action==='release_source') {
   if(action==='release_source'&&!plan.shared_bridge)throw Error('shared_source_lifecycle_required');
   if(state.lane!==null){
    const snap=await b({op:'snapshot',...common});
    if(snap.pending||snap.streamActive)throw Error('input_client_unsettled');
    const stopped=await b({op:'stop',...common});
    if(!stopped.stopped)throw Error('input_client_stop_unconfirmed');
    await b({op:'release',...common});state.lane=null;
   }
   if(action==='release_source')return {released:true,model_requests:0};
   if(s.saved.streamPost!==state.wireWrapper)throw Error('input_observer_changed');
   s.saved.streamPost=state.originalWire;
   if(s.wire.streamPost===state.wireWrapper)s.wire.streamPost=state.originalWire;
   if(s.wire.safeGet===state.getObserver)s.wire.safeGet=state.originalGet;
   if(!plan.shared_bridge||s.lanes.size===0)await b({op:'dispose',owner});delete globalThis[slot];
   return {disposed:true,model_requests:0};
  }
  throw Error('input_action_unknown');
 }.toString()})(${JSON.stringify(action)},${JSON.stringify(args)},${JSON.stringify(bridge)},${JSON.stringify(owner)},${JSON.stringify(revision)},${JSON.stringify(plan)})`;
 const result=await c.evaluate(`(async()=>{try{return {ok:true,value:await ${expression}};}catch(error){return {ok:false,error:{name:error?.name,message:error?.message,stack:error?.stack,evidence:error?.evidence}};}})()`);
 if(!result?.ok){save('input-client-dom-error-private.json',{action,error:result?.error});throw Error('input_client_action_failed_private_evidence_retained');}
 return result.value;
}
try {
 console.log(JSON.stringify({ready:true,inspection:await evaluate('inspect')}));
 for await(const line of createInterface({input:process.stdin,crlfDelay:Infinity})) {
  let command;
  try {command=JSON.parse(line);const value=await evaluate(command.action,command.args);console.log(JSON.stringify({id:command.id,value}));}
  catch(error){save('input-client-error-private.json',{action:command?.action,error:String(error)});console.log(JSON.stringify({id:command?.id,error:'input_client_operation_failed_private_evidence_retained'}));}
 }
}finally{c.close();}
