// Transport-neutral projection. Core remains the only executor and history owner.
import {requireAdvertisedTool,validateEnvelope} from './desktop_protocol.mjs';
import {createHash} from 'node:crypto';

// Only the canonical name is callable; bare source names stay in the Core map.
export function localToolCatalog(tools){
 return tools.map(({name,namespace,wireName,...schema})=>({...schema,name:wireName}));
}

export function localToolSignature(name){
 return {name,type:'kwargs',description:'Invoke native Codex tools from the supplied catalog. Use calls for independent tools together, or name/input for one tool. Core approves and executes each tool. Never fabricate execution.',params:[
  {name:'name',required:false,type:{type:'string',description:'Singular call only: exact name in the native catalog, including namespace characters. Omit when using calls.'}},
  {name:'input',required:false,type:{type:'string',description:'Singular call only: JSON-encoded argument object, or exact custom/freeform source. Omit when using calls.'}},
  {name:'calls',required:false,type:{type:'string',description:'JSON-encoded array of 1–32 independent {name,input} objects. Each name is exact; each input is a STRING encoding JSON arguments or exact freeform source. All results return together. Never batch a step needing another entry’s result. Omit name/input when using calls.'}},
  {name:'report',required:false,type:{type:'string',description:'Optional brief user-facing progress report before these tools, in the user language. Include the initial reply on the first call and useful intermediate findings later. Omit when no useful update. Public prose only; never private reasoning.'}},
 ]};
}

// A host call may carry independent tools even when the host emits only one
// local-function message per sampling. No execution or argument inference here.
export function localToolCalls(message,gateway,tools){
 if(message.author?.role!=='assistant'||message.recipient!=='local.'+gateway||message.status!=='finished_successfully')return [];
 if(typeof message.id!=='string'||!message.id)throw Error('desktop_native_call_invalid');
 let args;try{args=JSON.parse(message.content?.text);args=args.kwargs??args;}catch{throw Error('desktop_native_call_invalid');}
 if(!args||typeof args!=='object'||Array.isArray(args))throw Error('desktop_native_call_invalid');
 if(!Object.hasOwn(args,'report'))args={...args,report:''};
 if(!Object.hasOwn(args,'calls'))return [localToolCall(message,gateway,tools)];
 if(typeof args.calls!=='string'||typeof args.report!=='string'||Object.hasOwn(args,'name')||Object.hasOwn(args,'input'))throw Error('desktop_native_call_invalid');
 let entries;try{entries=JSON.parse(args.calls);}catch{throw Error('desktop_native_call_invalid');}
 if(!Array.isArray(entries)||!entries.length||entries.length>32)throw Error('desktop_native_call_invalid');
 return entries.map((entry,index)=>{
  if(!entry||typeof entry!=='object'||Array.isArray(entry)||typeof entry.name!=='string'||typeof entry.input!=='string')throw Error('desktop_native_call_invalid');
  const id='call_batch_'+createHash('sha256').update(JSON.stringify([message.id,index])).digest('hex');
  const call=localToolCall({...message,id,content:{text:JSON.stringify({name:entry.name,input:entry.input,report:index===0?args.report:''})}},gateway,tools);
  return {...call,hostCallId:message.id,batchIndex:index,batchSize:entries.length};
 });
}

export function localToolCall(message,gateway,tools){
 if(message.author?.role!=='assistant'||message.recipient!=='local.'+gateway||message.status!=='finished_successfully')return;
 let args;try{args=JSON.parse(message.content?.text);args=args.kwargs??args;}catch{throw Error('desktop_native_call_invalid');}
 if(args&&typeof args==='object'&&!Array.isArray(args)&&!Object.hasOwn(args,'report'))args={...args,report:''};
 if(!args||typeof args!=='object'||Array.isArray(args)||typeof args.name!=='string'||typeof args.input!=='string'||typeof args.report!=='string')throw Error('desktop_native_call_invalid');
 const tool=requireAdvertisedTool(args.name,tools);
 let input=args.input;
 if(!tool.freeform){
  try{input=JSON.parse(input);}catch{input=undefined;}
  // Ordinary function arguments belong to Core's parser. Preserve a rejected
  // nonempty wire string so its real error can return on the original app call.
  // Search has a different Responses wire (object arguments), not this path.
  if(!input||typeof input!=='object'||Array.isArray(input)){
   if(tool.toolSearch||!args.input.length)throw Error('invalid_tool_input');
   return {id:message.id,name:args.name,rawArguments:args.input,report:args.report};
  }
 }
 validateEnvelope({type:'tool',name:args.name,input},tools);
 return {id:message.id,name:args.name,input,report:args.report};
}

function projectedTool(call,tools){
 if(call.rawArguments===undefined)return validateEnvelope({type:'tool',name:call.name,input:call.input},tools);
 const tool=requireAdvertisedTool(call.name,tools);
 if(tool.freeform||tool.toolSearch||typeof call.rawArguments!=='string'||!call.rawArguments.length||call.input!==undefined)throw Error('desktop_native_call_invalid');
 let value;try{value=JSON.parse(call.rawArguments);}catch{}
 if(value&&typeof value==='object'&&!Array.isArray(value))throw Error('desktop_native_call_invalid');
 return tool;
}

