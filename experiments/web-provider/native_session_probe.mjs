// Read only owned new-chat capability/transport metadata; never persist page/account text.
import {pathToFileURL} from 'node:url';
import {join} from 'node:path';
import {writeFileSync} from 'node:fs';
const [candidate,descriptor,proof]=process.argv.slice(2);
const {connectNativeChromiumHost}=await import(pathToFileURL(join(candidate,'src/native-chromium-host.ts')).href);
const {CHATGPT_TEMPORARY_CHAT_URL,CHATGPT_COMPOSER_SELECTOR,detectChatGptAccountCapabilities,assertAuthenticatedChatGptPage}=await import(pathToFileURL(join(candidate,'src/chatgpt-session.ts')).href);
const {availableChatGptWebModelRoutes}=await import(pathToFileURL(join(candidate,'src/chatgpt-web-models.ts')).href);
const {browser,context}=await connectNativeChromiumHost(descriptor);
let page;const report={nativePersistentContext:true,documentStatuses:[],composerReady:false,captchaActions:0};
try{
 page=await context.newPage();
 page.on('response',response=>{if(response.request().resourceType()==='document'&&report.documentStatuses.length<8)report.documentStatuses.push(response.status());});
 await page.goto(CHATGPT_TEMPORARY_CHAT_URL,{waitUntil:'domcontentloaded',timeout:20000});
 try{await page.locator(CHATGPT_COMPOSER_SELECTOR).filter({visible:true}).first().waitFor({state:'visible',timeout:15000});report.composerCount=await page.locator(CHATGPT_COMPOSER_SELECTOR).filter({visible:true}).count();if(report.composerCount!==1)throw Error("Composer is not uniquely owned");await assertAuthenticatedChatGptPage(page);report.authenticated=true;report.composerReady=true;report.capabilities=await detectChatGptAccountCapabilities(page);const routes=availableChatGptWebModelRoutes(report.capabilities,true);const route=routes.find(row=>row.interactionMode==='automatic'&&row.legacy&&row.codexEffort==='low'&&!row.modelFamily);if(!route)throw Error('Current UI model route unavailable');report.currentUiModelSlug=route.slug;}
 catch{report.challengeTitle=/just a moment|attention required|verification/i.test(await page.title());report.challengeFrame=await page.locator('iframe[src*="challenges.cloudflare.com"]').count()>0;report.loginFields=await page.locator('input[type="email"],input[type="password"]').filter({visible:true}).count()>0;}
}catch{report.transportFailed=true;}
finally{await page?.close();await browser.close();writeFileSync(proof,JSON.stringify(report),{mode:0o600});}
console.log(JSON.stringify(report));
