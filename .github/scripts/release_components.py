"""Input-bound base reuse and immutable full-release transport composition."""
import argparse
from contextlib import contextmanager
from datetime import datetime, timezone, date
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib

from rald5_publication import (BASE_FILES, FLAT_ASSETS, ValidationError, canonical_release_base,
                               parse_manifest, sha256, verify_signature)
from release_heads import download
from runtime_inputs import fingerprint as runtime_fingerprint, cargo_dependencies, locked_packages

COMPONENT = 'component-input-v1'
MAP = 'asset-map-v1.json'
COMMON = frozenset(BASE_FILES) - {'generation.meta'}
PUBLIC_KEY_SHA = '62ab1640b6b4e63afbd5952d11a0bd0a9f1cb78ddde2472e003a42c4db2b832c'

CONTROL_ASSETS = {'release.manifest':'release.manifest','release.sig':'release.sig',
            'update-index-v1':'candidate-update-index-v1','update-index-v1.sig':'candidate-update-index-v1.sig',
            'update-public-key.pem':'update-public-key.pem',
            'compat/download-size-v1':'download-size-v1','compat/download-size-v1.sig':'download-size-v1.sig',
            COMPONENT:COMPONENT, COMPONENT+'.sig':COMPONENT+'.sig'}


def canonical(value):
    return (json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=True)+'\n').encode()


def unique(items):
    result = {}
    for key, value in items:
        if key in result:
            raise ValueError('duplicate component field')
        result[key] = value
    return result


def git(source, *args):
    environment={key:os.environ[key] for key in ['PATH','TMPDIR','SYSTEMROOT'] if key in os.environ}
    environment.update(GIT_CONFIG_NOSYSTEM='1',GIT_CONFIG_GLOBAL=os.devnull)
    return subprocess.check_output([shutil.which('git'), '-C', str(source), *args],
                                   stderr=subprocess.DEVNULL,env=environment)


def base_fingerprint(source, revision='HEAD'):
    """Bind the actual base source/build dependency closure, excluding Manager."""
    def read(name):
        return git(source, 'show', revision+':'+name)
    recipe = json.loads(read('release/upstream.json'))
    workspace = tomllib.loads(read('Cargo.toml').decode())
    workspace['workspace']['members'] = [name for name in workspace['workspace']['members'] if name != 'crates/manager']
    selected = cargo_dependencies(read, 'crates/core/Cargo.toml')
    roots = ['crates/core','crates/release-builder','crates/runtime-binding',
             '.github/workflows/qualify-termux-runtime.yml',
             '.github/workflows/termux-runtime-source.yml',
             '.github/scripts/runtime_inputs.py',
             '.github/scripts/release_components.py','.github/scripts/upstream_release.py']
    try:
        inventory = read('release/runtime-build-inputs-v1').decode()
    except subprocess.CalledProcessError:
        pass  # Historical releases predate the shared producer inventory.
    else:
        from runtime_inputs import assets
        roots.extend(assets(inventory))
    for member in workspace['workspace']['members']:
        manifest=tomllib.loads(read(member+'/Cargo.toml').decode())
        if (manifest['package']['name'],manifest['package']['version'],None) in selected and member not in roots:
            roots.append(member)
    blobs = git(source,'ls-tree','-r',revision,'--',*roots).decode().splitlines()
    if not blobs:
        raise ValueError('missing base sources')
    files = {}
    for row in blobs:
        identity, path = row.split('\t')
        mode, kind, digest = identity.split()
        if kind != 'blob' or mode not in ('100644','100755'):
            raise ValueError('unexpected base build input')
        files[path] = [mode,digest]
    value = {'schema':1,'files':files,'workspace':workspace,
             'lock':locked_packages(selected),
             'upstream':recipe['upstream'],'toolchains':recipe['toolchains'],
             'lock_audit':recipe['qualification']['lock_audit']}
    return hashlib.sha256(canonical(value)).hexdigest()


