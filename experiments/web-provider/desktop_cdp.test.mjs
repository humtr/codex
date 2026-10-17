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
 }finally{first?.close();second?.close();server.stop(true);rmSync(root,{recursive:true,force:true});}
});
