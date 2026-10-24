import {test,expect} from 'bun:test';
import {readFileSync,mkdtempSync,writeFileSync,rmSync} from 'node:fs';
import {join} from 'node:path';
import {pathToFileURL} from 'node:url';

const rawBridge=new Function('return ('+readFileSync(new URL('./src/app_bridge.js',import.meta.url),'utf8')+')')();
const bridge=command=>rawBridge({bridgeRevision:'1'.repeat(64),...command});
const slot=Symbol.for('termux.codex.app.bridge.v1');
test('saved-Chat adoption preserves selected branch and draft without allocating a Chat',async()=>fixture(async({f,bindings})=>{
 await bridge({op:'init',owner:'owned',returnPath:'mcp',bindings});
 const raw={conversation_id:'old-chat',gizmo_id:null,current_node:'alternate',mapping:{selected:{message:{status:'finished_successfully'}},alternate:{message:{status:'finished_successfully'}}}};
 f.chatGet=async()=>raw;f.draft='User unsent draft';
 const adopt={op:'chat_adopt',owner:'owned',lane:'input',conversation_id:'old-chat',branch:'selected',identity:{account:'account',user:'user',structure:'personal'},returnPath:'mcp',signature:{name:'unused'}};
 expect(await bridge(adopt)).toEqual({localId:'old-chat'});
 expect(f.created).toBe(0);expect(f.sent).toHaveLength(0);expect(f.draft).toBe('User unsent draft');
 expect(f.hydrated.options).toEqual({mergeActiveBranch:true,preserveAsyncStatus:true});
 expect(await bridge(adopt)).toEqual({localId:'old-chat'});
 await expect(bridge({...adopt,branch:'alternate'})).rejects.toThrow('app_lane_declaration_changed');
 await expect(bridge({...adopt,lane:'other'})).rejects.toThrow('app_chat_owned_work');
 await bridge({op:'submit',owner:'owned',lane:'input',round:'one',messageId:'stable-message',model:'host',messages:[{role:'user',text:'Related task'}]});
 await Promise.resolve();
 expect(f.submitOptions.conversationId).toBe('old-chat');expect(f.submitOptions.parentMessageId).toBe('selected');
 expect(f.submitOptions.conversationOrigin).toBeNull();expect(f.submitOptions.tppExecutionTarget).toBeNull();expect(f.submitOptions.requestInput).toEqual({isEverydayWorkMode:false});
 expect(f.sent[0].messages[0].id).toBe('stable-message');expect(f.draft).toBe('User unsent draft');
 await bridge({op:'release',owner:'owned',lane:'input'});
 f.active=true;await expect(bridge({...adopt,lane:'busy'})).rejects.toThrow('app_chat_active_unowned');
 expect(f.sent).toHaveLength(1);expect(f.created).toBe(0);
}));

test('release cannot abandon a stream that is still active',async()=>fixture(async({f,bindings})=>{
 await bridge({op:'init',owner:'owned',returnPath:'mcp',bindings});
 await bridge({op:'open',owner:'owned',lane:'busy',signature:{name:'unused'}});
 const lane=globalThis[slot].lanes.get('busy');lane.streamActive=true;
 await expect(bridge({op:'release',owner:'owned',lane:'busy'})).rejects.toThrow('app_lane_busy');
 await expect(bridge({op:'submit',owner:'owned',lane:'busy',round:'new'})).rejects.toThrow('app_lane_busy');
 expect(f.sent).toHaveLength(0);expect(globalThis[slot].lanes.has('busy')).toBe(true);
}));

for(const activity of ['streaming','active-async-turn','active-tpp-turn'])test('installed string activity status fences saved input: '+activity,async()=>fixture(async({f,bindings})=>{
 await bridge({op:'init',owner:'owned',returnPath:'mcp',bindings});
 f.activity=activity;
 await expect(bridge({op:'chat_adopt',owner:'owned',lane:'input',conversation_id:'old-chat',branch:'selected',identity:{account:'account',user:'user',structure:'personal'},returnPath:'mcp',signature:{name:'unused'}})).rejects.toThrow('app_chat_active_unowned');
 expect(f.hydrated).toBeUndefined();expect(f.sent).toHaveLength(0);expect(f.created).toBe(0);
}));

test('saved-input capability uses the installed predicate and status atom',async()=>fixture(async({f,bindings})=>{
 await bridge({op:'init',owner:'owned',returnPath:'mcp',bindings});
 expect((await bridge({op:'host_status',owner:'owned'})).capabilities.saved_input).toBe(true);
 const state=globalThis[slot];state.api={...state.api,pzn:undefined};
 expect((await bridge({op:'host_status',owner:'owned'})).capabilities.saved_input).toBe(false);
 await expect(bridge({op:'chat_adopt',owner:'owned',lane:'input',conversation_id:'old-chat',branch:'selected',identity:{account:'account',user:'user',structure:'personal'},returnPath:'mcp',signature:{name:'unused'}})).rejects.toThrow('app_chat_input_unavailable');
 expect(f.hydrated).toBeUndefined();expect(f.sent).toHaveLength(0);
}));
test('stopping an owned empty lane confirms zero dispatch without submitting a model',async()=>fixture(async({f,bindings})=>{
 expect(await bridge({op:'release',owner:'owned',lane:'absent'})).toEqual({released:true});
 await bridge({op:'init',owner:'owned',returnPath:'mcp',bindings});
 await bridge({op:'open',owner:'owned',lane:'empty',signature:{name:'unused'}});
 const result=await bridge({op:'stop',owner:'owned',lane:'empty'});
 expect(result.stopped).toBe(true);expect(result.dispatches).toBe(0);expect(f.sent).toHaveLength(0);
 await bridge({op:'release',owner:'owned',lane:'empty'});
 expect(await bridge({op:'dispose',owner:'owned'})).toEqual({disposed:true});
}));
for(const outcome of ['COMPLETE','IS_STREAMING','FAILURE','lost-stop','identity-missing'])test('active Stop requires server evidence after local abort: '+outcome,async()=>fixture(async({f,bindings})=>{
 let posts=0,reads=0;
 f.client.stopCompletion=async body=>{posts++;expect(body).toEqual({conversation_id:'server-owned',exclude_async_types:[]});if(outcome==='lost-stop')throw Error('lost Stop reply');};
 f.client.startCompletionStream=options=>{
  f.options.push(options);f.wire.streamPost('/f/conversation',{body:JSON.stringify(options.request),chatgptSse:{}});return true;
 };
 f.chatGet=async(path,options)=>{reads++;expect(path).toBe('/conversation/{conversation_id}/stream_status');expect(options.retry).toBe('never');expect(options.parameters.path.conversation_id).toBe('server-owned');return {status:outcome};};
 await bridge({op:'init',owner:'owned',returnPath:'mcp',bindings});
 await bridge({op:'open',owner:'owned',lane:'active-stop',signature:{name:'unused'}});
 await bridge({op:'submit',owner:'owned',lane:'active-stop',round:'one',messages:[{role:'user',text:'task'}]});await Promise.resolve();
 const lane=globalThis[slot].lanes.get('active-stop');
 if(outcome==='identity-missing')lane.serverId=undefined;
 // eHt/JU can synthesize an abort or even a local done; neither is server proof.
 f.cancelCallback=()=>f.options.at(-1).onComplete({reason:'done'});
 const stopped=await bridge({op:'stop',owner:'owned',lane:'active-stop'});
 expect(stopped.localStreamStopped).toBe(true);
 expect(stopped.stopped).toBe(['COMPLETE','FAILURE'].includes(outcome));
 expect(stopped.serverStopState).toBe(['COMPLETE','FAILURE'].includes(outcome)?'terminal_observed':outcome==='identity-missing'?'server_identity_unavailable':outcome==='IS_STREAMING'?'still_streaming':'unconfirmed');
 const repeated=await bridge({op:'stop',owner:'owned',lane:'active-stop'});
 expect(repeated.stopped).toBe(stopped.stopped);
 expect(posts).toBe(outcome==='identity-missing'?0:1);
 expect(reads).toBe(['IS_STREAMING','lost-stop'].includes(outcome)?2:['COMPLETE','FAILURE'].includes(outcome)?1:0);
 if(!stopped.stopped)await expect(bridge({op:'release',owner:'owned',lane:'active-stop'})).rejects.toThrow('app_stop_unconfirmed');
 expect(f.sent).toHaveLength(1);
}));
test('server-terminal proof clears stale active state and fences late owned starts',()=>fixture(async({bindings,f})=>{
 let posts=0;
 f.client.stopCompletion=async()=>{posts++;};
 f.client.startCompletionStream=options=>{
  f.options.push(options);f.wire.streamPost('/f/conversation',{body:JSON.stringify({...options.request,conversation_id:'server-owned'}),chatgptSse:{}});return true;
 };
 f.chatGet=async()=>({status:'COMPLETE'});
 await bridge({op:'init',owner:'owned',returnPath:'mcp',bindings});
 await bridge({op:'open',owner:'owned',lane:'stale-active',signature:{name:'unused'}});
 await bridge({op:'submit',owner:'owned',lane:'stale-active',round:'one',messages:[{role:'user',text:'task'}]});await Promise.resolve();
 const lane=globalThis[slot].lanes.get('stale-active');
 f.cancelCallback=()=>{lane.streamActive=true;};
 const result=await bridge({op:'stop',owner:'owned',lane:'stale-active'});
 expect(result.stopped).toBe(true);expect(result.streamActive).toBe(false);expect(posts).toBe(1);
 expect(()=>f.client.prepareCompletionStream({conversation_id:'server-owned',messages:[{metadata:{termux_codex_lane:'stale-active'}}]})).toThrow('app_dispatch_stopped');
 expect(()=>f.client.startCompletionStream({request:{conversation_id:'server-owned',messages:[{metadata:{termux_codex_lane:'stale-active'}}]}})).toThrow('app_dispatch_stopped');
 expect(lane.streamActive).toBe(false);
 expect((await bridge({op:'stop',owner:'owned',lane:'stale-active'})).stopped).toBe(true);expect(posts).toBe(1);
}));