def sign_component(source, output, generation, key, openssl, base=None, authority=None):
    source_sha = git(source,'rev-parse','HEAD').decode().strip()
    if not re.fullmatch('[0-9a-f]{40}', source_sha):
        raise ValueError('invalid component source')
    payload = {'schema':'codex-component-input-v1','generation_id':generation,
               'source_sha':source_sha,'base_inputs_sha256':base_fingerprint(source),
               'manifest_sha256':sha256(output/'releases'/generation/'release.manifest')}
    try:
        git(source,'cat-file','-e','HEAD:.github/scripts/runtime_inputs.py')
    except subprocess.CalledProcessError:
        pass  # Retained historical sources have no shared runtime recipe.
    else:
        payload['runtime_inputs_sha256'] = runtime_fingerprint(source,'HEAD',adapted=True)
    if generation.startswith('local-base-') or base is not None:
        base_generation = base_sequence = None
        if base is not None:
            verify_signature(openssl,authority,base/'release.manifest',base/'release.sig')
            manifest,_ = parse_manifest(base/'release.manifest')
            provenance=component(base,manifest['generation_id'],authority,openssl)
            if provenance['base_inputs_sha256'] != payload['base_inputs_sha256']:
                raise ValueError('display base input identity differs')
            base_generation=manifest['generation_id'];base_sequence=int(manifest['release_sequence'])
        payload['display']={'release_date':datetime.now(timezone.utc).date().isoformat(),
                            'base_generation':base_generation,'base_sequence':base_sequence}
    target = output/COMPONENT
    target.write_bytes(canonical(payload))
    subprocess.run([str(openssl),'pkeyutl','-sign','-rawin','-inkey',str(key),'-in',str(target),
                    '-out',str(output/(COMPONENT+'.sig'))],check=True,capture_output=True,env={})


def component(directory, generation, authority, openssl='openssl'):
    path = directory/COMPONENT
    if path.is_symlink() or path.stat().st_size > 4096:
        raise ValueError('invalid component metadata')
    verify_signature(openssl, authority, path, directory/(COMPONENT+'.sig'))
    data = path.read_bytes()
    value = json.loads(data,object_pairs_hook=unique)
    required={'schema','generation_id','source_sha','base_inputs_sha256','manifest_sha256'}
    if (not required.issubset(value) or set(value)-required-{'display','runtime_inputs_sha256'}
        or value['schema'] != 'codex-component-input-v1' or value['generation_id'] != generation
        or data != canonical(value) or not re.fullmatch('[0-9a-f]{40}',value['source_sha'])
        or any(not re.fullmatch('[0-9a-f]{64}',value[key]) for key in ['base_inputs_sha256','manifest_sha256'])
        or value['manifest_sha256'] != sha256(directory/'release.manifest')):
        raise ValueError('component metadata identity differs')
    if 'runtime_inputs_sha256' in value and not re.fullmatch('[0-9a-f]{64}',value['runtime_inputs_sha256']):
        raise ValueError('invalid runtime input identity')
    if 'display' in value:
        display=value['display']
        if (not isinstance(display,dict) or set(display)!={'release_date','base_generation','base_sequence'}
            or not isinstance(display['release_date'],str) or len(display['release_date'])!=10
            or date.fromisoformat(display['release_date']).isoformat()!=display['release_date']):
            raise ValueError('invalid release display date')
        if generation.startswith('local-base-'):
            if display['base_generation'] is not None or display['base_sequence'] is not None:
                raise ValueError('base display cannot inherit')
        elif (not isinstance(display['base_generation'],str)
              or not re.fullmatch(r'local-base-[a-z0-9._-]+',display['base_generation'])
              or type(display['base_sequence']) is not int
              or not 0 < display['base_sequence'] < int(parse_manifest(directory/'release.manifest')[0]['release_sequence'])):
            raise ValueError('invalid release display base')
    return value


