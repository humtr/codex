// Deterministic app-port fixture around the real HTTP adapter; never opens Chat.
// Core executes both commands. This fixture only chooses calls and consumes results.
import {DesktopLocalTransport} from './desktop_local_transport.mjs';
import {writeFileSync} from 'node:fs';
const [bridge,root]=process.argv.slice(2);
const interruptTool=process.env.CODEX_LOCAL_PROBE_INTERRUPT_TOOL==='1';
const rejectTool=process.env.CODEX_LOCAL_PROBE_REJECT_TOOL==='1';
const invalidTool=process.env.CODEX_LOCAL_PROBE_INVALID_TOOL==='1';
const invalidInput={malformed:'{"cmd":',array:'[1,2]',whitespace:' \n'}[process.env.CODEX_LOCAL_PROBE_INVALID_INPUT??'malformed'];
if(invalidTool&&invalidInput===undefined)throw Error('fixture_invalid_input_case');
if(!bridge||!root)throw Error('fixture_paths_required');
const exchange=DesktopLocalTransport.prototype.exchange;
DesktopLocalTransport.prototype.exchange=function(...args){
 if(!this.fixture){
  const f=this.fixture={round:0};
  this.acquire=async()=>({scopeKey:'offline',client:{generation:'offline-app-port'},release(){}});
  this.invoke=async(_lease,_lane,method,data)=>{
   if(method==='create'){f.gateway=data.signature.name;return true;}
   if(method==='close')return true;
   if(method==='submit'){
    f.round++;if(f.round>(rejectTool||invalidTool?4:3))throw Error('fixture_unexpected_generation');
    if(f.round===1){
     if(!data.prompt||data.results)throw Error('fixture_user_request_required');
     const context=JSON.parse(data.prompt.slice(data.prompt.indexOf('Native ordered task context:\n')+'Native ordered task context:\n'.length));
     f.marker=context.at(-1).content.match(/write exactly ([0-9a-f]{32}) to proof\.txt/)?.[1]??context.at(-1).content.match(/printf %s ([0-9a-f]{32}) > proof\.txt/)?.[1];
     if(!f.marker)throw Error('fixture_nonce_missing');
    }else if(rejectTool&&f.round===3){
     if(data.results||!data.prompt||!data.prompt.includes('Recover from the rejected tool'))throw Error('fixture_new_resume_input_required');
     if(!data.prompt.includes(f.marker))throw Error('fixture_completed_history_missing');
    }else{
     if(data.prompt!==undefined||data.results?.length!==1||data.results[0].id!=='fixture-call-'+(f.round-1)||(!interruptTool&&!invalidTool||f.round!==2)&&data.results[0].result.isError)throw Error('fixture_result_not_paired');
     if(invalidTool&&f.round===2){
      if(!/failed to parse|invalid|error/i.test(JSON.stringify(data.results[0].result.content)))throw Error('fixture_native_argument_error_missing');
      writeFileSync(root+'/fixture-input-error.json',JSON.stringify({callId:data.results[0].id,result:data.results[0].result,rawHistory:f.payload.messages.flatMap(m=>Array.isArray(m.content)?m.content:[]).find(p=>p.type==='toolCall'&&p.id==='fixture-call-1')?.arguments}),{mode:0o600});
     }
     writeFileSync(root+'/fixture-result.json',JSON.stringify({round:f.round,result:data.results[0].result,contextRoles:data.contextMessages?.map(m=>m.role),payloadRoles:f.payload?.messages.map(m=>({role:m.role,resume:JSON.stringify(m.content).includes('Read the current proof.txt'),callId:m.toolCallId}))}),{mode:0o600});
     if(interruptTool&&f.round===2&&!data.contextMessages?.some(m=>m.role==='user'&&JSON.stringify(m.content).includes('Read the current proof.txt')))throw Error('fixture_resume_input_missing');
     if(f.round===(rejectTool||invalidTool?4:3)&&!JSON.stringify(data.results[0].result.content).includes(f.marker))throw Error('fixture_readback_missing');
    }
    return true;
   }
   if(method!=='snapshot')throw Error('fixture_unexpected_operation');
   const text=(id,value,phase='commentary')=>({id,author:{role:'assistant'},recipient:'all',channel:phase,status:'finished_successfully',end_turn:phase==='final',content:{content_type:'text',parts:[value]}});
   // The recovery opens a new owned lane; its observation counters start at zero.
   const counters=rejectTool&&f.round>=3?{userSubmissions:1,toolSubmissions:f.round-3,dispatches:f.round-2}:
    {userSubmissions:interruptTool&&f.round>1?2:1,toolSubmissions:f.round-1,dispatches:f.round};
   if(f.round===(rejectTool||invalidTool?4:3))return {messages:[text('fixture-final',f.marker,'final')],accepted:true,roundDispatched:1,terminals:['done'],...counters};
   if(rejectTool&&f.round===2)return {messages:[{id:'rejected',author:{role:'assistant'},recipient:'local.'+f.gateway,status:'finished_successfully',end_turn:false,content:{text:JSON.stringify({name:'fixture_unadvertised',input:'{}',report:''})}}],accepted:true,roundDispatched:1,terminals:['done'],...counters};
   const tools=args[0].tools,cmd=f.round===(invalidTool?2:1)?"printf '%s' '"+f.marker+"' > proof.txt"+(interruptTool?'; echo $$ > command.pid; sleep 45; printf MUTATION_REPLAYED >> proof.txt':''):'cat proof.txt';
   let tool=tools.find(t=>t.name==='exec_command'),input;
   if(tool)input=invalidTool&&f.round===1?invalidInput:JSON.stringify({cmd,yield_time_ms:30000,max_output_tokens:300});
   else{
    if(invalidTool)throw Error('fixture_json_tool_required');
    tool=tools.find(t=>t.name==='exec'&&t.freeform);
    if(!tool)throw Error('fixture_native_shell_unavailable');
    input='const result = await tools.exec_command('+JSON.stringify({cmd,yield_time_ms:30000,max_output_tokens:300})+'); text(result);';
   }
   return {messages:[
    {id:'fixture-call-'+f.round,author:{role:'assistant'},recipient:'local.'+f.gateway,status:'finished_successfully',end_turn:false,content:{content_type:'code',text:JSON.stringify({name:tool.wireName,input,report:f.round===1?'LOCAL_INITIAL_REPORT':invalidTool&&f.round===2?'':'LOCAL_INTERMEDIATE_REPORT'})}}],
    accepted:true,roundDispatched:1,terminals:['done'],...counters};
 };
 }
 this.fixture.payload=args[0];
 return exchange.apply(this,args);
};
process.env.CODEX_CHAT_RETURN_TRANSPORT='local';
process.argv=[process.argv[0],process.argv[1],bridge,root+'/no-app',root+'/server.json'];
await import('./desktop_responses_server.mjs');
