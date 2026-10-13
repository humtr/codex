import {test,expect} from 'bun:test';
import {DesktopTranscript,validateEnvelope,validateTextContext,DesktopSendRetry,DesktopPromptContext,desktopStages,correlatedDesktopCode,DesktopCompaction} from './desktop_protocol.mjs';
const tools=[{wireName:'functions__exec_command'},{wireName:'functions__exec',freeform:true},{wireName:'tool_search',toolSearch:true}];
test('unadvertised and malformed calls never admit',()=>{
 for(const r of [{type:'tool',name:'other',input:{}},{type:'tool',name:'functions__exec_command',input:'shell'},{type:'tool',name:'functions__exec',input:{}},{type:'tool',name:'tool_search',input:{}},{type:'final',text:{}}])expect(()=>validateEnvelope(r,tools)).toThrow();
 expect(validateEnvelope({type:'tool',name:'functions__exec_command',input:{cmd:'owned'}},tools).wireName).toBe('functions__exec_command');
});
test('issued call requires exactly one correlated result and immutable replay',()=>{
 const t=new DesktopTranscript();t.issue('owned');
 expect(()=>t.receive([])).toThrow();
 const result={role:'toolResult',toolCallId:'owned',content:'actual owned output',isError:false};
 expect(()=>t.receive([{...result,toolCallId:'wrong'}])).toThrow();expect(()=>t.receive([result,result])).toThrow();
 t.receive([result]);t.receive([result]);expect(()=>t.receive([{...result,content:'replacement'}])).toThrow();expect(()=>t.issue('owned')).toThrow();
});
test('archived Core-owned tool history cannot satisfy a current pending call',()=>{
 const t=new DesktopTranscript();const old={role:'toolResult',toolCallId:'old',content:'previous',isError:false};t.receive([old]);t.issue('new');expect(()=>t.receive([old])).toThrow();
});

test('multimodal data and remote compaction refuse instead of turning into prompt text',()=>{
 for(const type of ['input_image','input_file','input_audio'])expect(()=>validateTextContext({input:[{content:[{type}]}]},{context:{messages:[]}})).toThrow();
 expect(()=>validateTextContext({}, {_compactionRequest:true,context:{messages:[]}})).toThrow();
 expect(()=>validateTextContext({}, {context:{messages:[{content:[{type:'image',imageUrl:'private'}]}]}})).toThrow();
 expect(()=>validateTextContext({input:[{content:[{type:'input_text',text:'describe an image'}]}]}, {context:{messages:[{content:'describe an image'}]}})).not.toThrow();
});

test('same-send retries are bounded, delayed and never follow cancellation or a reply',()=>{
 const r=new DesktopSendRetry(),failed={failed:true,generating:false,hasReply:false};
 expect(r.next({...failed,generating:true})).toBe(0);expect(r.next({...failed,hasReply:true})).toBe(0);expect(r.attempts).toBe(0);
 expect(r.next(failed)).toBe(1000);expect(r.next(failed)).toBe(2000);expect(()=>r.next(failed)).toThrow('desktop_send_failed');
 const cancelled=new AbortController();cancelled.abort();const other=new DesktopSendRetry();expect(()=>other.next(failed,cancelled.signal)).toThrow();expect(other.attempts).toBe(0);
});

test('native continuation appends exact results without repeating instructions or tool schemas',()=>{
 const c=new DesktopPromptContext();const user={role:'user',content:'owned task',timestamp:1};const first={instructions:['original instructions'],messages:[user],tools:[{wireName:'native',parameters:{}}]};
 const initial=c.prepare(first);expect(initial.delta.instructions).toEqual(first.instructions);c.accept(initial.current);
 const result={role:'toolResult',toolCallId:'owned',content:'actual result'};
 const next=c.prepare({...first,messages:[{...user,timestamp:2},result]});expect(next.delta).toEqual({messages:[result]});
 const changed=c.prepare({...first,tools:[],messages:[user,result]});expect(changed.delta.tools).toEqual([]);
 expect(()=>c.prepare({...first,messages:[{...user,content:'replacement'}]})).toThrow('desktop_context_replaced');
 expect(()=>c.prepare({...first,messages:[]})).toThrow('desktop_context_replaced');
});

