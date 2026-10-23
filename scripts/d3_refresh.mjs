// Owned connector refresh plus actual backend MCP readiness. No model or screen input.
import {readFileSync,writeFileSync,mkdirSync,existsSync} from 'node:fs';
import {dirname} from 'node:path';
import {connectDesktop} from '../providers/chatgpt/desktop_cdp.mjs';

function responseFailure(error){
 let detail;
 try{detail=JSON.parse(error?.message??'').detail;}catch{}
 return {status:error?.status??error?.responseStatus,upstream_status:detail?.upstream_status??null};
}

async function readConnectorWhenReady(read,{now=Date.now,sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms)),timeoutMs=30000}={}){
 const deadline=now()+timeoutMs;
 for(;;){
  try{return await read();}
  catch(error){
   // The cold app can expose its facade before the HTTP client is injected.
   // Wait only for this known pre-transport failure on the initial read.
   if(error?.name!=='TypeError'||error?.message!=="Cannot read properties of undefined (reading 'httpFetch')")throw error;
   if(now()>=deadline)throw Error('app_http_client_initialization_timeout; no mutation submitted');
   await sleep(Math.min(250,deadline-now()));
  }
 }
}

const directory=process.argv[2];
const info=JSON.parse(readFileSync(directory+'/info.json','utf8'));
const cooldownPath=(info.state?dirname(info.state):directory)+'/connector-refresh-cooldown.json';
if(existsSync(cooldownPath)){
 const cooldown=JSON.parse(readFileSync(cooldownPath,'utf8'));
 if(Number.isSafeInteger(cooldown.not_before_unix_ms)&&cooldown.not_before_unix_ms>Date.now())throw Error('MCP refresh cooldown active; no network request');
}
const operatorNames=['native_catalog','native_tool','codex_sessions','codex_attach','codex_history','codex_tools','codex_execute','codex_answer','codex_report','codex_finish','codex_status','codex_stop','codex_handoff','codex_native_summary'];
const expectedNames=info.expected_tools??(info.mode==='operator'?operatorNames:['native_catalog','native_tool']);
if(!Array.isArray(expectedNames)||new Set(expectedNames).size!==expectedNames.length||!expectedNames.every(v=>typeof v==='string'))throw Error('explicit_tool_catalog_invalid');
const expected=expectedNames.length;
if(typeof info.connector_reference!=='string'||!info.connector_reference.trim())throw Error('explicit_connector_reference_required');
const prior=JSON.parse(readFileSync(info.connector_reference,'utf8')).matches;
const connectorName=info.connector_name??prior[0]?.name;
if(prior.length!==1||typeof connectorName!=='string'||!connectorName.trim()||connectorName.length>160||prior[0].name!==connectorName)throw Error('exact_owned_connector_required');
if(typeof info.desktop_session!=='string'||!info.desktop_session.trim())throw Error('explicit_desktop_session_required');
const id=prior[0].connector_id;
const attemptDir=directory+'/refresh-'+Date.now();mkdirSync(attemptDir,{mode:0o700});
const c=await connectDesktop(info.desktop_session,undefined,{app:true});
try {
 for(let attempt=1;attempt<=3;attempt++) {
  const slot='__d3MetadataRefresh_'+attempt+'_'+Date.now();
  await c.evaluate(`globalThis[${JSON.stringify(slot)}]={pending:true};void(async()=>{
   const responseFailure=${responseFailure.toString()};
   const readConnectorWhenReady=${readConnectorWhenReady.toString()};
   const api=(await import('app://-/assets/app-shared-2d77298f0100.js')).$wt,id=${JSON.stringify(id)};
   const options={surface:'connector_setup',parameters:{path:{connector_id:id},query:{include_actions:true}}};
   const before=await readConnectorWhenReady(()=>api.safeGet('/aip/connectors/{connector_id}',options));
   if(before.name!==${JSON.stringify(connectorName)}||before.tunnel_id!==${JSON.stringify(prior[0].tunnel_id)})throw Error('owned_connector_changed');
   const {link}=await api.safeGet('/aip/connectors/{connector_id}/link',{surface:'settings',parameters:{path:{connector_id:id}},additionalHeaders:{'OAI-Product-Sku':'CONNECTOR_SETTING'}});
   if(!link||link.connector_id!==id||link.auth_status==='REAUTH_REQUIRED')return {ok:false,reauthRequired:link?.auth_status==='REAUTH_REQUIRED',before};
   let result,error;
   const started=new Date().toISOString();
   try {result=await api.safePost('/aip/connectors/mcp/refresh_actions',{surface:'settings',requestBody:{link_id:link.id},additionalHeaders:{'OAI-Product-Sku':'CONNECTOR_SETTING'},retry:'never'});}
   catch(e){error={keys:Object.getOwnPropertyNames(e),name:e?.name,...responseFailure(e),retryAtMs:e?.retryAtMs,message:e?.message,data:e?.data,body:e?.body,responseBody:e?.responseBody};}
   let cacheRefresh=false,probe,probeError;
   if(!error&&result?.error==null){
    // Match the app's Refresh onSuccess cache invalidation as well as its POST.
    // A direct metadata GET does not refresh plugin/app/system-hint queries.
    let scope=globalThis[Symbol.for('termux.codex.app.bridge.v1')]?.scope;
    if(!scope){
     let node=document.querySelector('[contenteditable=true]')||document.querySelector('button');
     while(node&&!Object.keys(node).some(k=>k.startsWith('__reactFiber')))node=node.parentElement;
     let fiber=node?.[Object.keys(node).find(k=>k.startsWith('__reactFiber'))];
     for(let i=0;fiber&&i<400&&!scope;i++,fiber=fiber.return)
      for(let h=fiber.memoizedState,j=0;h&&j<300;h=h.next,j++)
       if(h.memoizedState?.current?.queryClient?.invalidateQueries&&h.memoizedState.current.get)scope=h.memoizedState.current;
    }
    if(!scope?.queryClient?.invalidateQueries)throw Error('app_query_client_unavailable');
    const official=await import('app://-/assets/queries-9de9c2c4d846.js');official.t();await official.i(scope.queryClient);cacheRefresh=true;
    // Query the actual connector path, rather than infer health from metadata.
    // All probes are read-only and start no Chat, native work or model request.
    const samples=[];
    try{
     {
      const began=performance.now();
      const catalog=await api.safePost('/ecosystem/call_mcp',{surface:'other',retry:'never',requestBody:{app_uri:'connectors://'+id,launcher_host:'chatgpt',method:'tools/list',params:{}}});
      const names=catalog.tools?.map(v=>v.name).sort();
      if(names?.length!==${expected}||!${JSON.stringify(expectedNames)}.every(n=>names.includes(n)))throw Error('live_mcp_catalog_mismatch');
      let readCall;
      if(${expectedNames.includes('codex_sessions')}){
       readCall=await api.safePost('/ecosystem/call_mcp',{surface:'other',retry:'never',requestBody:{app_uri:'connectors://'+id,launcher_host:'chatgpt',method:'tools/call',params:{name:'codex_sessions',arguments:{action:'list'},_meta:{'openai/connector_id':id,'openai/link_id':link.id,'openai/action_name':'codex_sessions'}}}});
       const payload=readCall?.result??readCall?.content?.find(v=>v.type==='text')?.text;
       const value=typeof payload==='string'?JSON.parse(payload):readCall?.structuredContent;
       if(readCall?.isError!==false||!Array.isArray(value?.work))throw Error('read_only_mcp_probe_failed');
      }
      samples.push({sample:1,UTC:new Date().toISOString(),elapsed_ms:performance.now()-began,names,read_call:!!readCall});
     }
     probe={accepted:true,samples,model_requests:0,native_effects:0};
    }catch(e){probeError={name:e?.name,...responseFailure(e),retryAtMs:e?.retryAtMs,message:e?.message,data:e?.data};probe={accepted:false,samples,model_requests:0,native_effects:0};}
   }
   let after=await api.safeGet('/aip/connectors/{connector_id}',options);
   const matches=v=>v.id===before.id&&v.tunnel_id===before.tunnel_id&&Array.isArray(v.actions)&&v.actions.length===${expected}&&
    ${JSON.stringify(expectedNames)}.every(name=>v.actions.some(x=>x.name===name));
   if(!error&&result?.error==null)for(let i=0;i<5&&!matches(after);i++){await new Promise(resolve=>setTimeout(resolve,500*(i+1)));after=await api.safeGet('/aip/connectors/{connector_id}',options);}
   return {ok:!error&&result?.error==null&&matches(after)&&cacheRefresh&&probe?.accepted===true,started,finished:new Date().toISOString(),before,after,result,error,cacheRefresh,probe,probeError};
  })().then(value=>globalThis[${JSON.stringify(slot)}]={value},error=>globalThis[${JSON.stringify(slot)}]={failure:String(error)});true`);
  let value;
  for(let i=0;i<100;i++) {
   const state=await c.evaluate(`globalThis[${JSON.stringify(slot)}]`);
   if(state.failure){
    writeFileSync(attemptDir+'/attempt-'+attempt+'-failure-private.json',JSON.stringify(state,null,2),{mode:0o600});
    throw Error('Owned metadata/cache operation failed; private evidence retained');
   }
   if(state.value){value=state.value;break;}
   await new Promise(resolve=>setTimeout(resolve,750));
  }
  if(!value)throw Error('Refresh outcome unknown; reconcile before another attempt');
  writeFileSync(attemptDir+'/attempt-'+attempt+'-private.json',JSON.stringify(value,null,2),{mode:0o600});
  console.log(JSON.stringify({attempt,ok:value.ok,status:value.error?.upstream_status??value.error?.status??value.probeError?.upstream_status??value.probeError?.status??null,expected,actions:value.after?.actions?.map(x=>x.name),errorKeys:value.error?.keys??[],cache_refresh:value.cacheRefresh,live_mcp_samples:value.probe?.samples?.length??0,model_requests:0}));
  if(value.ok) {
   writeFileSync(directory+'/refresh-private.json',JSON.stringify({refresh:true,attempts:attempt,evidence:attemptDir,result:value.result,cache_refresh:value.cacheRefresh,live_mcp_probe:value.probe},null,2),{mode:0o600});
   process.exitCode=0;break;
  }
  // A definite metadata 424 response is safe to refresh again after a delay.
  // Unknown outcomes, auth/account changes and effectful tool calls are excluded.
  if([value.error?.status,value.error?.upstream_status,value.probeError?.status,value.probeError?.upstream_status].includes(429)){
   const offered=value.error?.retryAtMs??value.probeError?.retryAtMs;
   writeFileSync(cooldownPath,JSON.stringify({not_before_unix_ms:Math.max(Date.now()+60000,Number.isSafeInteger(offered)?offered:0),cause:'upstream_429',automatic_retry:false,model_requests:0},null,2),{mode:0o600});
   throw Error('MCP upstream rate limited; no refresh or model retry');
  }
  if(value.probeError)throw Error('Actual MCP readiness probe failed; no model request submitted; private evidence retained');
  if(value.error?.status!==424||attempt===3)throw Error('Owned connector metadata refresh did not succeed');
  await new Promise(resolve=>setTimeout(resolve,attempt*3000));
 }
} finally {c.close();}
