import {readFileSync,lstatSync,readlinkSync} from 'node:fs';
import {join} from 'node:path';
export function resolveDesktopRoot(path){
 const stat=lstatSync(path);
 if(stat.isDirectory())return path; // Existing disposable qualification interface.
 if(!stat.isFile()||stat.isSymbolicLink()||stat.uid!==process.getuid()||(stat.mode&0o777)!==0o600||stat.size>8192)throw Error('private_session_required');
 const d=JSON.parse(readFileSync(path,'utf8')),raw=readFileSync(`/proc/${d.pid}/stat`,'utf8'),fields=raw.slice(raw.lastIndexOf(')')+1).trim().split(/\s+/);
 if(!Number.isSafeInteger(d.pid)||d.pid<1||fields[0]==='Z'||fields[19]!==String(d.start)||readlinkSync(`/proc/${d.pid}/exe`)!==d.image||typeof d.root!=='string')throw Error('stale_desktop_controller');
 return d.root;
}
export function desktopConnection(root){
 root=resolveDesktopRoot(root);
 const file=join(root,'desktop-host.json'),stat=lstatSync(file);
 if(!stat.isFile()||stat.isSymbolicLink()||stat.uid!==process.getuid()||(stat.mode&0o777)!==0o600||stat.size>8192)throw Error('private_descriptor_required');
 const d=JSON.parse(readFileSync(file,'utf8')),u=new URL(d.endpoint);
 if(d.kind!=='termux-electron-pipe'||u.protocol!=='http:'||u.hostname!=='127.0.0.1'||!u.port||u.pathname!=='/'||u.username||u.password||u.search||u.hash||!/^[-\w]{43}$/.test(d.token))throw Error('owned_desktop_required');
 for(const identity of [d.host,d.browser]){
  if(!Number.isSafeInteger(identity?.pid)||identity.pid<1)throw Error('invalid_owner');
  const raw=readFileSync(`/proc/${identity.pid}/stat`,'utf8'),fields=raw.slice(raw.lastIndexOf(')')+1).trim().split(/\s+/);
  if(fields[0]==='Z'||fields[19]!==identity.start||readlinkSync(`/proc/${identity.pid}/exe`)!==identity.image)throw Error('stale_owner');
 }
 return {descriptor:d,generation:JSON.stringify([root,d.host.pid,d.host.start,d.browser.pid,d.browser.start,d.endpoint])};
}
export async function connectDesktop(root,targetId,options={}){
 root=resolveDesktopRoot(root);
 const {descriptor:d,generation}=desktopConnection(root);
 const socket=new WebSocket(d.endpoint.replace('http:','ws:')+'/cdp',{headers:{Authorization:'Bearer '+d.token}});
 let sequence=0;const pending=new Map();
 const rejectAll=()=>{for(const p of pending.values()){clearTimeout(p.timer);p.reject(Error('owned_relay_closed'));}pending.clear();};
 socket.onmessage=e=>{const m=JSON.parse(e.data),p=pending.get(m.id);if(p){clearTimeout(p.timer);pending.delete(m.id);m.error?p.reject(Error('owned_cdp_rejected_'+p.method)):p.resolve(m.result);}};
 socket.onclose=rejectAll;
 await new Promise((ok,no)=>{const timer=setTimeout(()=>{socket.close();no(Error('owned_relay_connect_timeout'));},10000);socket.onopen=()=>{clearTimeout(timer);ok();};socket.onerror=()=>{clearTimeout(timer);no(Error('owned_relay_connect_failed'));};});
 function request(method,params={},sessionId){const id=++sequence;return new Promise((resolve,reject)=>{const timer=setTimeout(()=>{pending.delete(id);reject(Error('owned_cdp_timeout_'+method));},15000);pending.set(id,{resolve,reject,timer,method});socket.send(JSON.stringify({id,method,params,...(sessionId?{sessionId}:{})}));});}
 let session,selected;const openedTemporary=new Set();
 try{
  for(let attempt=0;attempt<(options.app?60:20)&&!session;attempt++){
   const {targetInfos}=await request('Target.getTargets');
   if(targetId&&!targetInfos.some(t=>t.type==='page'&&t.targetId===targetId&&t.url.startsWith('app://-/')))targetId=undefined;
   const roots=targetInfos.filter(t=>t.type==='page'&&t.url==='app://-/index.html');
   const primary=options.app&&roots.length===1?roots[0].targetId:undefined;
   const candidates=targetInfos.filter(t=>t.type==='page'&&t.url.startsWith('app://-/')&&(!targetId||t.targetId===targetId));
   if(primary)candidates.sort((a,b)=>Number(b.targetId===primary)-Number(a.targetId===primary));
   for(const target of candidates){
    const {sessionId}=await request('Target.attachToTarget',{targetId:target.targetId,flatten:true});
    if(options.app){
     // This build's unique primary entry survives Home/Plugins navigation.
     // Scope discovery still validates the actual facade/capabilities afterwards.
     if(target.targetId===primary){session=sessionId;selected=target.targetId;break;}
     // The app also opens a small launcher, with its own empty composer. Its
     // Scope is not the main conversation/model context. Discovery never clicks.
     const {result}=await request('Runtime.evaluate',{expression:`(()=>{if(document.querySelector('button[aria-label="Select ChatGPT model"]'))return true;const labels=new Set([...document.querySelectorAll('button,span')].map(e=>e.textContent?.trim()));return labels.has('Chat')&&labels.has('Work');})()`,returnByValue:true},sessionId);
     if(result.value===true){session=sessionId;selected=target.targetId;break;}
     await request('Target.detachFromTarget',{sessionId});continue;
    }
    const {result}=await request('Runtime.evaluate',{expression:`[...document.querySelectorAll('button')].some(e=>e.textContent?.trim()==='Chat'&&e.getAttribute('aria-pressed')==='true')||[...document.querySelectorAll('button[aria-label="Select ChatGPT model"]')].some(e=>e.getBoundingClientRect().width>0)`,returnByValue:true},sessionId);
    if(!result.value&&!openedTemporary.has(target.targetId)){
     // Closing a settled temporary lane can leave the app on Home. Recover
     // through its public new-temporary-chat shortcut without touching a draft.
     const {result:home}=await request('Runtime.evaluate',{expression:`(()=>{const visible=e=>e.getBoundingClientRect().width>0;return ![...document.querySelectorAll('[contenteditable="true"]')].some(visible)&&![...document.querySelectorAll('button')].some(e=>visible(e)&&/^stop(?:\\b|$)/i.test(e.getAttribute('aria-label')||''))&&[...document.querySelectorAll('button')].filter(e=>visible(e)&&(e.textContent?.trim()==='Home'||e.getAttribute('aria-label')==='Home')).length===1;})()`,returnByValue:true},sessionId);
     if(home.value){openedTemporary.add(target.targetId);for(const type of ['keyDown','keyUp'])await request('Input.dispatchKeyEvent',{type,modifiers:10,key:'N',code:'KeyN',windowsVirtualKeyCode:78},sessionId);}
    }
    if(!result.value){
     await request('Runtime.evaluate',{expression:`(()=>{if([...document.querySelectorAll('[contenteditable="true"]')].some(e=>e.getBoundingClientRect().width>0&&(e.textContent||'').trim())||[...document.querySelectorAll('button')].some(e=>/^stop(?:\\b|$)/i.test(e.getAttribute('aria-label')||'')&&e.getBoundingClientRect().width>0))return false;const es=[...document.querySelectorAll('button')].filter(e=>e.textContent?.trim()==='Chat'&&e.getBoundingClientRect().width>0);if(es.length===1){es[0].click();return true;}return false;})()`,returnByValue:true},sessionId);
    }
    if(result.value){session=sessionId;selected=target.targetId;break;}await request('Target.detachFromTarget',{sessionId});
   }
   if(!session)await new Promise(ok=>setTimeout(ok,500));
  }
  if(!session)throw Error('actual_chat_mode_not_selected');
 }catch(e){socket.close();throw e;}
 return {targetId:selected,generation,async evaluate(expression){const {result,exceptionDetails}=await request('Runtime.evaluate',{expression,returnByValue:true,awaitPromise:true},session);if(exceptionDetails){
    const message=exceptionDetails.exception?.description?.split('\n')[0]?.replace(/^Error: /,'');
    const allowed=['owned_closed_model_selector_required','desktop_family_unavailable','owned_empty_composer_required'];
    throw Error(allowed.includes(message)?message:'owned_dom_evaluation_failed');
   }return result.value;},
  request:(method,params)=>request(method,params,session),
  close:()=>socket.close()};
}
