// Repository-owned MCP wire endpoint. Only Core can perform advertised actions.
import {localToolCatalog} from './desktop_tool_session.mjs';
import {callMcpBroker} from './desktop_mcp_broker.mjs';

const token={type:'string',minLength:64,maxLength:64,description:'Exact turn token supplied by the native adapter.'};
export const nativeMcpTools=[
 {name:'native_catalog',description:'Read the exact current Codex Core tool catalog, including schemas, namespaces, custom and deferred discovery tools.',inputSchema:{type:'object',properties:{turn_token:token},required:['turn_token'],additionalProperties:false},annotations:{readOnlyHint:true,destructiveHint:false,idempotentHint:true,openWorldHint:false}},
 {name:'native_tool',description:'Request one exact Core tool. Core owns execution, approval and transcript. Keep call_id unchanged when retrying delivery of the same action; use a new call_id for a new action. Return user-facing report with the call.',inputSchema:{type:'object',properties:{turn_token:token,call_id:{type:'string',minLength:1,maxLength:160},name:{type:'string'},input:{type:'string',description:'JSON encoded arguments for a function, exact source for custom/freeform tools.'},report:{type:'string',description:'Public progress report before the call, never private reasoning. Empty if unnecessary.'}},required:['turn_token','call_id','name','input','report'],additionalProperties:false},annotations:{readOnlyHint:false,destructiveHint:true,idempotentHint:false,openWorldHint:true}},
];
const errorMessage=error=>/^mcp_[a-z_]+$/.test(error?.message||'')?error.message:'mcp_binding_failed';
export async function handleNativeMcp(row,invoke,signal){
 if(!row||row.jsonrpc!=='2.0'||typeof row.method!=='string')throw Error('mcp_request_invalid');
 if(row.method==='initialize'){
  const version=row.params?.protocolVersion;
  if(!['2025-03-26','2025-06-18','2025-11-25'].includes(version))throw Error('mcp_protocol_unqualified');
  return {protocolVersion:version,capabilities:{tools:{}},serverInfo:{name:'codex-core-return',version:'0.1.0'},instructions:'Core alone executes native tools. Use the exact supplied turn token and a distinct logical call_id for each action. Do not replay an action when delivery is uncertain.'};
 }
 if(row.method==='ping')return {};
 if(row.method==='tools/list')return {tools:nativeMcpTools};
 if(row.method!=='tools/call')throw Error('mcp_method_unavailable');
 const {name,arguments:args}=row.params||{};
 if(!args||typeof args.turn_token!=='string'||!/^[a-f0-9]{64}$/.test(args.turn_token))throw Error('mcp_turn_unavailable');
 if(name==='native_catalog')return {content:[{type:'text',text:JSON.stringify({tools:localToolCatalog(await invoke({method:'catalog',token:args.turn_token},signal))})}]};
 if(name!=='native_tool')throw Error('mcp_tool_unavailable');
 const output=await invoke({method:'invoke',token:args.turn_token,args:{call_id:args.call_id,name:args.name,input:args.input,report:args.report}},signal);
 // A failed native command is still a valid tool return, not a tunnel/auth error.
 return {content:[{type:'text',text:JSON.stringify({call_id:args.call_id,output})}],isError:false};
}

export async function runNativeMcp(socketPath,input=Bun.stdin.stream(),write=line=>process.stdout.write(line)){
 const requests=new Map(),reader=input.getReader(),decoder=new TextDecoder('utf-8',{fatal:true});let buffer='',discard=false;
 const send=row=>write(JSON.stringify(row)+'\n');
 const receive=line=>{
  let row;try{row=JSON.parse(line);}catch{send({jsonrpc:'2.0',id:null,error:{code:-32700,message:'mcp_json_invalid'}});return;}
  if(row?.method==='notifications/cancelled'){requests.get(JSON.stringify(row.params?.requestId))?.abort(Error('mcp_observer_detached'));return;}
  if(row?.id===undefined)return; // No reply to notifications.
  if(!['string','number'].includes(typeof row.id)){send({jsonrpc:'2.0',id:null,error:{code:-32600,message:'mcp_request_invalid'}});return;}
  const key=JSON.stringify(row.id);if(requests.has(key)){send({jsonrpc:'2.0',id:row.id,error:{code:-32600,message:'mcp_rpc_id_in_flight'}});return;}
  const abort=new AbortController();requests.set(key,abort);
  void(async()=>{
   try{const result=await handleNativeMcp(row,(request,signal)=>callMcpBroker(socketPath,request,signal),abort.signal);if(!abort.signal.aborted)send({jsonrpc:'2.0',id:row.id,result});}
   catch(error){if(!abort.signal.aborted)send({jsonrpc:'2.0',id:row.id,error:{code:-32000,message:errorMessage(error)}});}
   finally{requests.delete(key);}
  })();
 };
 try{
  while(true){const {value,done}=await reader.read();if(done)break;buffer+=decoder.decode(value,{stream:true});
   for(let index;(index=buffer.indexOf('\n'))>=0;){const line=buffer.slice(0,index);buffer=buffer.slice(index+1);if(discard){discard=false;continue;}if(Buffer.byteLength(line)>16*1024*1024){send({jsonrpc:'2.0',id:null,error:{code:-32600,message:'mcp_request_too_large'}});continue;}receive(line);}
   if(Buffer.byteLength(buffer)>16*1024*1024){buffer='';discard=true;}
  }
  decoder.decode();
 }finally{for(const abort of requests.values())abort.abort(Error('mcp_observer_detached'));reader.releaseLock();}
}
if(import.meta.main){process.umask(0o077);const socket=process.argv[2];if(!socket)throw Error('mcp_socket_required');await runNativeMcp(socket);}
