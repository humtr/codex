// One ordinary-Chat metadata request. Pair with the metadata-only stdio tap.
// No native executor, automatic retry or production conversation is used.
import {createHash,randomBytes} from 'node:crypto';
import {readFileSync,writeFileSync} from 'node:fs';
import {acquireDesktopApp} from './desktop_app_transport.mjs';

export function installMetadataModelHooks(s,shared,marker,dryRun=false){
  if(s.mcpModelProbe)throw Error('owned_probe_already_running');
  const client=s.scope.get(s.m.oMn),api=shared.$wt;
  const p=s.mcpModelProbe={id:s.m.aCn(s.scope),marker:marker,messages:new Map(),starts:0,userMessages:0,toolMessages:0,terminals:[],controller:new AbortController(),saved:{},wrapped:{}};
  const owns=request=>request!=null&&(request.conversation_id!=null&&[p.id,p.serverId].includes(request.conversation_id)||request.messages?.some(m=>m.author?.role==='user'&&m.content?.parts?.some(t=>typeof t==='string'&&t.startsWith(p.marker+'\n'))));
  for(const name of ['prepareCompletionStream','startCompletionStream']){
   p.saved[name]=client[name];
   p.wrapped[name]=function(...args){
    const request=name==='startCompletionStream'?args[0]?.request:args[0];
    if(!owns(request))return p.saved[name].apply(this,args);
    // This is metadata qualification: do not expose Work/handoff executors.
    const owned={...request,local_function_signatures:[],local_function_names:[]};
    if(name==='prepareCompletionStream')return p.saved[name].call(this,owned,args[1]);
    if(dryRun){p.dryBlocked=true;throw Error('owned_metadata_dry_run');}
    const opts={...args[0],request:owned};
    for(const event of ['onDecodedEvent','onUpdate']){
     const original=opts[event];opts[event]=u=>{let m=u?.update?.message??u?.message;if(m?.message)m=m.message;if(m?.id&&m.author)p.messages.set(m.id,m);return original?.(u);};
    }
    const complete=opts.onComplete;opts.onComplete=u=>{p.terminals.push(u.reason);return complete?.(u);};
    const error=opts.onError;opts.onError=u=>{p.error=true;return error?.(u);};
    return p.saved[name].call(this,opts);
   };client[name]=p.wrapped[name];
  }
  p.saved.post=api.streamPost;
  p.wrapped.post=function(path,opts){
   let request;if(path==='/f/conversation')try{request=JSON.parse(opts?.body??'null');}catch{}
   if(!owns(request))return p.saved.post.call(this,path,opts);
   if(request.local_function_signatures?.length||request.local_function_names?.length)throw Error('owned_probe_executor_not_disabled');
   const before=opts.chatgptSse?.onBeforeRequestStart;
   return p.saved.post.call(this,path,{...opts,retry:undefined,chatgptSse:{...opts.chatgptSse,onBeforeRequestStart(r){
    if(p.starts>=1)throw Error('owned_probe_budget_exhausted');
    before?.(r);p.starts++;
    p.userMessages+=request.messages.filter(m=>m.author?.role==='user').length;
    p.toolMessages+=request.messages.filter(m=>m.author?.role==='tool').length;
   }}});
  };api.streamPost=p.wrapped.post;
  p.restore=()=>{for(const name of ['prepareCompletionStream','startCompletionStream'])if(client[name]===p.wrapped[name])client[name]=p.saved[name];if(api.streamPost===p.wrapped.post)api.streamPost=p.saved.post;};
  return p;
}

