// Stateless transport only. The native client still authenticates every byte.
const origin = 'https://github.com/humtr/codex/releases/download/';
const generationRE = /^local-[a-z0-9][a-z0-9._-]{0,126}$(?![\s\S])/;
const common = ['codex-code-mode-host', 'core', 'helpers/0', 'helpers/1', 'runtime'];
const control = {
  'release.manifest':'release.manifest', 'release.sig':'release.sig',
  'update-index-v1':'candidate-update-index-v1', 'update-index-v1.sig':'candidate-update-index-v1.sig',
  'update-public-key.pem':'update-public-key.pem',
  'compat/download-size-v1':'download-size-v1', 'compat/download-size-v1.sig':'download-size-v1.sig',
  'component-input-v1':'component-input-v1', 'component-input-v1.sig':'component-input-v1.sig',
};
const flat = path => path.startsWith('helpers/') ? `helper-${path.slice(8)}` : (control[path] || path);
export function validateMap(value, generation) {
  if (!value || Object.keys(value).sort().join() !== 'base_generation,files,generation_id,manifest_sha256,schema'
      || value.schema !== 'codex-asset-map-v1' || value.generation_id !== generation
      || !/^[0-9a-f]{64}$(?![\s\S])/.test(value.manifest_sha256)) throw Error('invalid map identity');
  const base = generation.startsWith('local-base-relay-hosted-');
  if (base ? value.base_generation !== null
      : typeof value.base_generation !== 'string' || !generationRE.test(value.base_generation)
        || !value.base_generation.startsWith('local-base-relay-hosted-')) throw Error('invalid base');
  const paths = [...common, 'generation.meta', ...Object.keys(control), ...(base ? [] : ['manager', 'helpers/2'])];
  if (!value.files || Object.keys(value.files).sort().join() !== paths.sort().join()) throw Error('invalid inventory');
  for (const path of paths) {
    const target = value.files[path];
    if (!target || Object.keys(target).sort().join() !== 'asset,generation' || target.asset !== flat(path)
        || target.generation !== (!base && common.includes(path) ? value.base_generation : generation)) {
      throw Error('invalid target');
    }
  }
  return value;
}
export async function handle(request, fetcher = fetch) {
  if (!['GET', 'HEAD'].includes(request.method)) return new Response(null, {status:405,headers:{Allow:'GET, HEAD'}});
  const url = new URL(request.url);
  const match = /^\/([^/]+)\/(.+)$/.exec(url.pathname);
  if (url.protocol !== 'https:' || url.search || url.hash || url.username || url.password || !match
      || !generationRE.test(match[1]) || !/^local-(base-)?relay-hosted-/.test(match[1])
      || /[%\\]/.test(url.pathname)) return new Response(null,{status:404});
  const [, generation, path] = match;
  if (path === 'asset-map-v1.json') return Response.redirect(origin+generation+'/'+path,302);
  if (![...common,'generation.meta','manager','helpers/2',...Object.keys(control)].includes(path)) {
    return new Response(null,{status:404});
  }
  try {
    const response = await fetcher(origin+generation+'/asset-map-v1.json', {
      redirect:'follow', cf:{cacheTtlByStatus:{'200-299':300,'400-599':0},cacheEverything:true}, headers:{Accept:'application/json'},
      signal:AbortSignal.timeout(15000),
    });
    if (response.status === 404) return new Response(null,{status:404});
    if (!response.ok || Number(response.headers.get('content-length') || 0) > 16384) throw Error('unavailable map');
    // Enforce the byte bound while reading, not after an unbounded json() call.
    const reader = response.body.getReader();
    const chunks = []; let length=0;
    try {
      for (;;) {
        const {value,done}=await reader.read(); if (done) break;
        length+=value.length; if (length>16384) throw Error('oversized map'); chunks.push(value);
      }
    } finally { await reader.cancel(); }
    const data=new Uint8Array(length);let offset=0;
    for (const chunk of chunks) {data.set(chunk,offset);offset+=chunk.length;}
    const map=validateMap(JSON.parse(new TextDecoder('utf-8',{fatal:true}).decode(data)),generation);
    const target=map.files[path];
    if (!target) return new Response(null,{status:404});
    return new Response(null,{status:302,headers:{Location:origin+target.generation+'/'+target.asset,'Cache-Control':'public, max-age=300'}});
  } catch { return new Response(null,{status:502}); }
}
export default {fetch:request => handle(request)};
