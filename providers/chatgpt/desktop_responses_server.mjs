// Thin candidate adapter; parsing, tool wire mapping and SSE reuse the existing bridge.
import {join,resolve} from 'node:path';
import {pathToFileURL} from 'node:url';
import {writeFileSync,readFileSync,readlinkSync,renameSync,mkdtempSync,rmSync} from 'node:fs';
import {randomBytes,timingSafeEqual,createHmac} from 'node:crypto';
import {desktopModelCatalog} from './desktop_model_controls.mjs';
import {desktopConnection} from './desktop_cdp.mjs';
import {DesktopTransport} from './desktop_response_transport.mjs';
import {DesktopLocalTransport} from './desktop_local_transport.mjs';
import {DesktopMcpTransport} from './desktop_mcp_transport.mjs';
import {DesktopMcpBroker,serveMcpBroker} from './desktop_mcp_broker.mjs';
import {readDesktopRequest,DesktopBodyError,DESKTOP_DECODED_LIMIT} from './desktop_http.mjs';
import {DesktopRequestTiming,DesktopTranscript,admitDesktopRequest,recordDesktopSubmission,recordDesktopContext,validateEnvelope,DesktopCompaction,desktopEstimatedUsage,markDesktopEstimatedUsage} from './desktop_protocol.mjs';
const [candidate,desktopRoot,statePath,runtimePath]=process.argv.slice(2);
process.umask(0o077);
const source=resolve(candidate,'src');
const bundled=candidate.endsWith('.mjs')?await import(pathToFileURL(candidate)):null;
const load=async path=>bundled||await import(pathToFileURL(join(source,path)));
const {parseRequest}=await load('responses/parser.ts');
const {AsyncEventQueue}=await load('event-queue.ts');
const {bridgeToResponsesSSE,formatErrorResponse}=await load('bridge.ts');
const {namespacedToolName}=await load('types.ts');
const {extractCodexTurnIdentityFromBody}=await load('adapters/chatgpt-web/environment.ts');
const {SUMMARY_PREFIX}=await load('responses/compaction.ts');
let runtime;
if(runtimePath){
 const {lstatSync}=await import('node:fs'),s=lstatSync(runtimePath);
 if(!s.isFile()||s.isSymbolicLink()||s.uid!==process.getuid()||(s.mode&0o777)!==0o600)throw Error('private_runtime_required');
 runtime=JSON.parse(readFileSync(runtimePath,'utf8'));
 const u=new URL(runtime.baseUrl);
 if(u.protocol!=='http:'||u.hostname!=='127.0.0.1'||!u.port||u.pathname!=='/v1'||u.username||u.password||u.search||u.hash||! /^[0-9a-f]{64}$/.test(runtime.token)||runtime.model!=='chatgpt-desktop/instant')throw Error('invalid_runtime_endpoint');
}
const token=runtime?.token||randomBytes(32).toString('hex'),model='chatgpt-desktop/instant';
async function ensureDesktop(signal){
 if(!runtime)return;
 try{desktopConnection(desktopRoot);return;}catch{}
 const child=Bun.spawn([process.env.PREFIX+'/bin/python','-B',join(runtime.source,'desktop_lifecycle.py'),'desktop','--runtime',runtimePath],{stdin:'ignore',stdout:'ignore',stderr:'ignore'});
 const cancel=()=>child.kill('SIGTERM');signal.addEventListener('abort',cancel,{once:true});
 try{signal.throwIfAborted();if(await child.exited!==0)throw Error('desktop_bootstrap_failed');signal.throwIfAborted();}
 finally{signal.removeEventListener('abort',cancel);}
}
const returnTransport=runtime?.returnTransport??process.env.CODEX_CHAT_RETURN_TRANSPORT??'text';
if(!['text','local','mcp'].includes(returnTransport))throw Error('unsupported_chat_return_transport');
const mcpConnectorName=process.env.CODEX_CHAT_MCP_CONNECTOR_NAME;
if(returnTransport==='mcp'&&!mcpConnectorName?.trim())throw Error('mcp_connector_required');
const maxLocalDispatches=process.env.CODEX_CHAT_LOCAL_MAX_DISPATCHES===undefined?undefined:Number(process.env.CODEX_CHAT_LOCAL_MAX_DISPATCHES);
if(maxLocalDispatches!==undefined&&(!Number.isSafeInteger(maxLocalDispatches)||maxLocalDispatches<1))throw Error('invalid_chat_trial_budget');
const mcpRoot=returnTransport==='mcp'?mkdtempSync(join(process.env.TMPDIR,'cm-')):undefined;
const mcpSocket=mcpRoot&&join(mcpRoot,'s'),mcpBroker=mcpRoot&&new DesktopMcpBroker();
const stopMcp=mcpBroker&&await serveMcpBroker(mcpBroker,mcpSocket);
function newLane(key,parent){return {key,parent,active:false,busy:false,transport:returnTransport==='mcp'?new DesktopMcpTransport(desktopRoot,{broker:mcpBroker,connectorName:mcpConnectorName}):returnTransport==='local'?new DesktopLocalTransport(desktopRoot,{maxDispatches:maxLocalDispatches}):new DesktopTransport(desktopRoot,{key,parent}),transcript:new DesktopTranscript(),checkpoint:new DesktopCompaction(SUMMARY_PREFIX),resultCount:0};}
const lanes=new Map();
const report={actualChatMode:true,model,requests:0,modelRequestedTools:0,returnedNativeResults:0,completedReplies:0,submittedRequests:0,cancelledRequests:0,retryAttempts:0,contextPackets:0,submittedProRequests:0,modelRequestedSearches:0,returnedNativeSearchResults:0,nativeSearchResultErrors:0,nativeSearchLoadedTools:0,compactionRequests:0,compactionSummaries:0,appliedCheckpoints:0,estimatedUsage:true};
report.guardianRequests=0;report.ownerConflicts=0;report.guardianInterleaves=0;report.guardianParentReplays=0;report.guardianIndependentConversations=0;report.maxConcurrentRequests=0;
report.arrivals=0;report.admissionFailures={};
report.observedChatSubmissions=0;report.submittedContextRequests=0;report.submissionSources={app_accepted:0,ui_observed:0};
report.returnTransport=returnTransport;report.toolResultSubmissions=0;
function save(){const temp=statePath+'.proof.'+randomBytes(6).toString('hex');writeFileSync(temp,JSON.stringify(report),{mode:0o600,flag:'wx'});renameSync(temp,statePath+'.proof');}
// Leave room for the streaming reader to report the smaller wire ceiling itself,
// including chunked bodies without Content-Length. Bun retains a hard outer cap.
const server=Bun.serve({hostname:'127.0.0.1',port:runtime?Number(new URL(runtime.baseUrl).port):0,idleTimeout:0,maxRequestBodySize:DESKTOP_DECODED_LIMIT,async fetch(req){
 const path=new URL(req.url).pathname;
 if(path==='/__desktop_ready'&&req.method==='GET'&&!req.headers.has('origin')){
  const nonce=req.headers.get('x-termux-trial-challenge')||'';
  if(!/^[0-9a-f]{64}$/.test(nonce))return new Response('Invalid challenge',{status:400});
  return Response.json({model,proof:createHmac('sha256',token).update('desktop-ready-v1:'+nonce).digest('hex')},{headers:{'Cache-Control':'no-store'}});
 }
 const auth=Buffer.from(req.headers.get('authorization')||''),expected=Buffer.from('Bearer '+token);
 if(auth.length!==expected.length||!timingSafeEqual(auth,expected)||req.headers.has('origin'))return new Response('Unauthorized',{status:401});
 if(path==='/v1/models'&&req.method==='GET')return Response.json({object:'list',data:[{id:model,object:'model',owned_by:'chatgpt-desktop'},...desktopModelCatalog().map(m=>({id:m.slug,object:'model',owned_by:'chatgpt-desktop'}))]});
 if(path!=='/v1/responses'||req.method!=='POST')return new Response('Not found',{status:404});
 report.arrivals++;const timing=new DesktopRequestTiming(report.arrivals);
 let raw,parsed,route,outputInstructions;
 try{raw=await readDesktopRequest(req);}catch(e){const known=e instanceof DesktopBodyError,code=known?e.message:'desktop_body_read_failed';report.admissionFailures[code]=(report.admissionFailures[code]||0)+1;save();return formatErrorResponse(known?e.status:400,'invalid_request_error',code);}
 try{
  ({parsed,route,outputInstructions}=admitDesktopRequest(raw,parseRequest));
 }catch(e){const code=/^(?:invalid_desktop_request|unsupported_desktop_route|unsupported_desktop_output_format|desktop_stream_required|desktop_response_cursor_unavailable|desktop_request_parse_failed|desktop_tool_declaration_unsupported|desktop_compaction_unavailable|desktop_text_only|opaque_context_unavailable)$/.test(e.message)?e.message:'invalid_desktop_request';report.admissionFailures[code]=(report.admissionFailures[code]||0)+1;save();return formatErrorResponse(400,'invalid_request_error',code);}
 timing.mark('admitted');
 const identity=extractCodexTurnIdentityFromBody(raw),key=identity.threadId||req.headers.get('session_id')||req.headers.get('x-codex-thread-id');
 if(!key)return formatErrorResponse(400,'invalid_request_error','native_thread_identity_required');
 const parent=identity.parentThreadId||req.headers.get('x-codex-parent-thread-id');
 const guardian=identity.agentName==='guardian'||identity.subagentKind==='guardian'||req.headers.get('x-openai-subagent')==='guardian';
 if(guardian)report.guardianRequests++;
 let lane=lanes.get(key);
 if(!lane){
  lane=newLane(key,guardian?parent:undefined);
  if(guardian){report.guardianIndependentConversations++;}
  lanes.set(key,lane);
 }
 if(lane.busy){report.ownerConflicts++;save();return formatErrorResponse(409,'invalid_request_error','desktop_chat_busy');}
 lane.busy=true;report.maxConcurrentRequests=Math.max(report.maxConcurrentRequests,[...lanes.values()].filter(l=>l.busy).length);
 let {transport,transcript,checkpoint}=lane;
 if(checkpoint.consume(parsed.context.messages)){lane.transcript=transcript=new DesktopTranscript();transport.retire();report.appliedCheckpoints++;}
 try{transcript.receive(parsed.context.messages);}catch{lane.busy=false;return formatErrorResponse(400,'invalid_request_error','native_tool_result_mismatch');}
 lane.active=true;report.errorCode=null;report.route=route;delete report.failureSite;delete report.nativeFailure;delete report.toolDiagnostic;
 report.requests++;report.contextChars=JSON.stringify(parsed.context).length;report.toolCount=parsed.context.tools?.length||0;report.instructionChars=(parsed.context.systemPrompt||[]).join('\n').length;report.hasNativeThreadKey=!!key;save();
 const queue=new AsyncEventQueue(),abort=new AbortController();req.signal.addEventListener('abort',()=>abort.abort(),{once:true});if(req.signal.aborted)abort.abort();
 const compact=parsed._compactionRequest===true;if(compact)report.compactionRequests++;
 const tools=(compact?[]:parsed.context.tools||[]).map(t=>({...t,wireName:namespacedToolName(t.namespace,t.name)}));
 const namespaceMap=new Map(tools.filter(t=>t.namespace).map(t=>[t.wireName,{namespace:t.namespace,name:t.name}]));
 const freeform=new Set(tools.filter(t=>t.freeform).map(t=>t.wireName)),toolSearch=new Set(tools.filter(t=>t.toolSearch).map(t=>t.wireName));
 let outcome='completed';
 const heartbeat=setInterval(()=>queue.push({type:'heartbeat'}),2000);
 void (async()=>{
  try{
   lane.resultCount=Math.max(lane.resultCount,parsed.context.messages.filter(m=>m.role==='toolResult').length);report.returnedNativeResults=[...lanes.values()].reduce((n,l)=>n+l.resultCount,0);const searches=parsed.context.messages.filter(m=>m.role==='toolResult'&&m.toolName==='tool_search');report.returnedNativeSearchResults=searches.length;report.nativeSearchResultErrors=searches.filter(m=>m.isError).length;report.nativeSearchLoadedTools=(Array.isArray(raw.input)?raw.input:[]).filter(m=>m.type==='tool_search_output').flatMap(m=>Array.isArray(m.tools)?m.tools:[]).reduce((n,t)=>n+(t.type==='namespace'&&Array.isArray(t.tools)?t.tools.length:1),0);save();
   const payload={instructions:[...(parsed.context.systemPrompt||[]),...outputInstructions],messages:parsed.context.messages,tools,compaction:compact,...returnTransport==='mcp'?{owner:{backend:statePath,thread:key,turn:identity.turnId}}:{}};
   await ensureDesktop(abort.signal);timing.mark('desktopReady');
   const response=await transport.exchange(payload,abort.signal,event=>{timing.mark('firstDispatchObserved');recordDesktopSubmission(report,event,route);save();},()=>{report.retryAttempts++;save();},event=>{recordDesktopContext(report,event);save();},route,event=>{if(event.type==='text_delta'&&event.text)timing.mark('firstPublicOutputObserved');queue.push(event);});
   report.selectedControl=transport.selectedControl;
   if(response.type==='tools'){
    if(compact)throw Error('desktop_compaction_must_not_call_tools');
    for(const call of response.calls){transcript.issue(call.id);report.modelRequestedTools++;const tool=tools.find(t=>t.wireName===call.name);if(tool?.toolSearch)report.modelRequestedSearches++;}
    queue.push({type:'done',endTurn:false,usage:desktopEstimatedUsage(payload,response)});
   }else if(response.type==='tool'){
    if(compact)throw Error('desktop_compaction_must_not_call_tools');
    const tool=validateEnvelope(response,tools);
    const callId='call_desktop_'+randomBytes(12).toString('hex');transcript.issue(callId);
    queue.push({type:'tool_call_start',id:callId,name:tool.wireName});
    queue.push({type:'tool_call_delta',arguments:tool.freeform?response.input:JSON.stringify(response.input)});queue.push({type:'tool_call_end'});report.modelRequestedTools++;if(tool.toolSearch)report.modelRequestedSearches++;
    queue.push({type:'done',endTurn:false,usage:desktopEstimatedUsage(payload,response)});
   }else if(response.type==='final'&&typeof response.text==='string'){
    if(compact){checkpoint.offer(response.text,parsed.context.messages);report.compactionSummaries++;}
    if(!response.streamed){if(response.text)timing.mark('firstPublicOutputObserved');queue.push({type:'text_delta',text:response.text,phase:'final_answer'});}queue.push({type:'done',endTurn:true,usage:desktopEstimatedUsage(payload,response)});report.completedReplies++;lane.active=false;
   }else throw Error('invalid_desktop_envelope');
   }catch(e){outcome=abort.signal.aborted?'cancelled':'failed';if(abort.signal.aborted)report.cancelledRequests++;lane.active=false;report.nativeFailure=transport.diagnostic;report.toolDiagnostic=transport.toolDiagnostic;
    // Content-free local diagnosis: never persist raw exception messages, paths,
    // requests or credentials when a standard error is outside our public codes.
    const frame=e.stack?.split('\n').map(f=>f.match(/\b(desktop_[a-z_]+\.mjs):(\d+):\d+/)).find(Boolean);
    report.failureSite={kind:['Error','TypeError','ReferenceError','RangeError','AbortError'].includes(e.name)?e.name:'unknown',frame:frame?frame[1]+':'+frame[2]:null};
    transport.retire();lane.transcript=new DesktopTranscript();lane.checkpoint=new DesktopCompaction(SUMMARY_PREFIX);report.errorCode=/^[a-z_]+$/.test(e.message)?e.message:'desktop_adapter_failed';queue.push({type:'error',message:report.errorCode,retryable:false});}
  finally{timing.finish(report,outcome);clearInterval(heartbeat);lane.busy=false;save();queue.close();}
 })();
 return new Response(markDesktopEstimatedUsage(bridgeToResponsesSSE(queue,raw.model,namespaceMap,freeform,toolSearch,()=>abort.abort(),2000)),{headers:{'Content-Type':'text/event-stream','Cache-Control':'no-store'}});
}});
const stat=readFileSync(`/proc/${process.pid}/stat`,'utf8');
const stateTemporary=statePath+'.'+randomBytes(6).toString('hex');
writeFileSync(stateTemporary,JSON.stringify({baseUrl:'http://127.0.0.1:'+server.port+'/v1',token,model,pid:process.pid,start:stat.slice(stat.lastIndexOf(')')+2).split(' ')[19],image:readlinkSync(`/proc/${process.pid}/exe`),returnTransport,...mcpSocket?{mcpSocket}:{}}),{mode:0o600,flag:'wx'});renameSync(stateTemporary,statePath);
save();
console.log(JSON.stringify({desktopResponsesReady:true,model,production:false}));
let stopping=false;
const stop=async()=>{if(stopping)return;stopping=true;server.stop(true);if(['local','mcp'].includes(returnTransport))await Promise.allSettled([...lanes.values()].map(l=>l.transport.close()));else{const {closeOwnedAppConversations}=await import('./desktop_app_transport.mjs');await closeOwnedAppConversations(desktopRoot);}if(stopMcp)await stopMcp();if(mcpRoot)rmSync(mcpRoot,{recursive:true});process.exit(0);};process.once('SIGTERM',()=>void stop());process.once('SIGINT',()=>void stop());
