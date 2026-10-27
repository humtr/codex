// Owned connector refresh plus actual backend MCP readiness. No model or screen input.
import {readFileSync,writeFileSync,mkdirSync,existsSync} from 'node:fs';
import {dirname} from 'node:path';
import {spawn} from 'node:child_process';
import {randomUUID} from 'node:crypto';
import {connectDesktop} from '../providers/chatgpt/desktop_cdp.mjs';

function responseFailure(error){
 let detail;
 try{detail=JSON.parse(error?.message??'').detail;}catch{}
 return {status:error?.status??error?.responseStatus,upstream_status:detail?.upstream_status??null};
}

// tools/list is the live producer; connector actions are the registered consumer.
// Matching names alone misses removed actions and missing cursor arguments.
function catalogAgreement(actions,tools){
 const canonical=value=>Array.isArray(value)?value.map(canonical):value&&typeof value==='object'?Object.fromEntries(Object.keys(value).sort().map(key=>[key,canonical(value[key])])):value;
 return Array.isArray(actions)&&Array.isArray(tools)&&actions.length===tools.length&&tools.every(tool=>{
  const matches=actions.filter(action=>action.name===tool.name);
  return matches.length===1&&matches[0].params?.type==='object'&&tool.inputSchema?.type==='object'&&JSON.stringify(canonical(matches[0].params))===JSON.stringify(canonical(tool.inputSchema));
 });
}

