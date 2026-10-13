// Thin candidate adapter; parsing, tool wire mapping and SSE reuse the existing bridge.
import {join,resolve} from 'node:path';
import {pathToFileURL} from 'node:url';
import {writeFileSync,readFileSync,readlinkSync,renameSync} from 'node:fs';
import {randomBytes,timingSafeEqual} from 'node:crypto';
import {DesktopChatTransport} from './desktop_response_transport.mjs';
import {DesktopTranscript,validateEnvelope,validateTextContext,DesktopCompaction,desktopEstimatedUsage,markDesktopEstimatedUsage,validateDesktopRoute} from './desktop_protocol.mjs';
const [candidate,desktopRoot,statePath]=process.argv.slice(2);
process.umask(0o077);
const source=resolve(candidate,'src');
const {parseRequest}=await import(pathToFileURL(join(source,'responses/parser.ts')));
const {AsyncEventQueue}=await import(pathToFileURL(join(source,'event-queue.ts')));
const {bridgeToResponsesSSE,formatErrorResponse}=await import(pathToFileURL(join(source,'bridge.ts')));
const {namespacedToolName}=await import(pathToFileURL(join(source,'types.ts')));
const {extractCodexTurnIdentityFromBody}=await import(pathToFileURL(join(source,'adapters/chatgpt-web/environment.ts')));
const {SUMMARY_PREFIX}=await import(pathToFileURL(join(source,'responses/compaction.ts')));
const token=randomBytes(32).toString('hex'),model='chatgpt-desktop/instant';
const transport=new DesktopChatTransport(desktopRoot);let busy=false,owner,active=false,transcript=new DesktopTranscript(),checkpoint=new DesktopCompaction(SUMMARY_PREFIX);
const report={actualChatMode:true,model,requests:0,modelRequestedTools:0,returnedNativeResults:0,completedReplies:0,submittedRequests:0,cancelledRequests:0,retryAttempts:0,contextPackets:0,modelRequestedSearches:0,returnedNativeSearchResults:0,nativeSearchResultErrors:0,nativeSearchLoadedTools:0,compactionRequests:0,compactionSummaries:0,appliedCheckpoints:0,estimatedUsage:true};
function save(){const temp=statePath+'.proof.'+randomBytes(6).toString('hex');writeFileSync(temp,JSON.stringify(report),{mode:0o600,flag:'wx'});renameSync(temp,statePath+'.proof');}
const server=Bun.serve({hostname:'127.0.0.1',port:0,idleTimeout:0,maxRequestBodySize:2*1024*1024,async fetch(req){
 const auth=Buffer.from(req.headers.get('authorization')||''),expected=Buffer.from('Bearer '+token);
 if(auth.length!==expected.length||!timingSafeEqual(auth,expected)||req.headers.has('origin'))return new Response('Unauthorized',{status:401});
 const path=new URL(req.url).pathname;
 if(path==='/v1/models'&&req.method==='GET')return Response.json({object:'list',data:[{id:model,object:'model',owned_by:'chatgpt-desktop'}]});
 if(path!=='/v1/responses'||req.method!=='POST')return new Response('Not found',{status:404});
 if(busy)return formatErrorResponse(409,'invalid_request_error','desktop_chat_busy');
 busy=true;
 let raw,parsed,route;try{raw=await req.json();route=validateDesktopRoute(raw);if(raw.model!==model||raw.stream!==true||raw.previous_response_id)throw Error('unsupported_desktop_request');parsed=parseRequest(raw);validateTextContext(raw,parsed);if(parsed._opaqueMultiAgentV2Payload)throw Error('opaque_context_unavailable');}catch{busy=false;return formatErrorResponse(400,'invalid_request_error','unsupported_desktop_request');}
 const identity=extractCodexTurnIdentityFromBody(raw),key=identity.threadId||req.headers.get('session_id')||req.headers.get('x-codex-thread-id');
 if(!key){busy=false;return formatErrorResponse(400,'invalid_request_error','native_thread_identity_required');}
 if(owner&&key!==owner){
  if(active){busy=false;return formatErrorResponse(409,'invalid_request_error','desktop_thread_owner_conflict');}
  transport.retire();transcript=new DesktopTranscript();checkpoint=new DesktopCompaction(SUMMARY_PREFIX);
 }
 if(checkpoint.consume(parsed.context.messages)){transcript=new DesktopTranscript();transport.retire();report.appliedCheckpoints++;}
 try{transcript.receive(parsed.context.messages);}catch{busy=false;return formatErrorResponse(400,'invalid_request_error','native_tool_result_mismatch');}
 owner=key;active=true;report.errorCode=null;
 report.requests++;report.contextChars=JSON.stringify(parsed.context).length;report.toolCount=parsed.context.tools?.length||0;report.instructionChars=(parsed.context.systemPrompt||[]).join('\n').length;report.hasNativeThreadKey=!!key;save();
 const queue=new AsyncEventQueue(),abort=new AbortController();req.signal.addEventListener('abort',()=>abort.abort(),{once:true});if(req.signal.aborted)abort.abort();
 const compact=parsed._compactionRequest===true;if(compact)report.compactionRequests++;
 const tools=(compact?[]:parsed.context.tools||[]).map(t=>({...t,wireName:namespacedToolName(t.namespace,t.name)}));
 const namespaceMap=new Map(tools.filter(t=>t.namespace).map(t=>[t.wireName,{namespace:t.namespace,name:t.name}]));
 const freeform=new Set(tools.filter(t=>t.freeform).map(t=>t.wireName)),toolSearch=new Set(tools.filter(t=>t.toolSearch).map(t=>t.wireName));
 const heartbeat=setInterval(()=>queue.push({type:'heartbeat'}),2000);
 void (async()=>{
  try{
   report.returnedNativeResults=parsed.context.messages.filter(m=>m.role==='toolResult').length;const searches=parsed.context.messages.filter(m=>m.role==='toolResult'&&m.toolName==='tool_search');report.returnedNativeSearchResults=searches.length;report.nativeSearchResultErrors=searches.filter(m=>m.isError).length;report.nativeSearchLoadedTools=(Array.isArray(raw.input)?raw.input:[]).filter(m=>m.type==='tool_search_output').flatMap(m=>Array.isArray(m.tools)?m.tools:[]).reduce((n,t)=>n+(t.type==='namespace'&&Array.isArray(t.tools)?t.tools.length:1),0);save();
   const payload={instructions:parsed.context.systemPrompt||[],messages:parsed.context.messages,tools};
   const response=await transport.exchange(payload,abort.signal,()=>{report.submittedRequests++;save();},()=>{report.retryAttempts++;save();},()=>{report.contextPackets++;save();},route);
   if(response.type==='tool'){
    if(compact)throw Error('desktop_compaction_must_not_call_tools');
    const tool=validateEnvelope(response,tools);
    const callId='call_desktop_'+randomBytes(12).toString('hex');transcript.issue(callId);
    queue.push({type:'tool_call_start',id:callId,name:tool.wireName});
    queue.push({type:'tool_call_delta',arguments:tool.freeform?response.input:JSON.stringify(response.input)});queue.push({type:'tool_call_end'});report.modelRequestedTools++;if(tool.toolSearch)report.modelRequestedSearches++;
    queue.push({type:'done',endTurn:false,usage:desktopEstimatedUsage(payload,response)});
   }else if(response.type==='final'&&typeof response.text==='string'){
    if(compact){checkpoint.offer(response.text);report.compactionSummaries++;}
    queue.push({type:'text_delta',text:response.text,phase:'final_answer'});queue.push({type:'done',endTurn:true,usage:desktopEstimatedUsage(payload,response)});report.completedReplies++;active=false;
   }else throw Error('invalid_desktop_envelope');
  }catch(e){if(abort.signal.aborted)report.cancelledRequests++;active=false;transport.retire();transcript=new DesktopTranscript();checkpoint=new DesktopCompaction(SUMMARY_PREFIX);report.errorCode=/^[a-z_]+$/.test(e.message)?e.message:'desktop_adapter_failed';queue.push({type:'error',message:report.errorCode,retryable:false});}
  finally{clearInterval(heartbeat);save();queue.close();busy=false;}
 })();
 return new Response(markDesktopEstimatedUsage(bridgeToResponsesSSE(queue,model,namespaceMap,freeform,toolSearch,()=>abort.abort(),2000)),{headers:{'Content-Type':'text/event-stream','Cache-Control':'no-store'}});
}});
const stat=readFileSync(`/proc/${process.pid}/stat`,'utf8');
writeFileSync(statePath,JSON.stringify({baseUrl:'http://127.0.0.1:'+server.port+'/v1',token,model,pid:process.pid,start:stat.slice(stat.lastIndexOf(')')+2).split(' ')[19],image:readlinkSync(`/proc/${process.pid}/exe`)}),{mode:0o600});
save();
console.log(JSON.stringify({desktopResponsesReady:true,model,production:false}));
const stop=()=>{server.stop(true);process.exit(0);};process.once('SIGTERM',stop);process.once('SIGINT',stop);