async function fixture(run){
 const root=mkdtempSync(join(process.env.TMPDIR,'app-bridge-'));
 const oldDocument=globalThis.document;
 const readSlot=Symbol.for('termux.codex.app.read-backoff.v1');
 delete globalThis[readSlot];
 const wire={async safeGet(path,options){const f=globalThis.appBridgeFixture;if(f.chatGet)return f.chatGet(path,options);if(path==='/conversation/{conversation_id}')return {gizmo_id:f.membership};return path==='/gizmos/snorlax/sidebar'?{items:f.projects.map(gizmo=>({gizmo})),cursor:null}:f.projects.find(v=>v.gizmo.id===options.parameters.path.gizmo_id_or_short_url);},async safePost(path,options){const f=globalThis.appBridgeFixture;expect(path).toBe('/projects');expect(options.retry).toBe('never');expect(options.requestBody).toEqual({name:options.requestBody.name,instructions:''});options.assertRequestCurrent();const p={gizmo:{id:'g-p-fixture',display:{name:options.requestBody.name},author:{user_id:'user'},share_recipient:'private'}};f.projects.push(p);return {resource:p};},streamPost(_path,options){const body=JSON.parse(options.body);globalThis.appBridgeFixture.sent.push(body);options.chatgptSse?.onBeforeRequestStart?.({});return true;}};
 const client={prepareCompletionStream(request){return request;},startCompletionStream(options){
  if(globalThis.appBridgeFixture.rejectStart)return Promise.reject(Error('fixture async failure'));
  globalThis.appBridgeFixture.options.push(options);
  wire.streamPost('/f/conversation',{body:JSON.stringify(options.request),chatgptSse:{}});
  options.onDecodedEvent?.({message:{id:'analysis',author:{role:'assistant'},recipient:'all',channel:'analysis',content:{content_type:'text',parts:['private reasoning']}}});
  options.onUpdate?.({message:{id:'public',author:{role:'assistant'},recipient:'all',channel:'final',content:{content_type:'text',parts:['Visible result']},status:'finished_successfully',end_turn:true}});
  options.onComplete?.({reason:'done'});return true;
 }};
 const original={...client,streamPost:wire.streamPost};
 const f=globalThis.appBridgeFixture={wire,client,sent:[],options:[],created:0,gate:true,projects:[]};
 writeFileSync(join(root,'initial.mjs'),`
 export const oMn={kind:'client'},JRn={kind:'messages'},ozn={kind:'status'};
 export function pzn(value){return ['streaming','active-async-turn','active-tpp-turn'].includes(value);}
 export function uCn(_scope,raw,options){globalThis.appBridgeFixture.hydrated={raw,options};}
 export function jLn(message){const request=message.metadata?.jit_plugin_data?.from_server;return {allowTargetMessageId:(request?.body?.actions?.find(a=>a.allow_once||a.allow)?.allow_once??request?.body?.actions?.find(a=>a.allow_once||a.allow)?.allow)?.target_message_id,model:"host",recipient:"api_tool.call_tool"};}
 export function gRt(){}
 export function aCn(){return 'owned-'+(++globalThis.appBridgeFixture.created);}
 export function eHt(){globalThis.appBridgeFixture.cancelCallback?.();}
 export async function yRt(_scope,options){
  globalThis.appBridgeFixture.approvalOptions=options.toolApproval?options:undefined;
  globalThis.appBridgeFixture.submitOptions=options;
  const approved=options.toolApproval?{id:"app-approval",author:{role:"tool"},metadata:{jit_plugin_data:{from_client:{type:options.toolApproval.action,target_message_id:options.toolApproval.targetMessageId,remember_answer:options.toolApproval.actionData?.remember_answer??false}}}}:undefined;
  const messages=[...(options.prependMessages||[]),options.userCompletionMessages?.message,approved,...(options.userCompletionMessages?.extraDeveloperInstructionMessages||[])].filter(Boolean);
  const client=globalThis.appBridgeFixture.client;
  options.onServerThreadIdChange?.('server-owned');
  const request=client.prepareCompletionStream({conversation_id:messages.some(m=>m.author.role==='tool')?'server-owned':undefined,gizmo_id:options.projectId,local_function_signatures:[{name:'handoff'}],messages});
  client.startCompletionStream({request});
  if(globalThis.appBridgeFixture.holdSubmission)await new Promise(resolve=>{globalThis.appBridgeFixture.releaseSubmission=resolve;});
  return true;
 }
 export function bRt(scope,{callId,conversationId,isTemporaryChat,model,onServerThreadIdChange,result,thinkingEffort,toolName}){return yRt(scope,{conversationId,isTemporaryChat,model,onServerThreadIdChange,thinkingEffort,userCompletionMessages:{message:{id:'tool-reply',author:{role:'tool'},content:{text:JSON.stringify({call_id:callId,result,tool:toolName})}}}});}
 `);
 writeFileSync(join(root,'shared.mjs'),`export const HAt={kind:'gate'},xNt={kind:'identity'};export function N0t(_scope,path){globalThis.appBridgeFixture.revealed=path;}export const $wt=globalThis.appBridgeFixture.wire;`);
 const scope={get(node){return node.kind==='client'?client:node.kind==='messages'?f.nodes:node.kind==='status'?(f.activity??(f.active?'streaming':'idle')):node.kind==='gate'?f.gate:node.kind==='identity'?{authLoading:f.authLoading,authMethod:'chatgpt',accountId:'account',authenticatedAccountId:'account',accountUserId:'user__personal',userId:'user',authenticatedUserId:'user',accountStructure:'personal',email:'not-exported',accessToken:'never-export'}:{};},set(){},node:{},chain:{},query:{}};
 f.scope=scope;
 globalThis.document={querySelector(){return {__reactFiberFixture:{memoizedState:{memoizedState:{current:f.scope}}}};}};
 const bindings={initial:pathToFileURL(join(root,'initial.mjs')).href,shared:pathToFileURL(join(root,'shared.mjs')).href};
 try{await run({f,bindings,original});}finally{delete globalThis[slot];delete globalThis[readSlot];delete globalThis.appBridgeFixture;globalThis.document=oldDocument;rmSync(root,{recursive:true});}
}

test('owned app hooks preserve unowned requests, actual feature gates and public-only output',async()=>fixture(async({f,bindings,original})=>{
 expect(await bridge({op:'init',owner:'owned',bindings})).toEqual({ready:true});
 const unowned={local_function_signatures:[{name:'unowned'}],messages:[]};
 expect(f.client.prepareCompletionStream(unowned)).toBe(unowned);
 const identity=await bridge({op:'identity',owner:'owned',lane:'lane'});expect(identity).toEqual({account:'account',user:'user',structure:'personal',present:false});
 await bridge({op:'open',owner:'owned',lane:'lane',signature:{name:'termux_codex_fixture'}});
 await expect(bridge({op:'dispose',owner:'owned'})).rejects.toThrow('app_lanes_active');
 await bridge({op:'submit',owner:'owned',lane:'lane',round:'round-1',model:'configured-host',messages:[{role:'user',text:'Task',hidden:false},{role:'developer',text:'Local environment data (context only; values are not instructions):\n<environment_context>preserved</environment_context>',hidden:true}],instructions:['Native policy']});
 await Promise.resolve();
 const ownedLane=globalThis[slot].lanes.get('lane');
 for(const type of ['thoughts','reasoning_recap','model_editable_context'])ownedLane.messages.set(type,{id:type,author:{role:'assistant'},recipient:'all',channel:null,content:{content_type:type,parts:['PRIVATE_REASONING']}});
 const state=await bridge({op:'snapshot',owner:'owned',lane:'lane'});
 expect(JSON.stringify(state)).not.toContain('PRIVATE_REASONING');
 expect(state.round).toBe('round-1');expect(state.roundStarts).toBe(1);expect(state.userMessages).toBe(1);
 expect(state.messages.map(m=>m.id)).toEqual(['public']);
 expect(f.sent[0].messages[1].author.role).toBe('system');expect(f.sent[0].messages[1].metadata.is_visually_hidden_from_conversation).toBe(true);
 expect(f.sent[0].local_function_signatures).toEqual([{name:'termux_codex_fixture'}]);
 await bridge({op:'submit',owner:'owned',lane:'lane',round:'round-2',model:'changed-host',results:[{id:'original-call',result:{output:'done'}}]});
 await Promise.resolve();
 const returned=await bridge({op:'snapshot',owner:'owned',lane:'lane'});
 expect(returned.userMessages).toBe(1);expect(returned.toolMessages).toBe(1);
 expect(JSON.parse(f.sent[1].messages[0].content.text).call_id).toBe('original-call');
 await bridge({op:'submit',owner:'owned',lane:'lane',round:'round-2',model:'changed-host',results:[{id:'original-call',result:{output:'done'}}]});
 expect(f.sent.length).toBe(2);
 await bridge({op:'release',owner:'owned',lane:'lane'});
 expect(f.client.prepareCompletionStream).toBe(original.prepareCompletionStream);
 expect(f.wire.streamPost).toBe(original.streamPost);
 expect(await bridge({op:'dispose',owner:'owned'})).toEqual({disposed:true});
 expect(await bridge({op:'init',owner:'new-owned',bindings})).toEqual({ready:true});
}));

