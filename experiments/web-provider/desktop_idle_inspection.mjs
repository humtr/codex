// Test-only fault admission. Reuse the actual owner's single CDP connection.
import {existsSync,readFileSync,writeFileSync,renameSync,unlinkSync} from 'node:fs';
const watchers=new Map();
export function watchIdleInspection(transport,root){
 let state=watchers.get(root);
 if(!state){
  state={owners:new Set(),busy:false};watchers.set(root,state);
  const timer=setInterval(async()=>{
   const request=root+'/inspect-request.json',result=root+'/inspect-result.json';
   if(state.busy||!existsSync(request)||existsSync(result))return;
   state.busy=true;let nonce;
   try{
    ({nonce}=JSON.parse(readFileSync(request,'utf8')));
    if(!/^[a-f0-9]{32}$/.test(nonce))throw Error('invalid_nonce');
    const lease=[...state.owners].map(o=>o.lease).find(Boolean);
    if(!lease)throw Error('no_owned_lease');
    const allowed=[...state.owners].map(o=>o.lane);
    const snapshot=await lease.client.evaluate(`(()=>{
     const allowed=${JSON.stringify(allowed)},foreign=[];let hiddenEnvironmentRecords=0,visibleEnvironmentRecords=0;
     for(const key of Object.getOwnPropertyNames(window).filter(k=>k.startsWith('codex_owned_chat_'))){
      const s=window[key];
      for(const lane of [...(s?.native?.lanes?.keys()||[]),...(s?.lanes?.keys()||[])])if(!allowed.includes(lane))foreign.push(lane);
      for(const [lane,l] of s?.native?.lanes||[]){
       if(!allowed.includes(lane))continue;
       for(const node of Object.values(s.scope.get(s.m.JRn,l.id)||{})){
        const m=node.message;
        if(m?.author?.role!=='user'||!m.content?.parts?.some(p=>typeof p==='string'&&p.includes('<environment_context>')))continue;
        if(m.metadata?.is_visually_hidden_from_conversation===true)hiddenEnvironmentRecords++;else visibleEnvironmentRecords++;
       }
      }
     }
     return {foreignLanes:foreign.length,hiddenEnvironmentRecords,visibleEnvironmentRecords,
      composerHasDraft:[...document.querySelectorAll('[contenteditable=true]')].some(e=>e.getBoundingClientRect().width>0&&(e.textContent||'').trim()),
      visibleGeneration:[...document.querySelectorAll('button')].some(e=>e.getBoundingClientRect().width>0&&/^stop(?:\\b|$)/i.test(e.getAttribute('aria-label')||''))};
    })()`);
    writeFileSync(result+'.tmp',JSON.stringify({nonce,state:snapshot}),{mode:0o600});
   }catch{writeFileSync(result+'.tmp',JSON.stringify({nonce,error:'owned_inspection_failed'}),{mode:0o600});}
   finally{renameSync(result+'.tmp',result);unlinkSync(request);state.busy=false;}
  },50);timer.unref();
 }
 state.owners.add(transport);
}
