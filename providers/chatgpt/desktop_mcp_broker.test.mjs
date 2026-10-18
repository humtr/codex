import {test} from 'node:test';
import assert from 'node:assert/strict';
import {mkdtempSync,rmSync,statSync} from 'node:fs';
import {join} from 'node:path';
import {DesktopMcpBroker,serveMcpBroker,callMcpBroker} from './desktop_mcp_broker.mjs';
import {handleNativeMcp,nativeMcpTools,runNativeMcp} from './desktop_mcp_stdio.mjs';

const tools=[{name:'exec',wireName:'functions__exec',freeform:true,format:{type:'grammar'}},{name:'tool_search',wireName:'tool_search',toolSearch:true,parameters:{type:'object'}},{name:'read',wireName:'mcp__app__read',parameters:{type:'object'}}];
const owner={backend:'owned-backend',thread:'a',turn:'1'};
const args=(id='one',name='functions__exec',input='text("한글");')=>({call_id:id,name,input,report:'진행 보고'});
const observed=p=>p.then(value=>({value}),error=>({error:error.message}));
const pause=()=>new Promise(resolve=>setTimeout(resolve,5));
async function until(predicate){for(let i=0;i<200;i++){if(predicate())return;await pause();}throw Error('fixture timeout');}

test('original logical calls settle once across distinct/reused RPCs, without losing exact native output',async()=>{
 const b=new DesktopMcpBroker(),token=b.register(owner,tools);
 try{
  const a=observed(b.invoke(token,args())),duplicate=observed(b.invoke(token,args()));
  assert.equal(b.pending(token).length,1);const calls=b.pending(token);b.offered(token,calls.map(c=>c.id));
  assert.throws(()=>b.finish(token),/pending_completion/);
  assert.throws(()=>b.update(token,tools),/pending_catalog_change/);
  assert.throws(()=>b.invoke(token,{...args(),input:'changed'}),/call_changed/);
  const result={content:[{type:'text',text:'실패 ⚙️'},{type:'image',data:'fixture',mimeType:'image/png'}],isError:true};
  b.settle(token,[{call:calls[0],result}]);assert.deepEqual((await a).value,result);assert.deepEqual((await duplicate).value,result);
  assert.deepEqual(await b.invoke(token,args()),result);assert.equal(b.pending(token).length,0);
  assert.throws(()=>b.settle(token,[{call:calls[0],result}]),/result_invalid/);
  b.update(token,tools);assert.deepEqual(b.catalog(token),tools);
  b.finish(token);assert.throws(()=>b.invoke(token,args()),/turn_unavailable/);
 }finally{b.close();}
});

test('parallel batch validation is atomic; observer detach retains action, result and sibling owner',async()=>{
 const b=new DesktopMcpBroker(),t=b.register(owner,tools),other=b.register({...owner,thread:'b'},tools),abort=new AbortController();
 const first=observed(b.invoke(t,args('one'),abort.signal)),second=observed(b.invoke(t,args('two'))),sibling=observed(b.invoke(other,args('one')));
 assert.notEqual(b.pending(t)[0].id,b.pending(other)[0].id);
  abort.abort(Error('detach'));assert.equal((await first).error,'detach');
 const calls=b.pending(t);
 assert.throws(()=>b.offered(t,[calls[0].id,'foreign']),/offer_invalid/);assert.equal(b.pending(t).length,2);
 b.offered(t,calls.map(c=>c.id));
 const result={content:'owned',isError:false};
 assert.throws(()=>b.settle(t,[{call:calls[0],result},{call:{id:'foreign'},result}]),/result_invalid/);
 assert.equal(b.get(t).calls.get('one').result,undefined);
 b.settle(t,calls.map(call=>({call,result})));
 assert.deepEqual(await b.invoke(t,args('one')),result);assert.deepEqual((await second).value,result);
 b.revoke(t);assert.equal(b.pending(other).length,1);
 b.revoke(other);assert.equal((await sibling).error,'mcp_turn_retired');b.close();
});