test('settled saved Chat accepts ordinary user input without the native lane owning it',async()=>fixture(async({f,bindings})=>{
 await bridge({op:'init',owner:'owned',returnPath:'mcp',bindings});
 await bridge({op:'open',owner:'owned',lane:'managed',signature:{name:'native-owned'}});
 await bridge({op:'submit',owner:'owned',lane:'managed',round:'one',messages:[{role:'user',text:'Native task'}]});
 await Promise.resolve();
 const before=await bridge({op:'snapshot',owner:'owned',lane:'managed'});
 const request={conversation_id:'server-owned',messages:[{id:'ordinary-user',author:{role:'user'},content:{parts:['Continue here']}}],local_function_signatures:[{name:'ordinary-local'}]};
 expect(f.client.prepareCompletionStream(request)).toBe(request);
 f.client.startCompletionStream({request});
 expect(f.sent.at(-1)).toEqual(request);
 const after=await bridge({op:'snapshot',owner:'owned',lane:'managed'});
 expect(after.dispatches).toBe(before.dispatches);
 expect(after.roundStarts).toBe(before.roundStarts);
 // Its subsequent ordinary tool reply also belongs to that app stream.
 const reply={conversation_id:'server-owned',messages:[{id:'ordinary-tool',author:{role:'tool'},content:{parts:['result']}}]};
 f.client.startCompletionStream({request:reply});
 expect(f.sent.at(-1)).toEqual(reply);
 await bridge({op:'stop',owner:'owned',lane:'managed'});
 const next={...request,messages:[{...request.messages[0],id:'after-stop'}]};
 expect(f.client.prepareCompletionStream(next)).toBe(next);
}));

test('project operations preserve app credentials and verify membership before inference',async()=>fixture(async({f,bindings})=>{
 await bridge({op:'init',owner:'owned',bindings});
 const identity={account:'account',structure:'personal',user:'user'};
 expect(await bridge({op:'project_list',owner:'owned',identity})).toEqual([]);
 const project=await bridge({op:'project_create',owner:'owned',identity,name:'Codex'});
 expect(project).toEqual({id:'g-p-fixture',name:'Codex',compatible:true});
 expect(await bridge({op:'project_read',owner:'owned',identity,project:project.id})).toEqual(project);
 expect(await bridge({op:'project_list',owner:'owned',identity})).toEqual([project]);
 expect(f.sent.length).toBe(0);
 await expect(bridge({op:'project_list',owner:'owned',identity:{...identity,account:'other'}})).rejects.toThrow('app_account_changed');
 await bridge({op:'open',owner:'owned',lane:'project-lane',project:project.id,signature:{name:'termux_codex_project'}});
 await bridge({op:'submit',owner:'owned',lane:'project-lane',round:'first',model:'configured',messages:[{role:'user',text:'Task',hidden:false}]});
 await Promise.resolve();expect(f.sent[0].gizmo_id).toBe(project.id);
 f.membership=project.id;expect(await bridge({op:'project_membership',owner:'owned',lane:'project-lane'})).toEqual({verified:true});
 f.membership='g-p-other';await expect(bridge({op:'project_membership',owner:'owned',lane:'project-lane'})).rejects.toThrow('project_membership_unconfirmed');
 const before=f.sent.length;
 expect(()=>f.client.prepareCompletionStream({conversation_id:'server-owned',local_function_signatures:[{name:'termux_codex_project'}],messages:[]})).toThrow('project_membership_unconfirmed');
 expect(f.sent.length).toBe(before);
}));

test('only an observed host permission reply continues MCP without a new user object or native return',async()=>fixture(async({f,bindings})=>{
 await bridge({op:'init',owner:'owned',returnPath:'mcp',bindings});
 await bridge({op:'open',owner:'owned',lane:'host-action',signature:{name:'unused'}});
 await bridge({op:'submit',owner:'owned',lane:'host-action',round:'round',model:'host',messages:[{role:'user',text:'Task'}]});
 await Promise.resolve();
 const state=globalThis[slot],lane=state.lanes.get('host-action');
 lane.approvalPending=true;
 const response={id:'host-reply',author:{role:'tool'},content:{parts:['PRIVATE_USER_TEXT']},metadata:{jit_plugin_data:{from_client:{target_message_id:'host-confirm',type:'allow',private:'PRIVATE_ARGUMENT'}}}};
 f.nodes={response:{message:response}};
 expect((await bridge({op:'snapshot',owner:'owned',lane:'host-action'})).hostResponses).toEqual([]);
 const send=message=>f.client.startCompletionStream({request:{conversation_id:'server-owned',messages:[message]}});
 expect(()=>send(response)).toThrow('app_dispatch_replayed');
 lane.messages.set('host-confirm',{id:'host-confirm',author:{role:'tool'},metadata:{jit_plugin_data:{from_server:{type:'confirm_action'}}}});
 expect(()=>send({...response,id:'unrelated',metadata:{jit_plugin_data:{from_client:{target_message_id:'wrong'}}}})).toThrow('app_dispatch_replayed');
 send(response);
 const snapshot=await bridge({op:'snapshot',owner:'owned',lane:'host-action'});
 expect(snapshot.hostResponses).toEqual(['host-confirm']);
 expect(snapshot.dispatches).toBe(2);expect(snapshot.userMessages).toBe(1);
 expect(snapshot.hostReplyMessages).toBe(1);expect(snapshot.roundHostReplies).toBe(1);expect(snapshot.toolMessages).toBe(0);
 expect(snapshot.roundStarts).toBe(2);expect(snapshot.terminals).toEqual(['done']);
 lane.streamActive=true;lane.terminals=[];f.options[0].onComplete({reason:'done'});f.options[0].onError(new Error('old stream'));
 expect(lane.streamActive).toBe(true);expect(lane.terminals).toEqual([]);expect(lane.error).toBe(null);
 f.options.at(-1).onComplete({reason:'done'});expect(lane.streamActive).toBe(false);
 expect(()=>send(response)).toThrow('app_dispatch_replayed');
 expect(()=>send({...response,id:'duplicate-target'})).toThrow('app_dispatch_replayed');
 expect(()=>send({id:'native-result',author:{role:'tool'},metadata:{termux_codex_lane:'host-action'},content:{text:'native result'}})).toThrow('app_dispatch_replayed');
 expect(JSON.stringify(snapshot)).not.toContain('PRIVATE_USER_TEXT');
 expect(JSON.stringify(snapshot)).not.toContain('PRIVATE_ARGUMENT');
 lane.messages.set('stopped-confirm',{id:'stopped-confirm',author:{role:'tool'},metadata:{jit_plugin_data:{from_server:{type:'confirm_action'}}}});
 lane.stopRequested=true;
 expect(()=>send({...response,id:'late-reply',metadata:{jit_plugin_data:{from_client:{target_message_id:'stopped-confirm',type:'allow'}}}})).toThrow('app_dispatch_stopped');
 expect(f.sent.length).toBe(2);
}));

test('MCP approval follows the server-offered invoking-message target, without accepting unrelated ancestors',async()=>fixture(async({f,bindings})=>{
 await bridge({op:'init',owner:'owned',returnPath:'mcp',bindings});
 await bridge({op:'open',owner:'owned',lane:'invocation',signature:{name:'unused'}});
 await bridge({op:'submit',owner:'owned',lane:'invocation',round:'round',model:'host',messages:[{role:'user',text:'Task'}]});
 await Promise.resolve();
 const lane=globalThis[slot].lanes.get('invocation');
 lane.approvalPending=true;
 lane.messages.set('invoking',{id:'invoking',author:{role:'assistant'},content:{content_type:'code'}});
 lane.messages.set('confirm',{id:'confirm',author:{role:'tool'},metadata:{parent_id:'invoking',jit_plugin_data:{from_server:{type:'confirm_action',body:{actions:[{allow_once:{target_message_id:'invoking'}}]}}}}});
 const reply={id:'reply',author:{role:'tool'},metadata:{jit_plugin_data:{from_client:{type:'allow',target_message_id:'invoking'}}}};
 const send=message=>f.client.startCompletionStream({request:{conversation_id:'server-owned',messages:[message]}});
 expect(()=>send({...reply,metadata:{jit_plugin_data:{from_client:{target_message_id:'unrelated'}}}})).toThrow('app_dispatch_replayed');
 expect(lane.streamActive).toBe(false);expect(lane.error).toBe('app_stream_failed');
 f.nodes={reply:{message:reply}};
 send(reply);
 const snapshot=await bridge({op:'snapshot',owner:'owned',lane:'invocation'});
 expect(snapshot.hostResponses).toEqual(['confirm']);expect(snapshot.hostReplyMessages).toBe(1);
 expect(snapshot.userMessages).toBe(1);expect(snapshot.error).toBe(null);expect(snapshot.toolMessages).toBe(0);
 expect(()=>send({...reply,id:'again',metadata:{jit_plugin_data:{from_client:{type:'deny',target_message_id:'confirm'}}}})).toThrow('app_dispatch_replayed');
 expect(f.sent.length).toBe(2);
}));

