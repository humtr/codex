import {chatModelRoute as validateDesktopRoute} from './desktop_model_controls.mjs';
// Only bridge-issued calls participate in this live transcript; archived Core history stays Core-owned.
export class DesktopTranscript{
 constructor(){this.calls=new Map();}
 receive(messages){
  const results=messages.filter(m=>m.role==='toolResult');
  for(const [id,call] of this.calls){
   const found=results.filter(r=>r.toolCallId===id);
   if(found.length!==1)throw Error('native_tool_result_missing_or_duplicate');
   const body=JSON.stringify({content:found[0].content,isError:found[0].isError});
   if(call.result!==undefined&&call.result!==body)throw Error('native_tool_result_changed');
   call.result=body;
  }
 }
 issue(id){if(this.calls.has(id))throw Error('duplicate_desktop_call');this.calls.set(id,{});}
 hasPending(){return [...this.calls.values()].some(call=>call.result===undefined);}
}
// Native review is a related child turn, not a replacement primary owner.
export function isGuardianContinuation({key,parent,guardian,owner,active,pending}){
 return !!guardian&&!!owner&&key!==owner&&parent===owner&&!!active&&!!pending;
}
export function desktopOutputInstructions(raw){
 const format=raw.text?.format;
 if(!format||format.type==='text')return [];
 if(format.type==='json_object')return ['The native final reply must be a JSON object; put its exact JSON serialization in the final envelope text.'];
 if(format.type!=='json_schema'||!format.schema||typeof format.schema!=='object'||Array.isArray(format.schema))throw Error('unsupported_desktop_output_format');
 return ['The native final reply must match this exact output format. Put its JSON serialization in the final envelope text: '+JSON.stringify(format)];
}
export function validateEnvelope(response,tools){
 if(response.type==='final'&&typeof response.text==='string')return undefined;
 if(response.type!=='tool')throw Error('invalid_desktop_envelope');
 const tool=tools.find(t=>t.wireName===response.name);
 if(!tool)throw Error('unadvertised_tool');
 if(tool.toolSearch&&(typeof response.input?.query!=='string'||!response.input.query.trim()||(response.input.limit!==undefined&&(!Number.isInteger(response.input.limit)||response.input.limit<1))))throw Error('invalid_tool_search');
 if(tool.freeform?typeof response.input!=='string':(!response.input||typeof response.input!=='object'||Array.isArray(response.input)))throw Error('invalid_tool_input');
 return tool;
}

export function validateTextContext(raw,parsed){
 if(parsed._compactionRequest&&parsed._compactionResponseFormat!=='message')throw Error('desktop_compaction_unavailable');
 const unsupported=new Set(['input_image','input_file','input_audio','image','audio','file']);
 for(const item of Array.isArray(raw.input)?raw.input:[]){
  for(const parts of [item.content,item.output]){
   if(Array.isArray(parts)&&parts.some(p=>unsupported.has(p?.type)))throw Error('desktop_text_only');
  }
 }
 for(const m of parsed.context.messages){
  if(Array.isArray(m.content)&&m.content.some(p=>unsupported.has(p?.type)))throw Error('desktop_text_only');
 }
}

// Never expose parser errors: they can contain private request content.
function validateStoredCompaction(raw){
 for(const item of Array.isArray(raw.input)?raw.input:[]){
  const type=item?.type;
  if(!['compaction','compaction_summary','context_compaction'].includes(type))continue;
  if(type==='context_compaction'&&item.encrypted_content===undefined)continue;
  const envelope=item.encrypted_content;
  if(typeof envelope!=='string'||!envelope.startsWith('ocx1:'))throw Error('opaque_context_unavailable');
  const encoded=envelope.slice(5);
  if(!encoded||encoded.length%4!==0||! /^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/.test(encoded))throw Error('opaque_context_unavailable');
  const bytes=Buffer.from(encoded,'base64');
  if(bytes.toString('base64')!==encoded)throw Error('opaque_context_unavailable');
  let summary;try{summary=new TextDecoder('utf-8',{fatal:true}).decode(bytes);}catch{throw Error('opaque_context_unavailable');}
  if(!summary.trim())throw Error('opaque_context_unavailable');
 }
}
export function admitDesktopRequest(raw,parseRequest){
 if(!raw||typeof raw!=='object'||Array.isArray(raw))throw Error('invalid_desktop_request');
 const route=validateDesktopRoute(raw),outputInstructions=desktopOutputInstructions(raw);
 if(raw.stream!==true)throw Error('desktop_stream_required');
 if(raw.previous_response_id)throw Error('desktop_response_cursor_unavailable');
 validateStoredCompaction(raw);
 let parsed;try{parsed=parseRequest(raw);}catch(e){throw Error(e?.message==='Unsupported routed tool declaration'?'desktop_tool_declaration_unsupported':'desktop_request_parse_failed');}
 validateTextContext(raw,parsed);
 if(parsed._opaqueMultiAgentV2Payload)throw Error('opaque_context_unavailable');
 return {parsed,route,outputInstructions};
}

