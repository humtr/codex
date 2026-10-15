// Actual Core discovery/MCP execution, deterministic app port, zero Chat.
import {DesktopLocalTransport} from './desktop_local_transport.mjs';
import {writeFileSync} from 'node:fs';
const [bridge,root]=process.argv.slice(2),exchange=DesktopLocalTransport.prototype.exchange;
DesktopLocalTransport.prototype.exchange=function(payload,...args){
 if(!this.fixture){
  const state=this.fixture={round:0};
  this.acquire=async()=>({scopeKey:'offline',client:{generation:'owned-discovery-fixture'},release(){}});
  this.invoke=async(_lease,_lane,method,data)=>{
   if(method==='create'){state.gateway=data.signature.name;return true;}
   if(method==='close')return true;
   if(method==='submit'){
    state.round++;
    if(state.round>(state.guardian?1:3))throw Error('fixture_generation_replayed');
    if(state.round===1){if(!data.prompt||data.results)throw Error('fixture_initial_request_required');}
    else{
     if(data.prompt!==undefined||data.results?.length!==1||data.results[0].id!=='discovery-'+(state.round-1)||data.results[0].result.isError)throw Error('fixture_result_mismatch');
     if(state.round===3){state.marker=JSON.stringify(data.results[0].result.content).match(/\b[0-9a-f]{32}\b/)?.[0];if(!state.marker){writeFileSync(root+'/fixture-result.json',JSON.stringify({ownedSyntheticMcpResult:data.results[0].result}),{mode:0o600});throw Error('fixture_mcp_marker_missing');}}
    }
    return true;
   }
   if(method!=='snapshot')throw Error('fixture_operation_invalid');
   const counts={accepted:true,roundDispatched:1,terminals:['done'],dispatches:state.round,userSubmissions:1,toolSubmissions:state.round-1};
   if(state.guardian||state.round===3)return {...counts,messages:[{id:'final',author:{role:'assistant'},recipient:'all',channel:'final',end_turn:true,status:'finished_successfully',content:{content_type:'text',parts:[state.guardian?JSON.stringify({risk_level:'low',user_authorization:'high',outcome:'allow',rationale:'Read only the explicitly requested private disposable fixture marker.'}):state.marker]}}]};
   const tool=state.round===1?state.tools.find(t=>t.toolSearch):state.tools.find(t=>t.name==='_read_marker'&&t.namespace?.includes('owned_catalog'));
   if(!tool)throw Error(state.round===1?'fixture_search_unavailable':'fixture_discovered_tool_missing');
   return {...counts,messages:[{id:'discovery-'+state.round,author:{role:'assistant'},recipient:'local.'+state.gateway,status:'finished_successfully',content:{content_type:'code',text:JSON.stringify({name:tool.wireName,input:JSON.stringify(state.round===1?{query:'owned_catalog _read_marker',limit:1}:{}),report:state.round===1?'LOCAL_INITIAL_REPORT':'LOCAL_INTERMEDIATE_REPORT'})}}]};
  };
 }
 this.fixture.tools=payload.tools;
 this.fixture.guardian=payload.instructions.some(s=>s.startsWith('The native final reply must match this exact output format.')&&s.includes('"outcome"'));
 return exchange.call(this,payload,...args);
};
process.env.CODEX_CHAT_RETURN_TRANSPORT='local';
process.argv=[process.argv[0],process.argv[1],bridge,root+'/no-app',root+'/server.json'];
await import('./desktop_responses_server.mjs');
