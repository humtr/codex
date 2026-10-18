import {test} from 'node:test';
import assert from 'node:assert/strict';
import {DesktopToolSession,localToolCall,localToolCalls,localToolSignature,localToolCatalog} from './desktop_tool_session.mjs';
import {recordDesktopSubmission} from './desktop_protocol.mjs';

const tools=[{wireName:'shell',parameters:{}},{wireName:'patch',freeform:true},{wireName:'mcp__github__read',parameters:{}},{wireName:'tool_search',toolSearch:true}];
const message=(id,text,extra={})=>({id,author:{role:'assistant'},recipient:'all',channel:'commentary',content:{content_type:'text',parts:[text]},status:'in_progress',end_turn:false,...extra});
test('public reports stream once in order; analysis, hidden payloads and remote tool code never become prose',()=>{
 const session=new DesktopToolSession(),events=[],emit=e=>events.push(e);
 const first=message('initial','먼저 확인');session.text(first,emit);session.text(first,emit);
 session.text(message('analysis','private',{channel:'analysis'}),emit);
 session.text(message('hidden','private',{metadata:{is_visually_hidden_from_conversation:true}}),emit);
 session.text(message('code','private',{content:{content_type:'code',text:'private'}}),emit);
 session.text(message('remote','private',{recipient:'local.owned'}),emit);
 session.text(message('initial','먼저 확인합니다.',{status:'finished_successfully'}),emit);
 session.issue({id:'call',name:'shell',input:{cmd:'pwd'}},tools,emit);
 session.text(message('progress','확인했습니다.',{status:'finished_successfully'}),emit);
 session.text(message('final','완료',{channel:null,end_turn:null}),emit);
 session.text(message('final','완료',{channel:null,end_turn:true,status:'finished_successfully'}),emit);
 assert.deepEqual(events.filter(e=>e.type==='text_delta').map(e=>[e.phase,e.text]),[['commentary','먼저 확인'],['commentary','합니다.'],['commentary','확인했습니다.'],['final_answer','완료']]);
 assert.ok(events.findIndex(e=>e.type==='tool_call_start')>0);
 assert.throws(()=>session.text(message('initial','rewritten'),emit),/rewritten/);
});
test('Chat final display channel before a tool is commentary until the actual end_turn',()=>{
 const s=new DesktopToolSession(),events=[];
 s.text(message('initial','initial report',{channel:'final',status:'finished_successfully',end_turn:false}),e=>events.push(e));
 s.text(message('last','final answer',{channel:'final',status:'finished_successfully',end_turn:true}),e=>events.push(e));
 assert.deepEqual(events.filter(e=>e.type==='text_delta').map(e=>e.phase),['commentary','final_answer']);
});
test('native gateway preserves namespaces, custom source and discovery arguments without executing tools',()=>{
 const request=(name,input,id='call')=>({id,author:{role:'assistant'},recipient:'local.gateway',status:'finished_successfully',content:{content_type:'code',text:JSON.stringify({kwargs:{name,input,report:''}})}});
 assert.deepEqual(localToolCall(request('mcp__github__read','{"path":"owned"}'),'gateway',tools),{id:'call',name:'mcp__github__read',input:{path:'owned'},report:''});
 assert.equal(localToolCall(request('patch','*** Begin Patch\n*** End Patch'),'gateway',tools).input,'*** Begin Patch\n*** End Patch');
 assert.equal(localToolCall(request('shell','{}'),'other',tools),undefined);
 assert.throws(()=>localToolCall(request('foreign','{}'),'gateway',tools),/unadvertised_tool/);
 assert.equal(localToolCall(request('shell','not JSON'),'gateway',tools).rawArguments,'not JSON');
 assert.throws(()=>localToolCall(request('tool_search','{"query":""}'),'gateway',tools),/invalid_tool_search/);
});
test('malformed ordinary arguments stay exact for Core errors; names, freeform and search retain their boundaries',()=>{
 const request=(name,input)=>({id:'bad',author:{role:'assistant'},recipient:'local.gateway',status:'finished_successfully',content:{text:JSON.stringify({name,input,report:''})}});
 for(const input of ['not JSON','{"cmd":','[1,2]','null','true','42','"string"',' \n']){
  const call=localToolCall(request('shell',input),'gateway',tools),s=new DesktopToolSession(),events=[];
  assert.equal(call.rawArguments,input);assert.equal(call.input,undefined);
  s.issue(call,tools,e=>events.push(e));s.issue(call,tools,e=>events.push(e));
  assert.deepEqual(events.filter(e=>e.type==='tool_call_delta').map(e=>e.arguments),[input]);
  const batch=s.results([{role:'toolResult',toolCallId:'bad',content:'actual Core parse error',isError:true}]);
  s.returned(batch);assert.equal(s.pending().length,0);assert.equal(batch[0].result.isError,true);
 }
 assert.throws(()=>localToolCall(request('shell',''),'gateway',tools),/invalid_tool_input/);
 assert.throws(()=>localToolCall(request('tool_search','broken'),'gateway',tools),/invalid_tool_input/);
 assert.throws(()=>localToolCall(request('foreign','broken'),'gateway',tools),/unadvertised_tool/);
 assert.equal(localToolCall(request('patch','broken'),'gateway',tools).input,'broken');
 for(const call of [{id:'a',name:'patch',rawArguments:'bad'},{id:'a',name:'tool_search',rawArguments:'bad'},
  {id:'a',name:'shell',rawArguments:'{}'},{id:'a',name:'shell',rawArguments:'bad',input:{cmd:'do not execute'}}]){
  assert.throws(()=>new DesktopToolSession().issue(call,tools,()=>assert.fail('no partial call')),/call_invalid/);
 }
});
test('catalog has one exact callable name; invalid names keep only bounded content-free diagnosis',()=>{
 const native={name:'_read',namespace:'mcp__owned',wireName:'mcp__owned___read',freeform:true,description:'source',parameters:{type:'string'}};
 assert.deepEqual(localToolCatalog([native]),[{freeform:true,description:'source',parameters:{type:'string'},name:'mcp__owned___read'}]);
 for(const [name,kind] of [['_read','bare_name'],['mcp__owned._read','dotted_namespace'],['mcp__owned','namespace_only'],['private user content','unknown']]){
  assert.throws(()=>record(name),error=>{
   assert.equal(error.message,'unadvertised_tool');assert.equal(error.toolDiagnostic.nameClass,kind);
   assert.match(error.toolDiagnostic.nameDigest,/^[a-f0-9]{64}$/);assert.match(error.toolDiagnostic.catalogDigest,/^[a-f0-9]{64}$/);
   assert.equal(error.toolDiagnostic.toolCount,1);assert.ok(!JSON.stringify(error.toolDiagnostic).includes(name));return true;
  });
 }
 function record(name){return localToolCall({id:'rejected',author:{role:'assistant'},recipient:'local.gateway',status:'finished_successfully',content:{text:JSON.stringify({name,input:'private arguments',report:'private report'})}},'gateway',[native]);}
});
test('a gateway report is public commentary before its tool, without duplicate prose or native argument changes',()=>{
 const s=new DesktopToolSession(),events=[],emit=e=>events.push(e);
 s.text(message('public','확인합니다.',{status:'finished_successfully'}),emit);
 const first={id:'first',name:'shell',input:{cmd:'pwd'},report:'확인합니다.'};
 s.issue(first,tools,emit);s.issue(first,tools,emit);
 s.issue({id:'second',name:'shell',input:{cmd:'ls'},report:'확인한 내용을 바탕으로 파일을 살핍니다.'},tools,emit);
 s.issue({id:'third',name:'shell',input:{cmd:'ls'},report:'확인한 내용을 바탕으로 파일을 살핍니다.'},tools,emit);
 assert.deepEqual(events.filter(e=>e.type==='text_delta').map(e=>e.text),['확인합니다.','확인한 내용을 바탕으로 파일을 살핍니다.','확인한 내용을 바탕으로 파일을 살핍니다.']);
 const middle=events.findIndex(e=>e.text==='확인한 내용을 바탕으로 파일을 살핍니다.');
 assert.ok(middle<events.findIndex(e=>e.id==='second'));
 assert.deepEqual(events.filter(e=>e.type==='tool_call_delta').map(e=>JSON.parse(e.arguments)),[{cmd:'pwd'},{cmd:'ls'},{cmd:'ls'}]);
});
test('parallel results validate atomically and bind each original ID once, including errors',()=>{
 const s=new DesktopToolSession(),emit=()=>{};
 for(const id of ['first','second'])s.issue({id,name:'shell',input:{id}},tools,emit);
 s.issue({id:'first',name:'shell',input:{id:'first'}},tools,()=>assert.fail('duplicate emitted'));
 assert.throws(()=>s.issue({id:'first',name:'patch',input:'changed'},tools,emit),/call_changed/);
 const result=(toolCallId,content,isError=false)=>({role:'toolResult',toolCallId,content,isError});
 assert.throws(()=>s.results([result('first','ok')]),/missing_or_duplicate/);
 assert.equal(s.pending().length,2);
 assert.throws(()=>s.results([result('first','ok'),result('first','again'),result('second','bad',true)]),/missing_or_duplicate/);
 const batch=s.results([result('second','failure',true),result('first','ok')]);
 assert.deepEqual(batch.map(x=>[x.call.id,x.result.isError]),[['first',false],['second',true]]);
 s.returned(batch);assert.equal(s.pending().length,0);
 assert.throws(()=>s.returned(batch),/replayed/);
 assert.throws(()=>s.results([result('first','changed')]),/result_changed/);
});
test('tool-role result submissions are counted separately from user messages',()=>{
 const report={observedChatSubmissions:0,submittedRequests:0,submittedContextRequests:0,submissionSources:{app_accepted:0,ui_observed:0}};
 for(const kind of ['user','tool','tool'])recordDesktopSubmission(report,{phase:'request',source:'app_accepted',kind},{mode:'instant'});
 assert.equal(report.observedChatSubmissions,1);assert.equal(report.toolResultSubmissions,2);
 assert.equal(report.submittedRequests,1);
});
test('message counts, including parallel results and new user input, stay separate from model dispatches',()=>{
 const r={observedChatSubmissions:0,submittedRequests:0,submissionSources:{}};
 recordDesktopSubmission(r,{phase:'request',source:'app_dispatched',kind:'mixed',userMessages:2,toolMessages:3},{mode:'instant'});
 assert.equal(r.observedChatSubmissions,2);assert.equal(r.toolResultSubmissions,3);
 assert.equal(r.submittedRequests,1);assert.equal(r.observedNativeDispatches,1);
 assert.throws(()=>recordDesktopSubmission(r,{phase:'request',source:'app_dispatched',kind:'tool',userMessages:1,toolMessages:1},{}),/invalid_desktop_submission/);
 assert.equal(r.observedNativeDispatches,1);
});

