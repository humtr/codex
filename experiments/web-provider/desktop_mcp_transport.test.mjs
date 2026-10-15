import {test} from 'node:test';
import assert from 'node:assert/strict';
import {DesktopMcpBroker} from './desktop_mcp_broker.mjs';
import {DesktopMcpTransport,installMcpAppHooks,mcpAppOperation,invokeMcpApp} from './desktop_mcp_transport.mjs';
import {chatModelRoute} from './desktop_model_controls.mjs';

const tools=[{name:'shell',wireName:'functions__shell',parameters:{type:'object'}},{name:'patch',wireName:'patch',freeform:true}];
const task={owner:{backend:'owned',thread:'a',turn:'1'},instructions:['full permission'],messages:[{role:'user',content:'owned task'}],tools};
const route=chatModelRoute({model:'chatgpt-desktop/gpt-6'});
const final=text=>({id:'final',author:{role:'assistant'},recipient:'all',channel:'final',content:{content_type:'text',parts:[text]},status:'finished_successfully',end_turn:true});
const result=(call,text)=>({role:'toolResult',toolCallId:call.id,content:[{type:'text',text}],isError:false});
function fixture(){
 const broker=new DesktopMcpBroker(),events=[],writes=[];let transport,ended=false,releases=0,remote;
 transport=new DesktopMcpTransport('/owned',{broker,connectorName:'User chosen app',sleep:async()=>new Promise(r=>setTimeout(r,1)),timeoutMs:1000,
  acquire:async()=>({scopeKey:'fixture',client:{generation:'owned'},release(){releases++;}}),
  invoke:async(_lease,_lane,method,data)=>{
   if(method==='create')return true;
   if(method==='close')return true;
   if(method==='submit'){
    writes.push(data);
    remote=(async()=>{
     for(const [call_id,report] of [['a','initial report'],['b','intermediate report']]){
      const output=await broker.invoke(transport.token,{call_id,name:'functions__shell',input:'{"cmd":"owned"}',report});
      assert.ok(output.content[0].text);
     }ended=true;
    })();remote.catch(()=>{});return true;
   }
   return {starts:1,messages:ended?[final('complete')]:[],terminals:ended?['done']:[]};
  }});
 return {broker,transport,events,writes,exchange:p=>transport.exchange(p,undefined,e=>events.push(e),undefined,undefined,route,e=>events.push(e)),releases:()=>releases};
}

test('two retained MCP calls share native projection and one Chat submission; exact results continue original host work',async()=>{
 const f=fixture();
 try{
  const first=await f.exchange(task);assert.equal(first.type,'tools');assert.equal(f.events[1].text,'initial report');
  const secondPayload={...task,messages:[...task.messages,result(first.calls[0],'first')]};
  const second=await f.exchange(secondPayload);assert.equal(second.type,'tools');
  const done=await f.exchange({...secondPayload,messages:[...secondPayload.messages,result(second.calls[0],'second')]});
  assert.equal(done.text,'complete');assert.equal(f.writes.length,1);assert.equal(f.events.filter(e=>e.kind==='user').length,1);
  assert.deepEqual(f.events.filter(e=>e.type==='text_delta').map(e=>e.text),['initial report','intermediate report','complete']);
  assert.equal(f.broker.turns.size,0);assert.equal(f.releases(),1);
 }finally{await f.transport.close();}
});

test('invalid pending input/route/results refuse without retiring the correct owner or submitting a replay',async()=>{
 const f=fixture();
 try{
  const first=await f.exchange(task),token=f.transport.token;
  await assert.rejects(()=>f.exchange(task),/missing_or_duplicate/);
  await assert.rejects(()=>f.exchange({...task,owner:{...task.owner,turn:'other'}}),/pending_owner_change/);
  await assert.rejects(()=>f.exchange({...task,messages:[...task.messages,result(first.calls[0],'ok'),{role:'user',content:'steer'}]}),/pending_new_input_unqualified/);
  assert.equal(f.transport.token,token);assert.equal(f.writes.length,1);assert.equal(f.broker.get(token).active,true);
 }finally{await f.transport.close();}
});

