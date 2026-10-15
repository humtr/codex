import {test} from 'node:test';
import assert from 'node:assert/strict';
import {DesktopLocalTransport,installLocalAppHooks,localAppOperation,invokeLocalOperation,pendingNativeContext} from './desktop_local_transport.mjs';
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
  const messages=rounds[round].map(m=>m.call?{id:m.call.id,author:{role:'assistant'},recipient:'local.'+gateway,status:'finished_successfully',content:{content_type:'code',text:JSON.stringify({name:m.call.name,input:m.call.input,report:m.call.report??''})}}:m);
  return {messages,accepted:true,roundDispatched:1,terminals:['done'],userSubmissions:submits.reduce((n,s)=>n+(s.prompt?1:0)+(s.contextMessages||[]).filter(m=>m.role==='user').length,0),toolSubmissions:submits.reduce((n,s)=>n+(s.results?.length||0),0),dispatches:round+1};
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
  assert.ok(f.submits[0].prompt);assert.ok(f.submits.slice(1).every(s=>s.prompt===undefined));
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
test('an ambiguous dispatched failure blocks a new generation even after retire',async()=>{
 const f=fixture([[{call:{id:'a',name:'foreign',input:'{}'}}]]);
 await assert.rejects(()=>f.exchange(),/unadvertised_tool/);assert.equal(f.submits.length,1);
 f.transport.retire();await f.transport.retiring;
 await assert.rejects(()=>f.exchange(),/delivery_unknown/);assert.equal(f.submits.length,1);
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
 const transport=new DesktopLocalTransport('/owned',{acquire:async()=>({release(){releases++;}}),
  invoke:async(_lease,_lane,method)=>{if(method==='close'){closes++;await new Promise(resolve=>{finish=resolve;});throw Error('desktop_disconnected');}return true;}});
 await transport.prepare();
 const first=transport.close();transport.retire();
 assert.equal(closes,1);assert.equal(transport.lease,undefined);
 finish();await assert.rejects(first,/desktop_disconnected/);await transport.retiring;
 assert.equal(releases,1);
});