test('a new bridge revision refuses old hooks without replacing active or idle lanes',async()=>fixture(async({bindings})=>{
 await bridge({op:'init',owner:'owned',bindings});
 const original=globalThis[slot];
 await expect(bridge({op:'init',owner:'owned',bindings,bridgeRevision:'2'.repeat(64)})).rejects.toThrow('app_bridge_revision_conflict');
 expect(globalThis[slot]).toBe(original);
 await bridge({op:'open',owner:'owned',lane:'active',signature:{name:'owned'}});
 await expect(bridge({op:'init',owner:'owned',bindings,bridgeRevision:'2'.repeat(64)})).rejects.toThrow('app_bridge_revision_conflict');
 expect(globalThis[slot]).toBe(original);expect(original.lanes.size).toBe(1);
 await expect(rawBridge({op:'init',owner:'owned',bindings})).rejects.toThrow('app_bridge_revision_required');
 await bridge({op:'release',owner:'owned',lane:'active'});await bridge({op:'dispose',owner:'owned'});
 await bridge({op:'init',owner:'owned',bindings,bridgeRevision:'2'.repeat(64)});
 expect(globalThis[slot].revision).toBe('2'.repeat(64));
}));

test('reveal selects the exact owned local conversation without inference or host approval',async()=>fixture(async({f,bindings})=>{
 await bridge({op:'init',owner:'owned',bindings});
 await bridge({op:'open',owner:'owned',lane:'chosen',signature:{name:'owned'}});
 globalThis[slot].lanes.get('chosen').serverId='exact-server';
 const identity={account:'account',user:'user',structure:'personal'};
 await expect(bridge({op:'reveal',owner:'owned',lane:'missing',identity})).rejects.toThrow('app_lane_missing');
 await expect(bridge({op:'reveal',owner:'owned',lane:'chosen',identity:{...identity,account:'other'}})).rejects.toThrow('app_account_changed');
 expect(f.revealed).toBeUndefined();
 expect(await bridge({op:'reveal',owner:'owned',lane:'chosen',identity})).toEqual({requested:true});
 expect(f.revealed).toBe('/c/exact-server');expect(f.sent.length).toBe(0);expect(f.created).toBe(1);
}));

test('owned private project keeps its memory scope while ownership and sharing still gate admission',async()=>fixture(async({f,bindings})=>{
 await bridge({op:'init',owner:'owned',bindings});
 const identity={account:'account',structure:'personal',user:'user'};
 const gizmo={id:'g-p-memory',display:{name:'Codex'},author:{user_id:'user'},share_recipient:'private',memory_scope:'project_v2'};
 f.projects.push({gizmo});
 const read=()=>bridge({op:'project_read',owner:'owned',identity,project:gizmo.id});
 expect((await read()).compatible).toBe(true);
 expect(gizmo.memory_scope).toBe('project_v2');
 gizmo.share_recipient='shared';expect((await read()).compatible).toBe(false);
 gizmo.share_recipient='private';gizmo.author.user_id='other';expect((await read()).compatible).toBe(false);
 gizmo.author.user_id='user';gizmo.sharing={subjects:[{id:'other'}]};expect((await read()).compatible).toBe(false);
 expect(f.sent.length).toBe(0);expect(f.projects.length).toBe(1);
}));

test('app feature gate is required and owner conflicts do not allocate conversations',async()=>fixture(async({f,bindings})=>{
 f.gate=false;await expect(bridge({op:'init',owner:'owned',bindings})).rejects.toThrow('app_local_functions_unavailable');
 expect(f.created).toBe(0);f.gate=true;
 await bridge({op:'init',owner:'owned',bindings});
 await expect(bridge({op:'init',owner:'other',bindings})).rejects.toThrow('app_bridge_owned');
 expect(f.created).toBe(0);
}));


test('opening and release reconcile lost replies and reattach the same owned app',async()=>fixture(async({f,bindings,original})=>{
 await bridge({op:'init',owner:'owned',bindings});
 const open={op:'open',owner:'owned',lane:'lane',signature:{name:'termux_codex_fixture'}};
 const first=await bridge(open);expect(await bridge(open)).toEqual(first);expect(f.created).toBe(1);
 await expect(bridge({...open,signature:{name:'changed'}})).rejects.toThrow('app_lane_declaration_changed');
 await bridge({op:'release',owner:'owned',lane:'lane'});
 await bridge({op:'release',owner:'owned',lane:'lane'});
 expect(f.client.prepareCompletionStream).toBe(original.prepareCompletionStream);
 await bridge({...open,lane:'next'});
 await bridge({op:'submit',owner:'owned',lane:'next',round:'new',model:'configured-host',messages:[{role:'user',text:'Resume latest',hidden:false}]});
 await Promise.resolve();expect(f.sent).toHaveLength(1);expect(f.created).toBe(2);
 expect((await bridge({op:'snapshot',owner:'owned',lane:'next'})).roundStarts).toBe(1);
}));

test('a terminal does not settle a submission whose app acceptance is still pending',async()=>fixture(async({f,bindings})=>{
 await bridge({op:'init',owner:'owned',returnPath:'mcp',bindings});
 await bridge({op:'open',owner:'owned',lane:'pending',signature:{name:'operator_no_local_tools'}});
 f.holdSubmission=true;
 await bridge({op:'submit',owner:'owned',lane:'pending',round:'first',model:'configured',messages:[{role:'user',text:'First'}]});
 const partial=await bridge({op:'snapshot',owner:'owned',lane:'pending'});
 expect(partial.terminals).toEqual(['done']);expect(partial.streamActive).toBe(false);expect(partial.pending).toBe(true);expect(partial.accepted).toBe(false);
 await expect(bridge({op:'submit',owner:'owned',lane:'pending',round:'second',model:'configured',messages:[{role:'user',text:'Follow-up'}]})).rejects.toThrow('app_lane_busy');
 expect(f.sent).toHaveLength(1);
 f.holdSubmission=false;f.releaseSubmission();await Promise.resolve();await Promise.resolve();
 const settled=await bridge({op:'snapshot',owner:'owned',lane:'pending'});
 expect(settled.pending).toBe(false);expect(settled.streamActive).toBe(false);expect(settled.accepted).toBe(true);
 await bridge({op:'submit',owner:'owned',lane:'pending',round:'second',model:'configured',messages:[{role:'user',text:'Follow-up'}]});
 expect(f.sent).toHaveLength(2);expect(f.created).toBe(1);
}));


for(const status of ['completed','failed','interrupted'])test('submission recovery observes '+status+' without replaying a stream',async()=>fixture(async({f,bindings})=>{
 f.client.startCompletionStream=options=>{
  f.options.push(options);f.wire.streamPost('/f/conversation',{body:JSON.stringify(options.request),chatgptSse:{}});return true;
 };
 await bridge({op:'init',owner:'owned',returnPath:'mcp',bindings});
 await bridge({op:'open',owner:'owned',lane:'recovered',signature:{name:'unused'}});
 const submit=round=>bridge({op:'submit',owner:'owned',lane:'recovered',round,messages:[{role:'user',text:'Task'}]});
 await submit('first');await Promise.resolve();
 const raw=f.options[0],completed=f.submitOptions.onCompletion;
 raw.onStreamErrorObserved({errorKind:'network',error:'PRIVATE ERROR',diagnostics:{request_url:'SECRET_URL'}});
 raw.onResume({type:'error',attemptCount:1,error:{responseStatus:404,error:'PRIVATE ERROR'},isNetworkError:false});
 let snapshot=await bridge({op:'snapshot',owner:'owned',lane:'recovered'});
 expect(snapshot.terminals).toEqual([]);expect(snapshot.streamActive).toBe(true);
 expect(snapshot.transportObservation).toEqual({first_error:{stage:'stream',http_status:null,network:true},resume:{stage:'error',attempt:1}});
 // The app hydrates the saved final and invokes its high-level completion,
 // without invoking raw.onComplete. Public text alone cannot settle the lane.
 f.nodes={final:{message:{id:'final',author:{role:'assistant'},recipient:'all',channel:'final',content:{content_type:'text',parts:['Recovered answer']},status:'finished_successfully',end_turn:true}}};
 expect((await bridge({op:'snapshot',owner:'owned',lane:'recovered'})).terminals).toEqual([]);
 completed(status);
 snapshot=await bridge({op:'snapshot',owner:'owned',lane:'recovered'});
 expect(snapshot.streamActive).toBe(false);
 expect(snapshot.terminals).toEqual(status==='completed'?['done']:[]);
 expect(snapshot.error).toBe(status==='completed'?null:status==='failed'?'app_completion_failed':'app_completion_interrupted');
 expect(f.sent).toHaveLength(1);
 await submit('second');await Promise.resolve();
 completed('completed');raw.onResume({type:'error',attemptCount:9,error:{responseStatus:503}});
 const next=await bridge({op:'snapshot',owner:'owned',lane:'recovered'});
 expect(next.round).toBe('second');expect(next.terminals).toEqual([]);expect(next.streamActive).toBe(true);expect(next.transportObservation).toBe(null);
 f.submitOptions.onCompletion('completed');
 await bridge({op:'release',owner:'owned',lane:'recovered'});
}));