test('observer failure after dispatch blocks replay even after retirement; sibling broker owner remains active',async()=>{
 const broker=new DesktopMcpBroker(),other=broker.register({...task.owner,thread:'sibling'},tools);let submits=0;
 const t=new DesktopMcpTransport('/owned',{broker,connectorName:'owned',acquire:async()=>({release(){}}),invoke:async(_l,_n,method)=>{if(method==='submit')submits++;if(method==='snapshot')throw Error('observer gone');return true;}});
 await assert.rejects(()=>t.exchange(task,undefined,undefined,undefined,undefined,route),/observer gone/);
 t.retire();await t.retiring;
 await assert.rejects(()=>t.exchange(task,undefined,undefined,undefined,undefined,route),/delivery_unknown/);
 assert.equal(submits,1);assert.equal(broker.get(other).active,true);broker.close();
});

test('host terminal completion cannot admit a queued native action after the host has departed',async()=>{
 const broker=new DesktopMcpBroker();let t,remote;
 t=new DesktopMcpTransport('/owned',{broker,connectorName:'owned',acquire:async()=>({release(){}}),invoke:async(_l,_n,method)=>{
  if(method==='submit'){remote=broker.invoke(t.token,{call_id:'late',name:'functions__shell',input:'{}',report:'late'}).catch(error=>error.message);}
  return method==='snapshot'?{starts:1,messages:[final('left')],terminals:['done']}:true;
 }});
 await assert.rejects(()=>t.exchange(task,undefined,undefined,undefined,undefined,route),/terminal_with_pending_calls/);
 assert.equal(await remote,'mcp_turn_retired');assert.equal(t.projection.calls.size,0);assert.equal(broker.turns.size,0);
});

test('MCP app hooks exclude local executors, preserve sibling traffic, budget one start and restore last owner',async()=>{
 const client={prepareCompletionStream:r=>r,startCompletionStream:o=>o},api={streamPost:(_p,o)=>o};
 const s={scope:{get:()=>client},m:{oMn:{},eHt(){}}};installMcpAppHooks(s,{$wt:api});
 const original=s.mcpNative.saved,lane=id=>({id,marker:id,messages:new Map(),terminals:[],starts:0});
 s.mcpNative.lanes.set('a',lane('a'));s.mcpNative.lanes.set('b',lane('b'));
 const req={conversation_id:'a',messages:[{author:{role:'user'},content:{parts:['owned']}}],local_function_signatures:[{name:'handoff'}]};
 const foreign={conversation_id:'other',local_function_signatures:[{name:'handoff'}]};assert.equal(client.prepareCompletionStream(foreign),foreign);
 const prepared=client.prepareCompletionStream(req);assert.deepEqual(prepared.local_function_signatures,[]);
 const opts=client.startCompletionStream({request:prepared});opts.onUpdate({message:{id:'owned-public',author:{role:'assistant'}}});assert.equal(s.mcpNative.lanes.get('b').messages.size,0);
 const dispatch=api.streamPost('/f/conversation',{body:JSON.stringify(prepared),retry:{attempts:3}});assert.equal(dispatch.retry,undefined);
 dispatch.chatgptSse.onBeforeRequestStart();assert.throws(()=>dispatch.chatgptSse.onBeforeRequestStart(),/replayed/);
 globalThis.window={fixture:s};
 try{await mcpAppOperation('fixture','a','close');assert.notEqual(client.startCompletionStream,original.startCompletionStream);await mcpAppOperation('fixture','b','close');assert.equal(client.startCompletionStream,original.startCompletionStream);assert.equal(api.streamPost,original.post);}
 finally{delete globalThis.window;}
});

test('serialized MCP operation strips private errors and reads only its own Scope branch',async()=>{
 const s={scope:{get:()=>({one:{message:final('owned')}})},m:{JRn:{}},mcpNative:{lanes:new Map([['a',{id:'owned',messages:new Map(),terminals:[],starts:0}]])}};
 const lease={scopeKey:'fixture',client:{evaluate:async text=>new Function('window','return '+text)({fixture:s})}};
 assert.equal((await invokeMcpApp(lease,'a','snapshot')).messages[0].content.parts[0],'owned');
 s.scope.get=()=>{throw Error('private account content');};
 await assert.rejects(()=>invokeMcpApp(lease,'a','snapshot'),/^Error: mcp_app_operation_failed$/);
});
