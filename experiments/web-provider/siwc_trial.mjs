// Independent public OAuth qualification. Vendor OAuth verifies OIDC/PKCE;
// this trial uses the official guide's owner-only file storage, not the
// devkit's high-level encrypted ConnectionStore or a fake encryption provider.
import { randomUUID, randomBytes } from 'node:crypto';
import { constants, lstatSync, fstatSync, openSync, closeSync, readFileSync, writeFileSync, mkdirSync, renameSync, unlinkSync, fsyncSync, existsSync } from 'node:fs';
import { createRequire } from 'node:module';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { spawn, spawnSync } from 'node:child_process';

export class PrivateStore {
  constructor(root) {
    this.root=resolve(root);mkdirSync(this.root,{recursive:true,mode:0o700});
    const s=lstatSync(this.root);
    if(!s.isDirectory()||s.isSymbolicLink()||s.uid!==process.getuid()||(s.mode&0o077))throw Error('unsafe_storage');
  }
  read(name) {
    if(!/^[a-z-]+\.json$/.test(name))throw Error('invalid_record');
    const p=join(this.root,name);
    let fd;try{fd=openSync(p,constants.O_RDONLY|constants.O_NOFOLLOW|constants.O_NONBLOCK);}catch(e){if(e.code==='ENOENT')return undefined;throw e;}
    try {
      const s=fstatSync(fd);
      if(!s.isFile()||s.uid!==process.getuid()||(s.mode&0o077)||s.size>1024*1024)throw Error('unsafe_record');
      return JSON.parse(readFileSync(fd,'utf8'));
    } finally {closeSync(fd);}
  }
  write(name,value) {
    if(!/^[a-z-]+\.json$/.test(name))throw Error('invalid_record');
    // Refuse unsafe existing records before replacing anything.
    this.read(name);
    const p=join(this.root,name),tmp=join(this.root,'.record-'+randomBytes(16).toString('hex'));
    const fd=openSync(tmp,constants.O_CREAT|constants.O_EXCL|constants.O_WRONLY|constants.O_NOFOLLOW,0o600);
    try {writeFileSync(fd,JSON.stringify(value)+'\n');fsyncSync(fd);} finally {closeSync(fd);}
    try {renameSync(tmp,p);const dir=openSync(this.root,constants.O_RDONLY);try{fsyncSync(dir);}finally{closeSync(dir);}}
    finally {if(existsSync(tmp))unlinkSync(tmp);}
  }
}

export function publicStatus(connection, phase, models=[], chosenModel=null, proof=null, errorCode=null) {
  return {phase,connected:connection?.status==='connected',sharing:connection?.status==='connected'&&!connection?.pendingRefresh&&!!connection?.credentials&&connection.scopes.includes('chatgpt.tokens.use.direct'),models:models.map(x=>({slug:x.slug,displayName:x.displayName})),chosenModel,proof,errorCode};
}

export function validateConnection(value) {
  if(value===undefined)return;
  if(!value||value.version!==1||!/^[-_a-zA-Z0-9]{1,200}$/.test(value.clientId)||value.clientId==='dynamic_agent_client'||!['connected','disconnected','reauth_required'].includes(value.status)||!Array.isArray(value.scopes)||value.scopes.some(x=>typeof x!=='string'))throw Error('invalid_connection');
  if(value.status==='connected'&&(!value.subject||typeof value.subject!=='string'||!value.credentials||typeof value.credentials.accessToken!=='string'||!value.credentials.accessToken||!Number.isFinite(value.credentials.expiresAt)))throw Error('invalid_connection');
}

export function renderLanding(nonce) {
  return String.raw`<!doctype html><html lang="ko"><meta name="viewport" content="width=device-width, initial-scale=1"><title>ChatGPT 구독 연결 시험</title><style>body{margin:0;background:#f5f6f8;color:#15202b;font:17px system-ui}main{max-width:540px;margin:8vh auto;padding:24px}h1{font-size:27px}p{line-height:1.6}button{width:100%;padding:16px;border:0;border-radius:14px;background:#15202b;color:white;font:inherit}pre{white-space:pre-wrap;background:white;padding:16px;border-radius:14px;font:14px system-ui}a{color:#315cc2}</style><main><h1>ChatGPT 구독 연결</h1><p>이 기기의 독립된 시험 앱 <b>codex-termux-trial</b>을 연결합니다. 동의하면 구독 사용 설정에 따라 짧은 모델 응답과 Codex 도구 실행을 시험합니다.</p><button id="connect">Continue with ChatGPT</button><p><a href="https://chatgpt.com/settings/usage">사용량·연결 권한 관리</a></p><pre id="status">준비됨</pre></main><script nonce="${nonce}">const b=document.querySelector('#connect'),s=document.querySelector('#status');b.onclick=async()=>{b.disabled=true;await fetch('./connect',{method:'POST'});};async function poll(){try{const x=await(await fetch('./status')).json();b.disabled=['connecting','models','model_probe','core_probe'].includes(x.phase);s.textContent='상태: '+x.phase+'\n구독 사용 동의: '+(x.sharing?'확인됨':'대기')+'\n선택 모델: '+(x.chosenModel||'대기')+'\n실제 모델 응답: '+(x.proof?.realModelReplyAccepted?'성공':'대기')+'\n실제 Codex 도구 실행: '+(x.proof?.realCoreToolLoopAccepted?'성공':'대기')+(x.errorCode?'\n오류: '+x.errorCode:'');}catch{}setTimeout(poll,1500);}poll();</script></html>`;
}

