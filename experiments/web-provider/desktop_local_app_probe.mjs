// Inert signed-app gate/ownership proof: create and close a local placeholder,
// with zero model requests and no conversation contents returned or recorded.
import {readFileSync,writeFileSync} from 'node:fs';
import {DesktopLocalTransport} from './desktop_local_transport.mjs';
import {acquireDesktopApp} from './desktop_app_transport.mjs';
import {chatModelRoute} from './desktop_model_controls.mjs';
const [runtimePath,reportPath]=process.argv.slice(2);
const runtime=JSON.parse(readFileSync(runtimePath,'utf8'));
const transport=new DesktopLocalTransport(runtime.session,{maxDispatches:3});
const report={actualSignedApp:true,newChatSubmissions:0,providerActivated:false};
let lease,observer;
try{
 await transport.prepare();lease=transport.lease;observer=await acquireDesktopApp(runtime.session);
 report.registered=await lease.client.evaluate(`(()=>{const s=window[${JSON.stringify(lease.scopeKey)}];s.localProbe={client:s.native.client,api:s.native.api,saved:s.native.saved};return s.native.lanes.size===1&&s.native.lanes.has(${JSON.stringify(transport.lane)})&&s.nativeBudget.used===0;})()`);
 if(process.argv.includes('--dry-run')){
  await lease.client.evaluate(`(()=>{const s=window[${JSON.stringify(lease.scopeKey)}],p=s.localProbe,l=s.native.lanes.get(${JSON.stringify(transport.lane)});p.lane=l;p.baseStart=p.saved.startCompletionStream;p.baseStream=p.saved.streamPost;p.startCalls=0;p.apiCalls=0;
   p.saved.startCompletionStream=function(opts){p.startCalls++;p.request={model:opts.request?.model,roles:opts.request?.messages?.map(m=>m.author?.role),hasGateway:opts.request?.local_function_signatures?.some(f=>f.name===l.signature.name)};throw Error('desktop_native_dry_blocked');};
   p.saved.streamPost=function(){p.apiCalls++;throw Error('desktop_native_dry_blocked');};return true;})()`);
  const close=transport.close.bind(transport);transport.close=async()=>{};
  try{await transport.exchange({instructions:['Owned dry protocol fixture.'],messages:[{role:'user',content:'Say hello.'}],tools:[]},AbortSignal.timeout(15000),undefined,undefined,undefined,chatModelRoute({model:'chatgpt-desktop/gpt-6'}));}
  catch(error){report.dryOutcome=error.message;}
  finally{
   report.dry=await lease.client.evaluate(`(()=>{const p=window[${JSON.stringify(lease.scopeKey)}].localProbe;p.saved.startCompletionStream=p.baseStart;p.saved.streamPost=p.baseStream;return {startCalls:p.startCalls,apiCalls:p.apiCalls,request:p.request,failure:p.lane.submitFailure,actualStarts:p.lane.starts.size};})()`);
   transport.close=close;
  }
 }
 await transport.close();
 report.restored=await lease.client.evaluate(`(()=>{const s=window[${JSON.stringify(lease.scopeKey)}],p=s.localProbe;const ok=!s.native&&['prepareCompletionStream','startCompletionStream'].every(n=>p.client[n]===p.saved[n])&&p.api.streamPost===p.saved.streamPost&&s.nativeBudget.used===0;delete s.localProbe;return ok;})()`);
 report.accepted=report.registered&&report.restored;
 if(report.dry)report.accepted&&=report.dry.startCalls===1&&report.dry.apiCalls===0&&report.dry.actualStarts===0&&report.dry.request?.hasGateway&&report.dry.failure?.code==='desktop_native_dry_blocked';
}catch(error){report.accepted=false;report.error=/^[a-z_]+$/.test(error.message)?error.message:'local_app_probe_failed';}
finally{await transport.close().catch(()=>{});observer?.release();}
writeFileSync(reportPath,JSON.stringify(report,null,2)+'\n',{mode:0o600});
console.log(JSON.stringify(report));if(!report.accepted)process.exitCode=1;
