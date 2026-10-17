import assert from 'node:assert/strict';
import {handle, validateMap} from './asset-redirect.mjs';
const generation='local-relay-hosted-owned-s2';
const base='local-base-relay-hosted-owned-s1';
const common=['codex-code-mode-host','core','helpers/0','helpers/1','runtime'];
const controls={'release.manifest':'release.manifest','release.sig':'release.sig','generation.meta':'generation.meta',
  manager:'manager','helpers/2':'helper-2','update-index-v1':'candidate-update-index-v1',
  'update-index-v1.sig':'candidate-update-index-v1.sig','update-public-key.pem':'update-public-key.pem',
  'compat/download-size-v1':'download-size-v1','compat/download-size-v1.sig':'download-size-v1.sig',
  'component-input-v1':'component-input-v1','component-input-v1.sig':'component-input-v1.sig'};
const map={schema:'codex-asset-map-v1',generation_id:generation,base_generation:base,manifest_sha256:'a'.repeat(64),files:{}};
for (const path of common) map.files[path]={generation:base,asset:path.startsWith('helpers/')?`helper-${path.slice(8)}`:path};
for (const [path,asset] of Object.entries(controls)) map.files[path]={generation,asset};
let fetches=0;
const fetcher=async (url,options)=>{
  assert.equal(options.cf.cacheTtlByStatus['400-599'],0);
  fetches++;assert.equal(url,`https://github.com/humtr/codex/releases/download/${generation}/asset-map-v1.json`);
  return new Response(JSON.stringify(map));
};
for (const [path,target] of Object.entries(map.files)) {
  for (const method of ['GET','HEAD']) {
    const response=await handle(new Request(`https://codex-releases.humtr.workers.dev/${generation}/${path}`,{method}),fetcher);
    assert.equal(response.status,302,path);assert.equal(await response.text(),'');
    assert.equal(response.headers.get('Location'),`https://github.com/humtr/codex/releases/download/${target.generation}/${target.asset}`);
  }
}
const before=fetches;
for (const path of ['','?url=https://evil.invalid','helpers/9','runtime?x=1','%72untime','component-input-v1/extra']) {
  const response=await handle(new Request(`https://codex-releases.humtr.workers.dev/${generation}/${path}`),fetcher);
  assert.equal(response.status,404,path);
}
assert.equal(fetches,before);
assert.equal((await handle(new Request(`https://codex-releases.humtr.workers.dev/${generation}/core`,{method:'POST'}),fetcher)).status,405);
for (const fault of ['foreign','missing','extra','cycle','trailing-newline']) {
  const copy=structuredClone(map);
  if (fault==='foreign')copy.files.core.generation='evil';
  if (fault==='missing')delete copy.files.runtime;
  if (fault==='extra')copy.files['helpers/9']=copy.files['helpers/0'];
  if (fault==='cycle')copy.base_generation=generation;
  if (fault==='trailing-newline')copy.base_generation=base+'\n';
  assert.throws(()=>validateMap(copy,generation),undefined,fault);
  const response=await handle(new Request(`https://codex-releases.humtr.workers.dev/${generation}/core`),async()=>new Response(JSON.stringify(copy)));
  assert.equal(response.status,502);
}
for (const value of [new Response('x'.repeat(16385)),new Response('{}',{status:503})]) {
  assert.equal((await handle(new Request(`https://codex-releases.humtr.workers.dev/${generation}/core`),async()=>value)).status,502);
}
console.log('redirect tests passed');
