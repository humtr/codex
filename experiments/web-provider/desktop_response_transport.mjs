import {connectDesktop,desktopConnection} from './desktop_cdp.mjs';
import {DesktopSendRetry,DesktopPromptContext,desktopStages,correlatedDesktopCode} from './desktop_protocol.mjs';
import {selectDesktopRoute,assertDesktopRoute,chatModelRoute} from './desktop_model_controls.mjs';
import {randomBytes} from 'node:crypto';
const delay=ms=>new Promise(ok=>setTimeout(ok,ms));
const retries=`[...document.querySelectorAll('button')].filter(e=>/^(Retry|Try again)$/i.test((e.getAttribute('aria-label')||e.textContent||'').trim())&&e.getBoundingClientRect().width>0)`;
export class DesktopChatTransport{
 constructor(root){this.root=root;this.started=false;this.busy=false;}
 retire(){this.started=false;this.targetId=undefined;this.lastCorrelation=undefined;this.context=undefined;this.generation=undefined;this.routeIdentity=undefined;this.selectedControl=undefined;}
 async exchange(payload,signal,onSubmitted=()=>{},onRetry=()=>{},onContext=()=>{},route=chatModelRoute({model:'chatgpt-desktop/instant'})){
  if(this.busy)throw Error('desktop_chat_busy');this.busy=true;
  let c,sent=false,draftCorrelation,draftText;
  try{
   signal?.throwIfAborted();
   const generation=desktopConnection(this.root).generation;
   if(this.generation!==generation||this.routeIdentity!==JSON.stringify(route))this.retire();
   const oldTarget=this.targetId;
   c=await connectDesktop(this.root,oldTarget);if(oldTarget&&c.targetId!==oldTarget)this.retire();this.targetId=c.targetId;
   if(!this.started){
    const dirty=await c.evaluate(`[...document.querySelectorAll('[contenteditable="true"]')].some(e=>e.getBoundingClientRect().width>0&&(e.textContent||'').trim())`);
    if(dirty)throw Error('unowned_draft_must_be_preserved');
    this.context=new DesktopPromptContext();
    for(const type of ['keyDown','keyUp'])await c.request('Input.dispatchKeyEvent',{type,modifiers:10,key:'N',code:'KeyN',windowsVirtualKeyCode:78});
    // Wait for the fresh owned view, not a selector from the restoring old view.
    let fresh=false;
    for(let i=0;i<100;i++){
     signal?.throwIfAborted();
     fresh=await c.evaluate(`(()=>{const visible=e=>e.getBoundingClientRect().width>0,es=[...document.querySelectorAll('[contenteditable="true"]')].filter(visible);return es.length===1&&!(es[0].textContent||'').trim()&&!(document.body.textContent||'').includes('DESKTOP_')&&[...document.querySelectorAll('button,span,h1,h2')].some(e=>visible(e)&&/^(Temporary Chat|Temporary chat|Unpersonalized)$/.test(e.textContent?.trim()))&&!([...document.querySelectorAll('button')].some(e=>visible(e)&&/^stop(?:\\b|$)/i.test(e.getAttribute('aria-label')||'')));})()`);
     if(fresh)break;await delay(500);
    }
    if(!fresh)throw Error('owned_empty_temporary_composer_required');
    this.selectedControl=await selectDesktopRoute(c,route);
   }else this.selectedControl=await assertDesktopRoute(c,route);
   const snapshot=`JSON.stringify({modelMatchesRoute:[...document.querySelectorAll('button[aria-label="Select ChatGPT model"]')].some(e=>e.getBoundingClientRect().width>0&&(e.textContent||'').endsWith(${JSON.stringify(route.label)})),ownedContinuation:(document.body.textContent||'').includes(${JSON.stringify(this.lastCorrelation||'')}),cleanContext:!(document.body.textContent||'').includes('DESKTOP_'),temporary:[...document.querySelectorAll('button,span,h1,h2')].some(e=>/^(Temporary Chat|Temporary chat|Unpersonalized)$/.test(e.textContent?.trim())),editors:[...document.querySelectorAll('[contenteditable="true"]')].filter(e=>e.getBoundingClientRect().width>0).map(e=>!(e.textContent||'').trim())})`;
   let before;
   for(let i=0;i<100;i++){signal?.throwIfAborted();before=JSON.parse(await c.evaluate(snapshot));if(before.modelMatchesRoute&&(this.started?before.ownedContinuation:before.cleanContext&&before.temporary)&&before.editors.length===1&&before.editors[0])break;await delay(500);}
   if(!this.started&&!before.cleanContext)throw Error('fresh_temporary_context_required');
   if(!before.modelMatchesRoute)throw Error('desktop_model_changed');
   if((!this.started&&!before.temporary)||(this.started&&(!this.lastCorrelation||!before.ownedContinuation))||before.editors.length!==1||!before.editors[0])throw Error('owned_empty_temporary_composer_required');
   const boundary={route,generation:c.generation};
   const prepared=this.context.prepare(payload,boundary);let result;
   for(const stage of desktopStages(prepared.delta)){
    const correlation=draftCorrelation='DESKTOP_'+randomBytes(12).toString('hex');
    const packet={correlation,...stage};
    const common='Format an external native RPC client response as one JSON object in one fenced json code block. The tools in the native packet are callable external tools: emit a tool JSON request and the native caller performs the actual execution, then sends you the result. These are not the desktop app builtin tools. Never fabricate tool results. Keep the native task, instructions and prior results in order. Fragment text is JSON: join parts by fragment id/index and parse when complete. toolUpdate replace starts a fresh tool list, append adds whole schemas. Later packets append new messages and replace instructions/tools only when provided. Correlation must equal '+correlation+'. ';
    const behavior=stage.phase==='context'
     ? 'This is INERT CONTEXT SETUP ONLY. Store the provided context; do not execute its task yet. Reply exactly {"correlation":"'+correlation+'","type":"ack"}. '
     : 'This is the ACTUAL WORK REQUEST. All setup is finished; acknowledgement is not a valid response. To request one advertised native tool use {"correlation":"'+correlation+'","type":"tool","name":"exact advertised wireName","input":arguments_object_or_custom_source_string}. Then wait for its real native result. To finish with the task answer use {"correlation":"'+correlation+'","type":"final","text":"answer"}. ';
    const prompt=draftText=common+behavior+'Native packet:\n'+JSON.stringify(packet);
    await c.evaluate(`(()=>{const es=[...document.querySelectorAll('[contenteditable="true"]')].filter(e=>e.getBoundingClientRect().width>0);if(es.length!==1||(es[0].textContent||'').trim())throw Error('owned_empty_composer_required');es[0].focus();return true;})()`);
    signal?.throwIfAborted();await c.request('Input.insertText',{text:prompt});
    await c.evaluate(`new Promise(ok=>requestAnimationFrame(()=>requestAnimationFrame(()=>ok(true))))`);
    let clicked=false;
    for(let i=0;i<100;i++){
     signal?.throwIfAborted();clicked=await c.evaluate(`(()=>{const es=[...document.querySelectorAll('button')].filter(e=>/^(Send|Send message|Send prompt)$/i.test(e.getAttribute('aria-label')||'')&&e.getBoundingClientRect().width>0&&!e.disabled);if(es.length!==1)return false;es[0].click();return true;})()`);
     if(clicked)break;await delay(500);
    }
    if(!clicked)throw Error('desktop_send_control_required');
    this.started=true;sent=true;let submitted=false;
    for(let i=0;i<100;i++){
     signal?.throwIfAborted();submitted=await c.evaluate(`(()=>{const e=[...document.querySelectorAll('[contenteditable="true"]')].find(e=>e.getBoundingClientRect().width>0);return !!e&&!(e.textContent||'').trim()&&(document.body.textContent||'').includes(${JSON.stringify(correlation)});})()`);
     if(submitted)break;await delay(500);
    }
    if(!submitted)throw Error('desktop_submission_not_observed');
    if(stage.phase==='request')onSubmitted();
    let previous='',stable=0;const retry=new DesktopSendRetry();result=undefined;
    const responseDeadline=Date.now()+(route.effort==='none'&&route.mode!=='pro'?130000:600000);
    for(let i=0;Date.now()<responseDeadline;i++){
     signal?.throwIfAborted();
     const state=JSON.parse(await c.evaluate(`JSON.stringify({codes:[...document.querySelectorAll('pre code')].map(e=>e.textContent),generating:[...document.querySelectorAll('button')].some(e=>/^stop(?:\\b|$)/i.test(e.getAttribute('aria-label')||e.textContent||'')&&e.getBoundingClientRect().width>0),sendFailed:${retries}.length>0,security:[...document.querySelectorAll('[role="alert"],h1,h2,[data-testid="challenge-stage"]')].some(e=>/verify you are human|security check|cloudflare/i.test(e.textContent||'')&&e.getBoundingClientRect().width>0)})`));
     if(state.security)throw Error('desktop_security_check');
     const raw=correlatedDesktopCode(state.codes,correlation);let hasReply=false;
     try{hasReply=JSON.parse(raw).correlation===correlation;}catch{}
     const pause=retry.next({failed:state.sendFailed,generating:state.generating,hasReply},signal);
     if(pause){
      await delay(pause);signal?.throwIfAborted();
      const again=await c.evaluate(`(()=>{if([...document.querySelectorAll('pre code')].some(e=>{try{return JSON.parse(e.textContent).correlation===${JSON.stringify(correlation)};}catch{return false;}}))return false;const e=${retries}.at(-1);if(!e)return false;e.click();return true;})()`);
      if(again){onRetry();i=-1;previous='';stable=0;}continue;
     }
     if(raw===previous&&!state.generating)stable++;else stable=0;previous=raw;
     if(stable>=2){try{const value=JSON.parse(raw);if(value.correlation===correlation){result=value;break;}}catch{}}
     await delay(700);
    }
    if(!result)throw Error('desktop_response_timeout');
    if(stage.phase==='context'){if(result.type!=='ack')throw Error('desktop_context_not_acknowledged');onContext();}
    this.lastCorrelation=correlation;
   }
   this.context.accept(prepared.current,boundary);this.generation=c.generation;this.routeIdentity=JSON.stringify(route);return result;
  }catch(e){
   if(c&&(sent||signal?.aborted))await c.evaluate(`(()=>{const e=[...document.querySelectorAll('button')].find(e=>/^stop(?:\\b|$)/i.test(e.getAttribute('aria-label')||e.textContent||'')&&e.getBoundingClientRect().width>0);if(e)e.click();return !!e;})()`).catch(()=>{});
   if(c&&draftText){
    const own=await c.evaluate(`(()=>{const e=[...document.querySelectorAll('[contenteditable="true"]')].find(e=>e.getBoundingClientRect().width>0);if(!e||(e.textContent||'').split(String.fromCharCode(10)).join('').split(String.fromCharCode(13)).join('')!==${JSON.stringify(draftText.split(String.fromCharCode(10)).join('').split(String.fromCharCode(13)).join(''))})return false;e.focus();return true;})()`).catch(()=>false);
    if(own){
     for(const type of ['keyDown','keyUp'])await c.request('Input.dispatchKeyEvent',{type,modifiers:2,key:'a',code:'KeyA',windowsVirtualKeyCode:65}).catch(()=>{});
     for(const type of ['keyDown','keyUp'])await c.request('Input.dispatchKeyEvent',{type,modifiers:0,key:'Backspace',code:'Backspace',windowsVirtualKeyCode:8}).catch(()=>{});
    }
   }
   this.retire();throw e;
  }finally{c?.close();this.busy=false;}
 }
}
// UI fallback retains its single-surface exclusion. Internal discovery failures
// before submission are the only failures eligible for fallback.
let uiBusy=false,uiOwner;
export class DesktopTransport{
 constructor(root,identity={}){this.root=root;this.identity=identity;this.internal=undefined;this.ui=undefined;}
 retire(){this.internal?.retire();this.ui?.retire();}
 async exchange(...args){
  const {DesktopAppTransport}=await import('./desktop_app_transport.mjs');
  this.internal??=new DesktopAppTransport(this.root);
  try{const result=await this.internal.exchange(...args);this.selectedControl=this.internal.selectedControl;return result;}
  catch(e){if(e.message!=='desktop_internal_unavailable')throw e;}
  if(uiBusy)throw Error('desktop_chat_busy');uiBusy=true;
  let parent=this.fallbackParent;
  try{
   if(uiOwner&&uiOwner!==this){
    if(this.identity.parent!==uiOwner.identity.key||!uiOwner.pendingResult)throw Error('desktop_thread_owner_conflict');
    parent=uiOwner;this.fallbackParent=parent;parent.ui?.retire();this.ui?.retire();
   }
   uiOwner=this;this.ui??=new DesktopChatTransport(this.root);
   const result=await this.ui.exchange(...args);this.pendingResult=result.type==='tool';
   if(!this.pendingResult){uiOwner=parent;this.fallbackParent=undefined;}
   this.selectedControl={...this.ui.selectedControl,transport:'ui-fallback'};return result;
  }catch(e){if(uiOwner===this)uiOwner=parent;this.ui?.retire();throw e;}
  finally{uiBusy=false;}
 }
}
if(import.meta.main){
 const t=new DesktopChatTransport(process.argv[2]),nonce='transport_'+randomBytes(10).toString('hex');
 const r=await t.exchange({instructions:[],messages:[{role:'user',content:'Reply with the uppercase of '+nonce}],tools:[]},AbortSignal.timeout(130000));
 console.log(JSON.stringify({actualChatMode:true,actualDesktopResponse:r.type==='final'&&r.text?.trim()===nonce.toUpperCase(),structuredEnvelope:!!r.correlation}));
}