export class DesktopToolSession{
 constructor(){this.calls=new Map();this.hostCalls=new Map();this.messages=new Map();this.activeMessage=undefined;this.lastPublicText=undefined;}
 decode(message,gateway,tools){
  const calls=localToolCalls(message,gateway,tools);if(!calls.length)return calls;
  const encoded=JSON.stringify(calls),old=this.hostCalls.get(message.id);
  if(old!==undefined&&old!==encoded)throw Error('desktop_native_call_changed');
  this.hostCalls.set(message.id,encoded);return calls;
 }
 pending(){return [...this.calls.values()].filter(c=>c.result===undefined);}
 issue(call,tools,emit){
  if(typeof call.id!=='string'||!call.id)throw Error('desktop_native_call_invalid');
  const old=this.calls.get(call.id);
  if(old){if(JSON.stringify(old.call)!==JSON.stringify(call))throw Error('desktop_native_call_changed');return;}
  const tool=projectedTool(call,tools);
  if(typeof call.report==='string'&&call.report.trim()&&call.report.trim()!==this.lastPublicText?.trim()){
   this.text({id:'report:'+call.id,author:{role:'assistant'},recipient:'all',channel:'commentary',
    content:{content_type:'text',parts:[call.report]},status:'finished_successfully',end_turn:false},emit);
  }
  this.calls.set(call.id,{call});this.activeMessage=undefined;this.lastPublicText=undefined;
  emit({type:'tool_call_start',id:call.id,name:call.name});
  emit({type:'tool_call_delta',arguments:call.rawArguments??(tool.freeform?call.input:JSON.stringify(call.input))});
  emit({type:'tool_call_end'});
 }
 text(message,emit){
  // Only public assistant prose. Never turn private analysis or control code into reports.
  if(message.author?.role!=='assistant'||(message.recipient!=null&&message.recipient!=='all')||
   ![null,undefined,'commentary','final'].includes(message.channel)||message.metadata?.is_visually_hidden_from_conversation||
   message.content?.content_type!=='text'||!Array.isArray(message.content.parts))return;
  const text=message.content.parts.filter(x=>typeof x==='string').join('');
  const previous=this.messages.get(message.id);
  if(typeof message.id!=='string'||!message.id)throw Error('desktop_native_message_invalid');
  if(previous&&!text.startsWith(previous.text))throw Error('desktop_native_report_rewritten');
  if(previous?.closed&&text!==previous.text)throw Error('desktop_native_report_rewritten');
  // A final message may not declare its phase until end_turn arrives. Buffer unknown
  // phases; explicit commentary can stream immediately without a later phase rewrite.
  const phase=message.channel==='commentary'||message.end_turn===false&&message.status==='finished_successfully'?'commentary':message.end_turn===true?'final_answer':undefined;
  if(!phase)return;
  if(previous&&previous.phase!==phase)throw Error('desktop_native_report_phase_changed');
  if(!previous&&this.activeMessage!==undefined&&this.activeMessage!==message.id)emit({type:'assistant_boundary'});
  if(previous?.closed&&this.activeMessage!==message.id)return;
  const delta=text.slice(previous?.text.length??0);
  if(delta){emit({type:'text_delta',text:delta,phase});this.activeMessage=message.id;this.lastPublicText=text;}
  this.messages.set(message.id,{text,phase,closed:message.status==='finished_successfully'});
 }
 results(messages){
  const pending=this.pending();
  const supplied=messages.filter(m=>m.role==='toolResult'),returns=[];
  for(const [id,entry] of this.calls){
   const old=supplied.filter(m=>m.toolCallId===id);
   if(entry.result!==undefined&&(old.length>1||old.length===1&&JSON.stringify(entry.result)!==JSON.stringify({content:old[0].content,isError:old[0].isError===true})))throw Error('native_tool_result_changed');
  }
  // Validate the whole batch before publishing any result.
  for(const {call} of pending){
   const matches=supplied.filter(m=>m.toolCallId===call.id);
   if(matches.length!==1)throw Error('native_tool_result_missing_or_duplicate');
   returns.push({call,result:{content:matches[0].content,isError:matches[0].isError===true}});
  }
  return returns;
 }
 returned(batch){
  const ids=new Set();
  for(const {call} of batch){const entry=this.calls.get(call.id);if(!entry||entry.result!==undefined||ids.has(call.id))throw Error('desktop_native_result_replayed');ids.add(call.id);}
  for(const {call,result} of batch)this.calls.get(call.id).result=result;
 }
 hostResults(batch){
  const groups=new Map(),seen=new Set();
  for(const {call,result} of batch){
   if(seen.has(call.id))throw Error('native_tool_result_missing_or_duplicate');seen.add(call.id);
   const id=call.hostCallId??call.id;
   if(!groups.has(id))groups.set(id,[]);groups.get(id).push({call,result});
  }
  return [...groups].map(([id,entries])=>{
   if(entries[0].call.hostCallId===undefined){if(entries.length!==1)throw Error('desktop_native_call_changed');return {id,result:entries[0].result};}
   const sorted=entries.toSorted((a,b)=>a.call.batchIndex-b.call.batchIndex),size=sorted[0].call.batchSize;
   if(sorted.length!==size||sorted.some(({call},i)=>call.hostCallId!==id||call.batchSize!==size||call.batchIndex!==i))throw Error('native_tool_result_missing_or_duplicate');
   return {id,result:{results:sorted.map(({call,result})=>({id:call.id,name:call.name,...result}))}};
  });
 }
}