test('single local tool result retains submission completion after SDK recovery',async()=>fixture(async({f,bindings})=>{
 f.client.startCompletionStream=options=>{f.options.push(options);f.wire.streamPost('/f/conversation',{body:JSON.stringify(options.request),chatgptSse:{}});return true;};
 await bridge({op:'init',owner:'owned',returnPath:'local_function',bindings});
 await bridge({op:'open',owner:'owned',lane:'result-recovery',signature:{name:'native_gateway'}});
 await bridge({op:'submit',owner:'owned',lane:'result-recovery',round:'one',results:[{id:'original-call',result:{output:'real result'}}]});await Promise.resolve();
 expect(f.sent).toHaveLength(1);expect(f.sent[0].messages).toHaveLength(1);
 expect(f.sent[0].messages[0].author.role).toBe('tool');
 expect(JSON.parse(f.sent[0].messages[0].content.text)).toEqual({call_id:'original-call',result:{output:'real result'},tool:'native_gateway'});
 expect((await bridge({op:'snapshot',owner:'owned',lane:'result-recovery'})).terminals).toEqual([]);
 f.submitOptions.onCompletion('completed');
 const done=await bridge({op:'snapshot',owner:'owned',lane:'result-recovery'});
 expect(done.terminals).toEqual(['done']);expect(done.streamActive).toBe(false);expect(f.sent).toHaveLength(1);
 await bridge({op:'release',owner:'owned',lane:'result-recovery'});
}));

test('initial auth loading installs no hooks and identity refresh is read-only',async()=>fixture(async({f,bindings,original})=>{
 f.authLoading=true;expect(await bridge({op:'init',owner:'owned',bindings})).toEqual({ready:false});
 expect(f.client.prepareCompletionStream).toBe(original.prepareCompletionStream);expect(f.created).toBe(0);
 f.authLoading=false;expect(await bridge({op:'init',owner:'owned',bindings})).toEqual({ready:true});
 f.authLoading=true;await expect(bridge({op:'identity',owner:'owned',lane:'not-opened'})).rejects.toThrow('app_auth_loading');
 f.authLoading=false;expect((await bridge({op:'identity',owner:'owned',lane:'not-opened'})).account).toBe('account');
 expect(f.sent).toHaveLength(0);expect(f.created).toBe(0);
}));

for(const replaceClient of [false,true])test(`login scope replacement uses the current account, client replaced=${replaceClient}`,async()=>fixture(async({f,bindings,original})=>{
 await bridge({op:'init',owner:'owned',returnPath:'mcp',bindings});
 await bridge({op:'open',owner:'owned',lane:'old',signature:{name:'old_gateway'}});
 const old=globalThis[slot],scope=f.scope;
 if(replaceClient)f.client={prepareCompletionStream:original.prepareCompletionStream,startCompletionStream:original.startCompletionStream};
 const auth={authMethod:'chatgpt',accountId:'second-account',authenticatedAccountId:'second-account',userId:'second-user',authenticatedUserId:'second-user',accountStructure:'workspace'};
 f.scope={...scope,get(node){return node.kind==='identity'?auth:node.kind==='client'?f.client:scope.get(node);}};
 expect((await bridge({op:'identity',owner:'owned',lane:'old',readOnlyContext:true,bindings})).account).toBe('second-account');
 await expect(bridge({op:'submit',owner:'owned',lane:'old',round:'race',model:'configured',messages:[{role:'user',text:'Must not reach the previous account'}]})).rejects.toThrow('app_account_changed');
 expect(f.sent).toHaveLength(0);
 // An account transition must not take hooks away from active work.
 old.lanes.get('old').pending=true;
 await expect(bridge({op:'init',owner:'owned',returnPath:'mcp',bindings})).rejects.toThrow('app_lanes_active');
 expect(globalThis[slot]).toBe(old);expect(f.sent).toHaveLength(0);
 old.lanes.get('old').pending=false;
 expect(await bridge({op:'init',owner:'owned',returnPath:'mcp',bindings})).toEqual({ready:true});
 await expect(bridge({op:'open',owner:'owned',lane:'wrong',signature:{name:'wrong_gateway'},identity:{account:'account',user:'user',structure:'personal'}})).rejects.toThrow('app_account_changed');
 await bridge({op:'open',owner:'owned',lane:'current',signature:{name:'current_gateway'}});
 expect(globalThis[slot].scope).toBe(f.scope);
 expect(globalThis[slot].lanes.get('current').accountIdentity).toEqual({account:'second-account',user:'second-user',structure:'workspace'});
 expect(globalThis[slot].lanes.has('old')).toBe(false);
 expect(f.created).toBe(2);expect(f.sent).toHaveLength(0);
 // The same renderer may switch back without reviving an old remote lane.
 f.scope=scope;
 await bridge({op:'init',owner:'owned',returnPath:'mcp',bindings});
 await bridge({op:'open',owner:'owned',lane:'returned',signature:{name:'return_gateway'}});
 expect(globalThis[slot].lanes.get('returned').accountIdentity.account).toBe('account');
 expect(globalThis[slot].lanes.size).toBe(1);expect(f.sent).toHaveLength(0);
}));

test('an account change after submit admission blocks the actual conversation POST',async()=>fixture(async({f,bindings})=>{
 await bridge({op:'init',owner:'owned',returnPath:'mcp',bindings});
 await bridge({op:'open',owner:'owned',lane:'race',signature:{name:'race_gateway'}});
 const scope=f.scope,start=f.client.startCompletionStream;
 f.client.startCompletionStream=function(...args){
  f.scope={...scope,get(node){return node.kind==='identity'?{authMethod:'chatgpt',accountId:'other',authenticatedAccountId:'other',userId:'other-user',authenticatedUserId:'other-user',accountStructure:'personal'}:scope.get(node);}};
  return start.apply(this,args);
 };
 await bridge({op:'submit',owner:'owned',lane:'race',round:'one',model:'configured',messages:[{role:'user',text:'Do not cross account contexts'}]});
 await Promise.resolve();
 const lane=globalThis[slot].lanes.get('race');
 expect(lane.rejectedBeforeDispatch).toBe(true);expect(lane.admissionCause).toBe('app_account_changed');
 expect(lane.starts.size).toBe(0);expect(f.sent).toHaveLength(0);
}));

test('MCP mode uses no local function feature gate or result resubmission',async()=>fixture(async({f,bindings,original})=>{
 f.gate=false;
 expect(await bridge({op:'init',owner:'owned',returnPath:'mcp',bindings})).toEqual({ready:true});
 expect(await bridge({op:'init',owner:'owned',returnPath:'local_function',bindings})).toEqual({ready:true});
 await expect(bridge({op:'open',owner:'owned',lane:'unavailable-local',returnPath:'local_function',signature:{name:'local_gateway'}})).rejects.toThrow('app_local_functions_unavailable');
 await bridge({op:'open',owner:'owned',lane:'mcp-lane',signature:{name:'virtual_gateway'}});
 const command={op:'submit',owner:'owned',lane:'mcp-lane',round:'first',model:'configured',messages:[{role:'user',text:'Owned task',hidden:false}],instructions:['MCP native turn policy']};
 await bridge(command);await Promise.resolve();
 expect(f.sent.length).toBe(1);expect(f.sent[0].local_function_signatures).toEqual([]);expect(f.sent[0].local_function_names).toEqual([]);
 const observed=await bridge({op:'snapshot',owner:'owned',lane:'mcp-lane'});
 expect(observed.dispatches).toBe(1);expect(observed.userMessages).toBe(1);expect(observed.toolMessages).toBe(0);
 await expect(bridge({...command,round:'forbidden',results:[{id:'original-mcp-call',result:'must use retained HTTP response'}]})).rejects.toThrow('app_mcp_result_resubmit_forbidden');
 expect(f.sent.length).toBe(1);expect((await bridge({op:'snapshot',owner:'owned',lane:'mcp-lane'})).round).toBe('first');
 await bridge({op:'release',owner:'owned',lane:'mcp-lane'});
 expect(f.client.prepareCompletionStream).toBe(original.prepareCompletionStream);
}));