if(import.meta.main){
const [runtimePath,reportPath,connectorName]=process.argv.slice(2);
const dryRun=process.argv.includes('--dry-run');
if(!connectorName)throw Error('explicit_connector_name_required');
const runtime=JSON.parse(readFileSync(runtimePath,'utf8'));
const lease=await acquireDesktopApp(runtime.session);
const marker='owned_unbound_'+randomBytes(16).toString('hex');
let report={accepted:false,modelStarts:0,ordinaryChatModel:true,coreExecutionPermitted:false,dryRun,turnTokenSha256:createHash('sha256').update(marker).digest('hex')};
try{
 await lease.client.evaluate(`(async()=>{
  const s=window[${JSON.stringify(lease.scopeKey)}],shared=await import('app://-/assets/app-shared-2d77298f0100.js');
  const p=(${installMetadataModelHooks.toString()})(s,shared,${JSON.stringify(marker)},${dryRun});
  const prompt=p.marker+'\\nUse the installed app named '+${JSON.stringify(connectorName)}+' to call codex_tool_inventory exactly once with turn_token '+p.marker+'. This is an isolated metadata test. No native broker exists: an unbound/broker error is expected. Do not retry, run commands, search the web or use other apps. After that call reply exactly MCP_METADATA_FINISHED.';
  Promise.resolve(s.m.yRt(s.scope,{conversationId:p.id,prompt,model:'gpt-6',thinkingEffort:null,isTemporaryChat:false,requireResponseAcceptance:true,startupSignal:p.controller.signal,onServerThreadIdChange:id=>{p.serverId=id;}})).then(value=>{p.accepted=!!value;},()=>{p.error=true;});
  return true;
 })()`);
 const deadline=Date.now()+100000;
 while(Date.now()<deadline){
  const state=await lease.client.evaluate(`(()=>{
   const s=window[${JSON.stringify(lease.scopeKey)}],p=s.mcpModelProbe;
   for(const node of Object.values(s.scope.get(s.m.JRn,p.id)||{})){const m=node.message;if(m?.id&&m.author)p.messages.set(m.id,m);}
   const messages=[...p.messages.values()];
   return {modelStarts:p.starts,userMessages:p.userMessages,toolMessages:p.toolMessages,terminals:p.terminals,error:!!p.error,requestAccepted:!!p.accepted,dryBlocked:!!p.dryBlocked,
    finalMatched:messages.some(m=>m.author?.role==='assistant'&&m.recipient==='all'&&m.status==='finished_successfully'&&m.content?.parts?.some(t=>typeof t==='string'&&t.trim()==='MCP_METADATA_FINISHED')),
    toolMessagesObserved:messages.filter(m=>m.author?.role==='tool').length,
    assistantToolCallsObserved:messages.filter(m=>m.author?.role==='assistant'&&m.recipient&&m.recipient!=='all').length};
  })()`);
  report={...report,...state};
  if(state.error||state.terminals.length)break;
  await new Promise(resolve=>setTimeout(resolve,200));
 }
 report.accepted=dryRun?report.modelStarts===0&&report.dryBlocked:report.modelStarts===1&&report.userMessages===1&&report.toolMessages===0&&report.finalMatched&&!report.error&&report.terminals?.length===1&&report.terminals[0]==='done';
}catch{
 report.failure='owned_metadata_model_probe_failed';
 try{Object.assign(report,await lease.client.evaluate(`(()=>{const p=window[${JSON.stringify(lease.scopeKey)}].mcpModelProbe;return p?{modelStarts:p.starts,userMessages:p.userMessages,toolMessages:p.toolMessages}:{modelStarts:0};})()`));}
 catch{report.submissionUnknown=true;}
}finally{
 await lease.client.evaluate(`(()=>{const s=window[${JSON.stringify(lease.scopeKey)}],p=s.mcpModelProbe;if(p){p.controller.abort();try{s.m.eHt(s.scope,p.id);}finally{p.restore();delete s.mcpModelProbe;}}return true;})()`).catch(()=>{});
 lease.release();
}
writeFileSync(reportPath,JSON.stringify(report,null,2)+'\n',{mode:0o600});
console.log(JSON.stringify(report));
if(!report.accepted)process.exitCode=1;
}
