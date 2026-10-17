"""Input-bound base reuse and immutable full-release transport composition."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import tomllib

from rald5_publication import (BASE_FILES, FLAT_ASSETS, canonical_release_base,
                               parse_manifest, sha256, verify_signature)
from release_heads import download

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
    lock = tomllib.loads(read('Cargo.lock').decode())
    packages = {(p['name'],p['version']):p for p in lock['package']}
    core_manifest=tomllib.loads(read('crates/core/Cargo.toml').decode())
    pending = [('codex',core_manifest['package']['version'])]
    selected = {}
    while pending:
        key = pending.pop()
        if key in selected:
            continue
        package = packages[key]
        selected[key] = package
        for dependency in package.get('dependencies', []):
            parts = dependency.split()
            matches = [k for k in packages if k[0] == parts[0] and (len(parts) == 1 or k[1] == parts[1])]
            if len(matches) != 1:
                raise ValueError('ambiguous base dependency closure')
            pending.append(matches[0])
    roots = ['crates/core','crates/release-builder','crates/runtime-binding',
             '.github/workflows/qualify-termux-runtime.yml',
             '.github/scripts/release_components.py','.github/scripts/upstream_release.py']
    for member in workspace['workspace']['members']:
        manifest=tomllib.loads(read(member+'/Cargo.toml').decode())
        if (manifest['package']['name'],manifest['package']['version']) in selected and member not in roots:
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
             'lock':[selected[key] for key in sorted(selected)],
             'upstream':recipe['upstream'],'core_rust':recipe['toolchains']['core_rust']}
    return hashlib.sha256(canonical(value)).hexdigest()


def sign_component(source, output, generation, key, openssl):
    source_sha = git(source,'rev-parse','HEAD').decode().strip()
    if not re.fullmatch('[0-9a-f]{40}', source_sha):
        raise ValueError('invalid component source')
    payload = {'schema':'codex-component-input-v1','generation_id':generation,
               'source_sha':source_sha,'base_inputs_sha256':base_fingerprint(source),
               'manifest_sha256':sha256(output/'releases'/generation/'release.manifest')}
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
    if (set(value) != {'schema','generation_id','source_sha','base_inputs_sha256','manifest_sha256'}
        or value['schema'] != 'codex-component-input-v1' or value['generation_id'] != generation
        or data != canonical(value) or not re.fullmatch('[0-9a-f]{40}',value['source_sha'])
        or any(not re.fullmatch('[0-9a-f]{64}',value[key]) for key in ['base_inputs_sha256','manifest_sha256'])
        or value['manifest_sha256'] != sha256(directory/'release.manifest')):
        raise ValueError('component metadata identity differs')
    return value


def matching_base(source, heads, authority, destination, fetch=download):
    """Small authenticated capability check. Does not launch a runtime or infer."""
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
    value = component(destination,generation,authority)
    if value['base_inputs_sha256'] != base_fingerprint(source):
        return None
    return generation


def verified_published_base(source, main_sha):
    from release_heads import authenticate
    with tempfile.TemporaryDirectory(prefix='verified-base-',dir=os.environ.get('TMPDIR')) as temp:
        root=Path(temp);key=root/'public.pem'
        download('https://github.com/humtr/codex/releases/download/local-20260914-update-channel-bridge-1/update-public-key.pem',key,16384)
        if sha256(key) != PUBLIC_KEY_SHA:raise ValueError('unexpected public authority')
        authenticate(main_sha,'base',key,root/'heads')
        return matching_base(source,root/'heads',key,root/'base')


def acquire_base(source, heads, authority, destination, fetch=download):
    generation = matching_base(source,heads,authority,destination,fetch)
    if generation is None:
        return None
    _, files = parse_manifest(destination/'release.manifest')
    for name,digest,mode in files:
        fetch(canonical_release_base(generation)+name,destination/name,1073741824)
        if sha256(destination/name) != digest:
            raise ValueError('base payload digest differs')
        (destination/name).chmod(int(mode,8))
    return generation


def compose(assets, base, authority):
    """Verify all bytes before omitting common files; map is only transport."""
    own, files = parse_manifest(assets/'release.manifest')
    generation = own['generation_id']
    verify_signature('openssl',authority,assets/'release.manifest',assets/'release.sig')
    own_component = component(assets,generation,authority)
    inherited = {}
    base_generation = None
    if not generation.startswith('local-base-'):
        if base is None:
            raise ValueError('full composition requires an authenticated matching base')
        base_manifest, base_files = parse_manifest(base/'release.manifest')
        base_generation = base_manifest['generation_id']
        if not base_generation.startswith('local-base-relay-hosted-'):
            raise ValueError('full composition requires qualified base provenance')
        verify_signature('openssl',authority,base/'release.manifest',base/'release.sig')
        base_component = component(base,base_generation,authority)
        if base_component['base_inputs_sha256'] != own_component['base_inputs_sha256']:
            raise ValueError('full and base input closure differs')
        inherited = {name:(digest,mode) for name,digest,mode in base_files if name in COMMON}
        if set(inherited) != COMMON:
            raise ValueError('base common inventory differs')
    mapped = {}
    omit = []
    for name,digest,mode in files:
        asset = FLAT_ASSETS[name]
        if sha256(assets/asset) != digest:
            raise ValueError('composition payload digest differs')
        target = generation
        if name in inherited:
            if inherited[name] != (digest,mode):
                raise ValueError('common payload differs from pinned base')
            target = base_generation
            omit.append(assets/asset)
        mapped[name] = {'generation':target,'asset':asset}

    mapped.update({name:{'generation':generation,'asset':asset} for name,asset in CONTROL_ASSETS.items()})
    value = {'schema':'codex-asset-map-v1','generation_id':generation,
             'base_generation':base_generation,'manifest_sha256':own_component['manifest_sha256'], 'files':mapped}
    validate_map(value,generation)
    (assets/MAP).write_bytes(canonical(value))
    for path in omit:
        path.unlink()  # Only verified generated release copies, never user data.
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
        expected={'generation':inherited if not base and name in COMMON else generation,
                  'asset':CONTROL_ASSETS.get(name,FLAT_ASSETS.get(name))}
        if target != expected:raise ValueError('invalid mapped target')
    return value


def candidate_from_base(base, manager, frontend, builder, generation, version, output):
    """Assemble private full output from an already authenticated base snapshot."""
    if output.exists() or output.is_symlink() or not generation.startswith('local-relay-hosted-'):
        raise ValueError('invalid candidate output or identity')
    manifest, files = parse_manifest(base/'release.manifest')
    if not manifest['generation_id'].startswith('local-base-relay-hosted-') or set(name for name,_,_ in files) != set(BASE_FILES):
        raise ValueError('candidate base inventory differs')
    descriptor = (base/'generation.meta').read_text().splitlines()
    fields = dict(line.split('\t') for line in descriptor[1:] if line.count('\t') == 1)
    if fields.get('upstream_package_version') != version or fields.get('manager_artifact_digest') != '-':
        raise ValueError('candidate base version or variant differs')
    for path in [manager, frontend, builder]:
        if path.is_symlink() or not path.is_file() or path.stat().st_size > 536870912 or path.stat().st_mode & 0o111 == 0:
            raise ValueError('candidate add-on is not an executable regular file')
    output.mkdir(mode=0o700)
    try:
        for name,digest,mode in files:
            source=base/name
            if source.is_symlink() or not source.is_file() or sha256(source) != digest:
                raise ValueError('candidate base digest differs')
            if name == 'generation.meta':
                continue
            target=output/name;target.parent.mkdir(exist_ok=True,parents=True)
            shutil.copyfile(source,target);target.chmod(int(mode,8))
        shutil.copyfile(manager,output/'manager');(output/'manager').chmod(0o755)
        subprocess.run([str(builder),'adapt-manager-tui','--artifact',str(frontend),'--openssl',shutil.which('openssl'),
                        '--output',str(output/'helpers/2')],check=True)
        replacement={'generation_id':generation,'manager_artifact_digest':sha256(output/'manager'),'helper_count':'3'}
        lines=[descriptor[0]]+[key+'\t'+replacement.get(key,value) if line.count('\t') == 1
                              else line for line in descriptor[1:] for key,value in [line.split('\t',1)]]
        lines.append('helper\ttermux-manager-tui-v1:'+version+'\t'+sha256(output/'helpers/2'))
        (output/'generation.meta').write_text('\n'.join(lines)+'\n');(output/'generation.meta').chmod(0o644)
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
    composition=sub.add_parser('compose')
    for name in ['assets','authority']:
        composition.add_argument('--'+name,type=Path,required=True)
    composition.add_argument('--base',type=Path)
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
    elif args.command=='target':
        if args.map.stat().st_size > 16384:raise ValueError('oversized transport map')
        value=validate_map(json.loads(args.map.read_bytes(),object_pairs_hook=unique),args.generation)
        target=value['files'][args.path]
        print('https://github.com/humtr/codex/releases/download/'+target['generation']+'/'+target['asset'])
    elif args.command=='candidate':
        candidate_from_base(args.base,args.manager,args.frontend,args.builder,args.generation,args.version,args.output)
    else:
        value=compose(args.assets,args.base,args.authority)
        print('base_generation='+str(value['base_generation'] or ''))


if __name__=='__main__':
    main()
