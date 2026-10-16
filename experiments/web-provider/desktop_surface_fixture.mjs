// Real native execution, deterministic model port: no Chat/API inference.
import {DesktopLocalTransport} from './desktop_local_transport.mjs';
import {writeFileSync} from 'node:fs';
const [bridge,root]=process.argv.slice(2),exchange=DesktopLocalTransport.prototype.exchange;
const observations=[];
DesktopLocalTransport.prototype.exchange=function(payload,...rest){
 if(!this.surface){
  const state=this.surface={round:0};
  this.acquire=async()=>({scopeKey:'offline',client:{generation:'owned-surface'},release(){}});
  this.invoke=async(_lease,_lane,method,data)=>{
   if(method==='create'){state.gateway=data.signature.name;return true;}
   if(method==='close')return true;
   if(method==='submit'){
    state.round++;if(state.round>10)throw Error('fixture_unexpected_generation');
    if(state.round===1){
     if(!data.prompt&&!data.taskMessages?.length)throw Error('fixture_initial_input_required');
     state.marker=(data.prompt??JSON.stringify(data.taskMessages)).match(/marker is ([a-f0-9]{32})/)?.[1];
     if(!state.guardian&&!state.marker)throw Error('fixture_marker_missing');
    }else if(!data.results?.length||data.prompt)throw Error('fixture_tool_return_required');
    if(!state.guardian&&[5,6].includes(state.round)){const expected=state.round===5?'surface-start-batch':'surface-poll-batch';if(data.results.length!==1||data.results[0].id!==expected||data.results[0].result.results?.length!==2)throw Error('fixture_original_batch_result_required');}
    state.user=(state.user??0)+(data.prompt?1:0)+(data.taskMessages||[]).filter(m=>m.role==='user').length+(data.contextMessages||[]).filter(m=>m.role==='user').length;
    state.returns=(state.returns??0)+(data.results?.length??0);return true;
   }
   if(method!=='snapshot')throw Error('fixture_operation_invalid');
   const tool=name=>{const t=state.tools.find(t=>t.name===name);if(!t)throw Error('fixture_missing_'+name);return t;};
   const call=(id,t,input,report='')=>({id,author:{role:'assistant'},recipient:'local.'+state.gateway,status:'finished_successfully',content:{content_type:'code',text:JSON.stringify({name:t.wireName,input:t.freeform?input:JSON.stringify(input),report})}});
   const final=text=>({id:'surface-final',author:{role:'assistant'},recipient:'all',channel:'final',status:'finished_successfully',end_turn:true,content:{content_type:'text',parts:[text]}});
   let messages;
   if(state.guardian)messages=[final(JSON.stringify({risk_level:'low',user_authorization:'high',outcome:'allow',rationale:'Read only the disposable proof marker explicitly requested by this trial.'}))];
   else if(state.round===1)messages=[call('surface-search',state.tools.find(t=>t.toolSearch),{query:'owned_catalog _read_marker',limit:1},'LOCAL_INITIAL_REPORT')];
   else if(state.round===2)messages=[call('surface-mcp',tool('_read_marker'),{})];
   else if(state.round===3)messages=[call('surface-patch',tool('apply_patch'),'*** Begin Patch\n*** Add File: proof.txt\n+'+state.marker+'\n*** End Patch')];
   else if(state.round===4)messages=['A','B'].map(label=>call('surface-'+label,tool('exec_command'),{cmd:'python worker.py '+label,login:false,yield_time_ms:1000,max_output_tokens:200}));
   else if(state.round===5){
    const results=state.payload.messages.filter(m=>m.role==='toolResult');
    messages=['A','B'].map(label=>{const child=[...this.projection.calls.values()].find(({call})=>call.hostCallId==='surface-start-batch'&&call.batchIndex===(label==='A'?0:1));const result=results.find(m=>m.toolCallId===child?.call.id),match=JSON.stringify(result?.content).match(/session ID (\d+)/);if(!match)throw Error('fixture_background_session_missing');return call('surface-poll-'+label,tool('write_stdin'),{session_id:Number(match[1]),chars:'',yield_time_ms:10000,max_output_tokens:200});});
   }else if(state.round===6)messages=[call('surface-question',tool('request_user_input'),{questions:[{id:'fixture_choice',header:'Surface',question:'Choose KEEP for this disposable test?',options:[{label:'KEEP',description:'Keep the fixture result.'},{label:'OTHER',description:'Other fixture choice.'}]}]},'LOCAL_INTERMEDIATE_REPORT')];
   else messages=[final(state.marker)];
   if(!state.guardian&&[4,5].includes(state.round)){
    const entries=messages.map(m=>{const {name,input}=JSON.parse(m.content.text);return {name,input};});
    messages=[{...messages[0],id:state.round===4?'surface-start-batch':'surface-poll-batch',content:{content_type:'code',text:JSON.stringify({calls:JSON.stringify(entries),report:''})}}];
   }
   const row={guardian:state.guardian,round:state.round,callIds:messages.filter(m=>m.recipient?.startsWith('local.')).map(m=>m.id)};
   if(!observations.some(r=>JSON.stringify(r)===JSON.stringify(row)))observations.push(row);
   writeFileSync(root+'/surface-host.json',JSON.stringify(observations),{mode:0o600});
   return {messages,accepted:true,roundDispatched:1,terminals:['done'],userSubmissions:state.user,toolSubmissions:state.returns,dispatches:state.round};
  };
 }
 this.surface.payload=payload;this.surface.tools=payload.tools;
 this.surface.guardian=payload.instructions.some(s=>s.startsWith('The native final reply must match this exact output format.')&&s.includes('"outcome"'));
 return exchange.call(this,payload,...rest).then(result=>{
  observations.push({nativeProjection:true,type:result.type,calls:(result.calls||[]).map(c=>({id:c.id,name:c.name,hostCallId:c.hostCallId}))});
  writeFileSync(root+'/surface-batches.json',JSON.stringify(observations.filter(r=>r.nativeProjection)),{mode:0o600});return result;
 });
};
process.env.CODEX_CHAT_RETURN_TRANSPORT='local';
process.argv=[process.argv[0],process.argv[1],bridge,root+'/no-app',root+'/server.json'];
await import('./desktop_responses_server.mjs');