async function candidateTools(binary,state){
 if(typeof binary!=='string'||typeof state!=='string')throw Error('candidate_provider_required');
 const id=randomUUID(),request={jsonrpc:'2.0',id,method:'tools/list',params:{}};
 return await new Promise((resolve,reject)=>{
  const child=spawn(binary,['mcp-stdio','--state',state],{stdio:['pipe','pipe','ignore']});
  let text='',tools,settled=false;
  const fail=code=>{if(settled)return;settled=true;clearTimeout(timer);child.kill();reject(Error(code));};
  const timer=setTimeout(()=>fail('candidate_catalog_read_timeout'),30000);
  child.once('error',()=>fail('candidate_catalog_read_failed'));
  child.stdin.once('error',()=>fail('candidate_catalog_read_failed'));
  child.stdout.setEncoding('utf8');
  child.stdout.on('data',chunk=>{
   text+=chunk;
   if(Buffer.byteLength(text)>16*1024*1024)return fail('candidate_catalog_response_invalid');
   if(tools||!text.includes('\n'))return;
   let reply;try{reply=JSON.parse(text.slice(0,text.indexOf('\n')));}catch{return fail('candidate_catalog_response_invalid');}
   tools=reply.result?.tools;
   if(reply.id!==id||reply.error||!Array.isArray(tools)||tools.length===0||new Set(tools.map(tool=>tool.name)).size!==tools.length||tools.some(tool=>typeof tool.name!=='string'||tool.inputSchema?.type!=='object'))return fail('candidate_catalog_response_invalid');
   // EOF detaches the MCP observer, so keep stdin open until its reply arrives.
   child.stdin.end();
  });
  child.once('close',code=>{
   if(settled)return;
   if(code!==0||!tools)return fail('candidate_catalog_read_failed');
   if(text.slice(text.indexOf('\n')+1).trim())return fail('candidate_catalog_response_invalid');
   settled=true;clearTimeout(timer);resolve(tools);
  });
  child.stdin.write(JSON.stringify(request)+'\n');
 });
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
if(typeof info.connector_reference!=='string'||!info.connector_reference.trim())throw Error('explicit_connector_reference_required');
const prior=JSON.parse(readFileSync(info.connector_reference,'utf8')).matches;
const connectorName=info.connector_name??prior[0]?.name;
if(prior.length!==1||typeof connectorName!=='string'||!connectorName.trim()||connectorName.length>160||prior[0].name!==connectorName)throw Error('exact_owned_connector_required');
if(typeof info.desktop_session!=='string'||!info.desktop_session.trim())throw Error('explicit_desktop_session_required');
const candidate=await candidateTools(info.binary,info.state);
const expectedNames=candidate.map(tool=>tool.name);
const expectedActions=candidate.map(tool=>({name:tool.name,params:tool.inputSchema}));
const expected=expectedNames.length;
const id=prior[0].connector_id;
const attemptDir=directory+'/refresh-'+Date.now();mkdirSync(attemptDir,{mode:0o700});
const c=await connectDesktop(info.desktop_session,undefined,{app:true});
try {
 for(let attempt=1;attempt<=3;attempt++) {
  const slot='__d3MetadataRefresh_'+attempt+'_'+Date.now();
  await c.evaluate(`globalThis[${JSON.stringify(slot)}]={pending:true};void(async()=>{
   const responseFailure=${responseFailure.toString()};
   const catalogAgreement=${catalogAgreement.toString()};
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
   let cacheRefresh=false,probe,probeError,liveTools;
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
      liveTools=catalog.tools;
      const names=liveTools?.map(v=>v.name).sort();
      const candidateAgreement=catalogAgreement(${JSON.stringify(expectedActions)},liveTools);
      if(!candidateAgreement)throw Error('live_mcp_catalog_mismatch');
      let readCall;
      if(${expectedNames.includes('codex_sessions')}){
       readCall=await api.safePost('/ecosystem/call_mcp',{surface:'other',retry:'never',requestBody:{app_uri:'connectors://'+id,launcher_host:'chatgpt',method:'tools/call',params:{name:'codex_sessions',arguments:{action:'list'},_meta:{'openai/connector_id':id,'openai/link_id':link.id,'openai/action_name':'codex_sessions'}}}});
       const payload=readCall?.result??readCall?.content?.find(v=>v.type==='text')?.text;
       const value=typeof payload==='string'?JSON.parse(payload):readCall?.structuredContent;
       if(readCall?.isError!==false||!Array.isArray(value?.work))throw Error('read_only_mcp_probe_failed');
      }
      samples.push({sample:1,UTC:new Date().toISOString(),elapsed_ms:performance.now()-began,names,read_call:!!readCall,candidate_schema_agreement:candidateAgreement});
     }
     probe={accepted:true,samples,model_requests:0,native_effects:0};
    }catch(e){probeError={name:e?.name,...responseFailure(e),retryAtMs:e?.retryAtMs,message:e?.message,data:e?.data};probe={accepted:false,samples,model_requests:0,native_effects:0};}
   }
   let after=await api.safeGet('/aip/connectors/{connector_id}',options);
   const matches=v=>v.id===before.id&&v.tunnel_id===before.tunnel_id&&Array.isArray(v.actions)&&v.actions.length===${expected}&&
    ${JSON.stringify(expectedNames)}.every(name=>v.actions.some(x=>x.name===name))&&catalogAgreement(v.actions,liveTools);
   if(!error&&result?.error==null)for(let i=0;i<5&&!matches(after);i++){await new Promise(resolve=>setTimeout(resolve,500*(i+1)));after=await api.safeGet('/aip/connectors/{connector_id}',options);}
   if(probe?.accepted===true)for(const sample of probe.samples)sample.schema_agreement=catalogAgreement(after.actions,liveTools);
   return {ok:!error&&result?.error==null&&matches(after)&&cacheRefresh&&probe?.accepted===true,schema_error:probe?.accepted===true&&!catalogAgreement(after.actions,liveTools)?'registered_mcp_schema_mismatch':null,started,finished:new Date().toISOString(),before,after,result,error,cacheRefresh,probe,probeError};
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
  console.log(JSON.stringify({attempt,ok:value.ok,status:value.error?.upstream_status??value.error?.status??value.probeError?.upstream_status??value.probeError?.status??null,expected,actions:value.after?.actions?.map(x=>x.name),errorKeys:value.error?.keys??[],cache_refresh:value.cacheRefresh,live_mcp_samples:value.probe?.samples?.length??0,schema_error:value.schema_error,model_requests:0}));
  if(value.ok) {
   writeFileSync(directory+'/refresh-private.json',JSON.stringify({refresh:true,attempts:attempt,evidence:attemptDir,result:value.result,cache_refresh:value.cacheRefresh,candidate_tools:candidate,live_mcp_probe:value.probe},null,2),{mode:0o600});
   process.exitCode=0;break;
  }
  // A definite metadata 424 response is safe to refresh again after a delay.
  // Unknown outcomes, auth/account changes and effectful tool calls are excluded.
  if([value.error?.status,value.error?.upstream_status,value.probeError?.status,value.probeError?.upstream_status].includes(429)){
   const offered=value.error?.retryAtMs??value.probeError?.retryAtMs;
   writeFileSync(cooldownPath,JSON.stringify({not_before_unix_ms:Math.max(Date.now()+60000,Number.isSafeInteger(offered)?offered:0),cause:'upstream_429',automatic_retry:false,model_requests:0},null,2),{mode:0o600});
   throw Error('MCP upstream rate limited; no refresh or model retry');
  }
  if(value.schema_error)throw Error('Registered connector schema differs from live MCP input schemas; no model request submitted');
  if(value.probeError)throw Error('Actual MCP readiness probe failed; no model request submitted; private evidence retained');
  if(value.error?.status!==424||attempt===3)throw Error('Owned connector metadata refresh did not succeed');
  await new Promise(resolve=>setTimeout(resolve,attempt*3000));
 }
} finally {c.close();}
