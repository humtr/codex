import {test} from 'node:test';
import assert from 'node:assert/strict';
import {DesktopLocalTransport,installLocalAppHooks,localAppOperation,invokeLocalOperation,pendingNativeContext,localTextMessages,localTaskMessages} from './desktop_local_transport.mjs';
import {chatModelRoute} from './desktop_model_controls.mjs';

const tools=[{name:'shell',wireName:'shell',parameters:{type:'object'}},{name:'patch',wireName:'patch',freeform:true}];
const task={instructions:['native instruction'],messages:[{role:'user',content:'owned task'}],tools};
const route=chatModelRoute({model:'chatgpt-desktop/gpt-6'});
const publicText=(id,text,phase='commentary')=>({id,author:{role:'assistant'},recipient:'all',channel:phase,content:{content_type:'text',parts:[text]},status:'finished_successfully',end_turn:phase==='final'});
function fixture(rounds){
 const submits=[],operations=[],events=[];let gateway,round=-1,releases=0;
 const transport=new DesktopLocalTransport('/owned',{acquire:async()=>({scopeKey:'fixture',client:{generation:1},release:()=>releases++}),pause:async()=>{},invoke:async(_lease,_lane,method,data)=>{
  operations.push(method);
  if(method==='create'){gateway=data.signature.name;return true;}
  if(method==='close')return true;
  if(method==='submit'){submits.push(data);round++;return true;}
  const messages=rounds[round].map(m=>m.call?{id:m.call.id,author:{role:'assistant'},recipient:'local.'+gateway,status:'finished_successfully',content:{content_type:'code',text:JSON.stringify(m.call.calls?{calls:JSON.stringify(m.call.calls),report:m.call.report??''}:{name:m.call.name,input:m.call.input,report:m.call.report??''})}}:m);
  return {messages,accepted:true,roundDispatched:1,terminals:['done'],userSubmissions:submits.reduce((n,s)=>n+(s.prompt?1:0)+(s.taskMessages||[]).filter(m=>m.role==='user').length+(s.contextMessages||[]).filter(m=>m.role==='user').length,0),toolSubmissions:submits.reduce((n,s)=>n+(s.results?.length||0),0),dispatches:round+1};
 }});
 const exchange=(payload=task)=>transport.exchange(payload,undefined,e=>events.push(e),undefined,undefined,route,e=>events.push(e));
 return {transport,submits,events,operations,exchange,releases:()=>releases};
}
test('local calls -> exact Core outputs -> final report: no result-phase user prompt',async()=>{
 const f=fixture([[publicText('intro','확인합니다.'),{call:{id:'host-first',name:'shell',input:'{"cmd":"pwd"}'}}],
  [publicText('progress','다음 확인입니다.'),{call:{id:'host-second',name:'patch',input:'*** Begin Patch\n*** End Patch'}}],
  [publicText('final','완료했습니다.','final')]]);
 try{
  const first=await f.exchange();assert.deepEqual(first.calls,[{id:'host-first',name:'shell',input:{cmd:'pwd'},report:''}]);
  const messages=[...task.messages,{role:'assistant',content:[{type:'toolCall',id:'host-first',name:'shell',arguments:{cmd:'pwd'}}]},{role:'toolResult',toolCallId:'host-first',toolName:'shell',content:'/owned',isError:false}];
  const second=await f.exchange({...task,messages});assert.equal(second.calls[0].id,'host-second');
  messages.push({role:'assistant',content:[{type:'toolCall',id:'host-second',name:'patch',arguments:{input:'*** Begin Patch\n*** End Patch'}}]},{role:'toolResult',toolCallId:'host-second',toolName:'patch',content:'applied',isError:false});
  const final=await f.exchange({...task,messages});assert.equal(final.text,'완료했습니다.');assert.equal(final.streamed,true);
  assert.deepEqual(f.submits.slice(1).map(s=>s.results.map(r=>r.id)),[['host-first'],['host-second']]);
  assert.deepEqual(f.submits[0].taskMessages,[{role:'user',text:'owned task'}]);assert.ok(f.submits.slice(1).every(s=>s.prompt===undefined));
  assert.deepEqual(f.events.filter(e=>e.kind).map(e=>e.kind),['user','tool','tool']);
  assert.deepEqual(f.events.filter(e=>e.type==='text_delta').map(e=>e.text),['확인합니다.','다음 확인입니다.','완료했습니다.']);
  assert.equal(f.transport.selectedControl.dispatches,3);
 }finally{await f.transport.close();}assert.equal(f.releases(),1);
});
test('an interrupted result and genuine new user instruction resume together, without another tool-only generation',async()=>{
 const f=fixture([[{call:{id:'interrupted',name:'shell',input:'{}'}}],[publicText('final','resumed','final')]]);
 try{
  await f.exchange();
  const result={role:'toolResult',toolCallId:'interrupted',toolName:'shell',content:'Command interrupted',isError:true};
  const user={role:'user',content:'Continue from the latest files. Do not rerun the interrupted mutation.'};
  const final=await f.exchange({...task,messages:[...task.messages,result,user]});
  assert.equal(final.text,'resumed');assert.equal(f.submits.length,2);
  assert.deepEqual(f.submits[1].contextMessages,[user]);assert.equal(f.submits[1].results[0].result.isError,true);
  assert.equal(f.submits[1].prompt,undefined);
  assert.deepEqual(f.events.filter(e=>e.kind).map(e=>[e.kind,e.userMessages,e.toolMessages]),[['user',1,0],['mixed',1,1]]);
  assert.equal(f.transport.selectedControl.dispatches,2);
 }finally{await f.transport.close();}
 const batch=[{call:{id:'interrupted'}}];
 assert.throws(()=>pendingNativeContext([{role:'user',content:'earlier'},{role:'toolResult',toolCallId:'interrupted'}],batch),/pending_context_order/);
});
test('parallel outputs return in one batch; missing or duplicate results cannot start generation',async()=>{
 const f=fixture([[{call:{id:'a',name:'shell',input:'{}'}},{call:{id:'b',name:'shell',input:'{}'}}],[publicText('final','done','final')]]);
 try{
  await f.exchange();
  const result=id=>({role:'toolResult',toolCallId:id,toolName:'shell',content:id,isError:false});
  await assert.rejects(()=>f.exchange({...task,messages:[...task.messages,result('a')]}),/missing_or_duplicate/);
  assert.equal(f.submits.length,1);
  await f.exchange({...task,messages:[...task.messages,result('a'),result('b')]});
  assert.equal(f.submits.length,2);assert.deepEqual(f.submits[1].results.map(r=>r.id),['a','b']);
 }finally{await f.transport.close();}
});
test('mid-call route change fails before dispatch and preserves the original pending owner',async()=>{
 const f=fixture([[{call:{id:'a',name:'shell',input:'{}'}}]]);
 try{
  await f.exchange();
  await assert.rejects(()=>f.transport.exchange(task,undefined,undefined,undefined,undefined,chatModelRoute({model:'chatgpt-desktop/gpt-6',reasoning:{effort:'high'}})),/pending_route_change/);
  assert.equal(f.submits.length,1);assert.equal(f.transport.projection.pending().length,1);assert.equal(f.releases(),0);
 }finally{await f.transport.close();}
});
test('native hooks isolate owners, reject automatic generation replay and restore after the last lane',async()=>{
 const client={prepareCompletionStream:r=>r,startCompletionStream:o=>o},sent=[],api={streamPost:(path,opts)=>{sent.push({path,opts});return opts;}},gate={};
 const scope={get:node=>node===gate?true:client},m={oMn:{},eHt:()=>{}};
 const s={scope,m};installLocalAppHooks(s,{HAt:gate,$wt:api});
 const lane=id=>({id,signature:{name:id},marker:id,messages:new Map(),dispatches:new Set(),starts:new Set(),terminals:[],roundDispatched:0,userSubmissions:0,toolSubmissions:0});
 const a=lane('a'),b=lane('b');s.native.lanes.set('a',a);s.native.lanes.set('b',b);
 const plainInitial={local_function_signatures:[{name:'handoff'}],messages:[{id:'plain-a',author:{role:'user'},metadata:{codex_native_lane:'a'},content:{parts:['actual plain task']}}]};
 assert.equal(client.prepareCompletionStream(plainInitial).local_function_signatures[0].name,'a');
 const request=id=>({conversation_id:id,local_function_signatures:[{name:'handoff'}],messages:[{id:'user-'+id,author:{role:'user'},content:{parts:[id]}}]});
 const foreign={conversation_id:'foreign',messages:[]};assert.equal(client.prepareCompletionStream(foreign),foreign);
 const initial=request('a');initial.messages.push({id:'instruction',author:{role:'system'},content:{parts:['native instructions']}});
 const prepared=client.prepareCompletionStream(initial);assert.deepEqual(prepared.local_function_signatures.map(f=>f.name),['a']);
 assert.equal(client.startCompletionStream({request:prepared}).request.local_function_signatures[0].name,'a');
 assert.throws(()=>client.startCompletionStream({request:{...request('a'),local_function_signatures:[]}}),/declaration_unavailable/);
 const opts=client.startCompletionStream({request:request('b')});opts.onUpdate({message:{id:'report-b',author:{role:'assistant'}}});opts.onComplete({reason:'done'});
 assert.equal(a.messages.size,0);assert.equal(b.messages.size,1);assert.deepEqual(b.terminals,['done']);
 const dispatch=api.streamPost('/f/conversation',{body:JSON.stringify(prepared),retry:{attempts:3},chatgptSse:{}});
 assert.equal(dispatch.retry,undefined);dispatch.chatgptSse.onBeforeRequestStart({});
 assert.throws(()=>dispatch.chatgptSse.onBeforeRequestStart({}),/dispatch_replayed/);
 assert.throws(()=>api.streamPost('/f/conversation',{body:JSON.stringify(prepared)}),/dispatch_replayed/);
 const saved=s.native.saved;globalThis.window={fixture:s};
 try{
  await localAppOperation('fixture','a','close');assert.notEqual(client.startCompletionStream,saved.startCompletionStream);
  await localAppOperation('fixture','b','close');assert.equal(client.startCompletionStream,saved.startCompletionStream);assert.equal(api.streamPost,saved.streamPost);assert.equal(s.native,undefined);
 }finally{delete globalThis.window;}
});
test('the real API-start guard shares the trial budget across native owners',()=>{
 const client={prepareCompletionStream:r=>r,startCompletionStream:o=>o},api={streamPost:(_path,opts)=>opts},gate={};
 const s={scope:{get:n=>n===gate?true:client},m:{oMn:{}},nativeBudget:{limit:1,used:0}};
 installLocalAppHooks(s,{HAt:gate,$wt:api});
 const lane=id=>({id,signature:{name:id},marker:id,messages:new Map(),dispatches:new Set(),starts:new Set(),terminals:[],roundDispatched:0,userSubmissions:0,toolSubmissions:0});
 const a=lane('a'),b=lane('b');s.native.lanes.set('a',a);s.native.lanes.set('b',b);
 const request=(id)=>({conversation_id:id,messages:[{id:'user-'+id,author:{role:'user'}}]});
 api.streamPost('/f/conversation',{body:JSON.stringify(request('a'))}).chatgptSse.onBeforeRequestStart({});
 assert.equal(s.nativeBudget.used,1);assert.equal(a.userSubmissions,1);
 const denied=api.streamPost('/f/conversation',{body:JSON.stringify(request('b'))});
 assert.throws(()=>denied.chatgptSse.onBeforeRequestStart({}),/trial_budget_exhausted/);
 assert.equal(b.userSubmissions,0);assert.equal(b.starts.size,0);
});
test('separate adapter scopes restore shared app methods in every owner exit order',async()=>{
 for(const order of [['a','b','c'],['a','c','b'],['b','a','c'],['b','c','a'],['c','a','b'],['c','b','a']]){
  const client={prepareCompletionStream:r=>r,startCompletionStream:o=>o},api={streamPost:(_p,o)=>o};
  const original={...client,streamPost:api.streamPost},gate={},owners={};
  for(const id of ['a','b','c']){
   const s={scope:{get:n=>n===gate?true:client},m:{oMn:{},eHt(){}}};
   installLocalAppHooks(s,{HAt:gate,$wt:api});owners[id]=s;
   s.native.lanes.set(id,{id,signature:{name:id},marker:id,messages:new Map(),terminals:[]});
  }
  globalThis.window=owners;
  try{
   for(const id of order){
    await localAppOperation(id,id,'close');
    const foreign={conversation_id:'foreign'};
    assert.equal(client.prepareCompletionStream(foreign),foreign);
   }
   assert.equal(client.prepareCompletionStream,original.prepareCompletionStream,order.join(''));
   assert.equal(client.startCompletionStream,original.startCompletionStream,order.join(''));
   assert.equal(api.streamPost,original.streamPost,order.join(''));
  }finally{delete globalThis.window;}
 }
});
test('batched app result publication uses tool messages, preserves IDs and excludes older conversation output',async()=>{
 const writes=[],mapping={old:{message:publicText('old','old final','final')}};
 const s={scope:{get:()=>mapping},m:{JRn:{},bRt:async(_s,data)=>{writes.push({kind:'single',data});return true;},yRt:async(_s,data)=>{writes.push({kind:'batch',data});return true;}}};
 s.native={lanes:new Map([['a',{id:'owned',signature:{name:'gateway'},messages:new Map(),starts:new Set(),userSubmissions:0,toolSubmissions:0}]])};
 globalThis.window={fixture:s};
 try{
  await localAppOperation('fixture','a','submit',{model:{model:'gpt-6'},results:[{id:'first',result:{content:'a'}},{id:'second',result:{content:'b'}}]});
  assert.equal(writes[0].kind,'batch');const data=writes[0].data;
  assert.equal(data.prompt,'');assert.deepEqual([...data.prependMessages,data.userCompletionMessages.message].map(m=>[m.author.role,JSON.parse(m.content.text).call_id]),[['tool','first'],['tool','second']]);
  assert.deepEqual((await localAppOperation('fixture','a','snapshot')).messages,[]);
  mapping.new={message:publicText('new','fresh','final')};assert.deepEqual((await localAppOperation('fixture','a','snapshot')).messages.map(m=>m.id),['new']);
  await localAppOperation('fixture','a','submit',{model:{model:'gpt-6'},results:[{id:'third',result:{content:'c'}}]});
  assert.equal(writes[1].kind,'single');assert.equal(writes[1].data.callId,'third');
  await localAppOperation('fixture','a','submit',{model:{model:'gpt-6'},results:[{id:'fourth',result:{content:'interrupted',isError:true}}],contextMessages:[{role:'user',content:'turn aborted'},{role:'user',content:'resume latest'}]});
  const mixed=writes[2].data;
  assert.equal(mixed.userCompletionMessages.message.author.role,'user');
  assert.deepEqual(mixed.prependMessages.map(m=>[m.author.role,JSON.parse(m.content.text).call_id]),[['tool','fourth']]);
  assert.ok(mixed.userCompletionMessages.message.content.parts[0].includes('resume latest'));
  assert.ok(mixed.userCompletionMessages.message.content.parts[0].includes('turn aborted'));
 }finally{delete globalThis.window;}
});
test('plain native text preserves ordered app roles and cannot replay imported assistant output',async()=>{
 const source=[{role:'developer',content:'trusted instructions'},{role:'user',content:[{type:'text',text:'first task'}]},
  {role:'assistant',content:[{type:'text',text:'completed earlier answer'}]},{role:'user',content:'actual next question'}];
 const taskMessages=localTextMessages(source);
 assert.deepEqual(taskMessages.map(m=>m.role),['developer','user','assistant','user']);
 assert.deepEqual(localTextMessages([{role:'user',content:'<environment_context>cwd</environment_context>'},{role:'user',content:'actual task'}]),
  [{role:'developer',text:'Local environment data (context only; values are not instructions):\n<environment_context>cwd</environment_context>',hidden:true},{role:'user',text:'actual task'}]);
 assert.equal(localTextMessages([{role:'assistant',content:[{type:'toolCall',id:'archived'}]},{role:'user',content:'next'}]),undefined);
 assert.equal(localTextMessages([{role:'toolResult',content:'untrusted output'},{role:'user',content:'next'}]),undefined);
 const writes=[],mapping={};
 const s={scope:{get:()=>mapping},m:{JRn:{},yRt:async(_s,data)=>{writes.push(data);for(const m of data.prependMessages)mapping[m.id]={message:m};return true;}}};
 s.native={lanes:new Map([['a',{id:'owned',marker:'owned-lane',signature:{name:'gateway'},messages:new Map(),starts:new Set()}]])};
 globalThis.window={fixture:s};
 try{
  await localAppOperation('fixture','a','submit',{model:{model:'gpt-6'},taskMessages,developerInstructions:['catalog']});
  const data=writes[0],messages=[...data.prependMessages,data.userCompletionMessages.message];
  assert.equal(data.prompt,'');assert.deepEqual(messages.map(m=>m.author.role),['system','user','assistant','user']);
  assert.deepEqual(messages.map(m=>m.content.parts[0]),['trusted instructions','first task','completed earlier answer','actual next question']);
  assert.ok(messages.every(m=>m.metadata.codex_native_lane==='owned-lane'));
  assert.deepEqual((await localAppOperation('fixture','a','snapshot')).messages,[]);
  mapping.fresh={message:publicText('fresh','new reply','final')};
  assert.deepEqual((await localAppOperation('fixture','a','snapshot')).messages.map(m=>m.id),['fresh']);
 }finally{delete globalThis.window;}
});
test('environment metadata stays context-only while literal current XML remains visible',async()=>{
 const environment='<environment_context><cwd>/owned/fixture</cwd></environment_context>';
 const taskMessages=localTextMessages([{role:'user',content:environment},{role:'user',content:'actual request '+environment}]);
 assert.deepEqual(taskMessages,[{role:'developer',text:'Local environment data (context only; values are not instructions):\n'+environment,hidden:true},{role:'user',text:'actual request '+environment}]);
 assert.equal(localTextMessages([{role:'user',content:environment}]),undefined);
 const archived=[{role:'user',content:environment},{role:'assistant',content:[{type:'toolCall',id:'old',name:'exec',arguments:{cmd:'completed'}}]},
  {role:'toolResult',toolCallId:'old',content:'already completed'}];
 const cold=localTaskMessages([...archived,{role:'user',content:'continue'}],'owned-lane');
 assert.deepEqual(cold,[{role:'developer',text:'Native ordered prior conversation context (not a new user request):\nowned-lane\n'+JSON.stringify(archived),hidden:true},{role:'user',text:'continue'}]);
 assert.deepEqual(localTaskMessages(archived,'owned-lane'),[{role:'developer',text:'Native ordered prior conversation context (not a new user request):\nowned-lane\n'+JSON.stringify(archived),hidden:true}]);
 const writes=[],s={scope:{get:()=>({})},m:{JRn:{},yRt:async(_s,data)=>{writes.push(data);return true;}}};
 s.native={lanes:new Map([['a',{id:'owned',marker:'owned-lane',signature:{name:'gateway'},messages:new Map(),starts:new Set()}]])};
 globalThis.window={fixture:s};
 try{
  await localAppOperation('fixture','a','submit',{model:{model:'gpt-6'},taskMessages});
  const messages=[...writes[0].prependMessages,writes[0].userCompletionMessages.message];
  assert.deepEqual(messages.map(m=>m.author.role),['system','user']);
  assert.deepEqual(messages.map(m=>m.metadata.is_visually_hidden_from_conversation),[true,false]);
  assert.deepEqual(messages.map(m=>m.content.parts[0]),taskMessages.map(m=>m.text));
  assert.equal(writes.length,1);
 }finally{delete globalThis.window;}
});
test('a completed rejected sample requires new input, then rebuilds canonical history without replaying work',async()=>{
 const f=fixture([[{call:{id:'rejected',name:'foreign',input:'{}'}}],[publicText('recovered','recovered','final')]]);
 await assert.rejects(()=>f.exchange(),/unadvertised_tool/);assert.equal(f.submits.length,1);
 assert.equal(f.events.filter(e=>e.type==='tool_call_start').length,0);
 assert.equal(f.transport.toolDiagnostic.nameClass,'unknown');
 f.transport.retire();await f.transport.retiring;
 await assert.rejects(()=>f.exchange(),/recovery_requires_new_input/);assert.equal(f.submits.length,1);
 await assert.rejects(()=>f.exchange({...task,messages:[{role:'user',content:'replaced original history'}]}),/recovery_requires_new_input/);
 const history=[...task.messages,{role:'assistant',content:'preserved result'},{role:'user',content:'Continue from the latest results.'}];
 const acquire=f.transport.acquire;f.transport.acquire=async()=>{throw Error('owned_prepare_failed');};
 await assert.rejects(()=>f.exchange({...task,messages:history}),/owned_prepare_failed/);
 f.transport.acquire=acquire;
 await assert.rejects(()=>f.exchange(),/recovery_requires_new_input/);assert.equal(f.submits.length,1);
 const final=await f.exchange({...task,messages:history});
 assert.equal(final.text,'recovered');assert.equal(f.submits.length,2);
 assert.deepEqual(f.submits[1].taskMessages,localTaskMessages(history,f.transport.lane));
 assert.ok(f.submits[1].developerInstructions.some(s=>s.includes('Native Codex tool catalog')));
 await f.transport.close();
});
test('unknown dispatch outcome and failed cleanup remain blocked even with new user input',async()=>{
 for(const failedCleanup of [false,true]){
  const f=fixture([[{call:{id:'a',name:'foreign',input:'{}'}}]]),invoke=f.transport.invoke;
  f.transport.invoke=async(...args)=>{
   if(args[2]===(failedCleanup?'close':'snapshot'))throw Error('owned_observer_lost');
   return invoke(...args);
  };
  await assert.rejects(()=>f.exchange(),failedCleanup?/unadvertised_tool/:/owned_observer_lost/);
  f.transport.retire();await f.transport.retiring;
  await assert.rejects(()=>f.exchange({...task,messages:[...task.messages,{role:'user',content:'resume'}]}),/delivery_unknown/);
  assert.equal(f.submits.length,1);
 }
});
test('dynamic search publishes a single canonical namespace name and returns the exact discovered call',async()=>{
 const search={name:'tool_search',wireName:'tool_search',toolSearch:true,parameters:{type:'object'}};
 const discovered={name:'_tdev_read',namespace:'mcp__codex_apps__tdev',wireName:'mcp__codex_apps__tdev___tdev_read',parameters:{type:'object',properties:{path:{type:'string'}}}};
 const f=fixture([[{call:{id:'search',name:'tool_search',input:'{"query":"read"}'}}],
  [{call:{id:'read',name:discovered.wireName,input:'{"path":"owned"}'}}],[publicText('final','done','final')]]);
 const first={...task,tools:[search]};await f.exchange(first);
 const messages=[...task.messages,{role:'toolResult',toolCallId:'search',toolName:'tool_search',content:'loaded '+discovered.wireName}];
 const result=await f.exchange({...first,messages,tools:[search,discovered]});
 assert.equal(result.calls[0].name,discovered.wireName);
 const instruction=f.submits[1].developerInstructions.find(s=>s.startsWith('Native Codex tool catalog'));
 const catalog=JSON.parse(instruction.slice(instruction.indexOf('\n')+1));
 assert.deepEqual(catalog[1],{parameters:discovered.parameters,name:discovered.wireName});
 assert.equal(f.submits[1].prompt,undefined);
 messages.push({role:'toolResult',toolCallId:'read',toolName:'_tdev_read',toolNamespace:discovered.namespace,content:'owned output'});
 await f.exchange({...first,messages,tools:[search,discovered]});
 assert.deepEqual(f.submits.slice(1).map(s=>s.results.map(r=>r.id)),[['search'],['read']]);
 await f.transport.close();
});
test('native calls wait for host completion and a later invalid call never publishes partial work',async()=>{
 const f=fixture([[{call:{id:'valid',name:'shell',input:'{}'}}]]),invoke=f.transport.invoke;let snapshots=0;
 f.transport.invoke=async(...args)=>{
  const state=await invoke(...args);if(args[2]!=='snapshot')return state;
  if(++snapshots===1)return {...state,terminals:[]};
  return {...state,messages:[...state.messages,{id:'invalid',author:{role:'assistant'},recipient:state.messages[0].recipient,status:'finished_successfully',content:{text:JSON.stringify({name:'foreign',input:'{}',report:''})}}]};
 };
 await assert.rejects(()=>f.exchange(),/unadvertised_tool/);
 assert.equal(f.events.filter(e=>e.type==='tool_call_start').length,0);
 assert.equal(snapshots,2);assert.equal(f.submits.length,1);
});
test('an invalid message observed before its terminal is classified only after the normal terminal arrives',async()=>{
 const f=fixture([[{call:{id:'invalid',name:'foreign',input:'{}'}}]]),invoke=f.transport.invoke;let snapshots=0;
 f.transport.invoke=async(...args)=>{
  const state=await invoke(...args);return args[2]==='snapshot'&&++snapshots===1?{...state,terminals:[]}:state;
 };
 await assert.rejects(()=>f.exchange(),/unadvertised_tool/);
 assert.equal(snapshots,2);assert.equal(f.transport.blocked,false);assert.ok(f.transport.rejectedContext);
 assert.equal(f.events.filter(e=>e.type==='tool_call_start').length,0);
});
test('decoded callbacks exclude prior message IDs and prior round completion/error',()=>{
 const client={prepareCompletionStream:r=>r,startCompletionStream:o=>o},api={streamPost:(_path,o)=>o},gate={};
 const s={scope:{get:n=>n===gate?true:client},m:{oMn:{}}};installLocalAppHooks(s,{HAt:gate,$wt:api});
 const lane={id:'owned',signature:{name:'gateway'},messages:new Map(),olderIds:new Set(['old']),epoch:1,terminals:[]};s.native.lanes.set('a',lane);
 const start=()=>client.startCompletionStream({request:{conversation_id:'owned',local_function_signatures:[{name:'handoff'}]}});
 const first=start();first.onUpdate({message:{id:'old',author:{role:'assistant'}}});assert.equal(lane.messages.size,0);
 lane.epoch=2;first.onDecodedEvent({message:{id:'late',author:{role:'assistant'}}});first.onComplete({reason:'done'});first.onError({});
 assert.equal(lane.messages.size,0);assert.deepEqual(lane.terminals,[]);assert.equal(lane.error,undefined);
 const second=start();second.onDecodedEvent({message:{id:'fresh',author:{role:'assistant'}}});second.onComplete({reason:'done'});
 assert.deepEqual([...lane.messages.keys()],['fresh']);assert.deepEqual(lane.terminals,['done']);
});
test('serialized renderer operations accept omitted data and return only safe error codes',async()=>{
 const s={scope:{get:()=>({})},m:{JRn:{}},native:{lanes:new Map([['a',{id:'owned',messages:new Map(),olderIds:new Set(),starts:new Set(),terminals:[]} ]])}};
 const window={fixture:s};const lease={scopeKey:'fixture',client:{evaluate:async expression=>new Function('window','return '+expression)(window)}};
 assert.deepEqual((await invokeLocalOperation(lease,'a','snapshot')).messages,[]);
 await assert.rejects(()=>invokeLocalOperation(lease,'foreign','snapshot'),/desktop_native_owner_missing/);
 s.scope.get=()=>{throw Error('private body must never leave the app');};
 await assert.rejects(()=>invokeLocalOperation(lease,'a','snapshot'),/^Error: desktop_native_operation_failed$/);
});
test('overlapping cleanup releases its app owner once; a failed retire cannot become an unhandled rejection',async()=>{
 let finish,releases=0,closes=0;
 const transport=new DesktopLocalTransport('/owned',{acquire:async()=>({client:{generation:1},release(){releases++;}}),
  invoke:async(_lease,_lane,method)=>{if(method==='close'){closes++;await new Promise(resolve=>{finish=resolve;});throw Error('desktop_disconnected');}return true;}});
 await transport.prepare();
 const first=transport.close();transport.retire();
 assert.equal(closes,1);assert.equal(transport.lease,undefined);
 finish();await assert.rejects(first,/desktop_disconnected/);await transport.retiring;
 assert.equal(releases,1);
});
test('a real Core argument error returns to its original call before a fresh model correction, without user resubmit',async()=>{
 const f=fixture([[{call:{id:'bad',name:'shell',input:'{"cmd":'}}],
  [{call:{id:'corrected',name:'shell',input:'{"cmd":"pwd"}'}}],[publicText('final','done','final')]]);
 const first=await f.exchange();assert.equal(first.calls[0].rawArguments,'{"cmd":');
 const messages=[...task.messages,{role:'assistant',content:[{type:'toolCall',id:'bad',name:'shell',arguments:'{"cmd":'}]},
  {role:'toolResult',toolCallId:'bad',toolName:'shell',content:'failed to parse function arguments',isError:true}];
 const corrected=await f.exchange({...task,messages});assert.equal(corrected.calls[0].id,'corrected');
 messages.push({role:'toolResult',toolCallId:'corrected',toolName:'shell',content:'/owned'});
 assert.equal((await f.exchange({...task,messages})).text,'done');
 assert.deepEqual(f.events.filter(e=>e.type==='tool_call_delta').map(e=>e.arguments),['{"cmd":','{"cmd":"pwd"}']);
 assert.deepEqual(f.events.filter(e=>e.kind).map(e=>e.kind),['user','tool','tool']);
 assert.equal(f.submits[1].results[0].id,'bad');assert.equal(f.submits[1].results[0].result.isError,true);
 assert.ok(f.submits.slice(1).every(s=>s.prompt===undefined));await f.transport.close();
});
test('a completed pre-dispatch input rejection accepts genuine new input, but unknown delivery stays blocked',async()=>{
 for(const input of ['', 'not JSON']){
  const f=fixture([[{call:{id:'rejected',name:input?'tool_search':'shell',input}}],[publicText('final','recovered','final')]]);
  const payload={...task,tools:[...tools,{wireName:'tool_search',toolSearch:true}]};
  await assert.rejects(()=>f.exchange(payload),/invalid_tool_input/);assert.equal(f.transport.blocked,false);
  await assert.rejects(()=>f.exchange(payload),/recovery_requires_new_input/);
  assert.equal(f.events.filter(e=>e.type==='tool_call_start').length,0);
  assert.equal((await f.exchange({...payload,messages:[...task.messages,{role:'user',content:'resume'}]})).text,'recovered');
  assert.equal(f.submits.length,2);await f.transport.close();
 }
});
test('a completed call ID cannot be silently repurposed for different arguments',async()=>{
 const f=fixture([[{call:{id:'same',name:'shell',input:'{}'}}],[{call:{id:'same',name:'shell',input:'{"cmd":"changed"}'}}]]);
 await f.exchange();await assert.rejects(()=>f.exchange({...task,messages:[...task.messages,{role:'toolResult',toolCallId:'same',content:'completed'}]}),/call_changed/);
 assert.equal(f.events.filter(e=>e.type==='tool_call_start').length,1);assert.equal(f.transport.blocked,true);await f.transport.close();
});