def base_controls(heads, authority, destination, fetch=download):
    """Authenticate the selected immutable Base once, before applying reuse policy."""
    state = json.loads((heads/'heads.json').read_text())
    base = state['heads'].get('base')
    if base is None or not base['generation'].startswith('local-base-relay-hosted-'):
        return None
    generation = base['generation']
    destination.mkdir(parents=True,exist_ok=False)
    for name in ['release.manifest','release.sig',COMPONENT,COMPONENT+'.sig']:
        fetch(canonical_release_base(generation)+name,destination/name,131072)
    verify_signature('openssl',authority,destination/'release.manifest',destination/'release.sig')
    manifest, _ = parse_manifest(destination/'release.manifest')
    if manifest['generation_id'] != generation:
        raise ValueError('base generation differs')
    return generation, component(destination,generation,authority)


def matching_base(source, heads, authority, destination, fetch=download):
    """Complete Base input equivalence, required by the existing Full composition."""
    admitted = base_controls(heads,authority,destination,fetch)
    if admitted is None:
        return None
    generation, value = admitted
    return generation if value['base_inputs_sha256'] == base_fingerprint(source) else None


def acquire_runtime_base(source, heads, authority, destination, fetch=download):
    """Reuse signed runtime bytes when Core changed but the runtime recipe did not."""
    admitted = base_controls(heads,authority,destination,fetch)
    if admitted is None:
        return None
    generation, provenance = admitted
    if provenance.get('runtime_inputs_sha256') != runtime_fingerprint(source,'HEAD',adapted=True):
        return None  # Historical metadata or genuinely changed runtime inputs.
    acquire_base_files(generation,destination,fetch,exclude={'core'})
    from rald3_preflight import parse_descriptor
    descriptor = parse_descriptor(destination/'generation.meta')
    recipe = json.loads(git(source,'show','HEAD:release/upstream.json'))['upstream']
    if (descriptor['upstream_package_version'] != recipe['version']
            or descriptor['source_artifact_digest'] != recipe['archive_sha256']):
        raise ValueError('signed runtime Base differs from the selected upstream tuple')
    return generation


def acquire_base_files(generation, destination, fetch=download, exclude=frozenset()):
    _, files = parse_manifest(destination/'release.manifest')
    for name,digest,mode in files:
        if name in exclude:
            continue
        fetch(canonical_release_base(generation)+name,destination/name,1073741824)
        if sha256(destination/name) != digest:
            raise ValueError('base payload digest differs')
        (destination/name).chmod(int(mode,8))


@contextmanager
def published_heads(main_sha):
    """One authority/download lifetime for public Base inspection and acquisition."""
    from release_heads import authenticate
    with tempfile.TemporaryDirectory(prefix='verified-base-',dir=os.environ.get('TMPDIR')) as temp:
        root=Path(temp);key=root/'public.pem'
        download('https://github.com/humtr/codex/releases/download/local-20260914-update-channel-bridge-1/update-public-key.pem',key,16384)
        if sha256(key) != PUBLIC_KEY_SHA:raise ValueError('unexpected public authority')
        authenticate(main_sha,'base',key,root/'heads')
        yield root/'heads',key


def acquire_published_runtime_base(source, main_sha, destination):
    with published_heads(main_sha) as (heads,key):
        return acquire_runtime_base(source,heads,key,destination)


def verified_published_base(source, main_sha):
    with published_heads(main_sha) as (heads,key):
        return matching_base(source,heads,key,heads.parent/'base')


def acquire_base(source, heads, authority, destination, fetch=download):
    generation = matching_base(source,heads,authority,destination,fetch)
    if generation is None:
        return None
    acquire_base_files(generation,destination,fetch)
    return generation


def transport_origin(directory, authority, scratch, fetch, required):
    """Authenticate an origin inventory and its bounded, non-authoritative routes."""
    manifest, files = parse_manifest(directory/'release.manifest')
    verify_signature('openssl',authority,directory/'release.manifest',directory/'release.sig')
    generation = manifest['generation_id']
    if not generation.startswith(('local-base-relay-hosted-', 'local-relay-hosted-')):
        if required:
            raise ValueError('composition requires a mapped public Base')
        print(f'Historical origin {generation} has no supported map; retaining publication payloads.',file=sys.stderr)
        return manifest, files, None
    path = directory/MAP
    if not path.exists() and not path.is_symlink():
        path = scratch/generation/MAP
        path.parent.mkdir(parents=True,exist_ok=True)
        if not fetch(canonical_release_base(generation)+MAP,path,16384,optional=not required):
            print(f'Previous map unavailable for {generation}; retaining publication payloads.',file=sys.stderr)
            return manifest, files, None
    if path.is_symlink() or not path.is_file() or path.stat().st_size > 16384:
        raise ValueError('invalid origin transport map')
    mapping = validate_map(json.loads(path.read_bytes(),object_pairs_hook=unique),generation)
    if mapping['manifest_sha256'] != sha256(directory/'release.manifest'):
        raise ValueError('origin transport map manifest differs')
    return manifest, files, mapping