test('explicit gateway batch keeps per-tool arguments and stable native Codex IDs, one report and one original host result',()=>{
 const entries=[{name:'shell',input:'{"cmd":"pwd"}'},{name:'patch',input:'*** Begin Patch\n*** End Patch'},{name:'mcp__github__read',input:'{"path":"owned"}'},{name:'shell',input:'{"cmd":'}];
 const request=(calls=entries)=>({id:'original-host',author:{role:'assistant'},recipient:'local.gateway',status:'finished_successfully',content:{text:JSON.stringify({kwargs:{calls:JSON.stringify(calls),report:'Public batch report'}})}});
 const s=new DesktopToolSession(),events=[],calls=s.decode(request(),'gateway',tools);
 assert.equal(calls.length,4);assert.equal(new Set(calls.map(c=>c.id)).size,4);
 assert.deepEqual(localToolCalls(request(),'gateway',tools),calls);
 assert.notEqual(localToolCalls({...request(),id:'other-host'},'gateway',tools)[0].id,calls[0].id);
 assert.equal(calls[1].input,entries[1].input);assert.equal(calls[3].rawArguments,entries[3].input);
 for(const c of calls)s.issue(c,tools,e=>events.push(e));
 assert.deepEqual(events.filter(e=>e.type==='text_delta').map(e=>e.text),['Public batch report']);
 assert.throws(()=>s.decode(request(entries.slice(0,2)),'gateway',tools),/call_changed/);
 const results=calls.map((c,i)=>({role:'toolResult',toolCallId:c.id,content:i===3?'Core parse error':i===1?'Command interrupted':'output-'+i,isError:i===1||i===3}));
 assert.throws(()=>s.results(results.slice(0,3)),/missing_or_duplicate/);
 const batch=s.results(results.toReversed()),host=s.hostResults(batch);
 assert.equal(host.length,1);assert.equal(host[0].id,'original-host');
 assert.deepEqual(host[0].result.results,calls.map((c,i)=>({id:c.id,name:c.name,content:results[i].content,isError:results[i].isError})));
 assert.throws(()=>s.hostResults(batch.slice(0,3)),/missing_or_duplicate/);
 assert.throws(()=>s.hostResults([...batch,batch[0]]),/missing_or_duplicate/);
 s.returned(batch);assert.throws(()=>s.returned(batch),/replayed/);
 const sig=localToolSignature('gateway');assert.equal(sig.params.find(p=>p.name==='calls').required,false);
});
test('batch envelope fails atomically for malformed, mixed, oversized or unknown entries',()=>{
 const envelope=args=>({id:'host',author:{role:'assistant'},recipient:'local.gateway',status:'finished_successfully',content:{text:JSON.stringify(args)}});
 const entry={name:'shell',input:'{}'};
 for(const args of [
  {calls:'bad',report:''},{calls:'[]',report:''},{calls:'{}',report:''},{calls:[],report:''},
  {calls:JSON.stringify([entry]),report:3},{calls:JSON.stringify([entry]),report:'',name:'shell',input:'{}'},
  {calls:JSON.stringify([entry,null]),report:''},{calls:JSON.stringify([entry,{name:'shell',input:{}}]),report:''},
  {calls:JSON.stringify(Array(33).fill(entry)),report:''},
 ])assert.throws(()=>localToolCalls(envelope(args),'gateway',tools),/call_invalid/);
 const s=new DesktopToolSession();
 assert.throws(()=>s.decode(envelope({calls:JSON.stringify([entry,{name:'unknown',input:'{}'}]),report:''}),'gateway',tools),/unadvertised_tool/);
 assert.equal(s.calls.size,0);assert.equal(s.hostCalls.size,0);
 assert.deepEqual(localToolCalls({...envelope({}),recipient:'local.foreign'},'gateway',tools),[]);
});

