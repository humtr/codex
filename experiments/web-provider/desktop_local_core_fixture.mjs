// Deterministic app-port fixture around the real HTTP adapter; never opens Chat.
// Core executes both commands. This fixture only chooses calls and consumes results.
import {DesktopLocalTransport} from './desktop_local_transport.mjs';
import {writeFileSync} from 'node:fs';
const [bridge,root]=process.argv.slice(2);
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
    f.round++;if(f.round>3)throw Error('fixture_unexpected_generation');
    if(f.round===1){
     if(!data.prompt||data.results)throw Error('fixture_user_request_required');
     const context=JSON.parse(data.prompt.slice(data.prompt.indexOf('Native ordered task context:\n')+'Native ordered task context:\n'.length));
     f.marker=context.at(-1).content.match(/write exactly ([0-9a-f]{32}) to proof\.txt/)?.[1];
     if(!f.marker)throw Error('fixture_nonce_missing');
    }else{
     if(data.prompt!==undefined||data.results?.length!==1||data.results[0].id!=='fixture-call-'+(f.round-1)||data.results[0].result.isError)throw Error('fixture_result_not_paired');
     writeFileSync(root+'/fixture-result.json',JSON.stringify({round:f.round,result:data.results[0].result}),{mode:0o600});
     if(f.round===3&&!JSON.stringify(data.results[0].result.content).includes(f.marker))throw Error('fixture_readback_missing');
    }
    return true;
   }
   if(method!=='snapshot')throw Error('fixture_unexpected_operation');
   const text=(id,value,phase='commentary')=>({id,author:{role:'assistant'},recipient:'all',channel:phase,status:'finished_successfully',end_turn:phase==='final',content:{content_type:'text',parts:[value]}});
   if(f.round===3)return {messages:[text('fixture-final',f.marker,'final')],accepted:true,roundDispatched:1,terminals:['done'],userSubmissions:1,toolSubmissions:2,dispatches:3};
   const tools=args[0].tools,cmd=f.round===1?"printf '%s' '"+f.marker+"' > proof.txt":'cat proof.txt';
   let tool=tools.find(t=>t.name==='exec_command'),input;
   if(tool)input=JSON.stringify({cmd,yield_time_ms:30000,max_output_tokens:300});
   else{
    tool=tools.find(t=>t.name==='exec'&&t.freeform);
    if(!tool)throw Error('fixture_native_shell_unavailable');
    input='const result = await tools.exec_command('+JSON.stringify({cmd,yield_time_ms:30000,max_output_tokens:300})+'); text(result);';
   }
   return {messages:[
    {id:'fixture-call-'+f.round,author:{role:'assistant'},recipient:'local.'+f.gateway,status:'finished_successfully',end_turn:false,content:{content_type:'code',text:JSON.stringify({name:tool.wireName,input,report:f.round===1?'LOCAL_INITIAL_REPORT':'LOCAL_INTERMEDIATE_REPORT'})}}],
    accepted:true,roundDispatched:1,terminals:['done'],userSubmissions:1,toolSubmissions:f.round-1,dispatches:f.round};
  };
 }
 return exchange.apply(this,args);
};
process.env.CODEX_CHAT_RETURN_TRANSPORT='local';
process.argv=[process.argv[0],process.argv[1],bridge,root+'/no-app',root+'/server.json'];
await import('./desktop_responses_server.mjs');
