// Transport-neutral projection. Core remains the only executor and history owner.
import {validateEnvelope} from './desktop_protocol.mjs';

export function localToolSignature(name){
 return {name,type:'kwargs',description:'Invoke one exact native Codex tool from the supplied catalog. The native Core executes it and returns the result. Never fabricate execution.',params:[
  {name:'name',required:true,type:{type:'string',description:'Exact wireName in the current native tool catalog.'}},
  {name:'input',required:true,type:{type:'string',description:'For a JSON tool: JSON-encoded arguments object. For a custom/freeform tool: its exact source string.'}},
  {name:'report',required:true,type:{type:'string',description:'Brief user-facing progress report before this tool, in the user language. Include the initial reply on the first call and useful intermediate findings later. Empty only when no useful update. Public prose only; never private reasoning.'}},
 ]};
}

export function localToolCall(message,gateway,tools){
 if(message.author?.role!=='assistant'||message.recipient!=='local.'+gateway||message.status!=='finished_successfully')return;
 let args;try{args=JSON.parse(message.content?.text);args=args.kwargs??args;}catch{throw Error('desktop_native_call_invalid');}
 if(typeof args.name!=='string'||typeof args.input!=='string'||typeof args.report!=='string')throw Error('desktop_native_call_invalid');
 const tool=tools.find(t=>t.wireName===args.name);
 if(!tool)throw Error('unadvertised_tool');
 let input=args.input;if(!tool.freeform)try{input=JSON.parse(input);}catch{throw Error('invalid_tool_input');}
 validateEnvelope({type:'tool',name:args.name,input},tools);
 return {id:message.id,name:args.name,input,report:args.report};
}

export class DesktopToolSession{
 constructor(){this.calls=new Map();this.messages=new Map();this.activeMessage=undefined;this.lastPublicText=undefined;}
 pending(){return [...this.calls.values()].filter(c=>c.result===undefined);}
 issue(call,tools,emit){
  if(typeof call.id!=='string'||!call.id)throw Error('desktop_native_call_invalid');
  const old=this.calls.get(call.id);
  if(old){if(JSON.stringify(old.call)!==JSON.stringify(call))throw Error('desktop_native_call_changed');return;}
  const tool=validateEnvelope({type:'tool',name:call.name,input:call.input},tools);
  if(typeof call.report==='string'&&call.report.trim()&&call.report.trim()!==this.lastPublicText?.trim()){
   this.text({id:'report:'+call.id,author:{role:'assistant'},recipient:'all',channel:'commentary',
    content:{content_type:'text',parts:[call.report]},status:'finished_successfully',end_turn:false},emit);
  }
  this.calls.set(call.id,{call});this.activeMessage=undefined;this.lastPublicText=undefined;
  emit({type:'tool_call_start',id:call.id,name:call.name});
  emit({type:'tool_call_delta',arguments:tool.freeform?call.input:JSON.stringify(call.input)});
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
}