test('several original host calls keep separate returns while batching only their own children',()=>{
 const request=(id,args)=>({id,author:{role:'assistant'},recipient:'local.gateway',status:'finished_successfully',content:{text:JSON.stringify(args)}});
 const s=new DesktopToolSession(),entries=[{name:'shell',input:'{}'},{name:'patch',input:'source'}];
 const calls=[...s.decode(request('host-one',{calls:JSON.stringify(entries),report:''}),'gateway',tools),
  ...s.decode(request('host-two',{calls:JSON.stringify([entries[0]]),report:''}),'gateway',tools),
  ...s.decode(request('host-single',{...entries[0],report:''}),'gateway',tools)];
 for(const call of calls)s.issue(call,tools,()=>{});
 const batch=s.results(calls.map(call=>({role:'toolResult',toolCallId:call.id,content:call.id})));
 const returned=s.hostResults(batch);assert.deepEqual(returned.map(r=>r.id),['host-one','host-two','host-single']);
 assert.deepEqual(returned.slice(0,2).map(r=>r.result.results.length),[2,1]);assert.equal(returned[2].result.content,'host-single');
});

test('missing public report never rejects an otherwise exact single or batch invocation',()=>{
 const host=args=>({id:'real-rejected',author:{role:'assistant'},recipient:'local.gateway',status:'finished_successfully',content:{content_type:'code',text:JSON.stringify(args)}});
 const single=localToolCalls(host({name:'patch',input:'exact patch source'}),'gateway',tools);
 assert.equal(single[0].input,'exact patch source');assert.equal(single[0].report,'');
 const batch=localToolCalls(host({calls:JSON.stringify([{name:'shell',input:'{}'},{name:'patch',input:'exact source'}])}),'gateway',tools);
 assert.equal(batch.length,2);assert.ok(batch.every(c=>c.report===''));
 for(const report of [null,17,{},[]])assert.throws(()=>localToolCalls(host({name:'shell',input:'{}',report}),'gateway',tools),/call_invalid/);
 const session=new DesktopToolSession(),events=[];session.issue(single[0],tools,e=>events.push(e));
 assert.equal(events.filter(e=>e.type==='text_delta').length,0);assert.equal(events.filter(e=>e.type==='tool_call_start').length,1);
});
