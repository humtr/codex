import {test} from 'node:test';
import assert from 'node:assert/strict';
import {internalModel} from './desktop_app_transport.mjs';
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
