// Account-free public diagnostic only. No production CAPTCHA action or login.
import {readFileSync,writeFileSync} from 'node:fs';
import {join} from 'node:path';
import {pathToFileURL} from 'node:url';
const [candidate,root,executable]=process.argv.slice(2);
const {connectNativeChromiumHost}=await import(pathToFileURL(join(candidate,'src/native-chromium-host.ts')).href);
const connection=executable?{browser:await (await import(pathToFileURL(join(candidate,'node_modules/playwright-core/index.mjs')).href)).chromium.launch({executablePath:executable,headless:false,ignoreDefaultArgs:['--enable-unsafe-swiftshader'],args:['--no-sandbox','--disable-dev-shm-usage']})}:await connectNativeChromiumHost(join(root,'host.json'));
const browser=connection.browser;const context=connection.context??await browser.newContext();
let page;const result={authenticationUsed:false,productionChallengeActions:0,documentStatuses:[]};
try{
 result.browserVersion=browser.version();
 result.unsafeSoftwareFallbackIgnored=Boolean(executable);
 page=await context.newPage();page.on('response',response=>{if(response.request().resourceType()==='document')result.documentStatuses.push(response.status());});
 await page.goto('https://debug.challenges.cloudflare.com/',{waitUntil:'domcontentloaded',timeout:20000});
 await page.waitForTimeout(3000);
 result.publicCompatibilityToolLoaded=new URL(page.url()).hostname==='debug.challenges.cloudflare.com'&&!/just a moment|attention required/i.test(await page.title());
 result.browserCapabilities=await page.evaluate(()=>{
  const canvas=document.createElement('canvas');
  return {javascript:true,secureContext:isSecureContext,cookiesEnabled:navigator.cookieEnabled,webCrypto:typeof crypto.subtle==='object',webgl:Boolean(canvas.getContext('webgl')),webgl2:Boolean(document.createElement('canvas').getContext('webgl2')),automationReported:navigator.webdriver};
 });
 // Only named diagnostic statuses, never IP addresses, fingerprints or page body.
 result.compatibilityStatusCounts=await page.evaluate(()=>{
  const cells=[...document.querySelectorAll('td,[data-status]')].map(element=>(element.textContent??'').trim().toLowerCase());
  return {pass:cells.filter(s=>/^(pass|passed|supported|true|success)$/.test(s)).length,fail:cells.filter(s=>/^(fail|failed|unsupported|false|error)$/.test(s)).length};
 });
}catch{result.publicToolUnavailable=true;}
finally{await page?.close();await browser.close();writeFileSync(join(root,'proof.json'),JSON.stringify(result),{mode:0o600});}
console.log(JSON.stringify(result));
