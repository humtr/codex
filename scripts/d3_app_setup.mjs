// D3-owned project/title metadata only. Never submit a model message.
import {readFileSync, writeFileSync, existsSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {connectDesktop} from '../providers/chatgpt/desktop_cdp.mjs';

const root=process.argv[2], action=process.argv[3], route=process.argv[4];
const setupPlan=JSON.parse(readFileSync(root+'/plan.json','utf8'));
if(typeof setupPlan.desktop_session!=='string')throw Error('explicit_desktop_session_required');
const name='Termux Provider: D3';
const bridge=readFileSync(root+'/candidate/app_bridge.js','utf8');
const revision=createHash('sha256').update(JSON.stringify(bridge)).digest('hex');
const c=await connectDesktop(setupPlan.desktop_session,undefined,{app:true});
try {
 if(action==='project'||action==='rename') {
  const prior=action==='rename'?JSON.parse(readFileSync(root+'/project.json','utf8')):null;
  const intent=root+(prior?'/project-rename-intent.json':'/project-intent.json');
  const reconcileOnly=!!prior&&existsSync(intent);
  if(!prior&&existsSync(intent))throw Error('Project operation already requested; inspect readback, never recreate blindly');
  if(!reconcileOnly)writeFileSync(intent,JSON.stringify({name,prior,UTC:new Date().toISOString()}),{flag:'wx',mode:0o600});
  const value=await c.evaluate(`(async()=>{
   const b=eval('('+${JSON.stringify(bridge)}+')'),owner=${JSON.stringify('D3-setup:'+root)};
   if(globalThis[Symbol.for('termux.codex.app.bridge.v1')])throw Error('Existing app bridge must be settled; do not replace its owner');
   await b({op:'init',owner,bridgeRevision:${JSON.stringify(revision)},returnPath:'mcp',bindings:{initial:'app://-/assets/app-initial-ed5807835d97.js',shared:'app://-/assets/app-shared-2d77298f0100.js'}});
   try {
    const identity=await b({op:'identity',owner});
    const prior=${JSON.stringify(prior)};
    if(prior){
     const before=await b({op:'project_read',owner,identity,project:prior.id});
     if(${JSON.stringify(reconcileOnly)}){
      if(!before.compatible||before.name!==${JSON.stringify(name)})throw Error('Earlier rename not positively confirmed; no automatic retry');
      return {...before,renamed:true,reconciled_without_mutation:true,screen_interaction:false,model_requests:0};
     }
     if(!before.compatible||before.name!==prior.name)throw Error('Project owner/name changed');
     const s=globalThis[Symbol.for('termux.codex.app.bridge.v1')],api=s.wire;
     const options={parameters:{path:{gizmo_id_or_short_url:prior.id}}};
     const raw=await api.safeGet('/gizmos/{gizmo_id_or_short_url}',options),g=raw.gizmo;
     const unchanged={emoji:g.display.emoji??null,instructions:g.instructions??'',theme:g.display.theme??null};
     if(JSON.stringify(await b({op:'identity',owner}))!==JSON.stringify(identity))throw Error('Account changed');
     const result=await api.safePatch('/projects/{project_id}',{parameters:{path:{project_id:prior.id}},requestBody:{...unchanged,name:${JSON.stringify(name)}},retry:'never'});
     if(result.error!=null)throw Error('Project rename rejected');
     const after=await api.safeGet('/gizmos/{gizmo_id_or_short_url}',options),v=after.gizmo;
     if(v.id!==g.id||v.display.name!==${JSON.stringify(name)}||(v.instructions??'')!==unchanged.instructions||v.memory_scope!==g.memory_scope||
        (v.display.emoji??null)!==unchanged.emoji||(v.display.theme??null)!==unchanged.theme)throw Error('Project rename/preserved-settings readback failed');
     const read=await b({op:'project_read',owner,identity,project:prior.id});
     if(!read.compatible)throw Error('Project compatibility changed');
     return {...read,renamed:true,screen_interaction:false,model_requests:0};
    }
    const matches=(await b({op:'project_list',owner,identity})).filter(x=>x.name===${JSON.stringify(name)});
    if(matches.length>1)throw Error('Ambiguous project name');
    const p=matches[0]??await b({op:'project_create',owner,identity,name:${JSON.stringify(name)}});
    const read=await b({op:'project_read',owner,identity,project:p.id});
    if(read.id!==p.id||read.name!==${JSON.stringify(name)}||!read.compatible)throw Error('Owned private project readback failed');
    return {...read,created:!matches.length,model_requests:0};
   } finally {await b({op:'dispose',owner});}
  })()`);
  if(prior)writeFileSync(root+'/project-before-rename.json',JSON.stringify(prior,null,2),{flag:'wx',mode:0o600});
  writeFileSync(root+'/project.json',JSON.stringify(value,null,2),{flag:prior?'w':'wx',mode:0o600});
  console.log(JSON.stringify(value));
 } else if(action==='titles') {
  const data=JSON.parse(readFileSync(root+'/'+route+'/read-backend-private.json','utf8'));
  const project=JSON.parse(readFileSync(root+'/project.json','utf8'));
  const combo=route.endsWith('operator')?'오퍼레이터mcp':'어댑터mcp';
  const failed=[1,2,3].find(step=>existsSync(root+'/'+route+'/failed-'+step+'.json')&&!existsSync(root+'/'+route+'/failed-'+step+'-resolved.json'));
  const plan=JSON.parse(readFileSync(root+'/plan.json','utf8'));
  const automatic=plan.automatic_fault_recovery;
  const topic=(route.startsWith('fault')?'결과단절·복구'+(automatic?'·자동재연결':'')+(plan.qualification==='unattended_from_start'?'·무감독시험':''):'개발1~3')+(failed?'·'+failed+'턴실패':'');
  const rows=[];
  for(const [index,chat] of data.saved.entries()) {
   const title='D3-'+combo+':'+topic+(data.saved.length>1?'·대화'+(index+1):'');
   const result=await c.evaluate(`(async()=>{
    const api=(await import('app://-/assets/app-shared-2d77298f0100.js')).$wt;
    const id=${JSON.stringify(chat.conversationId)},title=${JSON.stringify(title)};
    const options={parameters:{path:{conversation_id:id}}};
    const raw=await api.safeGet('/conversation/{conversation_id}',options),before=raw?.data??raw;
    if(before.gizmo_id!==${JSON.stringify(project.id)}||before.is_temporary_chat!==false)throw Error('Wrong D3 destination');
    const hash=async v=>[...new Uint8Array(await crypto.subtle.digest('SHA-256',new TextEncoder().encode(JSON.stringify(v))))].map(x=>x.toString(16).padStart(2,'0')).join('');
    const original=await hash(before.mapping);
    if(before.title!==title)await api.safePatch('/conversation/{conversation_id}',{...options,requestBody:{title},retry:'never'});
    const r=await api.safeGet('/conversation/{conversation_id}',options),after=r?.data??r;
    if(after.title!==title||await hash(after.mapping)!==original)throw Error('Title or unchanged-content readback failed');
    return {id,title,project:after.gizmo_id,content_unchanged:true,model_requests:0};
   })()`);
   rows.push(result);
   writeFileSync(root+'/'+route+'/titles.json',JSON.stringify(rows,null,2),{mode:0o600});
  }
  console.log(JSON.stringify({titles:rows.length,model_requests:0}));
 } else throw Error('Use project or titles');
} finally {c.close();}
