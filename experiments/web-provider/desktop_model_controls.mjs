// Observe/select actual Chat controls; never infer entitlement from an API model name.
const delay=ms=>new Promise(ok=>setTimeout(ok,ms));
export const CHAT_FAMILIES={'gpt-6':'GPT-6','gpt-5.6-sol':'GPT-5.6 Sol'};
export const CHAT_EFFORTS={none:'Instant',medium:'Medium',high:'High',xhigh:'Extra High'};
export function chatModelRoute(raw){
 const effort=raw.reasoning?.effort??'none',mode=raw.reasoning?.mode??'standard';
 let slug=raw.model==='chatgpt-desktop/instant'?'gpt-6':raw.model?.replace(/^chatgpt-desktop\//,'');
 if(typeof raw.model!=='string'||!raw.model.startsWith('chatgpt-desktop/'))throw Error('unsupported_desktop_route');
 if(raw.model==='chatgpt-desktop/instant'&&effort!=='none')throw Error('unsupported_desktop_route');
 const pro=slug?.endsWith('-pro');if(pro)slug=slug.slice(0,-4);
 if(!Object.hasOwn(CHAT_FAMILIES,slug)||!Object.hasOwn(CHAT_EFFORTS,effort)||mode!=='standard'||(pro&&effort!=='none'))throw Error('unsupported_desktop_route');
 return {provider:'chatgpt-desktop',model:raw.model,family:CHAT_FAMILIES[slug],effort,mode:pro?'pro':'standard',label:pro?'Pro':CHAT_EFFORTS[effort]};
}
export function desktopModelCatalog(){
 return Object.entries(CHAT_FAMILIES).flatMap(([slug,family])=>[
  {slug:'chatgpt-desktop/'+slug,family,displayName:'ChatGPT · '+family,efforts:Object.keys(CHAT_EFFORTS),pro:false},
  {slug:'chatgpt-desktop/'+slug+'-pro',family,displayName:'ChatGPT · '+family+' Pro',efforts:['none'],pro:true,executionUnverified:true}]);
}
async function idle(c){
 const ok=await c.evaluate(`(()=>{const v=e=>e.getBoundingClientRect().width>0;return ![...document.querySelectorAll('[contenteditable="true"]')].some(e=>v(e)&&(e.textContent||'').trim())&&![...document.querySelectorAll('button')].some(e=>v(e)&&/^stop(?:\\b|$)/i.test(e.getAttribute('aria-label')||''));})()`);
 if(!ok)throw Error('unowned_draft_or_generation_must_be_preserved');
}
async function open(c){
 // Creating a temporary Chat replaces the React view asynchronously.
 let available=false;
 for(let i=0;i<100;i++){
  available=await c.evaluate(`[...document.querySelectorAll('button[aria-label="Select ChatGPT model"]')].filter(e=>e.getBoundingClientRect().width>0).length===1`);
  if(available)break;await delay(100);
 }
 if(!available)throw Error('desktop_model_selector_unavailable');
 await idle(c);
 await c.evaluate(`(()=>{const es=[...document.querySelectorAll('button[aria-label="Select ChatGPT model"]')].filter(e=>e.getBoundingClientRect().width>0);if(es.length!==1||es[0].getAttribute('aria-expanded')==='true')throw Error('owned_closed_model_selector_required');es[0].click();})()`);await delay(100);
}
async function close(c){for(const type of ['keyDown','keyUp'])await c.request('Input.dispatchKeyEvent',{type,key:'Escape',code:'Escape',windowsVirtualKeyCode:27});}
async function state(c){
 const result=await c.evaluate(`(()=>{const menu=[...document.querySelectorAll('[role="menu"]')].find(e=>e.getBoundingClientRect().width>0),slider=menu?.querySelector('[role="slider"]');if(!slider)return null;const family=[...menu.querySelectorAll('[role="menuitemradio"]')].find(e=>e.getAttribute('aria-checked')==='true')?.textContent?.trim();let fiber=slider[Object.keys(slider).find(k=>k.startsWith('__reactFiber'))],options;for(let i=0;fiber&&i<40;i++,fiber=fiber.return){const value=fiber.memoizedProps?.options;if(Array.isArray(value)&&value.length&&value.every(e=>typeof e.id==='string'&&typeof e.label==='string')){options=value.map(e=>({id:e.id,label:e.label,locked:!!(e.locked||e.isLocked||e.disabled)}));break;}}return {family,index:Number(slider.getAttribute('aria-valuenow')),min:Number(slider.getAttribute('aria-valuemin')),max:Number(slider.getAttribute('aria-valuemax')),disabled:slider.hasAttribute('data-disabled')||slider.getAttribute('aria-disabled')==='true',options,visibleLabel:[...menu.querySelectorAll('[role="menuitem"]')].find(e=>e.getAttribute('aria-label')==='Select model')?.textContent};})()`);
 if(!result||!Object.values(CHAT_FAMILIES).includes(result.family)||result.disabled||result.min!==0||result.max!==4||result.options?.length!==5)throw Error('desktop_model_controls_unavailable');
 const labels=['Instant','Medium','High','Extra High','Pro'];
 const prefix=result.family==='GPT-6'?'gpt-6':'gpt-5-6';
 const ids=[prefix+(result.family==='GPT-6'?':':'-instant:'),prefix+'-thinking:standard',prefix+'-thinking:extended',prefix+'-thinking:max',prefix+'-pro:'];
 if(result.options.some((o,i)=>o.label!==labels[i]||o.id!==ids[i]))throw Error('desktop_model_controls_changed');
 return result;
}
export async function observeDesktopRoute(c){let opened=false;try{await open(c);opened=true;return await state(c);}finally{if(opened)await close(c);}}
export async function selectDesktopRoute(c,route){
 let opened=false;
 try{
  await open(c);opened=true;
  await c.evaluate(`(()=>{const es=[...document.querySelectorAll('[role="menuitemradio"]')].filter(e=>e.textContent?.trim()===${JSON.stringify(route.family)});if(es.length!==1||es[0].getAttribute('aria-disabled')==='true')throw Error('desktop_family_unavailable');if(es[0].getAttribute('aria-checked')!=='true')es[0].click();})()`);
  // Selecting another family closes its menu. Reopen only that owned selector.
  const expanded=await c.evaluate(`document.querySelector('button[aria-label="Select ChatGPT model"]')?.getAttribute('aria-expanded')==='true'`);
  if(!expanded){opened=false;await open(c);opened=true;}await delay(100);
  let selected=await state(c),index=selected.options.findIndex(o=>o.label===route.label);
  if(index<0||selected.options[index].locked)throw Error('desktop_mode_unavailable');
  // Use the public keyboard control, not React state setters or private inference APIs.
  await c.evaluate(`document.querySelector('[role="menu"] [role="slider"]').focus()`);
  const key=index>selected.index?'ArrowRight':'ArrowLeft',code=key,windowsVirtualKeyCode=key==='ArrowRight'?39:37;
  for(let i=0;i<Math.abs(index-selected.index);i++){for(const type of ['keyDown','keyUp'])await c.request('Input.dispatchKeyEvent',{type,key,code,windowsVirtualKeyCode});await delay(100);}
  selected=await state(c);
  if(selected.family!==route.family||selected.index!==index||!selected.visibleLabel?.endsWith(route.label))throw Error('desktop_model_selection_not_confirmed');
  return {family:selected.family,label:route.label,controlId:selected.options[index].id};
 }finally{if(opened)await close(c);}
}
export async function assertDesktopRoute(c,route){
 const selected=await observeDesktopRoute(c),option=selected.options[selected.index];
 if(selected.family!==route.family||option.label!==route.label||option.locked||!selected.visibleLabel?.endsWith(route.label))throw Error('desktop_model_changed');
 return {family:selected.family,label:option.label,controlId:option.id};
}
