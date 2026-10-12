import {pathToFileURL} from 'node:url';
import {join} from 'node:path';
import {writeFileSync} from 'node:fs';
const [candidate,descriptor,proof]=process.argv.slice(2);
const {connectNativeChromiumHost}=await import(pathToFileURL(join(candidate,'src/native-chromium-host.ts')).href);
const {CHATGPT_TEMPORARY_CHAT_URL,CHATGPT_COMPOSER_SELECTOR,assertAuthenticatedChatGptPage,assertTemporaryChatPage}=await import(pathToFileURL(join(candidate,'src/chatgpt-session.ts')).href);
const {browser,context}=await connectNativeChromiumHost(descriptor);
let page;const report={documentStatuses:[],composerCount:0,authenticated:false,temporary:false,captchaActions:0};
try{
 page=await context.newPage();page.on('response',response=>{if(response.request().resourceType()==='document'&&report.documentStatuses.length<8)report.documentStatuses.push(response.status());});
 await page.goto(CHATGPT_TEMPORARY_CHAT_URL,{waitUntil:'domcontentloaded',timeout:20000});
 await page.waitForTimeout(1500);
 report.composerCount=await page.locator(CHATGPT_COMPOSER_SELECTOR).filter({visible:true}).count();
 try{await assertAuthenticatedChatGptPage(page);report.authenticated=true;}catch{}
 try{await assertTemporaryChatPage(page);report.temporary=true;}catch{}
 report.challengeTitle=/just a moment|attention required|verification/i.test(await page.title());
 report.loginFields=await page.locator('input[type="email"],input[type="password"]').filter({visible:true}).count()>0;
}catch{report.transportFailed=true;}
finally{await browser.close();writeFileSync(proof,JSON.stringify(report),{mode:0o600});}
console.log(JSON.stringify(report));
