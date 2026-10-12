// Bounded real-service probe. No CAPTCHA clicks, stealth or browser spoofing.
import {writeFileSync,readFileSync,lstatSync} from 'node:fs';
import {randomBytes} from 'node:crypto';
import {spawnSync} from 'node:child_process';
import {dirname,isAbsolute,join} from 'node:path';
import {fileURLToPath,pathToFileURL} from 'node:url';
const [candidate,root,dataRoot]=process.argv.slice(2);
if(process.platform!=='android'||![candidate,root,dataRoot].every(value=>value&&isAbsolute(value)))throw Error('Native Android and owned absolute roots required');
const profile=join(dataRoot,'browser/manual-profile');
if(!lstatSync(profile).isDirectory()||lstatSync(profile).isSymbolicLink()||(lstatSync(profile).mode&0o077))throw Error('Private owned experimental profile required');
const old=JSON.parse(readFileSync(root+'/account-view-process.json','utf8'));
try{
 const stat=readFileSync('/proc/'+old.pid+'/stat','utf8');
 if(stat.slice(stat.lastIndexOf(')')+1).trim().split(/\s+/)[19]===String(old.start)&&stat.slice(stat.lastIndexOf(')')+1).trim().split(/\s+/)[0]!=='Z')throw Error('Owned manual browser must close normally before profile reuse');
}catch(error){if(error.code!=='ENOENT')throw error;}
const {chromium}=await import(pathToFileURL(join(candidate,'node_modules/playwright-core/index.mjs')).href);
const {CHATGPT_COMPOSER_SELECTOR,CHATGPT_SEND_BUTTON_SELECTOR,CHATGPT_TEMPORARY_CHAT_URL,assertTemporaryChatPage}=await import(pathToFileURL(join(candidate,'src/chatgpt-session.ts')).href);
const source=dirname(fileURLToPath(import.meta.url));
const stat=readFileSync('/proc/'+process.pid+'/stat','utf8');
writeFileSync(root+'/account-view-process.json',JSON.stringify({pid:process.pid,image:process.execPath,start:stat.slice(stat.lastIndexOf(')')+1).trim().split(/\s+/)[19]}),{mode:0o600});
let context;let stage='launch';let report={ordinaryManualPreviouslyAccepted:true,automatedPersistentProbe:true};
try{
 context=await chromium.launchPersistentContext(profile,{executablePath:process.env.PREFIX+'/lib/chromium/chrome',headless:false,viewport:null,ignoreDefaultArgs:['--password-store=basic','--use-mock-keychain'],args:['--no-sandbox','--disable-dev-shm-usage','--window-position=0,0']});
 const page=context.pages()[0]??await context.newPage();
 const fit=spawnSync(process.env.PREFIX+'/bin/python3',['-B',join(source,'fit_browser.py'),root],{encoding:'utf8'});
 report.fitted=fit.status===0;
 report.documentStatuses=[];
 page.on('response',response=>{if(response.request().resourceType()==='document'&&report.documentStatuses.length<8)report.documentStatuses.push(response.status());});
 report.conversationStatuses=[];
 page.on('response',response=>{if(new URL(response.url()).pathname.includes('/conversation')&&report.conversationStatuses.length<8)report.conversationStatuses.push(response.status());});
 stage='temporary-page';
 await page.goto(CHATGPT_TEMPORARY_CHAT_URL,{waitUntil:'domcontentloaded',timeout:20000});
 stage='composer';
 const composer=page.locator(CHATGPT_COMPOSER_SELECTOR).filter({visible:true}).first();
 try{await composer.waitFor({state:'visible',timeout:15000});}
 catch{
  report.challengeVisible=await page.getByText(/verify you are human|verifying you are human|사람인지|인간인지/i).first().isVisible().catch(()=>false);
  report.challengeTitle=/just a moment|attention required|verification/i.test(await page.title().catch(()=>''));
  report.loginForm=await page.locator('input[type="email"],input[type="password"]').filter({visible:true}).count()>0;
  report.pageOnChatGpt=new URL(page.url()).origin==='https://chatgpt.com';
  report.pageTextLength=(await page.locator('body').innerText().catch(()=>'' )).length;
  report.challengeFrame=await page.locator('iframe[src*="challenges.cloudflare.com"]').count()>0;
  throw Error('Composer unavailable');
 }
 await assertTemporaryChatPage(page);report.composerReady=true;
 const nonce='WEB_TERMUX_'+randomBytes(8).toString('hex');
 stage='submit';
 await composer.fill('Reply with exactly '+nonce+'. No other content and no tool calls.');
 const submit=page.locator(CHATGPT_SEND_BUTTON_SELECTOR).filter({visible:true}).first();
 report.sendButtonPresent=await submit.count()>0;
 if(report.sendButtonPresent)await submit.click();else await composer.press('Enter');
 await page.waitForTimeout(1500);
 report.userMessageElements=await page.locator('[data-message-author-role="user"]').count();
 report.assistantMessageElements=await page.locator('[data-message-author-role="assistant"],article[data-turn="assistant"]').count();
 report.composerStillHasPrompt=(await composer.textContent().catch(()=>''))?.includes(nonce)===true;
 report.visibleDialogs=await page.locator('[role="dialog"]').filter({visible:true}).count();
 stage='reply';
 await page.waitForFunction(expected=>Array.from(document.querySelectorAll('[data-message-author-role="assistant"],article[data-turn="assistant"]')).some(element=>element.textContent?.includes(expected)),nonce,{timeout:60000});
 report.realModelReplyMatched=true;report.assistantMessageElements=await page.locator('[data-message-author-role="assistant"],article[data-turn="assistant"]').count();
}catch{report.failedStage=stage;report.realModelReplyMatched=false;}
finally{await context?.close();writeFileSync(root+'/persistent-model-proof.json',JSON.stringify(report),{mode:0o600});console.log(JSON.stringify(report));}