test('strict catalog/custom/deferred input validation and unbound owner refuse before issuing work',()=>{
 const b=new DesktopMcpBroker();assert.throws(()=>b.register({thread:'a'},tools),/owner_required/);
 const t=b.register(owner,tools);assert.throws(()=>b.register(owner,tools),/owner_conflict/);
 assert.throws(()=>b.invoke('bad',args()),/turn_unavailable/);
 assert.throws(()=>b.invoke(t,args('one','foreign')),/unadvertised_tool/);
 assert.throws(()=>b.invoke(t,args('one','mcp__app__read','not json')),/input_invalid/);
 assert.throws(()=>b.invoke(t,args('one','tool_search','{}')),/input_invalid/);
 assert.equal(b.pending(t).length,0);b.close();
});

test('actual private Unix IPC detaches only a waiter and returns retained Unicode result on reconnect',async()=>{
 const root=mkdtempSync(join(process.env.TMPDIR,'mb-')),b=new DesktopMcpBroker(),socket=join(root,'s');
 const t=b.register(owner,tools),stop=await serveMcpBroker(b,socket);
 try{
  assert.equal(statSync(socket).mode&0o777,0o600);
  assert.deepEqual(await callMcpBroker(socket,{method:'catalog',token:t}),tools);
  const abort=new AbortController(),pending=observed(callMcpBroker(socket,{method:'invoke',token:t,args:args()},abort.signal));
  await until(()=>b.pending(t).length===1);const [call]=b.pending(t);b.offered(t,[call.id]);
  abort.abort(Error('fixture disconnect'));assert.equal((await pending).error,'fixture disconnect');
  const output={content:[{type:'text',text:'한글🙂'.repeat(25000)}],isError:false};b.settle(t,[{call,result:output}]);
  assert.deepEqual(await callMcpBroker(socket,{method:'invoke',token:t,args:args()}),output);assert.equal(b.pending(t).length,0);
  await assert.rejects(()=>callMcpBroker(socket,{method:'catalog',token:'foreign'}),/turn_unavailable/);
 }finally{await stop();rmSync(root,{recursive:true});}
});

test('MCP wire contract has repository-owned names, preserves IDs and returns command errors as data',async()=>{
 assert.deepEqual(nativeMcpTools.map(t=>t.name),['native_catalog','native_tool']);
 const request=(method,params)=>({jsonrpc:'2.0',id:0,method,params});
 assert.equal((await handleNativeMcp(request('initialize',{protocolVersion:'2025-11-25'}))).protocolVersion,'2025-11-25');
 await assert.rejects(()=>handleNativeMcp(request('initialize',{protocolVersion:'2026-07-28'})),/unqualified/);
 const catalog=await handleNativeMcp(request('tools/call',{name:'native_catalog',arguments:{turn_token:'a'.repeat(64)}}),async()=>tools);
 const entries=JSON.parse(catalog.content[0].text).tools;
 assert.deepEqual(entries.map(t=>t.name),['functions__exec','tool_search','mcp__app__read']);
 assert.equal(entries[0].freeform,true);assert.equal(entries[1].toolSearch,true);assert.deepEqual(entries[2].parameters,{type:'object'});
 assert.ok(entries.every(t=>!Object.hasOwn(t,'wireName')));
 const token='a'.repeat(64),result={content:[{type:'text',text:'error from native Codex'}],isError:true};
 const returned=await handleNativeMcp(request('tools/call',{name:'native_tool',arguments:{turn_token:token,...args()}}),async row=>{assert.equal(row.token,token);return result;});
 assert.equal(returned.isError,false);assert.deepEqual(JSON.parse(returned.content[0].text).output,result);
 const lines=[];let controller;
 const input=new ReadableStream({start(c){controller=c;}});
 const running=runNativeMcp('/absent/socket',input,line=>lines.push(JSON.parse(line)));
 const put=row=>controller.enqueue(new TextEncoder().encode(JSON.stringify(row)+'\n'));
 put(request('tools/list'));await until(()=>lines.length===1);assert.equal(lines[0].id,0);
 const [metadata,execution]=lines[0].result.tools;
 assert.deepEqual(metadata.annotations,{readOnlyHint:true,destructiveHint:false,idempotentHint:true,openWorldHint:false});
 assert.deepEqual(execution.annotations,{readOnlyHint:false,destructiveHint:true,idempotentHint:false,openWorldHint:true});
 assert.ok(lines[0].result.tools.every(t=>!/(MUST|Core owns|retry)/.test(t.description)));
 put(request('ping'));await until(()=>lines.length===2);assert.equal(lines[1].id,0);
 put({jsonrpc:'2.0',method:'notifications/initialized'});controller.close();await running;
 assert.equal(lines.length,2);
});
