import {desktopConnection} from './desktop_cdp.mjs';
import {mkdtempSync,writeFileSync,readFileSync,readlinkSync,rmSync} from 'node:fs';
import {join} from 'node:path';
import {test,expect} from 'bun:test';
import {DesktopTranscript,validateEnvelope,validateTextContext,DesktopSendRetry,DesktopPromptContext,desktopStages,correlatedDesktopCode,DesktopCompaction,desktopEstimatedUsage,markDesktopEstimatedUsage,validateDesktopRoute} from './desktop_protocol.mjs';
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

test('missing server usage gets a nonzero local estimate with explicit provenance',async()=>{
 const usage=desktopEstimatedUsage({instructions:['owned'],messages:[{role:'user',content:'한글 😀'.repeat(20)}],tools:[]},{type:'final',text:'owned'});
 expect(usage.inputTokens).toBeGreaterThan(50);expect(usage.outputTokens).toBeGreaterThan(0);expect(usage.totalTokens).toBe(usage.inputTokens+usage.outputTokens);
 const body='event: response.output_text.delta\ndata: {"delta":"unchanged"}\n\nevent: response.completed\ndata: {"response":{"usage":{"input_tokens":10},"metadata":{"retained":"owned"}}}\n\n';
 const bytes=new TextEncoder().encode(body),stream=new ReadableStream({start(c){for(const b of bytes)c.enqueue(new Uint8Array([b]));c.close();}});
 const result=await new Response(markDesktopEstimatedUsage(stream)).text();
 expect(result).toContain('data: {"delta":"unchanged"}');
 const final=JSON.parse(result.split('data: ').at(-1));expect(final.response.metadata).toEqual({retained:'owned',termux_usage_source:'estimated_context'});expect(final.response.usage.input_tokens).toBe(10);
});

test('estimated-usage annotation preserves streaming and cancellation',async()=>{
 let cancelled=false;const encoder=new TextEncoder();
 const source=new ReadableStream({start(c){c.enqueue(encoder.encode('event: heartbeat\ndata: {}\n\n'));},cancel(){cancelled=true;}});
 const reader=markDesktopEstimatedUsage(source).getReader();const first=await reader.read();expect(new TextDecoder().decode(first.value)).toBe('event: heartbeat\ndata: {}\n\n');
 await reader.cancel();await new Promise(ok=>setTimeout(ok,20));expect(cancelled).toBe(true);
});

test('route and transport transitions replay while unchanged owned context appends',()=>{
 const p={instructions:['prefix'],messages:[{role:'user',content:'owned task'}],tools:[]};
 const a={provider:'chat',account:'owned-a',model:'sol',effort:'low',generation:'one'};
 const cursor=new DesktopPromptContext();let prepared=cursor.prepare(p,a);expect(prepared.mode).toBe('replay');cursor.accept(prepared.current,a);
 const next={...p,messages:[...p.messages,{role:'assistant',content:'result'}]};
 expect(cursor.prepare(next,a).mode).toBe('append');
 for(const changed of [{...a,provider:'codex'},{...a,account:'owned-b'},{...a,model:'pro'},{...a,effort:'xhigh'},{...a,generation:'two'}])expect(cursor.prepare(next,changed).mode).toBe('replay');
 // Preparation is not delivery: an interrupted switch does not commit a cursor.
 expect(cursor.prepare(next,a).mode).toBe('append');
 const target={...a,model:'pro'};prepared=cursor.prepare(next,target);cursor.accept(prepared.current,target);
 expect(cursor.prepare(next,a).mode).toBe('replay');
 expect(()=>cursor.prepare({...p,messages:[]},target)).toThrow('desktop_context_replaced');
});
test('unqualified model effort and Pro mode refuse rather than silently running Instant',()=>{
 expect(validateDesktopRoute({model:'chatgpt-desktop/instant'}).effort).toBe('none');
 expect(()=>validateDesktopRoute({model:'chatgpt-desktop/instant',reasoning:{effort:'none',mode:'standard'}})).not.toThrow();
 for(const raw of [{model:'gpt-6-sol'},{model:'chatgpt-desktop/instant',reasoning:{effort:'xhigh'}},{model:'chatgpt-desktop/instant',reasoning:{mode:'pro'}}])expect(()=>validateDesktopRoute(raw)).toThrow('unsupported_desktop_route');
});

