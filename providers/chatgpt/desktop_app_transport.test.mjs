import {test} from 'node:test';
import assert from 'node:assert/strict';
import {internalModel,linkOwnedReply,waitForAppScope} from './desktop_app_transport.mjs';
import {chatModelRoute} from './desktop_model_controls.mjs';
import {DesktopTransport} from './desktop_response_transport.mjs';
test('Chat control identities map to app submit arguments, not Codex model aliases',()=>{
 for(const [effort,model,power] of [['none','gpt-6',null],['medium','gpt-6-thinking','standard'],['high','gpt-6-thinking','extended'],['xhigh','gpt-6-thinking','max']]){
  assert.deepEqual(internalModel(chatModelRoute({model:'chatgpt-desktop/gpt-6',reasoning:{effort}})),{model,thinkingEffort:power});
 }
 assert.deepEqual(internalModel(chatModelRoute({model:'chatgpt-desktop/gpt-5.6-sol'})),{model:'gpt-5-6-instant',thinkingEffort:null});
 assert.deepEqual(internalModel(chatModelRoute({model:'chatgpt-desktop/gpt-6-pro'})),{model:'gpt-6-pro',thinkingEffort:null});
 assert.throws(()=>internalModel({family:'invented'}),/unsupported_desktop_route/);
});
test('continuation links only a completed correlated assistant in its owned conversation',()=>{
 const lane={id:'owned'},reply={id:'answer',author:{role:'assistant'},status:'finished_successfully',content:{parts:['DESKTOP_owned']}},mapping={answer:{message:reply}};
 globalThis.window={test:{lanes:new Map([['lane',lane]]),m:{JRn:{}},scope:{get:(_,id)=>id==='owned'?mapping:{}}}};
 try{
  assert.equal(linkOwnedReply('test','lane','foreign','DESKTOP_owned'),false);
  assert.equal(linkOwnedReply('test','lane','answer','DESKTOP_other'),false);
  reply.status='in_progress';assert.equal(linkOwnedReply('test','lane','answer','DESKTOP_owned'),false);
  reply.status='finished_successfully';reply.author.role='user';assert.equal(linkOwnedReply('test','lane','answer','DESKTOP_owned'),false);
  reply.author.role='assistant';assert.equal(linkOwnedReply('test','lane','answer','DESKTOP_owned'),true);assert.equal(lane.parent,'answer');
  assert.equal(linkOwnedReply('test','missing','answer','DESKTOP_owned'),false);
 }finally{delete globalThis.window;}
});
test('app startup waits for its scope without treating initialization as an absent capability',async()=>{
 const states=['starting','starting',true];let pauses=0;
 await waitForAppScope(async()=>states.shift(),30000,async()=>{pauses++;});assert.equal(pauses,2);
 await assert.rejects(()=>waitForAppScope(async()=>false),/desktop_internal_unavailable/);
 await assert.rejects(()=>waitForAppScope(async()=>'starting',0),/desktop_internal_unavailable/);
});
test('UI fallback protects pending owner and allows isolated guardian replay; ambiguous errors never switch transports',async()=>{
 const lane=(key,parent)=>{const t=new DesktopTransport('/owned-app',{key,parent});t.internal={exchange:async()=>{throw Error('desktop_internal_unavailable');}};t.ui={exchange:async()=>({type:'tool'}),retire:()=>{t.retired=true;}};return t;};
 const parent=lane('parent'),unrelated=lane('unrelated'),guardian=lane('guardian','parent');
 await parent.exchange({});
 await assert.rejects(()=>unrelated.exchange({}),/desktop_thread_owner_conflict/);
 guardian.ui.exchange=async()=>({type:'final'});await guardian.exchange({});assert.equal(parent.retired,true);
 await assert.rejects(()=>unrelated.exchange({}),/desktop_thread_owner_conflict/);
 parent.ui.exchange=async()=>({type:'final'});await parent.exchange({});
 unrelated.internal.exchange=async()=>{throw Error('desktop_internal_submit_failed');};let fallbackCalled=false;unrelated.ui.exchange=async()=>{fallbackCalled=true;};
 await assert.rejects(()=>unrelated.exchange({}),/desktop_internal_submit_failed/);assert.equal(fallbackCalled,false);
});
