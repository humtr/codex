// One-time login import into a private persistent manual-browser profile.
// No navigation, challenge solving, browser camouflage or credential output.
import {readFile,mkdir,writeFile,chmod,lstat} from 'node:fs/promises';
import {join,resolve} from 'node:path';
import {pathToFileURL} from 'node:url';
const [candidate,profile,stateFile]=process.argv.slice(2).map(value=>resolve(value));
if(process.platform!=='android')throw Error('Native Android required');
const parent=await lstat(profile);
if(!parent.isDirectory()||parent.isSymbolicLink()||(parent.mode&0o777)!==0o700)throw Error('Private profile required');
const marker=join(profile,'.login-imported');
try{if((await lstat(marker)).isFile())process.exit(0);}catch(error){if(error.code!=='ENOENT')throw error;}
if((await lstat(stateFile)).mode&0o077)throw Error('Private saved login required');
const state=JSON.parse(await readFile(stateFile,'utf8'));
const cookies=state.cookies.filter(cookie=>{
 const domain=cookie.domain.replace(/^\./,'');
 return (domain==='chatgpt.com'||domain.endsWith('.chatgpt.com')||domain==='openai.com'||domain.endsWith('.openai.com'))
  && !cookie.name.startsWith('cf_')&&!cookie.name.startsWith('__cf');
});
if(!cookies.length)throw Error('No saved account cookies');
const {chromium}=await import(pathToFileURL(join(candidate,'node_modules/playwright-core/index.mjs')).href);
const context=await chromium.launchPersistentContext(profile,{executablePath:process.env.PREFIX+'/lib/chromium/chrome',headless:true,args:['--no-sandbox','--disable-dev-shm-usage'],ignoreDefaultArgs:['--password-store=basic','--use-mock-keychain']});
try{await context.addCookies(cookies);}finally{await context.close();}
await writeFile(marker,JSON.stringify({accountLoginImported:true,challengeClearanceImported:false}),{mode:0o600});
await chmod(marker,0o600);
