// Read the signed app's own connector catalog. Run only in a bounded owned app.
// No Chat, Core execution, plugin installation or credential export.
// --channel-inventory permits only a synthetic unbound metadata request.
import {writeFileSync,readFileSync} from 'node:fs';
import {acquireDesktopApp} from './desktop_app_transport.mjs';
const [runtimePath,reportPath,connectorName]=process.argv.slice(2);
const cachedOnly=process.argv.includes('--cached-only');
const toolsFromCache=process.argv.includes('--tools-from-cache');
const channelList=process.argv.includes('--channel-list');
const channelInventory=process.argv.includes('--channel-inventory');
const refreshTools=process.argv.includes('--refresh-tools');
if(!connectorName)throw Error('explicit_connector_name_required');
if(refreshTools&&(!toolsFromCache||channelList||channelInventory)||cachedOnly&&toolsFromCache||channelList&&channelInventory||(channelList||channelInventory)&&!toolsFromCache)throw Error('invalid_metadata_probe_mode');
const runtime=JSON.parse(readFileSync(runtimePath,'utf8'));
let lease;
let report={failed:true,modelStarts:0,reason:'inspection_not_completed'};
try{
 lease=await acquireDesktopApp(runtime.session);
 await lease.client.evaluate(`(async()=>{
  const s=window[${JSON.stringify(lease.scopeKey)}],shared=await import('app://-/assets/app-shared-2d77298f0100.js');
  const client=s.scope.get(s.m.oMn),api=shared.$wt,start=client.startCompletionStream,post=api.streamPost;
  let attempts=0;
  client.startCompletionStream=function(){attempts++;throw Error('owned_inspection_inference_blocked');};
  api.streamPost=function(path,...args){if(path==='/f/conversation'){attempts++;throw Error('owned_inspection_inference_blocked');}return post.call(this,path,...args);};
  s.catalogProbe={pending:true,restore(){client.startCompletionStream=start;api.streamPost=post;}};
  void (async()=>{
  try{
   let element=document.querySelector('[contenteditable=true]')||document.querySelector('button'),queryClient;
   while(element&&!Object.keys(element).some(k=>k.startsWith('__reactFiber')))element=element.parentElement;
   let fiber=element?.[Object.keys(element).find(k=>k.startsWith('__reactFiber'))];
   for(let i=0;fiber&&i<500;i++,fiber=fiber.return){const value=fiber.memoizedProps?.value;if(typeof value?.getQueryCache==='function'&&typeof value?.fetchQuery==='function'){queryClient=value;break;}}
   if(!queryClient)return {queryClientFound:false,modelStarts:0,blockedAttempts:attempts};
   const matches=[],shapes=[],appIds=new Set();
   for(const q of queryClient.getQueryCache().getAll().filter(q=>['apps','plugins'].includes(q.queryKey[0]))){
    const data=q.state.data;
    if(data==null)continue;
    shapes.push({category:q.queryKey.slice(0,2),shape:Array.isArray(data)?'array':typeof data,count:Array.isArray(data)?data.length:null,fields:!Array.isArray(data)&&typeof data==='object'?Object.keys(data):[]});
    let visited=0;
    const scan=(value,depth)=>{
     if(value==null||typeof value!=='object'||depth>6||++visited>4000)return;
     if([value.name,value.displayName,value.summary?.interface?.displayName].includes(${JSON.stringify(connectorName)})){
      matches.push({name:${JSON.stringify(connectorName)},fields:Object.keys(value),category:q.queryKey.slice(0,2),tools:(value.toolSummaries||[]).map(t=>t.name)});
      for(const id of [...value.appIds||[],value.canonicalAppId,...(value.plugin?.apps||[]).map(a=>a.id)])if(typeof id==='string'&&id)appIds.add(id);
     }
     for(const [key,child] of Object.entries(value))if(!/credential|token|secret|config|conversation|message|user/i.test(key))scan(child,depth+1);
    };scan(data,0);
   }
   if(${toolsFromCache}){
    s.catalogProbe.stage='known_app_tools';
    const privateQueries=new queryClient.constructor({defaultOptions:{queries:{retry:false,networkMode:'always'}}});
    s.catalogProbe.queryClient=privateQueries;
    try{
     let detail=appIds.size?await privateQueries.fetchQuery(s.m.sSt({scope:s.scope,appIds:[...appIds],hostId:'durable',includeTools:true,queryClient:privateQueries})):[];
     let refreshed=false;
     if(${refreshTools}){
      if(detail.length!==1||detail[0].name!==${JSON.stringify(connectorName)})return {failed:true,reason:'connector_not_unique',modelStarts:0};
      if(typeof shared._Ft?.safePost!=='function'||typeof shared._Ft?.safeGet!=='function'||typeof s.m.dHn!=='function')return {failed:true,reason:'official_refresh_unavailable',modelStarts:0};
      const options={surface:'settings',parameters:{path:{connector_id:detail[0].id}},additionalHeaders:s.m.dHn(),retry:'never',signal:AbortSignal.timeout(20000)};
      // A metadata request need not populate the UI link cache. Resolve the exact
      // selected connector/link through the same authenticated settings API.
      const [connectorResult,{link}]=await Promise.all([shared._Ft.safeGet('/aip/connectors/{connector_id}',options),shared._Ft.safeGet('/aip/connectors/{connector_id}/link',options)]);
      const connector=connectorResult?.connector??connectorResult;
      if(connector?.id!==detail[0].id||connector.connector_type!=='MCP'||link?.connector_id!==detail[0].id||typeof link.id!=='string')return {failed:true,reason:'owned_mcp_link_not_unique',modelStarts:0,linkResolution:{connectorPresent:!!connector,linkPresent:!!link,connectorMatchesApp:connector?.id===detail[0].id,linkMatchesApp:link?.connector_id===detail[0].id,linkMatchesConnector:!!link&&link.connector_id===connector?.id,mcp:connector?.connector_type==='MCP',linkHasId:typeof link?.id==='string',connectorFields:Object.keys(connector||{}),linkFields:Object.keys(link||{})}};
      s.catalogProbe.stage='refresh_registered_tools';
      // Same signed-app settings action; auth/account headers remain inside it.
      // This changes only the selected link discovery, not URL/auth/history.
      await shared._Ft.safePost('/aip/connectors/mcp/refresh_actions',{surface:'settings',requestBody:{link_id:link.id},additionalHeaders:s.m.dHn(),retry:'never',signal:AbortSignal.timeout(30000)});
      refreshed=true;privateQueries.clear();
      detail=await privateQueries.fetchQuery(s.m.sSt({scope:s.scope,appIds:[...appIds],hostId:'durable',includeTools:true,queryClient:privateQueries}));
     }
     if(${channelList||channelInventory}){
      if(detail.length!==1)return {failed:true,reason:'connector_not_unique',modelStarts:0};
      s.catalogProbe.stage='channel_tools_list';
      s.m.Cut?.();
      const links=queryClient.getQueryCache().getAll().filter(q=>q.queryKey[0]==='apps'&&q.queryKey[1]==='link').map(q=>q.state.data).filter(v=>v?.connector_id===detail[0].id);
      const originMetadata=links.length===1?{attribution_id:detail[0].id,link_id:links[0].id}:undefined;
      const handler=s.m.Sut({scope:s.scope,accountSource:'apps',appUri:'connectors://'+detail[0].id,originMetadata,signal:AbortSignal.timeout(20000)});
      const result=await handler.handleRequest(${channelInventory}?{method:'tools/call',params:{name:'codex_tool_inventory',arguments:{turn_token:'owned_unbound_'+crypto.randomUUID().replaceAll('-','')}}}:{method:'tools/list',params:{}});
      return {queryClientFound:true,modelStarts:0,blockedAttempts:attempts,channelList:${channelList},metadataCall:${channelInventory},resultFields:Object.keys(result),toolCount:result.tools?.length??null,error:result.isError===true};
     }
     return {queryClientFound:true,modelStarts:0,blockedAttempts:attempts,knownAppIds:appIds.size,registrationRefreshed:refreshed,apps:detail.map(a=>({name:a.name,toolCount:a.toolSummaries?.length??null,toolNames:(a.toolSummaries||[]).map(t=>t.name)}))};
    }finally{privateQueries.clear();}
   }
   if(${cachedOnly})return {queryClientFound:true,modelStarts:0,blockedAttempts:attempts,cachedOnly:true,matches,shapes};
   s.catalogProbe.queryClient=queryClient;s.catalogProbe.stage='listing';
   const apps=await queryClient.fetchQuery(s.m.IVn(s.scope,'durable'));
   const selected=apps.filter(a=>[a.name,...a.pluginDisplayNames||[]].includes(${JSON.stringify(connectorName)}));
   const result={queryClientFound:true,accessibleApps:apps.length,matchingConnectors:selected.length,modelStarts:0};
   if(selected.length===1){
    s.catalogProbe.stage='tool_metadata';
    const detail=await queryClient.fetchQuery(s.m.sSt({scope:s.scope,appIds:[selected[0].id],hostId:'durable',includeTools:true,queryClient}));
    result.connector={name:selected[0].name,accessible:selected[0].isAccessible,enabled:selected[0].isEnabled,
     apps:detail.map(a=>({name:a.name,toolNames:(a.toolSummaries||[]).map(t=>t.name),toolCount:a.toolSummaries?.length??null}))};
   }
   result.blockedAttempts=attempts;return result;
  }catch(error){return {failed:true,metadataCall:${channelInventory},status:Number.isInteger(error?.status)?error.status:null,
   errorName:['Error','TypeError','ZodError','AbortError','TimeoutError'].includes(error?.name)?error.name:'other',
   errorFlags:{schema:Array.isArray(error?.issues),unbound:/broker|ENOENT|turn.token|unknown turn/i.test(error?.message||''),aborted:/abort|timeout/i.test(error?.name||''),method:/unsupported|method.not.found/i.test(error?.message||'')},
   modelStarts:0,blockedAttempts:attempts};}
  })().then(result=>{s.catalogProbe.result=result;s.catalogProbe.pending=false;});
  return true;
 })()`);
 const deadline=Date.now()+60000;
 while(Date.now()<deadline){
  const state=await lease.client.evaluate(`(()=>{const p=window[${JSON.stringify(lease.scopeKey)}].catalogProbe;return {pending:p.pending,result:p.result,stage:p.stage,queries:p.queryClient?.getQueryCache().getAll().filter(q=>q.queryKey[0]==='apps'||q.queryKey[0]==='plugins').map(q=>({category:q.queryKey.slice(0,2),status:q.state.status,fetchStatus:q.state.fetchStatus,errorStatus:Number.isInteger(q.state.error?.status)?q.state.error.status:null}))};})()`);
  if(!state.pending){report=state.result;break;}
  report={failed:true,reason:'metadata_timeout',modelStarts:0,stage:state.stage,pendingQueries:state.queries};
  await new Promise(resolve=>setTimeout(resolve,200));
 }
}catch(error){report.reason=/^owned_[a-z_.]+$/.test(error.message)?error.message:'inspection_failed';}
finally{
 if(lease){
  await lease.client.evaluate(`(()=>{const s=window[${JSON.stringify(lease.scopeKey)}];s.catalogProbe?.restore();return true;})()`).catch(()=>{});
  lease.release();
 }
}
writeFileSync(reportPath,JSON.stringify(report,null,2)+'\n',{mode:0o600});
console.log(JSON.stringify(report));
const expectedRefusal=channelInventory&&report.errorFlags?.unbound;
if(!expectedRefusal&&(report.failed||!report.queryClientFound||cachedOnly&&!report.matches?.length||channelInventory&&!report.error||channelList&&!report.toolCount||toolsFromCache&&!channelList&&!channelInventory&&!report.apps?.some(a=>a.name===connectorName&&a.toolCount>0)))process.exitCode=1;