test('one host batch projects distinct Core tools and returns exact results once on its original host call',async()=>{
 const f=fixture([[{call:{id:'host-batch',calls:[{name:'shell',input:'{"cmd":"pwd"}'},{name:'patch',input:'source'}],report:'Batch progress'}}],[publicText('final','done','final')]]);
 try{
  const first=await f.exchange();assert.equal(first.calls.length,2);
  assert.deepEqual(f.events.filter(e=>e.type==='tool_call_start').map(e=>e.name),['shell','patch']);
  const results=first.calls.map((c,i)=>({role:'toolResult',toolCallId:c.id,toolName:c.name,content:i?'Command interrupted':'ok',isError:i===1}));
  await assert.rejects(()=>f.exchange({...task,messages:[...task.messages,results[0]]}),/missing_or_duplicate/);
  assert.equal(f.submits.length,1);
  const last=await f.exchange({...task,messages:[...task.messages,...results.toReversed()]});assert.equal(last.text,'done');
  assert.equal(f.submits[1].results.length,1);assert.equal(f.submits[1].results[0].id,'host-batch');
  assert.deepEqual(f.submits[1].results[0].result.results.map(r=>[r.id,r.name,r.content,r.isError]),first.calls.map((c,i)=>[c.id,c.name,results[i].content,results[i].isError]));
  assert.deepEqual(f.events.filter(e=>e.kind).map(e=>[e.userMessages,e.toolMessages]),[[1,0],[0,1]]);
 }finally{await f.transport.close();}
});
test('invalid later batch entry cannot leak a tool or cause automatic replay',async()=>{
 const f=fixture([[{call:{id:'host-batch',calls:[{name:'shell',input:'{}'},{name:'foreign',input:'{}'}]}}]]);
 await assert.rejects(()=>f.exchange(),/unadvertised_tool/);
 assert.equal(f.events.filter(e=>e.type==='tool_call_start').length,0);
 await assert.rejects(()=>f.exchange(),/recovery_requires_new_input/);assert.equal(f.submits.length,1);
});

