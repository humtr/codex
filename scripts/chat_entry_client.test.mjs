import {test, expect} from 'bun:test';
import {readFileSync,mkdtempSync,writeFileSync,rmSync} from 'node:fs';
import {join} from 'node:path';
import {spawnSync} from 'node:child_process';

// Exercise the actual self-contained function injected into the app context.
const source=readFileSync(new URL('./chat_entry_client.mjs',import.meta.url),'utf8');
const definition=source.match(/const canonical=(.*);\n   const digest=/)[1];
const canonical=eval('('+definition+')');
const graph=()=>({a:{id:'a',parent:null,children:['b'],message:{id:'a',update_time:1,content:{parts:['original'],update_time:7},metadata:{hidden:true}}},b:{id:'b',parent:'a',children:[],message:null}});

test('title preservation ignores only volatile message timestamps and object order',()=>{
 const before=graph(),after=graph();after.a.message.update_time=2;
 after.a.message={metadata:after.a.message.metadata,content:after.a.message.content,update_time:2,id:'a'};
 expect(canonical(before)).toEqual(canonical(after));
 expect(before.a.message.update_time).toBe(1);
});

test('title preservation detects text, hidden metadata, graph and nested timestamp changes',()=>{
 for(const mutate of [g=>g.a.message.content.parts[0]='changed',g=>g.a.message.metadata.hidden=false,
  g=>g.b.parent=null,g=>g.a.children=[],g=>g.a.message.id='other',g=>g.a.message.content.update_time=8,
  g=>g.a.update_time=2,g=>delete g.b]){
  const before=graph(),after=graph();mutate(after);
  expect(canonical(before)).not.toEqual(canonical(after));
 }
});

test('cleanup retains the original error in an exclusive evidence file',()=>{
 const body=source.match(/const save=([\s\S]*?\n});/)[1],files=new Map();let id=0;
 const write=(name,data,options)=>{
  if(options.flag==='wx'&&files.has(name))throw Error('existing evidence');
  files.set(name,{data,options});
 };
 const save=new Function('root','writeFileSync','randomUUID','return ('+body+')')('/fixture',write,()=>String(++id));
 save('error-private.json',{action:'init',error:'assignment_budget_changed'});
 save('error-private.json',{action:'snapshot',error:'input_client_owner_changed'});
 expect(JSON.parse(files.get('/fixture/error-private-1.json').data).action).toBe('init');
 expect(JSON.parse(files.get('/fixture/error-private-2.json').data).action).toBe('snapshot');
 expect(JSON.parse(files.get('/fixture/error-private.json').data).action).toBe('snapshot');
 expect(files.get('/fixture/error-private-1.json').options).toEqual({mode:0o600,flag:'wx'});
});

test('connector refresh identifies nested gateway rate limits inside HTTP 424',()=>{
 const refresh=readFileSync(new URL('./d3_refresh.mjs',import.meta.url),'utf8');
 const parse=eval('('+refresh.match(/function responseFailure\(error\)\{[\s\S]*?\n\}/)[0]+')');
 expect(parse({responseStatus:424,message:JSON.stringify({detail:{upstream_status:429}})})).toEqual({status:424,upstream_status:429});
 expect(parse({status:429,message:'rate limited'})).toEqual({status:429,upstream_status:null});
 expect(parse({status:424,message:'ordinary dependency rejection'})).toEqual({status:424,upstream_status:null});
});

test('connector refresh honors an existing cooldown before connector or network access',()=>{
 const root=mkdtempSync(join(process.env.TMPDIR,'g-refresh-cooldown-'));
 try{
  writeFileSync(join(root,'info.json'),JSON.stringify({state:join(root,'state'),connector_reference:join(root,'must-not-be-read')}));
  writeFileSync(join(root,'connector-refresh-cooldown.json'),JSON.stringify({not_before_unix_ms:Date.now()+60000}));
  const result=spawnSync(process.execPath,[new URL('./d3_refresh.mjs',import.meta.url).pathname,root],{encoding:'utf8'});
  expect(result.status).not.toBe(0);
  expect(result.stderr).toContain('MCP refresh cooldown active; no network request');
 }finally{rmSync(root,{recursive:true});}
});

test('connector refresh rejects a mismatched configured name before app access',()=>{
 const root=mkdtempSync(join(process.env.TMPDIR,'g-refresh-owner-'));
 try{
  writeFileSync(join(root,'info.json'),JSON.stringify({connector_name:'Termux Bridge',connector_reference:join(root,'connector.json')}));
  writeFileSync(join(root,'connector.json'),JSON.stringify({matches:[{name:'Different connector',connector_id:'owned'}]}));
  const result=spawnSync(process.execPath,[new URL('./d3_refresh.mjs',import.meta.url).pathname,root],{encoding:'utf8'});
  expect(result.status).not.toBe(0);
  expect(result.stderr).toContain('exact_owned_connector_required');
 }finally{rmSync(root,{recursive:true});}
});

const refreshSource=readFileSync(new URL('./d3_refresh.mjs',import.meta.url),'utf8');
const readConnectorWhenReady=eval('('+refreshSource.match(/async function readConnectorWhenReady\([\s\S]*?\n\}/)[0]+')');
const uninitialized=()=>new TypeError("Cannot read properties of undefined (reading 'httpFetch')");

test('cold connector initialization waits before its first transport request',async()=>{
 let clock=0,attempts=0,transportRequests=0;
 const result=await readConnectorWhenReady(async()=>{
  if(++attempts<3)throw uninitialized();
  transportRequests++;return {connector_id:'owned'};
 },{now:()=>clock,sleep:async ms=>{clock+=ms;}});
 expect(result).toEqual({connector_id:'owned'});
 expect(attempts).toBe(3);expect(transportRequests).toBe(1);expect(clock).toBe(500);
});

test('cold connector initialization is bounded and submits no mutation',async()=>{
 let clock=0,attempts=0;
 await expect(readConnectorWhenReady(async()=>{attempts++;throw uninitialized();},
  {now:()=>clock,sleep:async ms=>{clock+=ms;},timeoutMs:1000})).rejects.toThrow('no mutation submitted');
 expect(clock).toBe(1000);expect(attempts).toBe(5);
});

test('connector initialization never retries rate limits or unknown failures',async()=>{
 for(const error of [Object.assign(Error('rate limited'),{status:429}),Error('delivery unknown'),new TypeError('unrelated bug')]){
  let attempts=0;
  await expect(readConnectorWhenReady(async()=>{attempts++;throw error;})).rejects.toBe(error);
  expect(attempts).toBe(1);
 }
});