def compose(assets, base, authority, previous=None, fetch=download):
    """Authenticate inventories, flatten physical routes, then omit verified copies."""
    own, files = parse_manifest(assets/'release.manifest')
    generation = own['generation_id']
    verify_signature('openssl',authority,assets/'release.manifest',assets/'release.sig')
    own_component = component(assets,generation,authority)
    inherited = {}
    base_generation = None
    with tempfile.TemporaryDirectory(prefix='.composition-',dir=assets) as temporary:
        scratch=Path(temporary)
        if not generation.startswith('local-base-'):
            if base is None:
                raise ValueError('full composition requires an authenticated matching base')
            base_manifest, base_files, base_map = transport_origin(base,authority,scratch,fetch,True)
            base_generation = base_manifest['generation_id']
            if not base_generation.startswith('local-base-relay-hosted-'):
                raise ValueError('full composition requires qualified base provenance')
            base_component = component(base,base_generation,authority)
            if base_component['base_inputs_sha256'] != own_component['base_inputs_sha256']:
                raise ValueError('full and base input closure differs')
            if 'display' in own_component and (
                    own_component['display']['base_generation'] != base_generation
                    or own_component['display']['base_sequence'] != int(base_manifest['release_sequence'])):
                raise ValueError('display base differs from actual composition')
            inherited = {name:(digest,mode,base_map['files'][name])
                         for name,digest,mode in base_files if name in COMMON}
            if set(inherited) != COMMON:
                raise ValueError('base common inventory differs')
        prior = {}
        if previous is not None:
            previous_manifest, previous_files, previous_map = transport_origin(previous,authority,scratch,fetch,False)
            if int(previous_manifest['release_sequence']) >= int(own['release_sequence']):
                raise ValueError('reuse origin must be an earlier release')
            if previous_map is not None:
                prior = {name:(digest,mode,previous_map['files'][name])
                         for name,digest,mode in previous_files if name != 'generation.meta'}
        mapped = {}
        omit = []
        for name,digest,mode in files:
            asset = FLAT_ASSETS[name]
            if sha256(assets/asset) != digest:
                raise ValueError('composition payload digest differs')
            target = {'generation':generation,'asset':asset}
            inherited_file = inherited.get(name)
            if inherited_file is not None:
                if inherited_file[:2] != (digest,mode):
                    raise ValueError('common payload differs from pinned base')
            elif name in prior and prior[name][:2] == (digest,mode):
                inherited_file = prior[name]
            if inherited_file is not None:
                target = inherited_file[2]
                if target['generation'] == generation:
                    raise ValueError('reuse route points to candidate generation')
                omit.append(assets/asset)
            mapped[name] = target
        mapped.update({name:{'generation':generation,'asset':asset} for name,asset in CONTROL_ASSETS.items()})
        value = {'schema':'codex-asset-map-v1','generation_id':generation,
                 'base_generation':base_generation,'manifest_sha256':own_component['manifest_sha256'], 'files':mapped}
        validate_map(value,generation)
        (scratch/MAP).write_bytes(canonical(value))
        os.replace(scratch/MAP,assets/MAP)
        for path in omit:
            path.unlink()  # Verified generated release copies; original assets stay immutable.
        return value