test('one hook owner isolates simultaneous MCP and local declarations per lane',async()=>fixture(async({f,bindings,original})=>{
 await bridge({op:'init',owner:'owned',returnPath:'mcp',bindings});
 const hook=f.client.prepareCompletionStream;
 await bridge({op:'open',owner:'owned',lane:'mcp',returnPath:'mcp',signature:{name:'mcp_gateway'}});
 await bridge({op:'open',owner:'owned',lane:'local',returnPath:'local_function',signature:{name:'local_gateway'}});
 expect(f.client.prepareCompletionStream).toBe(hook);
 await expect(bridge({op:'open',owner:'owned',lane:'mcp',returnPath:'local_function',signature:{name:'mcp_gateway'}})).rejects.toThrow('app_lane_declaration_changed');
 for(const lane of ['mcp','local']) {
  await bridge({op:'submit',owner:'owned',lane,round:'first',messages:[{role:'user',text:lane}],instructions:[]});
  await Promise.resolve();
 }
 expect(f.sent[0].local_function_signatures).toEqual([]);
 expect(f.sent[1].local_function_signatures).toEqual([{name:'local_gateway'}]);
 await bridge({op:'release',owner:'owned',lane:'mcp'});
 expect(f.client.prepareCompletionStream).toBe(hook);
 await bridge({op:'release',owner:'owned',lane:'local'});
 expect(f.client.prepareCompletionStream).toBe(original.prepareCompletionStream);
}));

test('saved Chat APIs read unrelated sources and host state without opening lanes or submitting inference',async()=>fixture(async({f,bindings})=>{
 await bridge({op:'init',owner:'owned',returnPath:'mcp',bindings});
 const calls=[];
 f.chatGet=async(path,options)=>{
  calls.push({path,options});
  if(path==='/conversations')return {items:[{id:'saved-one',title:'same'},{id:'saved-two',title:'same'}],total:2};
  return {conversation_id:'unrelated',title:'Unrelated',current_node:'public',mapping:{
   hidden:{id:'hidden',parent:null,children:['public'],message:{id:'hidden',author:{role:'assistant'},channel:'analysis',content:{content_type:'text',parts:['NEVER_EXPORT']},metadata:{accessToken:'NEVER_EXPORT'}}},
   public:{id:'public',parent:'hidden',children:[],message:{id:'public',author:{role:'assistant'},channel:'final',content:{content_type:'text',parts:['saved public']}}}
  }};
 };
 const list=await bridge({op:'chat_list',owner:'owned',offset:0,limit:2});
 expect(list.items.map(i=>i.id)).toEqual(['saved-one','saved-two']);
 const read=await bridge({op:'chat_read',owner:'owned',conversation_id:'unrelated',identity:{account:'account',structure:'personal',user:'user'}});
 expect(read.mapping.public.parent).toBe('hidden');
 expect(read.mapping.public.message.content.parts).toEqual(['saved public']);
 expect(read.mapping.hidden.message.unavailable).toBe('hidden');
 expect(JSON.stringify(read)).not.toContain('NEVER_EXPORT');
 for(const call of calls){
  expect(call.options.retry).toBe('never');
  expect(call.options.additionalHeaders).toEqual({'ChatGPT-Account-ID':'account'});
  expect(call.options.parameters?.query?.include_full_conversation).toBeUndefined();
 }
 const host=await bridge({op:'host_status',owner:'owned'});expect(host.active_lanes).toEqual([]);
 expect(f.created).toBe(0);expect(f.sent).toEqual([]);
 await expect(bridge({op:'chat_read',owner:'owned',conversation_id:'unrelated',identity:{account:'other'}})).rejects.toThrow('app_account_changed');
}));

test('model-free reading neither installs nor replaces another live bridge owner',async()=>fixture(async({f,bindings,original})=>{
 f.chatGet=async()=>({items:[],total:0});f.gate=false;
 const request={op:'chat_list',owner:'reader',readOnlyContext:true,bindings};
 expect(await bridge(request)).toEqual({items:[],total:0});
 expect(globalThis[slot]).toBeUndefined();expect(f.client.prepareCompletionStream).toBe(original.prepareCompletionStream);
 await bridge({op:'init',owner:'writer',returnPath:'mcp',bindings});
 const state=globalThis[slot],hook=f.client.prepareCompletionStream;
 expect(await bridge(request)).toEqual({items:[],total:0});
 expect(globalThis[slot]).toBe(state);expect(f.client.prepareCompletionStream).toBe(hook);
 await expect(bridge({...request,op:'submit'})).rejects.toThrow('app_operation_not_read_only');
 expect(f.created).toBe(0);expect(f.sent).toEqual([]);
}));

test('one native assignment budget spans successor lanes and rejects excess starts before dispatch',async()=>fixture(async({f,bindings})=>{
 await bridge({op:'init',owner:'owned',returnPath:'mcp',bindings});
 const limits={maxDispatches:1,deadlineUnixMs:Date.now()+60000};
 for(const lane of ['source','successor'])await bridge({op:'open',owner:'owned',nativeThread:'same-thread',lane,limits,signature:{name:lane}});
 await bridge({op:'submit',owner:'owned',lane:'source',round:'one',messages:[{role:'user',text:'first'}]});await Promise.resolve();
 expect(f.sent).toHaveLength(1);
 await bridge({op:'submit',owner:'owned',lane:'successor',round:'two',messages:[{role:'user',text:'second'}]});await Promise.resolve();
 expect(f.sent).toHaveLength(1);
 const status=await bridge({op:'snapshot',owner:'owned',lane:'successor'});
 expect(status.rejectedBeforeDispatch).toBe(true);expect(status.budget.reserved).toBe(1);
 expect(status.error).toBe('assignment_app_budget_exhausted');
 await expect(bridge({op:'open',owner:'owned',nativeThread:'same-thread',lane:'increase',limits:{...limits,maxDispatches:2},signature:{name:'increase'}})).rejects.toThrow('assignment_budget_changed');
}));


for(const returnPath of ['mcp','local_function'])test(`fresh ${returnPath} operator bootstrap is non-user and requires its explicit lane admission`,async()=>fixture(async({f,bindings})=>{
 await bridge({op:'init',owner:'owned',returnPath,bindings});
 await bridge({op:'open',owner:'owned',lane:'successor',signature:{name:returnPath==='mcp'?'operator_no_local_tools':'operator_local_'+'a'.repeat(48)}});
 await bridge({op:'submit',owner:'owned',lane:'successor',round:'handoff',model:'configured',bootstrap:true,messages:[{role:'developer',text:'Attach through the original MCP control return.',hidden:true}],instructions:['Same native work; read its current attachment.']});
 await Promise.resolve();
 expect(f.sent.length).toBe(1);
 expect(f.sent[0].messages.every(m=>m.author.role==='system')).toBe(true);
 const observed=await bridge({op:'snapshot',owner:'owned',lane:'successor'});
 expect(observed.userMessages).toBe(0);expect(observed.dispatches).toBe(1);expect(observed.error).toBe(null);
 // A settled native summary may also wake the same Chat without a user turn.
 await bridge({op:'submit',owner:'owned',lane:'successor',round:'summary',model:'configured',bootstrap:true,messages:[{role:'developer',text:'Read the pending native summary request through MCP.',hidden:true}]});
 await Promise.resolve();
 const summary=await bridge({op:'snapshot',owner:'owned',lane:'successor'});
 expect(summary.dispatches).toBe(2);expect(summary.userMessages).toBe(0);expect(summary.error).toBe(null);
}));

test('distinct local operator declarations keep overlapping native work submissions separate',async()=>fixture(async({f,bindings})=>{
 await bridge({op:'init',owner:'owned',bindings});
 const first='operator_local_'+'a'.repeat(48),second='operator_local_'+'b'.repeat(48);
 await bridge({op:'open',owner:'owned',lane:'first-work',signature:{name:first}});
 await bridge({op:'open',owner:'owned',lane:'second-work',signature:{name:second}});
 for(const lane of ['first-work','second-work']){
  await bridge({op:'submit',owner:'owned',lane,round:'initial',model:'configured',messages:[{role:'user',text:lane}]});
  await Promise.resolve();
 }
 expect(f.sent.map(v=>v.local_function_signatures[0].name)).toEqual([first,second]);
 for(const lane of ['first-work','second-work']){
  const observed=await bridge({op:'snapshot',owner:'owned',lane});
  expect(observed.dispatches).toBe(1);expect(observed.userMessages).toBe(1);expect(observed.error).toBe(null);
 }
}));

test('locally rejected non-user submit has zero dispatches and confirms Stop after settling',async()=>fixture(async({f,bindings})=>{
 await bridge({op:'init',owner:'owned',returnPath:'mcp',bindings});
 await bridge({op:'open',owner:'owned',lane:'not-admitted',signature:{name:'operator_no_local_tools'}});
 await bridge({op:'submit',owner:'owned',lane:'not-admitted',round:'bad-bootstrap',model:'configured',messages:[{role:'developer',text:'Not admitted.',hidden:true}]});
 await Promise.resolve();
 const observed=await bridge({op:'snapshot',owner:'owned',lane:'not-admitted'});
 expect(observed.dispatches).toBe(0);expect(observed.userMessages).toBe(0);expect(f.sent.length).toBe(0);
 expect(observed.rejectedBeforeDispatch).toBe(true);expect(observed.error).toBe('app_submit_failed');expect(observed.stopped).toBe(false);
 const stopped=await bridge({op:'stop',owner:'owned',lane:'not-admitted'});
 expect(stopped.stopped).toBe(true);
 await bridge({op:'release',owner:'owned',lane:'not-admitted'});
 expect(globalThis[slot].lanes.size).toBe(0);
}));