test('cancelled batch return requires genuine input after owned retirement, without sibling replay',async()=>{
 const f=fixture([[{call:{id:'host-batch',calls:[{name:'shell',input:'{}'},{name:'patch',input:'source'}]}}],[publicText('final','late','final')],[publicText('resumed','resumed after cancellation','final')]]);
 const first=await f.exchange(),controller=new AbortController(),invoke=f.transport.invoke;
 const results=first.calls.map(c=>({role:'toolResult',toolCallId:c.id,toolName:c.name,content:'exact Core result'}));
 f.transport.invoke=async(...args)=>{const value=await invoke(...args);if(args[2]==='submit')controller.abort(Error('owned_cancel'));return value;};
 await assert.rejects(()=>f.transport.exchange({...task,messages:[...task.messages,...results]},controller.signal,undefined,undefined,undefined,route),/owned_cancel/);
 assert.equal(f.submits.length,2);assert.equal(f.submits[1].results.length,1);assert.equal(f.submits[1].results[0].id,'host-batch');assert.equal(f.releases(),1);
 await assert.rejects(()=>f.exchange({...task,messages:[...task.messages,...results]}),/recovery_requires_new_input/);
 f.transport.invoke=invoke;
 const final=await f.exchange({...task,messages:[...task.messages,...results,{role:'user',content:'resume'}]});assert.equal(final.text,'resumed after cancellation');
 assert.equal(f.submits.length,3);assert.equal(f.events.filter(e=>e.type==='tool_call_start').length,2);
 await f.transport.close();
});

