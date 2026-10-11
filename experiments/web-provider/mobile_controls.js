import UI from './app/ui.js';

const input=document.getElementById('noVNC_keyboardinput');
const keyboardButton=document.getElementById('owned_keyboard');
const automaticButton=document.getElementById('owned_keyboard_auto');
const fullscreenButton=document.getElementById('owned_fullscreen');
const controls=document.getElementById('owned_mobile_controls');
const container=document.getElementById('noVNC_container');
let keyboardAutoEnabled=true,touchStart=null,touchProofCount=0,errorCount=0,imeGeometryObserved=false;
try{keyboardAutoEnabled=localStorage.getItem('owned-keyboard-auto')!=='0';}catch{}
automaticButton.setAttribute('aria-checked',String(keyboardAutoEnabled));

function imeGeometryVisible(){
 const viewport=window.visualViewport;
 return Boolean(viewport&&Math.abs(viewport.scale-1)<.05&&window.innerHeight-viewport.height-Math.max(0,viewport.offsetTop)>120);
}
function keyboardVisible(){
 return document.activeElement===input&&(!imeGeometryObserved||imeGeometryVisible());
}
function updateKeyboardButton(){
 const visible=imeGeometryVisible();if(visible)imeGeometryObserved=true;
 const open=keyboardVisible();
 keyboardButton.setAttribute('aria-pressed',String(open));
 keyboardButton.setAttribute('aria-label',open?'키보드 닫기':'키보드 열기');
}
function focusKeyboard(){
 // noVNC otherwise closes its menu after 2s and steals the initial IME focus.
 UI.keepControlbar();
 if(document.getElementById('noVNC_control_bar').classList.contains('noVNC_open'))UI.closeControlbar();
 // Android Back can hide IME without clearing DOM focus. Refresh only then;
 // an already visible keyboard retains focus without a blur/reopen cycle.
 if(document.activeElement===input&&!imeGeometryVisible())input.blur();
 input.focus({preventScroll:true});
 try{input.setSelectionRange(input.value.length,input.value.length);}catch{}
 updateKeyboardButton();
}
function manualKeyboard(){
 if(keyboardVisible())UI.hideVirtualKeyboard();else focusKeyboard();
 updateKeyboardButton();setTimeout(displayProof,500);
}
keyboardButton.onclick=manualKeyboard;
const originalKeyboardButton=document.getElementById('noVNC_keyboard_button');
originalKeyboardButton.removeEventListener('click',UI.toggleVirtualKeyboard);
originalKeyboardButton.addEventListener('click',manualKeyboard);
// This is a preference, independent of manual keyboard visibility.
automaticButton.addEventListener('pointerdown',event=>event.preventDefault());
automaticButton.onclick=()=>{
 keyboardAutoEnabled=!keyboardAutoEnabled;
 automaticButton.setAttribute('aria-checked',String(keyboardAutoEnabled));
 try{localStorage.setItem('owned-keyboard-auto',keyboardAutoEnabled?'1':'0');}catch{}
};
input.addEventListener('focus',updateKeyboardButton);input.addEventListener('blur',updateKeyboardButton);
container.addEventListener('pointerdown',event=>{
 if(keyboardAutoEnabled&&event.pointerType==='touch'&&event.isPrimary&&event.target.tagName==='CANVAS')touchStart={id:event.pointerId,x:event.clientX,y:event.clientY};
 else touchStart=null;
},true);
container.addEventListener('pointercancel',()=>{touchStart=null;},true);
container.addEventListener('pointerup',event=>{
 const start=touchStart;touchStart=null;
 if(!keyboardAutoEnabled||!start||start.id!==event.pointerId||Math.hypot(event.clientX-start.x,event.clientY-start.y)>12||event.target.tagName!=='CANVAS'||!document.documentElement.classList.contains('noVNC_connected'))return;
 // Preserve remote click delivery; scrolling and multi-touch do not open IME.
 focusKeyboard();if(touchProofCount++<3)setTimeout(displayProof,500);
},true);
function updateVisibleControls(){
 const viewport=window.visualViewport;
 const keyboardInset=viewport?Math.max(0,window.innerHeight-viewport.height-viewport.offsetTop):0;
 // Keep manual controls reachable above the Android keyboard.
 controls.style.bottom=keyboardInset+'px';updateKeyboardButton();
}
window.visualViewport?.addEventListener('resize',updateVisibleControls);
window.visualViewport?.addEventListener('scroll',updateVisibleControls);
window.addEventListener('resize',updateVisibleControls);updateVisibleControls();
fullscreenButton.onclick=()=>{
 const promise=document.fullscreenElement?document.exitFullscreen?.():document.documentElement.requestFullscreen?.();
 promise?.catch(()=>{});
};
window.addEventListener('error',()=>errorCount++);window.addEventListener('unhandledrejection',()=>errorCount++);
async function displayProof(){
 const canvas=container.querySelector('canvas'),viewport=window.visualViewport;
 const record={connected:document.documentElement.classList.contains('noVNC_connected'),secureContext:window.isSecureContext,viewportWidth:window.innerWidth,viewportHeight:window.innerHeight,keyboardFocused:document.activeElement===input,errorCount,keyboardAutoEnabled,toolbarHeight:controls.getBoundingClientRect().height};
 if(viewport)Object.assign(record,{visualViewportHeight:viewport.height,visualViewportOffsetTop:viewport.offsetTop});
 if(canvas){
  const box=canvas.getBoundingClientRect();Object.assign(record,{canvasWidth:canvas.width,canvasHeight:canvas.height,visibleWidth:box.width,visibleHeight:box.height});
  try{const data=canvas.getContext('2d').getImageData(0,0,canvas.width,canvas.height).data;let lit=0;const colors=new Set();
   for(let i=0;i<data.length;i+=16){if(data[i]+data[i+1]+data[i+2]>30)lit++;colors.add(((data[i]>>4)<<8)|((data[i+1]>>4)<<4)|(data[i+2]>>4));}
   Object.assign(record,{nonBlackRatio:lit/Math.max(1,Math.ceil(data.length/16)),colorBuckets:colors.size});
  }catch{record.errorCount++;}
 }
 await fetch('./display-proof',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(record)}).catch(()=>{});
}
for(const delay of [1500,5000,12000])setTimeout(displayProof,delay);
