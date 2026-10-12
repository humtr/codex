import {readFileSync,writeFileSync,chmodSync,symlinkSync,unlinkSync} from 'node:fs';
import {pathToFileURL} from 'node:url';
import {join} from 'node:path';
const [candidate,root]=process.argv.slice(2);
const state=JSON.parse(readFileSync(join(root,'host.json'),'utf8'));
const headers={Authorization:'Bearer '+state.token};
const {readNativeChromiumDescriptor,connectNativeChromiumHost}=await import(pathToFileURL(join(candidate,'src/native-chromium-host.ts')).href);
readNativeChromiumDescriptor(join(root,'host.json'));
const stale={...state,host:{...state.host,start:'0'}};
writeFileSync(join(root,'host.json'),JSON.stringify(stale),{mode:0o600});
let staleRefused=false;try{readNativeChromiumDescriptor(join(root,'host.json'));}catch{staleRefused=true;}
writeFileSync(join(root,'host.json'),JSON.stringify(state),{mode:0o600});
if(!staleRefused)throw Error('Stale owner was admitted');
chmodSync(join(root,'host.json'),0o644);let modeRefused=false;try{readNativeChromiumDescriptor(join(root,'host.json'));}catch{modeRefused=true;}finally{chmodSync(join(root,'host.json'),0o600);}
if(!modeRefused)throw Error('Public descriptor was admitted');
symlinkSync(join(root,'host.json'),join(root,'link.json'));let symlinkRefused=false;try{readNativeChromiumDescriptor(join(root,'link.json'));}catch{symlinkRefused=true;}finally{unlinkSync(join(root,'link.json'));}
if(!symlinkRefused)throw Error('Symlink descriptor was admitted');
const report={actualNativeChromium:true,authenticationUsed:false,staleOwnerRefused:staleRefused,publicDescriptorRefused:modeRefused,symlinkDescriptorRefused:symlinkRefused};
for(const [key,extra] of [['unauthorized',{}],['wrongBearer',{Authorization:'Bearer wrong'}],['browserOrigin',{...headers,Origin:'https://example.invalid'}]]){
 const response=await fetch(state.endpoint+'/json/version',{headers:extra});
 if(response.status!==401)throw Error('Native CDP authorization refusal failed');report[key+'Refused']=true;
}
// Reserve the upgrade before its open callback: two simultaneous clients must
// not both acquire the single persistent pipe root session.
const competitors=[];
const outcomes=await Promise.all([0,1].map(()=>new Promise(resolve=>{
 const socket=new WebSocket(state.endpoint.replace(/^http:/,'ws:')+'/cdp',{headers});
 competitors.push(socket);
 const timer=setTimeout(()=>resolve('timeout'),2000);
 socket.onopen=()=>{clearTimeout(timer);resolve('open');};
 socket.onerror=()=>{clearTimeout(timer);resolve('refused');};
})));
for(const socket of competitors)socket.close();
if(outcomes.filter(x=>x==='open').length!==1||outcomes.filter(x=>x==='refused').length!==1)throw Error('Concurrent CDP clients shared the persistent root session');
report.concurrentCdpOwnerRefused=true;
const {chromium}=await import(pathToFileURL(join(candidate,'node_modules/playwright-core/index.mjs')).href);
try{await chromium.connectOverCDP(state.endpoint,{headers:{Authorization:'Bearer wrong'},timeout:1000});throw Error('Bad credentials connected');}catch(error){if(error.message==='Bad credentials connected')throw error;report.badCdpConnectionRefused=true;}
const documentServer=Bun.serve({hostname:'127.0.0.1',port:0,fetch(){return new Response('<input id=network><output id=result></output><script>network.oninput=()=>result.textContent=network.value</script>',{headers:{'content-type':'text/html'}});}});
const {browser}=await connectNativeChromiumHost(join(root,'host.json'));
try{
 const context=browser.contexts()[0];if(!context)throw Error('Persistent native context missing');
 const page=await context.newPage();
 await page.setContent('<main><input id="value"><button id="button">Set</button><output id="result"></output></main><script>button.onclick=()=>result.textContent=value.value</script>');
 await page.locator('#value').fill('NATIVE_OWNED_DOM');await page.locator('#button').click();
 if(await page.locator('#result').textContent()!=='NATIVE_OWNED_DOM')throw Error('Native CDP DOM roundtrip failed');
 report.actualDomReadbackAccepted=true;report.navigatorWebdriver=await page.evaluate(()=>navigator.webdriver);
 await context.addCookies([{name:'owned-proof',value:'synthetic',domain:'example.invalid',path:'/'}]);
 await page.close();
}finally{await browser.close();}
await new Promise(resolve=>setTimeout(resolve,300));
const {browser:browser2}=await connectNativeChromiumHost(join(root,'host.json'));
try{
 const context=browser2.contexts()[0];
 if(!(await context.cookies('https://example.invalid')).some(c=>c.name==='owned-proof'&&c.value==='synthetic'))throw Error('Persistent native context was lost');
 report.persistentContextRetained=true;
 const page=await context.newPage();await page.setContent('<input id=second><output id=echo></output><script>second.oninput=()=>echo.textContent=second.value</script>');await page.locator('#second').fill('SECOND_REAL_DOM');if(await page.locator('#echo').textContent()!=='SECOND_REAL_DOM')throw Error('Reconnected default context is paused or unusable');
 await page.goto('http://127.0.0.1:'+documentServer.port,{waitUntil:'domcontentloaded',timeout:5000});await page.locator('#network').fill('RECONNECTED_NETWORK_DOM');if(await page.locator('#result').textContent()!=='RECONNECTED_NETWORK_DOM')throw Error('Reconnected navigation is unusable');
 report.actualReconnectNetworkDomAccepted=true;report.actualReconnectDomAccepted=true;report.actualReconnectAccepted=true;await page.close();documentServer.stop(true);
}finally{await browser2.close();}
writeFileSync(join(root,'proof.json'),JSON.stringify(report),{mode:0o600});console.log(JSON.stringify(report));