def validate_map(value, generation):
    from rald5_publication import safe_generation
    safe_generation(generation)
    if (not generation.startswith(('local-relay-hosted-','local-base-relay-hosted-'))
        or set(value) != {'schema','generation_id','base_generation','manifest_sha256','files'}
        or value['schema'] != 'codex-asset-map-v1' or value['generation_id'] != generation
        or not re.fullmatch('[0-9a-f]{64}',value['manifest_sha256'])):
        raise ValueError('invalid transport map identity')
    base = generation.startswith('local-base-')
    inherited = value['base_generation']
    if base:
        if inherited is not None:raise ValueError('base cannot inherit')
    elif not isinstance(inherited,str) or not inherited.startswith('local-base-relay-hosted-'):
        raise ValueError('invalid pinned base')
    if inherited is not None:safe_generation(inherited)

    paths=set(BASE_FILES)|set(CONTROL_ASSETS)| (set() if base else {'manager','helpers/2'})
    if set(value['files']) != paths:raise ValueError('invalid mapped inventory')
    for name,target in value['files'].items():
        if (not isinstance(target,dict) or set(target)!={'generation','asset'}
            or target['asset'] != CONTROL_ASSETS.get(name,FLAT_ASSETS.get(name))
            or not isinstance(target['generation'],str)):
            raise ValueError('invalid mapped target')
        try:
            safe_generation(target['generation'])
        except ValidationError as error:
            raise ValueError('invalid mapped origin') from error
        if not target['generation'].startswith(('local-base-relay-hosted-', 'local-relay-hosted-')):
            raise ValueError('invalid mapped origin')
        if name in CONTROL_ASSETS or name == 'generation.meta':
            if target['generation'] != generation:
                raise ValueError('release controls must belong to the current generation')
    return value


