import {test} from 'node:test';
import assert from 'node:assert/strict';
import {DesktopToolSession,localToolCall} from './desktop_tool_session.mjs';
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
 assert.throws(()=>localToolCall(request('shell','not JSON'),'gateway',tools),/invalid_tool_input/);
 assert.throws(()=>localToolCall(request('tool_search','{"query":""}'),'gateway',tools),/invalid_tool_search/);
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
