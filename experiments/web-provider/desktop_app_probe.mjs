// Real signed-app concurrency/cancellation. Tiny non-Pro GPT-6 requests only.
import {randomBytes} from 'node:crypto';
import {writeFileSync} from 'node:fs';
import {DesktopAppTransport,closeOwnedAppConversations} from './desktop_app_transport.mjs';
const [session,reportPath]=process.argv.slice(2);
const payload=nonce=>({instructions:[],messages:[{role:'user',content:'Reply with exactly '+nonce+' and no other text.'}],tools:[]});
const nonces=[0,1].map(i=>'APP_'+i+'_'+randomBytes(8).toString('hex'));
const lanes=nonces.map(()=>new DesktopAppTransport(session)),times=[{},{}];
const responses=await Promise.all(lanes.map(async(lane,i)=>{
 const value=await lane.exchange(payload(nonces[i]),AbortSignal.timeout(130000),({acceptedAt})=>{times[i].submitted=acceptedAt;});
 times[i].completed=Date.now();return value.type==='final'&&value.text?.trim()===nonces[i];
}));
const overlapping=Math.max(...times.map(t=>t.submitted))<Math.min(...times.map(t=>t.completed));
const cancelled=new DesktopAppTransport(session),survivor=new DesktopAppTransport(session),abort=new AbortController(),survivorNonce='SURVIVE_'+randomBytes(8).toString('hex');
const cancelling=cancelled.exchange(payload('CANCEL_'+randomBytes(8).toString('hex')),abort.signal,()=>abort.abort()).then(()=>false,e=>abort.signal.aborted&&e.name==='AbortError');
const surviving=survivor.exchange(payload(survivorNonce),AbortSignal.timeout(130000));
const [cancelledOwn,value]=await Promise.all([cancelling,surviving]);
const survived=value.type==='final'&&value.text?.trim()===survivorNonce;
const report={actualSignedApp:true,internalSubmission:true,uiSendClicks:0,independentReplies:responses,overlappingGenerations:overlapping,submissionTimesFromApp:true,cancelledOwn,siblingSurvived:survived,proInference:false,accepted:responses.every(Boolean)&&overlapping&&cancelledOwn&&survived};
writeFileSync(reportPath,JSON.stringify(report),{mode:0o600});console.log(JSON.stringify(report));if(!report.accepted)process.exitCode=1;
await closeOwnedAppConversations(session);
