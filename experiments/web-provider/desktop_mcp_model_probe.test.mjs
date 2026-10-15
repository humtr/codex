import {test} from 'node:test';
import assert from 'node:assert/strict';
import {installMetadataModelHooks} from './desktop_mcp_model_probe.mjs';

function fixture(dry=false){
 const seen=[],client={prepareCompletionStream(request){seen.push(request);return request;},
  startCompletionStream(opts){seen.push(opts);return opts;}},api={streamPost(path,opts){seen.push({path,opts});return opts;}};
 const original={prepare:client.prepareCompletionStream,start:client.startCompletionStream,post:api.streamPost};
 const s={scope:{get(){return client;}},m:{aCn(){return 'owned-conversation';}}};
 const p=installMetadataModelHooks(s,{$wt:api},'owned_unbound_test',dry);
 return {p,seen,client,api,original};
}
const own={conversation_id:'owned-conversation',messages:[{id:'user',author:{role:'user'},content:{parts:['test']}}],local_function_signatures:[{name:'handoff'}],local_function_names:['handoff']};

test('metadata hooks disable only owned executors, preserve unrelated traffic and restore exact methods',()=>{
 const {p,seen,client,api,original}=fixture();
 const foreign={conversation_id:'other',local_function_signatures:[{name:'handoff'}]};
 assert.equal(client.prepareCompletionStream(foreign),foreign);
 const prepared=client.prepareCompletionStream(own);
 assert.deepEqual(prepared.local_function_signatures,[]);assert.deepEqual(prepared.local_function_names,[]);
 const started=client.startCompletionStream({request:own});
 assert.deepEqual(started.request.local_function_signatures,[]);
 const other=api.streamPost('/metadata',{body:'{}'});assert.equal(other,seen.at(-1).opts);
 assert.throws(()=>api.streamPost('/f/conversation',{body:JSON.stringify(own)}),/executor_not_disabled/);
 const output=api.streamPost('/f/conversation',{body:JSON.stringify(prepared),retry:{attempts:5}});
 assert.equal(output.retry,undefined);output.chatgptSse.onBeforeRequestStart();
 assert.equal(p.starts,1);assert.equal(p.userMessages,1);assert.equal(p.toolMessages,0);
 assert.throws(()=>output.chatgptSse.onBeforeRequestStart(),/budget_exhausted/);assert.equal(p.starts,1);
 p.restore();assert.equal(client.prepareCompletionStream,original.prepare);assert.equal(client.startCompletionStream,original.start);assert.equal(api.streamPost,original.post);
});

test('dry-run refuses before native start; undefined request cannot claim the owner',()=>{
 const {p,client,seen,api}=fixture(true);
 assert.doesNotThrow(()=>api.streamPost('/metadata',{}));
 const foreign={request:{messages:[]}};assert.equal(client.startCompletionStream(foreign),foreign);
 assert.throws(()=>client.startCompletionStream({request:own}),/metadata_dry_run/);
 assert.equal(p.dryBlocked,true);assert.equal(p.starts,0);assert.equal(seen.length,2);
 p.restore();
});