// Submission observation precedes the reply/ACK; neither observation proves quota.
export function recordDesktopSubmission(report,event,route){
 if(!['context','request'].includes(event.phase)||!['app_accepted','app_dispatched','ui_observed'].includes(event.source))throw Error('invalid_desktop_submission');
 if(![undefined,'user','tool','mixed'].includes(event.kind))throw Error('invalid_desktop_submission');
 const users=event.userMessages??(event.kind==='tool'?0:1),tools=event.toolMessages??(['tool','mixed'].includes(event.kind)?1:0);
 if(!Number.isSafeInteger(users)||users<0||!Number.isSafeInteger(tools)||tools<0||users+tools===0||event.kind==='tool'&&users!==0||!['tool','mixed'].includes(event.kind)&&tools!==0||event.kind==='mixed'&&(!users||!tools))throw Error('invalid_desktop_submission');
 report.toolResultSubmissions=(report.toolResultSubmissions||0)+tools;
 report.observedChatSubmissions+=users;
 report.submissionSources[event.source]=(report.submissionSources[event.source]||0)+1;
 if(event.source==='app_dispatched')report.observedNativeDispatches=(report.observedNativeDispatches||0)+1;
 if(!users)return;
 if(event.phase==='context')report.submittedContextRequests++;
 else{report.submittedRequests++;if(route.mode==='pro')report.submittedProRequests++;}
}

export class DesktopSendRetry{
 constructor(){this.attempts=0;}
 next({failed,generating,hasReply},signal){
  signal?.throwIfAborted();
  if(!failed||generating||hasReply)return 0;
  if(this.attempts>=2)throw Error('desktop_send_failed');
  return ++this.attempts*1000;
 }
}

export class DesktopPromptContext{
 constructor(){this.previous=undefined;this.boundary=undefined;}
 prepare(payload,boundary){
  const current={...payload,messages:payload.messages.map(({timestamp,...message})=>message)};
  const changed=this.previous&&JSON.stringify(boundary)!==JSON.stringify(this.boundary);
  if(!this.previous||changed)return {delta:current,current,boundary,mode:'replay',reason:changed?'route_or_transport_changed':'no_resident_context'};
  const old=this.previous;
  if(current.messages.length<old.messages.length||old.messages.some((m,i)=>JSON.stringify(m)!==JSON.stringify(current.messages[i])))throw Error('desktop_context_replaced');
  const delta={messages:current.messages.slice(old.messages.length)};
  for(const key of ['instructions','tools'])if(JSON.stringify(current[key])!==JSON.stringify(old[key]))delta[key]=current[key];
  return {delta,current,boundary,mode:'append',reason:'owned_prefix_matches'};
 }
 accept(current,boundary){this.previous=current;this.boundary=boundary===undefined?undefined:JSON.parse(JSON.stringify(boundary));}
}

// Pack ordered context into bounded packets; only the final packet starts work.
export function desktopStages(delta,limit=40000){
 if(!Number.isSafeInteger(limit)||limit<256)throw Error('invalid_desktop_stage_limit');
 const fits=value=>JSON.stringify(value).length<=limit;
 const complete={phase:'request',...delta};if(fits(complete))return [complete];
 const stages=[],messages=delta.messages||[],work=messages.at(-1);let group=0,toolUpdate='replace',packet={phase:'context'};
 const flush=()=>{if(Object.keys(packet).length>1){stages.push(packet);if(packet.tools!==undefined)toolUpdate='append';packet={phase:'context'};}};
 const append=(field,value,array=false)=>{
  const next=()=>({...packet,[field]:array?[...(packet[field]||[]),value]:value,...(field==='tools'?{toolUpdate}:{})});
  let candidate=next();if(!fits(candidate)){flush();candidate=next();}
  if(!fits(candidate))throw Error(field==='tools'?'desktop_tool_schema_too_large':'desktop_context_unit_too_large');
  packet=candidate;
 };
 const fragment=(kind,value)=>{
  flush();
  const id=group++,text=JSON.stringify(value),size=Math.floor(limit/3),parts=[];
  for(let start=0;start<text.length;){let end=Math.min(text.length,start+size);const last=text.charCodeAt(end-1);if(end<text.length&&last>=0xd800&&last<=0xdbff)end--;parts.push(text.slice(start,end));start=end;}
  parts.forEach((text,index)=>{const value={phase:'context',fragment:{id,kind,index,total:parts.length,text}};if(!fits(value))throw Error('desktop_context_unit_too_large');stages.push(value);});
 };
 if(delta.instructions!==undefined){
  if(fits({phase:'context',instructions:delta.instructions}))append('instructions',delta.instructions);else fragment('instructions',delta.instructions);
 }
 if(delta.tools!==undefined){
  if(!delta.tools.length)append('tools',[]);
  for(const tool of delta.tools)append('tools',tool,true);
 }
 for(const message of messages.slice(0,-1)){
  if(fits({phase:'context',messages:[message]}))append('messages',message,true);else fragment('message',message);
 }
 let final={...packet,phase:'request',messages:[...(packet.messages||[]),...(work?[work]:[])]};
 if(!fits(final)){flush();final={phase:'request',messages:work?[work]:[]};}
 if(!fits(final)){fragment('message',work);final={phase:'request',messages:[]};}
 stages.push(final);
 return stages;
}