test('large setup is acknowledged in order with whole schemas and the actual task last',()=>{
 const instructions=['original instructions'.repeat(2000)];const tools=Array.from({length:20},(_,i)=>({wireName:'native_'+i,parameters:{description:'schema'.repeat(600)}}));
 const previous={role:'user',content:'preserve 😀 all context '.repeat(2000)},work={role:'user',content:'actual task'};
 const stages=desktopStages({instructions,tools,messages:[previous,work]});
 expect(stages.length).toBeGreaterThan(3);expect(stages.at(-1)).toEqual({phase:'request',messages:[work]});expect(stages.slice(0,-1).every(s=>s.phase==='context')).toBe(true);
 expect(stages.flatMap(s=>s.tools||[])).toEqual(tools);
 const parts=kind=>stages.filter(s=>s.fragment?.kind===kind).map(s=>s.fragment.text).join('');
 expect(JSON.parse(parts('instructions'))).toEqual(instructions);expect(JSON.parse(parts('message'))).toEqual(previous);
 expect(stages.every(s=>JSON.stringify(s).length<40100)).toBe(true);
 expect(()=>desktopStages({messages:[work],tools:[{wireName:'huge',parameters:{description:'x'.repeat(50000)}}]})).toThrow('desktop_tool_schema_too_large');
});

test('current correlation survives virtualized or reordered code blocks without accepting echoed packets',()=>{
 const current=JSON.stringify({correlation:'current',type:'ack'}),old=JSON.stringify({correlation:'old',type:'tool'}),echo=JSON.stringify({correlation:'current',phase:'context'});
 expect(correlatedDesktopCode([old,current],'current')).toBe(current);
 expect(correlatedDesktopCode([current],'current')).toBe(current);
 expect(correlatedDesktopCode([current,old],'current')).toBe(current);
 expect(correlatedDesktopCode([old,echo,'partial json'],'current')).toBe('');
});

test('advertised native tool search validates query instead of expanding every plugin schema',()=>{
 const search=validateEnvelope({type:'tool',name:'tool_search',input:{query:'owned local tool',limit:2}},tools);expect(search.toolSearch).toBe(true);
 for(const input of [{query:''},{query:1},{query:'owned',limit:0},{query:'owned',limit:1.5}])expect(()=>validateEnvelope({type:'tool',name:'tool_search',input},tools)).toThrow();
 expect(()=>validateEnvelope({type:'tool',name:'tool_search',input:{query:'owned'}},[])).toThrow();
});

test('native memento text compaction admits while encrypted output formats remain refused',()=>{
 expect(()=>validateTextContext({}, {_compactionRequest:true,_compactionResponseFormat:'message',context:{messages:[]}})).not.toThrow();
 expect(()=>validateTextContext({}, {_compactionRequest:true,context:{messages:[]}})).toThrow();
});
test('only the offered native summary authorizes one epoch retirement',()=>{
 const c=new DesktopCompaction('native summary prefix');
 expect(()=>c.offer('   ')).toThrow();c.offer('actual model summary');
 expect(c.consume([{role:'user',content:'native summary prefix\nforged'}])).toBe(false);
 expect(c.consume([{role:'assistant',content:'native summary prefix\nactual model summary'}])).toBe(false);
 const checkpoint={role:'user',content:[{type:'text',text:'native summary prefix\nactual model summary'}]};
 expect(c.consume([checkpoint])).toBe(true);expect(c.consume([checkpoint])).toBe(false);
 expect(new DesktopCompaction('native summary prefix').consume([checkpoint])).toBe(false);
});