export async function serve({sdk,root,prefix,coreProbe,seconds=1800}) {
  process.umask(0o077);
  const store=new PrivateStore(root);
  const lifecycle=new AbortController();let coreChild,authController;
  const require=createRequire(pathToFileURL(join(sdk,'package.json')));
  const release=await require('proper-lockfile').lock(root,{realpath:true,stale:30000,update:5000,retries:0,onCompromised:()=>{lifecycle.abort();authController?.abort();coreChild?.kill('SIGINT');process.exit(2);}});
  let host=store.read('host.json');if(!host){host={id:'urn:uuid:'+randomUUID()};store.write('host.json',host);}
  if(!/^urn:uuid:[0-9a-f-]{36}$/.test(host.id))throw Error('invalid_host');
  const oauth=await import(pathToFileURL(join(sdk,'packages/local/src/oauth.ts')).href);
  const {listModels}=await import(pathToFileURL(join(sdk,'packages/local/src/models.ts')).href);
  const {streamResponse}=await import(pathToFileURL(join(sdk,'packages/local/src/responses.ts')).href);
  const capability=randomBytes(32).toString('hex');
  let connection=store.read('connection.json'),phase='ready',models=[],chosenModel=null,proof=null,errorCode=null,busy=false;
  validateConnection(connection);
  const appName='codex-termux-trial';
  const headers={'Cache-Control':'no-store','Referrer-Policy':'no-referrer','X-Frame-Options':'DENY','X-Content-Type-Options':'nosniff'};
  const status=()=>publicStatus(connection,phase,models,chosenModel,proof,errorCode);
  const saveStatus=()=>{if(!lifecycle.signal.aborted)store.write('status.json',status());};
  const notify=()=>{
    const action=[prefix+'/bin/termux-am-socket',`start -a android.intent.action.VIEW -d http://127.0.0.1:${server.port}/${capability}/`];
    // shell quoting is required for Termux:API's action string, including spaces.
    const quote=s=>"'"+s.replaceAll("'","'\\''")+"'";
    const r=spawnSync(prefix+'/bin/termux-notification',['--id','codex-plan-login','--group','codex-web-experiment','--title','공식 ChatGPT 구독 연결','--content','한 번 터치해 로그인·사용 동의를 진행하세요. 기존 Codex 프로필과 별도입니다.','--action',action.map(quote).join(' '),'--priority','high'],{timeout:20000,stdio:'ignore'});
    if(r.status!==0)throw Error('notification_failed');
  };
  async function verify() {
    if(!connection?.credentials||!connection.scopes.includes('chatgpt.tokens.use.direct')){phase='consent_required';saveStatus();return;}
    if(connection.pendingRefresh||connection.credentials.expiresAt<Date.now()+180000) {
      connection=await oauth.refreshConnection(connection,AbortSignal.any([lifecycle.signal,AbortSignal.timeout(30000)]),async rotation=>{
        lifecycle.signal.throwIfAborted();
        connection={...connection,pendingRefresh:rotation};store.write('connection.json',connection);
      });store.write('connection.json',connection);
    }
    phase='models';saveStatus();
    models=await listModels(connection.credentials.accessToken,AbortSignal.any([lifecycle.signal,AbortSignal.timeout(30000)]));
    chosenModel=models.find(x=>x.slug==='gpt-6.1-sol')?.slug??models[0]?.slug;
    if(!chosenModel)throw Error('no_models');
    phase='model_probe';saveStatus();
    const nonce='PLAN_'+randomBytes(12).toString('hex');
    const result=await streamResponse(connection.credentials.accessToken,{model:chosenModel,input:'Reply with exactly '+nonce+'. No tools and no additional text.'},AbortSignal.any([lifecycle.signal,AbortSignal.timeout(120000)]));
    proof={realModelReplyAccepted:result.text.trim()===nonce,realCoreToolLoopAccepted:false};
    if(!proof.realModelReplyAccepted)throw Error('unexpected_model_reply');
    phase='core_probe';saveStatus();
    const child=spawn(prefix+'/bin/python',['-B',coreProbe,'--connection',join(root,'connection.json'),'--model',chosenModel,'--report',join(root,'core-proof.json'),'--app-name',appName],{env:{HOME:process.env.HOME,PREFIX:prefix,PATH:prefix+'/bin',TMPDIR:process.env.TMPDIR,LANG:'C.UTF-8'},stdio:['ignore','pipe','pipe']});
    coreChild=child;
    // Never forward child stderr or model/tool bodies into chat/browser logs.
    child.stdout.resume();child.stderr.resume();
    const exit=await new Promise((accept,reject)=>{child.once('error',()=>reject(Error('core_start_failed')));child.once('exit',accept);});
    coreChild=null;const core=store.read('core-proof.json');
    proof={...proof,realCoreToolLoopAccepted:exit===0&&core?.realCoreToolLoopAccepted===true,coreExit:exit};
    phase=proof.realCoreToolLoopAccepted?'complete':'core_incomplete';saveStatus();
  }
  async function signin() {
    if(busy)return;busy=true;phase='connecting';errorCode=null;saveStatus();
    authController=new AbortController();const timer=setTimeout(()=>authController.abort(),15*60*1000);
    try {
      const pending=store.read('registration.json');
      const previous=connection??(pending?{version:1,clientId:pending.clientId,status:'disconnected',scopes:[],savedAt:pending.savedAt}:undefined);
      const config={appName,redirectPort:0,sendHostId:true,openBrowser:async url=>{
        const u=new URL(url);if(u.origin!=='https://auth.openai.com'||u.pathname!=='/api/accounts/authorize')throw Error('untrusted_authorization');
        const r=spawnSync(prefix+'/bin/termux-am-socket',['start -a android.intent.action.VIEW -d '+url],{timeout:20000,stdio:'ignore'});
        if(r.status!==0)throw Error('browser_open_failed');
      }};
      connection=await oauth.authorize(config,previous,host.id,AbortSignal.any([lifecycle.signal,authController.signal]),{reconsent:previous?.status==='connected'&&!previous.scopes.includes('chatgpt.tokens.use.direct'),onRegistration:async clientId=>{lifecycle.signal.throwIfAborted();store.write('registration.json',{clientId,savedAt:new Date().toISOString()});}});
      lifecycle.signal.throwIfAborted();validateConnection(connection);
      store.write('connection.json',connection);phase='connected';saveStatus();
      await verify();
    } catch(e) {errorCode=typeof e?.code==='string'?e.code:/^[a-z_]+$/.test(e?.message??'')?e.message:'trial_failed';phase='failed';saveStatus();}
    finally {clearTimeout(timer);busy=false;}
  }
  const server=Bun.serve({hostname:'127.0.0.1',port:0,async fetch(request){
    const u=new URL(request.url),base='http://127.0.0.1:'+server.port;
    if(u.pathname!==`/${capability}/`&&u.pathname!==`/${capability}/status`&&u.pathname!==`/${capability}/connect`)return new Response('Not found',{status:404,headers});
    if(request.method==='POST') {
      if(request.headers.get('origin')!==base||u.pathname!==`/${capability}/connect`)return new Response('Forbidden',{status:403,headers});
      if(busy)return new Response('Already running',{status:409,headers});
      void signin();return new Response(null,{status:202,headers});
    }
    if(request.method!=='GET')return new Response(null,{status:405,headers});
    if(u.pathname.endsWith('/status'))return Response.json(status(),{headers});
    const nonce=randomBytes(16).toString('hex');
    const html=renderLanding(nonce);
    return new Response(html,{headers:{...headers,'Content-Type':'text/html;charset=utf-8','Content-Security-Policy':`default-src 'none'; style-src 'unsafe-inline'; script-src 'nonce-${nonce}'; connect-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'`}});
  }});
  saveStatus();store.write('viewer.json',{port:server.port,capability,pid:process.pid});notify();
  console.log(JSON.stringify({planConnectionReady:true,notificationSent:true,authenticated:connection?.status==='connected',production:false}));
  let stopping=false;
  const stop=async()=>{if(stopping)return;stopping=true;lifecycle.abort();authController?.abort();server.stop(true);if(coreChild){const child=coreChild;child.kill('SIGINT');await new Promise(done=>{child.once('exit',done);setTimeout(done,20000);});}spawnSync(prefix+'/bin/termux-notification-remove',['codex-plan-login'],{timeout:15000,stdio:'ignore'});await release();process.exit(0);};
  process.on('SIGTERM',stop);process.on('SIGINT',stop);setTimeout(stop,seconds*1000);
}

if(process.argv[1]&&resolve(process.argv[1])===resolve(new URL(import.meta.url).pathname)) {
  const args=Object.fromEntries(Array.from({length:(process.argv.length-2)/2},(_,i)=>[process.argv[2+i*2].replace(/^--/,''),process.argv[3+i*2]]));
  if(!args.sdk||!args.root||!args.prefix||!args['core-probe'])throw Error('sdk, root, prefix and core-probe required');
  await serve({sdk:resolve(args.sdk),root:resolve(args.root),prefix:resolve(args.prefix),coreProbe:resolve(args['core-probe']),seconds:Number(args.seconds??1800)});
}