export function desktopPrompt(stage,correlation){
 const common='Format an external native RPC client response as one JSON object in one fenced json code block. The tools in the native packet are callable external tools: emit a tool JSON request and the native caller performs the actual execution, then sends you the result. These are not the desktop app builtin tools. Never fabricate tool results. Keep the native task, instructions and prior results in order. Fragment text is JSON: join parts by fragment id/index and parse when complete. toolUpdate replace starts a fresh tool list, append adds whole schemas. Later packets append new messages and replace instructions/tools only when provided. Correlation must equal '+correlation+'. ';
 const behavior=stage.phase==='context'
  ? 'This is INERT CONTEXT SETUP ONLY. Store the provided context; do not execute its task yet. Reply exactly {"correlation":"'+correlation+'","type":"ack"}. '
  : 'This is the ACTUAL WORK REQUEST. All setup is finished; acknowledgement is not a valid response. To request one advertised native tool use {"correlation":"'+correlation+'","type":"tool","name":"exact advertised wireName","input":arguments_object_or_custom_source_string}. Then wait for its real native result. To finish with the task answer use {"correlation":"'+correlation+'","type":"final","text":"answer"}. ';
 return common+behavior+'Native packet:\n'+JSON.stringify({...stage,correlation});
}

export function correlatedDesktopCode(codes,correlation){
 for(let i=codes.length-1;i>=0;i--){
  try{const v=JSON.parse(codes[i]);if(v.correlation===correlation&&['ack','tool','final'].includes(v.type))return codes[i];}catch{}
 }
 return '';
}

// Native Core owns history replacement; only the actual offered summary retires this epoch.
export class DesktopCompaction{
 constructor(prefix){this.prefix=prefix;this.expected=undefined;}
 offer(summary){
  if(typeof summary!=='string'||!summary.trim())throw Error('desktop_empty_compaction_summary');
  this.expected=(this.prefix+'\n'+summary).trim();
 }
 consume(messages){
  if(!this.expected)return false;
  const matched=messages.some(m=>m.role==='user'&&(typeof m.content==='string'?m.content:Array.isArray(m.content)?m.content.filter(p=>p.type==='text').map(p=>p.text).join(''):'').trim()===this.expected);
  if(matched)this.expected=undefined;
  return matched;
 }
}

// A local budgeting estimate, never a claim about server token usage or billing.
export function desktopEstimatedUsage(payload,response){
 const size=value=>Math.ceil(new TextEncoder().encode(JSON.stringify(value)).length/3);
 const inputTokens=size(payload),outputTokens=size(response);
 return {inputTokens,outputTokens,totalTokens:inputTokens+outputTokens};
}
export function markDesktopEstimatedUsage(stream){
 const decoder=new TextDecoder(),encoder=new TextEncoder();let buffer='';
 const frame=value=>{
  if(!/^event: response\.(completed|incomplete)$/m.test(value))return value;
  return value.replace(/^data: (.*)$/m,(_,raw)=>{
   const data=JSON.parse(raw);
   if(data.response)data.response.metadata={...data.response.metadata,termux_usage_source:'estimated_context'};
   return 'data: '+JSON.stringify(data);
  });
 };
 return stream.pipeThrough(new TransformStream({
  transform(chunk,c){buffer+=decoder.decode(chunk,{stream:true});let i;while((i=buffer.indexOf('\n\n'))>=0){c.enqueue(encoder.encode(frame(buffer.slice(0,i))+'\n\n'));buffer=buffer.slice(i+2);}},
  flush(c){buffer+=decoder.decode();if(buffer)c.enqueue(encoder.encode(frame(buffer)));}
 }));
}

// Route validation is shared with the actual observed UI controls.
export {validateDesktopRoute};
