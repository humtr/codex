import {test,expect} from 'bun:test';
import {readFileSync,mkdtempSync,writeFileSync,rmSync} from 'node:fs';
import {join} from 'node:path';
import {pathToFileURL} from 'node:url';

const bridge=new Function('return ('+readFileSync(new URL('./src/app_bridge.js',import.meta.url),'utf8')+')')();
const slot=Symbol.for('termux.codex.app.bridge.v1');
async function fixture(run){
 const root=mkdtempSync(join(process.env.TMPDIR,'app-bridge-'));
 const oldDocument=globalThis.document;
 const wire={streamPost(_path,options){const body=JSON.parse(options.body);globalThis.appBridgeFixture.sent.push(body);options.chatgptSse?.onBeforeRequestStart?.({});return true;}};
 const client={prepareCompletionStream(request){return request;},startCompletionStream(options){
  wire.streamPost('/f/conversation',{body:JSON.stringify(options.request),chatgptSse:{}});
  options.onDecodedEvent?.({message:{id:'analysis',author:{role:'assistant'},recipient:'all',channel:'analysis',content:{content_type:'text',parts:['private reasoning']}}});
  options.onUpdate?.({message:{id:'public',author:{role:'assistant'},recipient:'all',channel:'final',content:{content_type:'text',parts:['Visible result']},status:'finished_successfully',end_turn:true}});
  options.onComplete?.({reason:'done'});return true;
 }};
 const original={...client,streamPost:wire.streamPost};
 const f=globalThis.appBridgeFixture={wire,client,sent:[],created:0,gate:true};
 writeFileSync(join(root,'initial.mjs'),`
 export const oMn={kind:'client'},JRn={kind:'messages'};
 export function gRt(){}
 export function aCn(){return 'owned-'+(++globalThis.appBridgeFixture.created);}
 export function eHt(){}
 export function yRt(_scope,options){
  const messages=[...(options.prependMessages||[]),options.userCompletionMessages?.message,...(options.userCompletionMessages?.extraDeveloperInstructionMessages||[])].filter(Boolean);
  const client=globalThis.appBridgeFixture.client;
  options.onServerThreadIdChange?.('server-owned');
  const request=client.prepareCompletionStream({conversation_id:messages.some(m=>m.author.role==='tool')?'server-owned':undefined,local_function_signatures:[{name:'handoff'}],messages});
  client.startCompletionStream({request});return Promise.resolve(true);
 }
 export function bRt(scope,options){return yRt(scope,{...options,userCompletionMessages:{message:{id:'tool-reply',author:{role:'tool'},content:{text:JSON.stringify({call_id:options.callId,result:options.result})}}}});}
 `);
 writeFileSync(join(root,'shared.mjs'),`export const HAt={kind:'gate'},xNt={kind:'identity'};export const $wt=globalThis.appBridgeFixture.wire;`);
 const scope={get(node){return node.kind==='client'?client:node.kind==='gate'?f.gate:node.kind==='identity'?{authMethod:'chatgpt',accountId:'account',authenticatedAccountId:'account',userId:'user',authenticatedUserId:'user',accountStructure:'personal',email:'not-exported',accessToken:'never-export'}:{};},set(){},node:{},chain:{},query:{}};
 globalThis.document={querySelector(){return {__reactFiberFixture:{memoizedState:{memoizedState:{current:scope}}}};}};
 const bindings={initial:pathToFileURL(join(root,'initial.mjs')).href,shared:pathToFileURL(join(root,'shared.mjs')).href};
 try{await run({f,bindings,original});}finally{delete globalThis[slot];delete globalThis.appBridgeFixture;globalThis.document=oldDocument;rmSync(root,{recursive:true});}
}