test('rejecting a new dispatch cannot confirm an older unobserved stream stopped',async()=>fixture(async({f,bindings})=>{
 await bridge({op:'init',owner:'owned',returnPath:'mcp',bindings});
 await bridge({op:'open',owner:'owned',lane:'unsettled',signature:{name:'operator_no_local_tools'}});
 globalThis[slot].lanes.get('unsettled').streamActive=true;
 await expect(bridge({op:'submit',owner:'owned',lane:'unsettled',round:'rejected-new',model:'configured',messages:[{role:'developer',text:'Not admitted.',hidden:true}]})).rejects.toThrow('app_lane_busy');
 await Promise.resolve();
 const stopped=await bridge({op:'stop',owner:'owned',lane:'unsettled'});
 expect(stopped.dispatches).toBe(0);expect(stopped.rejectedBeforeDispatch).toBe(false);expect(stopped.stopped).toBe(false);expect(f.sent.length).toBe(0);
}));


test('delegated one-time app action resumes the original MCP lane without revealing a window',async()=>fixture(async({f,bindings})=>{
 await bridge({op:'init',owner:'owned',returnPath:'mcp',bindings});
 await bridge({op:'open',owner:'owned',lane:'permission',signature:{name:'unused'}});
 await bridge({op:'submit',owner:'owned',lane:'permission',round:'round',model:'host',messages:[{role:'user',text:'Task'}]});
 await Promise.resolve();
 const lane=globalThis[slot].lanes.get('permission');
 const prompt={id:'confirm',author:{role:'tool'},metadata:{jit_plugin_data:{from_server:{type:'confirm_action',body:{connector_name:'Owned connector',actions:[{name:'allow',allow:{target_message_id:'invoke'}}]}}}}};
 lane.messages.set(prompt.id,prompt);
 const command={op:'approve',owner:'owned',lane:'permission',connector:'Owned connector',identity:{account:'account',user:'user',structure:'personal'}};
 await expect(bridge({...command,identity:{...command.identity,account:'other'}})).rejects.toThrow('app_account_changed');
 await expect(bridge({...command,connector:'Other connector'})).rejects.toThrow('app_approval_manual_required');
 const other={...prompt,id:'other'};lane.messages.set(other.id,other);
 await expect(bridge(command)).rejects.toThrow('app_approval_ambiguous');lane.messages.delete(other.id);
 prompt.metadata.jit_plugin_data.from_server.body.codex_mcp_elicitation={kind:'openaiForm'};
 await expect(bridge(command)).rejects.toThrow('app_approval_manual_required');delete prompt.metadata.jit_plugin_data.from_server.body.codex_mcp_elicitation;
 expect(f.sent.length).toBe(1);
 expect(await bridge(command)).toEqual({requested:true,scope:'once',path:'action'});
 expect(f.approvalOptions.toolApproval).toEqual({action:'allow',targetMessageId:'invoke',recipient:'api_tool.call_tool'});
 expect(f.approvalOptions.prompt).toBe('');expect(f.revealed).toBeUndefined();
 const snapshot=await bridge({op:'snapshot',owner:'owned',lane:'permission'});
 expect(snapshot.dispatches).toBe(2);expect(snapshot.userMessages).toBe(1);expect(snapshot.hostReplyMessages).toBe(1);expect(snapshot.toolMessages).toBe(0);
 await expect(bridge(command)).rejects.toThrow('app_approval_ambiguous');
 lane.hostReplies.clear();await expect(bridge(command)).rejects.toThrow('app_approval_unconfirmed');
 lane.stopRequested=true;await expect(bridge(command)).rejects.toThrow('app_approval_unavailable');
 expect(f.sent.length).toBe(2);
}));

test('conversation scope is preferred over once and a successful approval never sends both',async()=>fixture(async({f,bindings})=>{
 await bridge({op:'init',owner:'owned',returnPath:'mcp',bindings});
 await bridge({op:'open',owner:'owned',lane:'scope',signature:{name:'unused'}});
 await bridge({op:'submit',owner:'owned',lane:'scope',round:'round',model:'host',messages:[{role:'user',text:'Task'}]});await Promise.resolve();
 const state=globalThis[slot],lane=state.lanes.get('scope');
 const action={type:'allow',target_message_id:'scoped-invoke',remember_answer:true};
 lane.messages.set('confirm',{id:'confirm',author:{role:'tool'},metadata:{jit_plugin_data:{from_server:{type:'confirm_action',body:{connector_name:'Owned connector',actions:[{name:'allow',allow:{target_message_id:'once-invoke'},split_action_options:[{label:'Allow Owned connector for this conversation',action}]}]}}}}});
 const command={op:'approve',owner:'owned',lane:'scope',connector:'Owned connector',identity:{account:'account',user:'user',structure:'personal'}};
 expect(await bridge(command)).toEqual({requested:true,scope:'conversation',path:'action'});
 expect(f.approvalOptions.toolApproval.actionData).toEqual(action);
 expect(f.sent[1].messages[0].metadata.jit_plugin_data.from_client.remember_answer).toBe(true);
 expect(f.sent.length).toBe(2);
 await expect(bridge({...command,op:'approve_ui'})).rejects.toThrow('app_approval_ambiguous');expect(f.sent.length).toBe(2);
}));

test('visible fallback clicks the offered conversation menu item, or once only when that scope is absent',async()=>{
 for(const [conversation,alreadyOpen,delayed] of [[true,false,false],[true,true,false],[false,false,false],[true,false,true],[true,false,'stop'],[true,false,'render'],[true,false,'render-stop']])await fixture(async({f,bindings})=>{
  await bridge({op:'init',owner:'owned',returnPath:'mcp',bindings});await bridge({op:'open',owner:'owned',lane:'ui',signature:{name:'unused'}});
  await bridge({op:'submit',owner:'owned',lane:'ui',round:'round',model:'host',messages:[{role:'user',text:'Task'}]});await Promise.resolve();
  const state=globalThis[slot],lane=state.lanes.get('ui'),label='Allow Owned connector for this conversation';
  const action={type:'allow',target_message_id:'invoke',remember_answer:true};
  lane.messages.set('confirm',{id:'confirm',author:{role:'tool'},metadata:{jit_plugin_data:{from_server:{type:'confirm_action',body:{connector_name:'Owned connector',actions:[{name:'allow',allow:{target_message_id:'invoke'},...(conversation?{split_action_options:[{label,action}]}:{})}]}}}}});
  let menus=alreadyOpen?1:0,scopedClicks=0,onceClicks=0,focused=0,delayedError; const pointerEvents=[];
  const node=(text,attributes={})=>({textContent:text,getAttribute:k=>attributes[k]??null,hasAttribute:k=>k in attributes,getBoundingClientRect:()=>({width:100,height:30})});
  const send=remember=>f.client.startCompletionStream({request:{conversation_id:'ui-alias',messages:[{id:'actual-ui-handler',author:{role:'tool'},metadata:{jit_plugin_data:{from_client:{type:'allow',target_message_id:'invoke',remember_answer:remember}}}}]}});
  const option={...node(label),click(){scopedClicks++;if(delayed&&delayed!=='render')setTimeout(()=>{try{send(true);}catch(error){delayedError=error;}},40);else send(true);}};
  const menu={...node(''),querySelectorAll:()=>[option]};
  const trigger={...node('',{'aria-controls':'owned-menu','aria-haspopup':'menu','aria-expanded':alreadyOpen?'true':'false'}),dispatchEvent(event){pointerEvents.push(event.type);if(event.type==='pointerdown')menus++;},click(){}};
  const once={...node('Allow once⏎'),click(){onceClicks++;send(false);}};
  const card={__reactFiberFixture:{memoizedProps:{conversationId:'ui-alias'},memoizedState:{memoizedState:{current:f.scope}}},...node('',{'data-chatgpt-search-message-ids':'confirm'}),contains:()=>false,querySelectorAll:selector=>selector.includes('aria-haspopup')?[trigger]:[once,trigger]};
  const old=globalThis.electronBridge,oldPointer=globalThis.PointerEvent;
  globalThis.PointerEvent=class {constructor(type,options){this.type=type;Object.assign(this,options);}};
  globalThis.electronBridge={sendMessageFromView(){focused++;}};
  let rendered=!['render','render-stop'].includes(delayed);
  document.querySelectorAll=selector=>rendered&&selector.includes('data-chatgpt-search-message-ids')?[card]:[];
  document.getElementById=id=>menus&&id==='owned-menu'?menu:null;
  try{
   if(['render','render-stop'].includes(delayed)){
    expect(await bridge({op:'approve_ui',owner:'owned',lane:'ui',connector:'Owned connector',identity:{account:'account',user:'user',structure:'personal'}})).toEqual({pending:true});
    expect(lane.approvalIntents).toBeUndefined();expect(lane.approvalPending).not.toBe(true);expect(scopedClicks+onceClicks).toBe(0);expect(f.sent.length).toBe(1);rendered=true;
    if(delayed==='render-stop'){
     expect((await bridge({op:'stop',owner:'owned',lane:'ui'})).stopped).toBe(true);
     await expect(bridge({op:'approve_ui',owner:'owned',lane:'ui',connector:'Owned connector',identity:{account:'account',user:'user',structure:'personal'}})).rejects.toThrow('app_approval_unavailable');
     expect(scopedClicks+onceClicks).toBe(0);expect(f.sent.length).toBe(1);await bridge({op:'release',owner:'owned',lane:'ui'});return;
    }
   }
   expect(await bridge({op:'approve_ui',owner:'owned',lane:'ui',connector:'Owned connector',identity:{account:'account',user:'user',structure:'personal'}})).toEqual({requested:true,scope:conversation?'conversation':'once',path:'ui',messageId:'confirm'});
   if(delayed&&delayed!=='render'){
    const queued=await bridge({op:'snapshot',owner:'owned',lane:'ui'});
    expect(queued.approvalResponses).toEqual([]);expect(queued.approvalPending).toBe(true);
    if(delayed==='stop'){
     expect((await bridge({op:'stop',owner:'owned',lane:'ui'})).stopped).toBe(false);
     await expect(bridge({op:'release',owner:'owned',lane:'ui'})).rejects.toThrow('app_lane_busy');
     await expect(bridge({op:'submit',owner:'owned',lane:'ui',round:'new'})).rejects.toThrow('app_lane_busy');
    }
    await new Promise(r=>setTimeout(r,60));
    const settled=await bridge({op:'snapshot',owner:'owned',lane:'ui'});
    expect(settled.approvalPending).toBe(false);
    if(delayed==='stop'){
     expect(delayedError?.message).toBe('app_dispatch_stopped');expect(settled.stopped).toBe(true);
     expect(settled.approvalResponses).toEqual([]);expect(settled.hostReplyMessages).toBe(0);
     await bridge({op:'release',owner:'owned',lane:'ui'});
    }else expect(settled.approvalResponses).toEqual(['confirm']);
   }
   expect(focused).toBe(1);expect(f.revealed).toBe('/c/server-owned');
   expect(scopedClicks).toBe(conversation?1:0);expect(onceClicks).toBe(conversation?0:1);expect(menus).toBe(conversation?1:0);expect(pointerEvents).toEqual(conversation&&!alreadyOpen?['pointerdown','pointerup']:[]);
   expect(f.sent.length).toBe(delayed==='stop'?1:2);expect(f.approvalOptions).toBeUndefined();
  }finally{globalThis.electronBridge=old;globalThis.PointerEvent=oldPointer;}
 });
});

