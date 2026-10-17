// Actual app request construction, blocked before inference; synthetic data only.
import {readFileSync,writeFileSync} from 'node:fs';
import {acquireDesktopApp} from './desktop_app_transport.mjs';
import {invokeMcpApp} from './desktop_mcp_transport.mjs';
const [runtimePath,reportPath]=process.argv.slice(2);
const runtime=JSON.parse(readFileSync(runtimePath)),proof=JSON.parse(readFileSync(runtime.binding+'.proof'));
if(proof.requests||proof.arrivals)throw Error('operating_adapter_in_use');
const lease=await acquireDesktopApp(runtime.session),lane='owned_preflight';let created=false,result;
try{
 await lease.client.evaluate(`(()=>{const s=window[${JSON.stringify(lease.scopeKey)}],client=s.scope.get(s.m.oMn);return import('app://-/assets/app-shared-2d77298f0100.js').then(shared=>{
  const api=shared.$wt,start=client.startCompletionStream,post=api.streamPost;
  s.preflight={starts:0,posts:0};
  client.startCompletionStream=function(opts){const p=s.preflight,l=s.mcpNative?.lanes.get(${JSON.stringify(lane)}),request=opts?.request;p.starts++;p.matchesInputIds=!!request?.messages?.some(m=>l?.inputIds.has(m.id));p.localSignatures=request?.local_function_signatures?.length??0;p.userObjects=request?.messages?.filter(m=>m.author?.role==='user').length??0;p.visibleUserPreserved=request?.messages?.some(m=>m.author?.role==='user'&&!m.metadata?.is_visually_hidden_from_conversation&&m.content?.parts?.join('')==='literal <environment_context /> request');throw Error('owned_preflight_blocked');};
  api.streamPost=function(path,...args){if(path==='/f/conversation'){s.preflight.posts++;throw Error('owned_preflight_blocked');}return post.call(this,path,...args);};
  s.preflight.restore=()=>{client.startCompletionStream=start;api.streamPost=post;};return true;
 });})()`);
 await invokeMcpApp(lease,lane,'create',{marker:'owned_preflight_marker'});created=true;
 await invokeMcpApp(lease,lane,'submit',{instructions:['Synthetic request preparation only.'],model:{model:'gpt-6',thinkingEffort:null},taskMessages:[
  {role:'user',text:'<environment_context>synthetic</environment_context>',hidden:true},
  {role:'assistant',text:'old synthetic reply'},
  {role:'user',text:'literal <environment_context /> request'},
 ]});
 const deadline=Date.now()+20000;
 while(Date.now()<deadline){
  result=await lease.client.evaluate(`(()=>{const p=window[${JSON.stringify(lease.scopeKey)}].preflight;return {starts:p.starts,posts:p.posts,matchesInputIds:p.matchesInputIds,localSignatures:p.localSignatures,userObjects:p.userObjects,visibleUserPreserved:p.visibleUserPreserved};})()`);
  if(result.starts||result.posts)break;await new Promise(r=>setTimeout(r,50));
 }
 result={...result,actualChatSubmissions:0,actualModelStarts:0,scope:'Actual signed-app construction blocked at client start and conversation POST'};
 result.accepted=result.starts===1&&result.posts===0&&result.matchesInputIds&&result.localSignatures===0&&result.userObjects===2&&result.visibleUserPreserved;
}finally{
 if(created)await invokeMcpApp(lease,lane,'close').catch(()=>{});
 await lease.client.evaluate(`(()=>{const s=window[${JSON.stringify(lease.scopeKey)}];s.preflight?.restore();delete s.preflight;return true;})()`).catch(()=>{});
 lease.release();
}
writeFileSync(reportPath,JSON.stringify(result,null,2)+'\n',{mode:0o600});console.log(JSON.stringify(result));
if(!result.accepted)process.exitCode=1;
