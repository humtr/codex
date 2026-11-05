import {test,expect} from 'bun:test';
import {mkdtempSync,writeFileSync,readFileSync,readlinkSync,rmSync} from 'node:fs';
import {join} from 'node:path';
import {tmpdir} from 'node:os';
import {connectDesktop} from './desktop_cdp.mjs';

for(const mode of ['ambiguous','unique-primary-home'])test('owned app discovery skips auxiliary pages without UI actions: '+mode,async()=>{
 const root=mkdtempSync(join(tmpdir(),'cdp-main-')),token='x'.repeat(43),requests=[];
 const server=Bun.serve({hostname:'127.0.0.1',port:0,
  fetch(req,server){
   if(req.headers.get('authorization')!=='Bearer '+token)return new Response('Unauthorized',{status:401});
   if(server.upgrade(req))return;return new Response('WebSocket required',{status:400});
  },
  websocket:{message(socket,raw){
   const req=JSON.parse(String(raw));requests.push(req);let result;
   if(req.method==='Target.getTargets')result={targetInfos:['launcher','main'].map(targetId=>({targetId,type:'page',url:mode==='unique-primary-home'&&targetId==='launcher'?'app://-/index.html?initialRoute=%2Favatar-overlay':'app://-/index.html'}))};
   else if(req.method==='Target.attachToTarget')result={sessionId:'session-'+req.params.targetId};
   else if(req.method==='Runtime.evaluate')result={result:{value:mode==='ambiguous'&&req.sessionId==='session-main'}};
   else if(req.method==='Target.detachFromTarget')result={};
   else throw Error('unexpected_fixture_action');
   socket.send(JSON.stringify({id:req.id,result}));
  }}
 });
 const stat=readFileSync('/proc/self/stat','utf8'),owner={pid:process.pid,start:stat.slice(stat.lastIndexOf(')')+1).trim().split(/\s+/)[19],image:readlinkSync('/proc/self/exe')};
 writeFileSync(join(root,'desktop-host.json'),JSON.stringify({kind:'termux-electron-pipe',endpoint:'http://127.0.0.1:'+server.port+'/',token,host:owner,browser:owner}),{mode:0o600});
 let first,second;
 try{
  first=await connectDesktop(root,undefined,{app:true});
  expect(first.targetId).toBe('main');
  expect(requests.some(r=>r.method==='Target.detachFromTarget'&&r.params.sessionId==='session-launcher')).toBe(mode==='ambiguous');
  if(mode==='unique-primary-home')expect(requests.some(r=>r.method==='Runtime.evaluate')).toBe(false);
  const before=requests.length;
  second=await connectDesktop(root,first.targetId,{app:true});
  expect(second.targetId).toBe(first.targetId);
  expect(requests.slice(before).filter(r=>r.method==='Target.attachToTarget').map(r=>r.params.targetId)).toEqual(['main']);
  expect(requests.some(r=>r.method==='Target.createTarget'||r.method.startsWith('Input.'))).toBe(false);
 }finally{await first?.close();await second?.close();server.stop(true);rmSync(root,{recursive:true,force:true});}
});

for(const mode of ['roundtrip','discovery-failure'])test('Provider-owned maintenance retains generation and observer ownership: '+mode,async()=>{
 const root=mkdtempSync(join(tmpdir(),'cdp-provider-')),token='a'.repeat(48),requests=[];
 const raw=readFileSync('/proc/self/stat','utf8'),owner={pid:process.pid,start:raw.slice(raw.lastIndexOf(')')+1).trim().split(/\s+/)[19],image:readlinkSync('/proc/self/exe')};
 let generation='owned-app';
 const server=Bun.serve({hostname:'127.0.0.1',port:0,async fetch(req){
  expect(req.headers.get('authorization')).toBe('Bearer '+token);
  expect(new URL(req.url).pathname).toBe('/__desktop_cdp');
  if(req.method==='GET')return Response.json({generation});
  const command=await req.json();
  if(command.generation!==generation)return Response.json({error:{code:'desktop_generation_changed'}},{status:409});
  requests.push(command);
  let result;
  if(command.method==='Target.getTargets')result={targetInfos:(mode==='discovery-failure'?['aux','main']:['main']).map(targetId=>({targetId,type:'page',url:'app://-/index.html'}))};
  else if(command.method==='Target.attachToTarget')result={sessionId:command.params.targetId==='main'?'owned-observer':'aux-observer'};
  else if(command.method==='Runtime.evaluate'){
   if(mode==='discovery-failure'){
    if(command.session_id==='aux-observer')return Response.json({result:{result:{value:false}}});
    return Response.json({error:{code:'cdp_operation_rejected'}},{status:409});
   }
   expect(command.session_id).toBe('owned-observer');
   expect(command.params.expression).toBe('6 * 7');
   result={result:{value:42}};
  }else if(command.method==='Target.detachFromTarget')result={};
  else throw Error('unexpected_maintenance_command');
  return Response.json({result});
 }});
 const binary=join(root,'provider'),state=join(root,'state');
 const status={schema:'termux-codex-provider-status-v1',live:true,base_url:'http://127.0.0.1:'+server.port+'/v1',process:owner};
 writeFileSync(binary,`#!${process.execPath}\nconst action=process.argv[2];if(!['status','token'].includes(action)||process.argv[3]!=='--state'||process.argv[4]!==${JSON.stringify(state)})process.exit(1);console.log(action==='status'?${JSON.stringify(JSON.stringify(status))}:${JSON.stringify(token)});\n`,{mode:0o700});
 let client;
 try{
  if(mode==='discovery-failure'){
   await expect(connectDesktop({kind:'provider-owned',binary,state},undefined,{app:true})).rejects.toThrow('cdp_operation_rejected');
   expect(requests.filter(r=>r.method==='Target.detachFromTarget').map(r=>r.params.sessionId)).toEqual(['aux-observer','owned-observer']);
   return;
  }
  client=await connectDesktop({kind:'provider-owned',binary,state},undefined,{app:true});
  expect(await client.evaluate('6 * 7')).toBe(42);
  const delivered=requests.length;
  generation='replacement-app';
  await expect(client.evaluate('6 * 7')).rejects.toThrow('desktop_generation_changed');
  expect(requests.length).toBe(delivered);
  generation='owned-app';
  await client.close();client=undefined;
  expect(requests.at(-1)).toMatchObject({method:'Target.detachFromTarget',params:{sessionId:'owned-observer'}});
  expect(requests.some(r=>r.method==='Target.createTarget'||r.method.startsWith('Input.'))).toBe(false);
 }finally{await client?.close();server.stop(true);rmSync(root,{recursive:true,force:true});}
});
