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
 constructor(){this.previous=undefined;}
 prepare(payload){
  const current={...payload,messages:payload.messages.map(({timestamp,...message})=>message)};
  if(!this.previous)return {delta:current,current};
  const old=this.previous;
  if(current.messages.length<old.messages.length||old.messages.some((m,i)=>JSON.stringify(m)!==JSON.stringify(current.messages[i])))throw Error('desktop_context_replaced');
  const delta={messages:current.messages.slice(old.messages.length)};
  for(const key of ['instructions','tools'])if(JSON.stringify(current[key])!==JSON.stringify(old[key]))delta[key]=current[key];
  return {delta,current};
 }
 accept(current){this.previous=current;}
}

// Context is inert until the final request; schemas remain whole and ordered.
export function desktopStages(delta,limit=40000){
 if(JSON.stringify(delta).length<=limit)return [{phase:'request',...delta}];
 const stages=[],messages=delta.messages||[],work=messages.at(-1);let group=0;
 const add=value=>{if(JSON.stringify(value).length>limit)throw Error('desktop_context_unit_too_large');stages.push({phase:'context',...value});};
 const fragment=(kind,value)=>{
  const id=group++,text=JSON.stringify(value),size=Math.floor(limit/3),parts=[];
  for(let start=0;start<text.length;){let end=Math.min(text.length,start+size);const last=text.charCodeAt(end-1);if(end<text.length&&last>=0xd800&&last<=0xdbff)end--;parts.push(text.slice(start,end));start=end;}
  parts.forEach((text,index)=>add({fragment:{id,kind,index,total:parts.length,text}}));
 };
 if(delta.instructions!==undefined){
  if(JSON.stringify({instructions:delta.instructions}).length<=limit)add({instructions:delta.instructions});else fragment('instructions',delta.instructions);
 }
 if(delta.tools!==undefined){
  let batch=[];add({tools:[],toolUpdate:'replace'});
  for(const tool of delta.tools){
   if(JSON.stringify({tools:[tool]}).length>limit)throw Error('desktop_tool_schema_too_large');
   if(JSON.stringify({tools:[...batch,tool]}).length>limit){add({tools:batch,toolUpdate:'append'});batch=[];}
   batch.push(tool);
  }
  if(batch.length)add({tools:batch,toolUpdate:'append'});
 }
 for(const message of messages.slice(0,-1)){
  if(JSON.stringify({messages:[message]}).length<=limit)add({messages:[message]});else fragment('message',message);
 }
 if(work&&JSON.stringify({messages:[work]}).length<=limit)stages.push({phase:'request',messages:[work]});
 else{if(work)fragment('message',work);stages.push({phase:'request',messages:[]});}
 return stages;
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
