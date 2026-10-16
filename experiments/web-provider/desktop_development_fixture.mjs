// Real Core tools and compaction, deterministic Chat port, no real inference.
import {DesktopLocalTransport} from './desktop_local_transport.mjs';
import {writeFileSync} from 'node:fs';
const [bridge,root]=process.argv.slice(2),exchange=DesktopLocalTransport.prototype.exchange,observations=[];
const catalog=`def normalize_names(values):
    result, seen = [], set()
    for value in values:
        if not isinstance(value, str):
            raise TypeError('expected string')
        key = ' '.join(value.split()).casefold()
        if key and key not in seen:
            seen.add(key)
            result.append(key)
    return result
`;
const cli=`import json, sys
from catalog import normalize_names
try:
    values = json.load(sys.stdin)
    if not isinstance(values, list):
        raise TypeError('expected list')
    result = normalize_names(values)
except (ValueError, TypeError):
    print('invalid input', file=sys.stderr)
    sys.exit(2)
print(json.dumps(result, ensure_ascii=False))
`;
DesktopLocalTransport.prototype.exchange=function(payload,...rest){
 if(!this.development){
  const state=this.development={round:0};
  this.acquire=async()=>({scopeKey:'offline',client:{generation:'owned-development'},release(){}});
  this.invoke=async(_lease,_lane,method,data)=>{
   if(method==='create'){state.gateway=data.signature.name;state.user=0;state.returns=0;state.starts=0;return true;}
   if(method==='close')return true;
   if(method==='submit'){state.round++;state.starts++;state.user+=(data.prompt?1:0)+(data.taskMessages||[]).filter(m=>m.role==='user').length;state.returns+=data.results?.length??0;if(process.env.CODEX_CHAT_DEVELOPMENT_COMPACTION==='mid-turn'&&state.payload.compaction)state.midCompactCount=(state.midCompactCount||0)+1;return true;}
   if(method!=='snapshot')throw Error('development_fixture_invalid_operation');
   const tool=name=>{const t=state.payload.tools.find(t=>t.name===name);if(!t)throw Error('development_fixture_missing_'+name);return t;};
   const call=(id,name,input,report='')=>{const t=tool(name);return {id,author:{role:'assistant'},recipient:'local.'+state.gateway,status:'finished_successfully',content:{content_type:'code',text:JSON.stringify({name:t.wireName,input:t.freeform?input:JSON.stringify(input),report})}};};
   const final=text=>({id:'development-final-'+state.round,author:{role:'assistant'},recipient:'all',channel:'final',status:'finished_successfully',end_turn:true,content:{content_type:'text',parts:[text]}});
   if(state.round===2){
    const output=JSON.stringify(state.payload.messages.filter(m=>m.role==='toolResult'));
    state.secret=output.match(/(?<![a-f0-9])[a-f0-9]{32}(?![a-f0-9])/)?.[0];if(!state.secret)throw Error('fixture_missing_actual_tool_token');
   }
   let round=state.round;
   const automatic=process.env.CODEX_CHAT_DEVELOPMENT_COMPACTION==='automatic';
   const midTurn=process.env.CODEX_CHAT_DEVELOPMENT_COMPACTION==='mid-turn';
   if(midTurn){
    if(state.payload.compaction)return {messages:[final('Preserve completed changes and token '+state.secret+'. Continue actual remaining tests/repair without repeating completed edits.')],accepted:true,roundDispatched:1,terminals:['done'],userSubmissions:state.user,toolSubmissions:state.returns,dispatches:state.starts};
    round-=state.midCompactCount||0;if(round>=5)round++;
   }
   const interrupted=process.env.CODEX_CHAT_DEVELOPMENT_COMPACTION==='interrupted';
   if(interrupted&&round===5)return {messages:[],accepted:true,roundDispatched:1,terminals:[],userSubmissions:state.user,toolSubmissions:state.returns,dispatches:state.starts};
   if(interrupted&&round===6)return {messages:[final(state.secret)],accepted:true,roundDispatched:1,terminals:['done'],userSubmissions:state.user,toolSubmissions:state.returns,dispatches:state.starts};
   if(interrupted&&round>6)round-=2;
   if(automatic&&round>6)round--;
   let messages;
   if(round===1)messages=[call('development-baseline','exec_command',{cmd:'cat README.md catalog.py cli.py project-token.txt; python -m unittest -v test_catalog.py',login:false,max_output_tokens:3000},'LOCAL_INITIAL_REPORT')];
   else if(round===2){
    const patch='*** Begin Patch\n*** Update File: catalog.py\n@@\n-def normalize_names(values):\n-    return sorted(set(str(v).lower() for v in values if v))\n'+catalog.trimEnd().split('\n').map(l=>'+'+l).join('\n')+'\n*** Update File: cli.py\n@@\n-import json,sys\n-from catalog import normalize_names\n-print(normalize_names(json.load(sys.stdin)))\n'+cli.trimEnd().split('\n').map(l=>'+'+l).join('\n')+'\n*** End Patch';
    messages=[call('development-repair','apply_patch',patch)];
    const args=JSON.parse(messages[0].content.text);delete args.report;messages[0].content.text=JSON.stringify(args);
   }else if(round===3)messages=[call('development-tests','exec_command',{cmd:'python -m unittest -v test_catalog.py',login:false,max_output_tokens:1500})];
   else if(round===4)messages=[final('DEVELOPMENT_STAGE_DONE')];
   else if(round===5)messages=[final('Development complete: order, Unicode casefold, type checks and CLI error handling fixed; tests pass. Project token '+state.secret+'. Preserve implementations; run regressions and write remembered token to proof.txt after resume.')];
   else if(automatic&&state.round===6)messages=[final(state.secret)];
   else if(round===6)messages=[call('development-resume','exec_command',{cmd:'python -m unittest -v test_catalog.py && printf "%s\\n" '+state.secret+' > proof.txt',login:false,max_output_tokens:1500},'LOCAL_INTERMEDIATE_REPORT')];
   else if([7,8].includes(round))messages=[final(state.secret)];
   else throw Error('development_fixture_unexpected_generation');
   return {messages,accepted:true,roundDispatched:1,terminals:['done'],userSubmissions:state.user,toolSubmissions:state.returns,dispatches:state.starts};
  };
 }
 this.development.payload=payload;
 return exchange.call(this,payload,...rest).then(result=>{observations.push({type:result.type,calls:(result.calls||[]).map(c=>({id:c.id,name:c.name}))});writeFileSync(root+'/surface-batches.json',JSON.stringify(observations),{mode:0o600});return result;},error=>{
  if(error.message==='desktop_native_recovery_requires_new_input')writeFileSync(root+'/fixture-context.json',JSON.stringify({anchor:this.rejectedContext,current:payload.messages}),{mode:0o600});
  throw error;
 });
};
process.env.CODEX_CHAT_RETURN_TRANSPORT='local';
process.argv=[process.argv[0],process.argv[1],bridge,root+'/no-app',root+'/server.json'];
await import('./desktop_responses_server.mjs');