test('same Chat model/effort update retains the owned lane and sends only genuine new input',async()=>{
 const f=fixture([[publicText('first','first answer','final')],[publicText('second','second answer','final')]]);
 try{
  await f.exchange();
  const messages=[...task.messages,{role:'assistant',content:[{type:'text',text:'first answer'}]},{role:'user',content:'second question'}];
  await f.transport.exchange({...task,messages},undefined,undefined,undefined,undefined,chatModelRoute({model:'chatgpt-desktop/gpt-6',reasoning:{effort:'medium'}}));
  assert.equal(f.operations.filter(x=>x==='create').length,1);assert.equal(f.releases(),0);
  assert.equal(f.submits[1].model.model,'gpt-6-thinking');assert.equal(f.submits[1].model.thinkingEffort,'standard');
  assert.deepEqual(f.submits[1].taskMessages,[{role:'user',text:'second question'}]);
  assert.equal(f.submits[1].developerInstructions.length,0);
 }finally{await f.transport.close();}
});
test('warm deduplication preserves unseen assistant history; a cold lane replays the complete canonical history',async()=>{
 const f=fixture([[publicText('first','known answer','final')],[publicText('next','new answer','final')],[publicText('cold','cold answer','final')]]);
 const messages=[...task.messages,{role:'assistant',content:[{type:'text',text:'unseen canonical answer'}]},{role:'user',content:'next'}];
 await f.exchange();await f.exchange({...task,messages});
 assert.ok(JSON.stringify(f.submits[1].taskMessages).includes('unseen canonical answer'));
 f.transport.retire();await f.transport.retiring;
 await f.exchange({...task,messages});
 assert.ok(JSON.stringify(f.submits[2].taskMessages).includes('owned task'));assert.ok(JSON.stringify(f.submits[2].taskMessages).includes('unseen canonical answer'));
 assert.ok(f.submits[2].developerInstructions.length>0);
 await f.transport.close();
});
test('an uncommitted successful summary does not poison the warm native prefix or create another conversation',async()=>{
 const f=fixture([[publicText('first','completed work','final')],[publicText('summary','checkpoint text','final')],[publicText('last','continued work','final')]]);
 const contexts=[];
 const exchange=payload=>f.transport.exchange(payload,undefined,undefined,undefined,event=>contexts.push(event),route);
 await exchange(task);
 const history=[...task.messages,{role:'assistant',content:'completed work'}];
 await exchange({...task,compaction:true,messages:[...history,{role:'user',content:'Transient native summary instruction'}]});
 const result=await exchange({...task,messages:[...history,{role:'user',content:'Next real development task'}]});
 assert.equal(result.text,'continued work');
 assert.equal(f.operations.filter(m=>m==='create').length,1);
 assert.deepEqual(f.submits[2].taskMessages,[{role:'user',text:'Next real development task'}]);
 assert.equal(f.submits[2].developerInstructions.length,0);
 assert.deepEqual(contexts.map(e=>[e.mode,e.deltaMessages,e.compaction]),[['replay',1,false],['append',2,true],['append',1,false]]);
 await f.transport.close();
});
test('cancelled compaction excludes its transient prompt, but cannot automatically retry itself',async()=>{
 const f=fixture([[publicText('first','completed work','final')],[],[publicText('last','recovered','final')]]);
 await f.exchange();
 const history=[...task.messages,{role:'assistant',content:'completed work'}];
 const compact={...task,compaction:true,messages:[...history,{role:'user',content:'Internal summarization instruction'}]};
 const controller=new AbortController(),invoke=f.transport.invoke;
 f.transport.invoke=async(...args)=>{const value=await invoke(...args);if(args[2]==='submit')controller.abort(Error('owned_cancel'));return value;};
 await assert.rejects(()=>f.transport.exchange(compact,controller.signal,undefined,undefined,undefined,route),/owned_cancel/);
 await assert.rejects(()=>f.exchange(compact),/recovery_requires_new_input/);
 await assert.rejects(()=>f.exchange({...task,messages:history}),/recovery_requires_new_input/);
 assert.equal(f.submits.length,2);
 f.transport.invoke=invoke;
 const result=await f.exchange({...task,messages:[...history,{role:'user',content:'Continue from completed work'}]});
 assert.equal(result.text,'recovered');assert.equal(f.submits.length,3);
 assert.equal(f.events.filter(e=>e.type==='tool_call_start').length,0);
 await f.transport.close();
});
test('identified app replacement reconstructs canonical completed results without replaying old tools',async()=>{
 const f=fixture([[{call:{id:'before-restart',name:'shell',input:'{"cmd":"owned mutation"}'}}],[publicText('recovered','recovered after app restart','final')]]);
 let generation=1;
 const acquire=f.transport.acquire;
 f.transport.acquire=async()=>({...await acquire(),currentGeneration:()=>generation});
 const first=await f.exchange();assert.equal(first.calls.length,1);generation=2;
 const messages=[...task.messages,{role:'assistant',content:[{type:'toolCall',id:first.calls[0].id,name:'shell',arguments:{cmd:'owned mutation'}}]},
  {role:'toolResult',toolCallId:first.calls[0].id,toolName:'shell',content:'exact completed result',isError:false}];
 const final=await f.exchange({...task,messages});
 assert.equal(final.text,'recovered after app restart');assert.equal(f.releases(),1);
 assert.equal(f.operations.filter(m=>m==='create').length,2);assert.equal(f.submits.length,2);
 assert.equal(f.submits[1].results,undefined);assert.ok(f.submits[1].taskMessages.some(m=>m.hidden&&m.text.includes('exact completed result')));
 assert.equal(f.events.filter(e=>e.type==='tool_call_start').length,1);
 await f.transport.close();
});
