import {test,expect} from 'bun:test';
import {mkdtempSync,writeFileSync,existsSync,readFileSync,rmSync} from 'node:fs';
import {join} from 'node:path';
import {watchIdleInspection} from './desktop_idle_inspection.mjs';
test('idle restart inspection uses the existing lease and reports foreign work without changing it',async()=>{
 const root=mkdtempSync(join(process.env.TMPDIR,'idle-inspection-'));let calls=0;
 const nonce='c'.repeat(32);
 const state={foreignLanes:1,composerHasDraft:false,visibleGeneration:false};
 try{
  watchIdleInspection({lane:'owned',lease:{client:{evaluate:async expression=>{
   calls++;expect(expression).toContain('["owned"]');expect(expression).toContain('codex_owned_chat_');
   expect(expression).not.toContain('.abort(');return state;
  }}}},root);
  writeFileSync(root+'/inspect-request.json',JSON.stringify({nonce}));
  const end=Date.now()+2000;
  while(!existsSync(root+'/inspect-result.json')&&Date.now()<end)await Bun.sleep(10);
  expect(JSON.parse(readFileSync(root+'/inspect-result.json','utf8'))).toEqual({nonce,state});
  expect(calls).toBe(1);expect(existsSync(root+'/inspect-request.json')).toBe(false);
 }finally{rmSync(root,{recursive:true});}
});
