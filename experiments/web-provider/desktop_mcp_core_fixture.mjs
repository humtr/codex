// Real stdio MCP and Unix result return with a deterministic host, never Chat.
// This fixture requests commands; the actual signed Codex Core executes them.
import {readFileSync,writeFileSync} from 'node:fs';
import {fileURLToPath} from 'node:url';
import {DesktopMcpTransport} from './desktop_mcp_transport.mjs';

const [bridge,root]=process.argv.slice(2);
if(!bridge||!root)throw Error('fixture_paths_required');
const exchange=DesktopMcpTransport.prototype.exchange;
DesktopMcpTransport.prototype.exchange=function(payload,...rest){
 if(!this.fixture){
  const f=this.fixture={calls:0,returns:0,rpcIds:[],done:false};
  this.acquire=async()=>({scopeKey:'offline-mcp',client:{generation:'offline-mcp'},release(){}});
  this.invoke=async(_lease,_lane,method,data)=>{
   if(method==='create')return true;
   if(method==='close'){f.child?.kill();return true;}
   if(method==='submit'){
    const messages=JSON.parse(data.prompt.slice('Native ordered task context:\n'.length));
    const marker=messages.at(-1).content.match(/write exactly ([0-9a-f]{32}) to proof\.txt/)?.[1];
    if(!marker)throw Error('fixture_nonce_missing');
    f.child=Bun.spawn([process.execPath,fileURLToPath(new URL('./desktop_mcp_stdio.mjs',import.meta.url)),JSON.parse(readFileSync(root+'/server.json','utf8')).mcpSocket],{stdin:'pipe',stdout:'pipe',stderr:'ignore'});
    const reader=f.child.stdout.getReader(),decoder=new TextDecoder();let buffered='';
    const receive=async()=>{
     while(!buffered.includes('\n')){const {value,done}=await reader.read();if(done)throw Error('fixture_stdio_closed');buffered+=decoder.decode(value,{stream:true});}
     const index=buffered.indexOf('\n'),row=JSON.parse(buffered.slice(0,index));buffered=buffered.slice(index+1);return row;
    };
    const rpc=async(method,params)=>{await f.child.stdin.write(JSON.stringify({jsonrpc:'2.0',id:0,method,params})+'\n');const row=await receive();if(row.id!==0||row.error)throw Error('fixture_rpc_result_invalid');return row.result;};
    f.running=(async()=>{
     const initialized=await rpc('initialize',{protocolVersion:'2025-11-25',capabilities:{}});if(initialized.serverInfo.name!=='codex-core-return')throw Error('fixture_foreign_mcp_server');
     const listing=await rpc('tools/list');if(listing.tools.map(t=>t.name).join(',')!=='native_catalog,native_tool')throw Error('fixture_foreign_catalog');
     const catalog=JSON.parse((await rpc('tools/call',{name:'native_catalog',arguments:{turn_token:this.token}})).content[0].text).tools;
     if(JSON.stringify(catalog)!==JSON.stringify(f.tools))throw Error('fixture_catalog_changed');
     for(const [index,cmd] of ["printf '%s' '"+marker+"' > proof.txt",'cat proof.txt'].entries()){
      let tool=f.tools.find(t=>t.name==='exec_command'),input=JSON.stringify({cmd,yield_time_ms:30000,max_output_tokens:300});
      if(!tool){tool=f.tools.find(t=>t.name==='exec'&&t.freeform);if(!tool)throw Error('fixture_native_shell_unavailable');input='const result = await tools.exec_command('+JSON.stringify({cmd,yield_time_ms:30000,max_output_tokens:300})+'); text(result);';}
      f.calls++;f.rpcIds.push(0);
      const response=await rpc('tools/call',{name:'native_tool',arguments:{turn_token:this.token,call_id:'step-'+index,name:tool.wireName,input,report:index===0?'LOCAL_INITIAL_REPORT':'LOCAL_INTERMEDIATE_REPORT'}});
      const result=JSON.parse(response.content[0].text).output;
      writeFileSync(root+'/fixture-result.json',JSON.stringify({index,tool:tool.wireName,result}),{mode:0o600});
      if(response.isError||result.isError||index===1&&!JSON.stringify(result.content).includes(marker))throw Error('fixture_native_output_invalid');
      f.returns++;
     }
     f.marker=marker;f.done=true;
     writeFileSync(root+'/mcp-fixture-proof.json',JSON.stringify({server:'codex-core-return',protocol:'2025-11-25',nativeCalls:f.calls,exactCoreReturns:f.returns,reusedRpcId:true,hostInitialSubmissions:1,hostResultUserSubmissions:0,hostContinuationFromOriginalCalls:true}),{mode:0o600});
    })().catch(error=>{f.error=/^fixture_[a-z_]+$/.test(error?.message||'')?error.message:'mcp_fixture_host_failed';});
    return true;
   }
   if(method!=='snapshot')throw Error('fixture_operation_invalid');
   return {starts:1,error:f.error,messages:f.done?[{id:'fixture-final',author:{role:'assistant'},recipient:'all',channel:'final',status:'finished_successfully',end_turn:true,content:{content_type:'text',parts:[f.marker]}}]:[],terminals:f.done?['done']:[]};
  };
 }
 this.fixture.tools=payload.tools;
 return exchange.call(this,payload,...rest);
};
process.env.CODEX_CHAT_RETURN_TRANSPORT='mcp';process.env.CODEX_CHAT_MCP_CONNECTOR_NAME='Owned offline MCP fixture';
process.argv=[process.argv[0],process.argv[1],bridge,root+'/no-app',root+'/server.json'];
await import('./desktop_responses_server.mjs');