def candidate_from_base(base, manager, frontend, builder, generation, version, output, core=None):
    """Assemble Base/Core replacement or Full add-ons from an authenticated subset."""
    base_only = core is not None
    prefix = ('local-base-relay-hosted-', 'base-qualification-') if base_only else 'local-relay-hosted-'
    if output.exists() or output.is_symlink() or not generation.startswith(prefix):
        raise ValueError('invalid candidate output or identity')
    if base_only and any(value is not None for value in (manager,frontend,builder)):
        raise ValueError('Base assembly replaces Core only; add-ons require the complete Base')
    manifest, files = parse_manifest(base/'release.manifest')
    if not manifest['generation_id'].startswith('local-base-relay-hosted-') or set(name for name,_,_ in files) != set(BASE_FILES):
        raise ValueError('candidate base inventory differs')
    descriptor = (base/'generation.meta').read_text().splitlines()
    fields = dict(line.split('\t') for line in descriptor[1:] if line.count('\t') == 1)
    if fields.get('upstream_package_version') != version or fields.get('manager_artifact_digest') != '-':
        raise ValueError('candidate base version or variant differs')
    for path in ([core] if base_only else [manager, frontend, builder]):
        if path.is_symlink() or not path.is_file() or path.stat().st_size > 536870912 or path.stat().st_mode & 0o111 == 0:
            raise ValueError('candidate add-on is not an executable regular file')
    output.mkdir(mode=0o700)
    try:
        for name,digest,mode in files:
            if base_only and name == 'core':
                continue
            source=base/name
            if source.is_symlink() or not source.is_file() or sha256(source) != digest:
                raise ValueError('candidate base digest differs')
            if name == 'generation.meta':
                continue
            target=output/name;target.parent.mkdir(exist_ok=True,parents=True)
            shutil.copyfile(source,target);target.chmod(int(mode,8))
        if base_only:
            shutil.copyfile(core,output/'core');(output/'core').chmod(0o755)
            replacement={'generation_id':generation,'core_artifact_digest':sha256(output/'core')}
        else:
            shutil.copyfile(manager,output/'manager');(output/'manager').chmod(0o755)
            subprocess.run([str(builder),'adapt-manager-tui','--artifact',str(frontend),'--openssl',shutil.which('openssl'),
                            '--output',str(output/'helpers/2')],check=True)
            replacement={'generation_id':generation,'manager_artifact_digest':sha256(output/'manager'),'helper_count':'3'}
        lines=[descriptor[0]]+[key+'\t'+replacement.get(key,value) if line.count('\t') == 1
                              else line for line in descriptor[1:] for key,value in [line.split('\t',1)]]
        if not base_only:
            lines.append('helper\ttermux-manager-tui-v1:'+version+'\t'+sha256(output/'helpers/2'))
        (output/'generation.meta').write_text('\n'.join(lines)+'\n');(output/'generation.meta').chmod(0o644)
        if not base_only:
            (output/'.manager-probe-deferred').write_bytes(b'codex-manager-probe-deferred-v1\n')
            (output/'.manager-probe-deferred').chmod(0o644)
    except BaseException:
        shutil.rmtree(output)
        raise


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    sub=parser.add_subparsers(dest='command',required=True)
    fingerprint=sub.add_parser('fingerprint');fingerprint.add_argument('--source',type=Path,required=True)
    reuse=sub.add_parser('acquire-base')
    for name in ['source','heads','authority','output']:
        reuse.add_argument('--'+name,type=Path,required=True)
    match=sub.add_parser('match-base')
    for name in ['source','heads','authority','output']:
        match.add_argument('--'+name,type=Path,required=True)
    runtime=sub.add_parser('acquire-published-runtime-base')
    runtime.add_argument('--source',type=Path,required=True)
    runtime.add_argument('--main',required=True);runtime.add_argument('--output',type=Path,required=True)
    base_candidate=sub.add_parser('candidate-base')
    for name in ['base','core','output','authority','source']:
        base_candidate.add_argument('--'+name,type=Path,required=True)
    base_candidate.add_argument('--base-generation',required=True)
    base_candidate.add_argument('--generation',required=True);base_candidate.add_argument('--version',required=True)
    composition=sub.add_parser('compose')
    for name in ['assets','authority']:
        composition.add_argument('--'+name,type=Path,required=True)
    composition.add_argument('--base',type=Path)
    composition.add_argument('--previous',type=Path)
    candidate=sub.add_parser('candidate')
    for name in ['base','manager','frontend','builder','output']:
        candidate.add_argument('--'+name,type=Path,required=True)
    candidate.add_argument('--generation',required=True);candidate.add_argument('--version',required=True)
    target=sub.add_parser('target');target.add_argument('--map',type=Path,required=True)
    target.add_argument('--generation',required=True);target.add_argument('--path',required=True)
    args=parser.parse_args()
    if args.command=='fingerprint':
        print(base_fingerprint(args.source))
    elif args.command in ['acquire-base','match-base']:
        action=acquire_base if args.command=='acquire-base' else matching_base
        generation=action(args.source,args.heads,args.authority,args.output)
        print('base_generation='+str(generation or ''))
        print('reuse_base='+str(generation is not None).lower())
    elif args.command=='acquire-published-runtime-base':
        generation=acquire_published_runtime_base(args.source,args.main,args.output)
        print('base_generation='+str(generation or ''))
        print('reuse_runtime='+str(generation is not None).lower())
    elif args.command=='candidate-base':
        verify_signature('openssl',args.authority,args.base/'release.manifest',args.base/'release.sig')
        provenance=component(args.base,args.base_generation,args.authority)
        if provenance.get('runtime_inputs_sha256') != runtime_fingerprint(args.source,'HEAD',adapted=True):
            raise ValueError('runtime subset no longer matches the selected source recipe')
        candidate_from_base(args.base,None,None,None,args.generation,args.version,args.output,core=args.core)
    elif args.command=='target':
        if args.map.stat().st_size > 16384:raise ValueError('oversized transport map')
        value=validate_map(json.loads(args.map.read_bytes(),object_pairs_hook=unique),args.generation)
        target=value['files'][args.path]
        print('https://github.com/humtr/codex/releases/download/'+target['generation']+'/'+target['asset'])
    elif args.command=='candidate':
        candidate_from_base(args.base,args.manager,args.frontend,args.builder,args.generation,args.version,args.output)
    else:
        value=compose(args.assets,args.base,args.authority,args.previous)
        print('base_generation='+str(value['base_generation'] or ''))


if __name__=='__main__':
    main()
