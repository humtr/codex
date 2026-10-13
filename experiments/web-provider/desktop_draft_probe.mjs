import {readFileSync,writeFileSync} from 'node:fs';
import {join} from 'node:path';
import {randomBytes} from 'node:crypto';
import {connectDesktop} from './desktop_cdp.mjs';
import {DesktopChatTransport} from './desktop_response_transport.mjs';
process.umask(0o077);
const data=join(process.env.HOME,'.local/share/codex-web-experiment'),root=JSON.parse(readFileSync(join(data,'linux-desktop/session.json'),'utf8')).root;
let c=await connectDesktop(root);const text='OWNED_DRAFT_'+randomBytes(10).toString('hex');let inserted=false,cleaned=false,rejected=false,preserved=false;
try{
 const empty=await c.evaluate(`(()=>{const es=[...document.querySelectorAll('[contenteditable="true"]')].filter(e=>e.getBoundingClientRect().width>0);if(es.length!==1||(es[0].textContent||'').trim()||[...document.querySelectorAll('button')].some(e=>/^stop/i.test(e.getAttribute('aria-label')||'')&&e.getBoundingClientRect().width>0))return false;es[0].focus();return true;})()`);
 if(!empty)throw Error('unowned_editor_busy');
 await c.request('Input.insertText',{text});inserted=true;await c.evaluate(`new Promise(ok=>requestAnimationFrame(()=>requestAnimationFrame(()=>ok(true))))`);
 c.close();await new Promise(ok=>setTimeout(ok,150));
 const t=new DesktopChatTransport(root);
 const abort=new AbortController(),timer=setTimeout(()=>abort.abort(),20000);
 try{await t.exchange({instructions:[],messages:[{role:'user',content:'Owned draft protection test; no work requested.'}],tools:[]},abort.signal);}catch(e){rejected=e.message==='unowned_draft_must_be_preserved';}finally{clearTimeout(timer);}
 await new Promise(ok=>setTimeout(ok,150));c=await connectDesktop(root);
 preserved=await c.evaluate(`(()=>{const e=[...document.querySelectorAll('[contenteditable="true"]')].find(e=>e.getBoundingClientRect().width>0);return !!e&&(e.textContent||'').split(String.fromCharCode(10)).join('').split(String.fromCharCode(13)).join('')===${JSON.stringify(text)};})()`);
 if(!rejected||!preserved)throw Error('draft_protection_failed');
}finally{
 if(inserted){
  const owned=await c.evaluate(`(()=>{const e=[...document.querySelectorAll('[contenteditable="true"]')].find(e=>e.getBoundingClientRect().width>0);if(!e||(e.textContent||'').split(String.fromCharCode(10)).join('').split(String.fromCharCode(13)).join('')!==${JSON.stringify(text)})return false;e.focus();return true;})()`);
  if(owned){for(const type of ['keyDown','keyUp'])await c.request('Input.dispatchKeyEvent',{type,modifiers:2,key:'a',code:'KeyA',windowsVirtualKeyCode:65});for(const type of ['keyDown','keyUp'])await c.request('Input.dispatchKeyEvent',{type,modifiers:0,key:'Backspace',code:'Backspace',windowsVirtualKeyCode:8});cleaned=await c.evaluate(`[...document.querySelectorAll('[contenteditable="true"]')].filter(e=>e.getBoundingClientRect().width>0).every(e=>!(e.textContent||'').trim())`);}
 }
 c.close();
}
const report={draftPreserved:preserved,replayRefusedBeforeSend:rejected,ownedFixtureCleaned:cleaned};writeFileSync(join(data,'native-provider/route-draft-proof.json'),JSON.stringify(report),{mode:0o600});console.log(JSON.stringify(report));if(!cleaned)process.exitCode=1;