test('connection generation follows the verified relay identity, not just model name',()=>{
 const root=mkdtempSync(join(process.env.TMPDIR,'desktop-route-'));
 try{
  const raw=readFileSync(`/proc/${process.pid}/stat`,'utf8'),start=raw.slice(raw.lastIndexOf(')')+2).split(' ')[19],identity={pid:process.pid,start,image:readlinkSync(`/proc/${process.pid}/exe`)};
  const d={kind:'termux-electron-pipe',endpoint:'http://127.0.0.1:12345',token:'a'.repeat(43),host:identity,browser:identity};
  const file=join(root,'desktop-host.json');writeFileSync(file,JSON.stringify(d),{mode:0o600});const first=desktopConnection(root).generation;
  writeFileSync(file,JSON.stringify({...d,endpoint:'http://127.0.0.1:12346'}));expect(desktopConnection(root).generation).not.toBe(first);
  writeFileSync(file,JSON.stringify({...d,browser:{...identity,start:String(Number(start)+1)}}));expect(()=>desktopConnection(root)).toThrow('stale_owner');
 }finally{rmSync(root,{recursive:true});}
});

test('current controller marker resolves a new generation and refuses stale ownership',()=>{
 const base=mkdtempSync(join(process.env.TMPDIR,'desktop-marker-'));
 try{
  const raw=readFileSync(`/proc/${process.pid}/stat`,'utf8'),start=raw.slice(raw.lastIndexOf(')')+2).split(' ')[19],identity={pid:process.pid,start,image:readlinkSync(`/proc/${process.pid}/exe`)};
  const file=join(base,'session.json');writeFileSync(file,JSON.stringify({...identity,root:base}),{mode:0o600});
  writeFileSync(join(base,'desktop-host.json'),JSON.stringify({kind:'termux-electron-pipe',endpoint:'http://127.0.0.1:12345',token:'a'.repeat(43),host:identity,browser:identity}),{mode:0o600});
  expect(desktopConnection(file).generation).toBe(desktopConnection(base).generation);
  writeFileSync(file,JSON.stringify({...identity,start:String(Number(start)+1),root:base}));expect(()=>desktopConnection(file)).toThrow('stale_desktop_controller');
 }finally{rmSync(base,{recursive:true});}
});

test('committed route boundary is a snapshot, not a mutable selector reference',()=>{
 const c=new DesktopPromptContext(),p={instructions:[],tools:[],messages:[]},route={model:'owned-model',account:'owned-account'};
 const first=c.prepare(p,route);c.accept(first.current,route);route.model='other-model';expect(c.prepare(p,route).mode).toBe('replay');
});


test('observed Chat families preserve actual effort semantics and Pro is separate',()=>{
 for(const model of ['chatgpt-desktop/gpt-6','chatgpt-desktop/gpt-5.6-sol']){
  for(const [effort,label] of [['none','Instant'],['medium','Medium'],['high','High'],['xhigh','Extra High']]){
   const route=validateDesktopRoute({model,reasoning:{effort}});expect(route.model).toBe(model);expect(route.label).toBe(label);expect(route.mode).toBe('standard');
  }
  const pro=validateDesktopRoute({model:model+'-pro'});expect(pro.mode).toBe('pro');expect(pro.label).toBe('Pro');
  expect(()=>validateDesktopRoute({model:model+'-pro',reasoning:{effort:'xhigh'}})).toThrow('unsupported_desktop_route');
 }
 for(const model of ['chatgpt-desktop/gpt-6-sol','chatgpt-desktop/__proto__','chatgpt-desktop/unknown'])expect(()=>validateDesktopRoute({model})).toThrow('unsupported_desktop_route');
 for(const effort of ['low','max','ultra','constructor'])expect(()=>validateDesktopRoute({model:'chatgpt-desktop/gpt-6',reasoning:{effort}})).toThrow('unsupported_desktop_route');
});
