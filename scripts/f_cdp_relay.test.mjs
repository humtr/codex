import {test,expect} from 'bun:test';
import {relay} from './f_cdp_relay.mjs';

test('private F relay isolates simultaneous client IDs and sessions without browser lifecycle commands',async()=>{
 let next=0;const incoming=[];
 const upstream=Bun.serve({hostname:'127.0.0.1',port:0,fetch(req,s){if(s.upgrade(req))return;return new Response('',{status:400});},websocket:{message(ws,raw){const m=JSON.parse(String(raw));incoming.push(m);const result=m.method==='Target.attachToTarget'?{sessionId:'s'+(++next)}:{echo:m.params?.value};setTimeout(()=>ws.send(JSON.stringify({id:m.id,result})),m.params?.value==='first'?10:0);}}});
 const r=await relay({endpoint:'http://127.0.0.1:'+upstream.port,token:'fixture'});
 const sockets=[];
 async function client(){const ws=new WebSocket(r.endpoint,{headers:{Authorization:'Bearer '+r.token}});sockets.push(ws);await new Promise((ok,no)=>{ws.onopen=ok;ws.onerror=no;});return ws;}
 function call(ws,id,method,params={},sessionId){return new Promise(ok=>{ws.onmessage=e=>ok(JSON.parse(String(e.data)));ws.send(JSON.stringify({id,method,params,...(sessionId?{sessionId}:{})}));});}
 try{
  expect((await fetch(r.endpoint.replace('ws:','http:'))).status).toBe(401);
  const a=await client(),b=await client();
  const [one,two]=await Promise.all([call(a,1,'Target.attachToTarget',{targetId:'main'}),call(b,1,'Target.attachToTarget',{targetId:'main'})]);
  expect(one.result.sessionId).not.toBe(two.result.sessionId);
  const [x,y]=await Promise.all([call(a,2,'Runtime.evaluate',{value:'first'},one.result.sessionId),call(b,2,'Runtime.evaluate',{value:'second'},two.result.sessionId)]);
  expect(x.result.echo).toBe('first');expect(y.result.echo).toBe('second');
  expect((await call(b,3,'Runtime.evaluate',{},one.result.sessionId)).error.message).toBe('relay_session_not_owned');
  expect((await call(a,4,'Browser.close')).error.message).toBe('relay_method_refused');
  expect(incoming.some(m=>m.method==='Browser.close')).toBe(false);
 }finally{for(const ws of sockets)ws.close();await r.close();upstream.stop(true);}
});