test('an asynchronous app stream startup failure settles only its owned stream',async()=>fixture(async({f,bindings})=>{
 await bridge({op:'init',owner:'owned',returnPath:'mcp',bindings});
 await bridge({op:'open',owner:'owned',lane:'failure',signature:{name:'unused'}});
 const lane=globalThis[slot].lanes.get('failure');
 f.rejectStart=true;
 await expect(f.client.startCompletionStream({request:{conversation_id:lane.localId,messages:[{metadata:{termux_codex_lane:'failure'}}]}})).rejects.toThrow('fixture async failure');
 const observed=await bridge({op:'snapshot',owner:'owned',lane:'failure'});
 expect(lane.streamActive).toBe(false);expect(observed.error).toBe('app_stream_failed');expect(observed.dispatches).toBe(0);
 expect((await bridge({op:'stop',owner:'owned',lane:'failure'})).stopped).toBe(false);
}));

for(const status of [401,403,404,429,500,503,null])test('saved read failure preserves static HTTP cause without exception details: '+status,async()=>fixture(async({f,bindings})=>{
 await bridge({op:'init',owner:'owned',bindings});
 f.chatGet=async()=>{throw Object.assign(Error('private request token must not escape'),{responseStatus:status});};
 const code=({401:'app_chat_auth_expired',403:'app_chat_forbidden',404:'app_chat_not_found',429:'app_chat_rate_limited'})[status]??([500,503].includes(status)?'app_chat_temporarily_unavailable':'app_chat_request_failed');
 await expect(bridge({op:'chat_read',owner:'owned',conversation_id:'saved'})).rejects.toThrow(code);
 const host=await bridge({op:'host_status',owner:'owned'});
 expect(JSON.stringify(host)).not.toContain('private request token');
 if(![401,403,404].includes(status))expect(host.saved_read_failure).toMatchObject({code,http_status:status});
 expect(f.sent).toHaveLength(0);
}));
for(const response of [null,{mapping:{incomplete:null}}])test('invalid saved graph remains an explicit response error: '+JSON.stringify(response),async()=>fixture(async({f,bindings})=>{
 await bridge({op:'init',owner:'owned',bindings});f.chatGet=async()=>response;
 await expect(bridge({op:'chat_read',owner:'owned',conversation_id:'saved'})).rejects.toThrow('app_chat_response_invalid');
 expect(f.sent).toHaveLength(0);
}));

// Fake clocks and fixture wire clients only; these cases never contact ChatGPT.
test('account cooldown suppresses repeated and cross-reader GETs, survives owner release, and backs off',async()=>fixture(async({f,bindings})=>{
 const realNow=Date.now;let now=1000000,calls=0;Date.now=()=>now;
 try{
  await bridge({op:'init',owner:'owned',bindings});
  f.chatGet=async()=>{calls++;throw Object.assign(Error('private429'),{responseStatus:429});};
  const read=()=>bridge({op:'chat_read',owner:'owned',conversation_id:'saved'});
  await expect(read()).rejects.toThrow('app_chat_rate_limited');
  for(let n=0;n<50;n++)await expect(read()).rejects.toThrow('app_chat_rate_limited');
  await expect(bridge({op:'chat_list',owner:'owned'})).rejects.toThrow('app_chat_rate_limited');
  await expect(bridge({op:'chat_page',owner:'owned',conversation_id:'saved'})).rejects.toThrow('app_chat_rate_limited');
  expect(calls).toBe(1);
  await bridge({op:'dispose',owner:'owned'});
  await expect(bridge({op:'chat_read',owner:'other-reader',readOnlyContext:true,bindings,conversation_id:'saved'})).rejects.toThrow('app_chat_rate_limited');
  expect(globalThis[slot]).toBeUndefined();expect(calls).toBe(1);
  now+=60000;
  await expect(bridge({op:'chat_read',owner:'other-reader',readOnlyContext:true,bindings,conversation_id:'saved'})).rejects.toThrow('app_chat_rate_limited');
  expect(calls).toBe(2);
  const host=await bridge({op:'host_status',owner:'other-reader',readOnlyContext:true,bindings});
  expect(host.saved_read_backoff.retry_after_seconds).toBe(120);
  expect(JSON.stringify(host)).not.toContain('private429');
 }finally{Date.now=realNow;}
}));
for(const header of ['180',new Date(1180000).toUTCString()])test('saved GET respects Retry-After '+header,async()=>fixture(async({f,bindings})=>{
 const realNow=Date.now;let now=1000000,calls=0;Date.now=()=>now;
 try{
  await bridge({op:'init',owner:'owned',bindings});
  f.chatGet=async()=>{calls++;throw Object.assign(Error('limited'),{responseStatus:429,responseHeaders:{'retry-after':header}});};
  const read=()=>bridge({op:'chat_read',owner:'owned',conversation_id:'saved'});
  await expect(read()).rejects.toThrow('app_chat_rate_limited');now+=60000;
  await expect(read()).rejects.toThrow('app_chat_rate_limited');expect(calls).toBe(1);
  expect((await bridge({op:'host_status',owner:'owned'})).saved_read_backoff.retry_after_seconds).toBe(120);
 }finally{Date.now=realNow;}
}));
test('successful read resets backoff, while another account has independent pacing',async()=>fixture(async({f,bindings})=>{
 const realNow=Date.now;let now=1000000,calls=0,limited=true;Date.now=()=>now;
 try{
  await bridge({op:'init',owner:'owned',bindings});
  f.chatGet=async()=>{calls++;if(limited)throw Object.assign(Error('limited'),{responseStatus:429});return {mapping:{}};};
  const read=()=>bridge({op:'chat_read',owner:'owned',conversation_id:'saved'});
  await expect(read()).rejects.toThrow('app_chat_rate_limited');
  const get=f.scope.get.bind(f.scope);let account='other';f.scope.get=node=>node.kind==='identity'?{...get(node),accountId:account,authenticatedAccountId:account}:get(node);
  limited=false;await read();expect(calls).toBe(2);account='account';
  await expect(read()).rejects.toThrow('app_chat_rate_limited');expect(calls).toBe(2);
  now+=60000;await read();limited=true;
  await expect(read()).rejects.toThrow('app_chat_rate_limited');
  expect((await bridge({op:'host_status',owner:'owned'})).saved_read_backoff.retry_after_seconds).toBe(60);
 }finally{Date.now=realNow;}
}));
test('an in-flight saved GET does not multiply requests or block an idle lane Stop',async()=>fixture(async({f,bindings})=>{
 await bridge({op:'init',owner:'owned',bindings});await bridge({op:'open',owner:'owned',lane:'idle',signature:{name:'unused'}});
 let release,calls=0;f.chatGet=()=>{calls++;return new Promise(resolve=>{release=resolve;});};
 const first=bridge({op:'chat_read',owner:'owned',conversation_id:'saved'});
 await expect(bridge({op:'chat_read',owner:'owned',conversation_id:'saved'})).rejects.toThrow('app_chat_read_pending');
 expect((await bridge({op:'stop',owner:'owned',lane:'idle'})).stopped).toBe(true);
 expect(calls).toBe(1);release({mapping:{}});await first;expect(f.sent).toHaveLength(0);
}));