test('owned app hooks preserve unowned requests, actual feature gates and public-only output',async()=>fixture(async({f,bindings,original})=>{
 expect(await bridge({op:'init',owner:'owned',bindings})).toEqual({ready:true});
 const unowned={local_function_signatures:[{name:'unowned'}],messages:[]};
 expect(f.client.prepareCompletionStream(unowned)).toBe(unowned);
 const identity=await bridge({op:'identity',owner:'owned',lane:'lane'});expect(identity).toEqual({account:'account',user:'user',structure:'personal',present:false});
 await bridge({op:'open',owner:'owned',lane:'lane',signature:{name:'termux_codex_fixture'}});
 await expect(bridge({op:'dispose',owner:'owned'})).rejects.toThrow('app_lanes_active');
 await bridge({op:'submit',owner:'owned',lane:'lane',round:'round-1',model:'configured-host',messages:[{role:'user',text:'Task',hidden:false},{role:'user',text:'<environment_context>preserved</environment_context>',hidden:true}],instructions:['Native policy']});
 await Promise.resolve();
 const state=await bridge({op:'snapshot',owner:'owned',lane:'lane'});
 expect(state.round).toBe('round-1');expect(state.roundStarts).toBe(1);expect(state.userMessages).toBe(2);
 expect(state.messages.map(m=>m.id)).toEqual(['public']);
 expect(f.sent[0].messages[1].metadata.is_visually_hidden_from_conversation).toBe(true);
 expect(f.sent[0].local_function_signatures).toEqual([{name:'termux_codex_fixture'}]);
 await bridge({op:'submit',owner:'owned',lane:'lane',round:'round-2',model:'changed-host',results:[{id:'original-call',result:{output:'done'}}]});
 await Promise.resolve();
 const returned=await bridge({op:'snapshot',owner:'owned',lane:'lane'});
 expect(returned.userMessages).toBe(2);expect(returned.toolMessages).toBe(1);
 expect(JSON.parse(f.sent[1].messages[0].content.text).call_id).toBe('original-call');
 await bridge({op:'submit',owner:'owned',lane:'lane',round:'round-2',model:'changed-host',results:[{id:'original-call',result:{output:'done'}}]});
 expect(f.sent.length).toBe(2);
 await bridge({op:'release',owner:'owned',lane:'lane'});
 expect(f.client.prepareCompletionStream).toBe(original.prepareCompletionStream);
 expect(f.wire.streamPost).toBe(original.streamPost);
 expect(await bridge({op:'dispose',owner:'owned'})).toEqual({disposed:true});
 expect(await bridge({op:'init',owner:'new-owned',bindings})).toEqual({ready:true});
}));

test('app feature gate is required and owner conflicts do not allocate conversations',async()=>fixture(async({f,bindings})=>{
 f.gate=false;await expect(bridge({op:'init',owner:'owned',bindings})).rejects.toThrow('app_local_functions_unavailable');
 expect(f.created).toBe(0);f.gate=true;
 await bridge({op:'init',owner:'owned',bindings});
 await expect(bridge({op:'init',owner:'other',bindings})).rejects.toThrow('app_bridge_owned');
 expect(f.created).toBe(0);
}));


test('opening and release reconcile lost replies and reattach the same owned app',async()=>fixture(async({f,bindings,original})=>{
 await bridge({op:'init',owner:'owned',bindings});
 const open={op:'open',owner:'owned',lane:'lane',signature:{name:'termux_codex_fixture'}};
 const first=await bridge(open);expect(await bridge(open)).toEqual(first);expect(f.created).toBe(1);
 await expect(bridge({...open,signature:{name:'changed'}})).rejects.toThrow('app_lane_declaration_changed');
 await bridge({op:'release',owner:'owned',lane:'lane'});
 await bridge({op:'release',owner:'owned',lane:'lane'});
 expect(f.client.prepareCompletionStream).toBe(original.prepareCompletionStream);
 await bridge({...open,lane:'next'});
 await bridge({op:'submit',owner:'owned',lane:'next',round:'new',model:'configured-host',messages:[{role:'user',text:'Resume latest',hidden:false}]});
 await Promise.resolve();expect(f.sent).toHaveLength(1);expect(f.created).toBe(2);
 expect((await bridge({op:'snapshot',owner:'owned',lane:'next'})).roundStarts).toBe(1);
}));
