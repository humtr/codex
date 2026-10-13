import { test } from 'bun:test';
import assert from 'node:assert/strict';
import { mkdtempSync, rmSync, chmodSync, statSync, symlinkSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
import { PrivateStore, publicStatus, validateConnection, renderLanding } from './siwc_trial.mjs';

test('actual Android record publication replaces credentials without hardlinks',()=>{
  const root=mkdtempSync(join(process.env.TMPDIR,'siwc-record-'));
  try {const s=new PrivateStore(root);s.write('connection.json',{old:true});s.write('connection.json',{new:true});assert.deepEqual(s.read('connection.json'),{new:true});assert.equal(statSync(join(root,'connection.json')).mode&0o777,0o600);}
  finally {rmSync(root,{recursive:true,force:true});}
});
test('unsafe or broken record symlinks cannot be replaced or read',()=>{
  const root=mkdtempSync(join(process.env.TMPDIR,'siwc-link-'));
  try {const s=new PrivateStore(root);s.write('sentinel.json',{keep:true});symlinkSync(join(root,'sentinel.json'),join(root,'connection.json'));assert.throws(()=>s.read('connection.json'));assert.throws(()=>s.write('connection.json',{destroy:true}));assert.deepEqual(s.read('sentinel.json'),{keep:true});symlinkSync(join(root,'missing.json'),join(root,'registration.json'));assert.throws(()=>s.write('registration.json',{}));}
  finally {rmSync(root,{recursive:true,force:true});}
});
test('world-readable credentials and nonprivate storage refuse before mutation',()=>{
  const root=mkdtempSync(join(process.env.TMPDIR,'siwc-mode-'));
  try {const s=new PrivateStore(root);s.write('connection.json',{keep:true});chmodSync(join(root,'connection.json'),0o644);assert.throws(()=>s.read('connection.json'));assert.throws(()=>s.write('connection.json',{}));assert.deepEqual(JSON.parse(readFileSync(join(root,'connection.json'))),{keep:true});chmodSync(root,0o755);assert.throws(()=>new PrivateStore(root));}
  finally {rmSync(root,{recursive:true,force:true});}
});
test('status and interrupted refresh never disclose or admit stored tokens',()=>{
  const c={version:1,clientId:'synthetic_client',subject:'synthetic_subject',status:'connected',scopes:['chatgpt.tokens.use.direct'],credentials:{accessToken:'synthetic_access_secret',refreshToken:'synthetic_refresh_secret',expiresAt:Date.now()+3600000},profileIdToken:'synthetic_id_secret'};
  validateConnection(c);const safe=publicStatus(c,'ready');assert.equal(safe.sharing,true);assert(!JSON.stringify(safe).includes('synthetic_'));assert.equal(publicStatus({...c,pendingRefresh:{}},'refreshing').sharing,false);
  for(const change of [{clientId:'dynamic_agent_client'},{subject:''},{scopes:null},{credentials:{accessToken:'s',expiresAt:'bad'}}])assert.throws(()=>validateConnection({...c,...change}));
});
test('a FIFO cannot block a private-record read or overwrite',()=>{
  const root=mkdtempSync(join(process.env.TMPDIR,'siwc-fifo-'));
  try {const s=new PrivateStore(root);const r=spawnSync(process.env.PREFIX+'/bin/mkfifo',[join(root,'connection.json')]);assert.equal(r.status,0);assert.throws(()=>s.read('connection.json'));assert.throws(()=>s.write('connection.json',{}));assert(statSync(join(root,'connection.json')).isFIFO());}
  finally {rmSync(root,{recursive:true,force:true});}
});
test('the delivered landing page has executable JavaScript and its CSP nonce',()=>{
  const nonce='owned_test_nonce',html=renderLanding(nonce);
  const [,script]=html.match(/<script nonce="owned_test_nonce">([\s\S]*?)<\/script>/);
  const r=spawnSync(process.env.PREFIX+'/bin/node',['--check','--input-type=module'],{input:script,encoding:'utf8'});
  assert.equal(r.status,0,r.stderr);assert(html.includes('Continue with ChatGPT'));
});
