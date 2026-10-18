import {test} from 'node:test';
import assert from 'node:assert/strict';
import {parseNativeToolHistory} from './desktop_protocol.mjs';

const parse=raw=>({context:{messages:[{role:'assistant',content:raw.input.filter(i=>i.type==='function_call').map(i=>({type:'toolCall',id:i.call_id,name:i.name,namespace:i.namespace,arguments:JSON.parse(i.arguments)}))},...raw.input.filter(i=>i.type==='function_call_output').map(i=>({role:'toolResult',toolCallId:i.call_id,content:i.output}))]}});
test('canonical history preserves malformed function bytes, IDs, namespaces and exact Core results without mutating native input',()=>{
 for(const value of ['{"cmd":','[1,2]','null','false','42','"string"',' \n']){
  const raw={input:[{type:'function_call',call_id:'owned',name:'run',namespace:'functions',arguments:value},{type:'function_call_output',call_id:'owned',output:'actual Core argument error'}]};
  const before=JSON.stringify(raw),parsed=parseNativeToolHistory(raw,parse);
  assert.deepEqual(parsed.context.messages[0].content[0],{type:'toolCall',id:'owned',name:'run',namespace:'functions',arguments:value});
  assert.deepEqual(parsed.context.messages[1],{role:'toolResult',toolCallId:'owned',content:'actual Core argument error'});
  assert.equal(JSON.stringify(raw),before);
 }
});
test('valid object history and all other parser validation remain authoritative',()=>{
 const raw={input:[{type:'function_call',call_id:'ok',name:'run',arguments:' {"cmd":"pwd"} '}]};
 const parsed=parseNativeToolHistory(raw,source=>{assert.equal(source,raw);return parse(source);});
 assert.deepEqual(parsed.context.messages[0].content[0].arguments,{cmd:'pwd'});
 const rejected={input:[{type:'function_call',call_id:'bad',name:'run',arguments:'malformed'}]};
 assert.throws(()=>parseNativeToolHistory(rejected,()=>{throw Error('orphan or duplicate call result');}),/orphan or duplicate/);
 assert.throws(()=>parseNativeToolHistory(rejected,()=>({context:{messages:[]}})),/invalid_desktop_history/);
 assert.throws(()=>parseNativeToolHistory({input:[...rejected.input,...rejected.input]},parse),/invalid_desktop_history/);
});
