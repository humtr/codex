"""Register proven frontend inputs and advance only an accepted exact caller.

No signing key enters this controller. All public runtime publication stays in
auto-release-termux, including Android execution and public disposable proof.
"""
import argparse
import base64
import gzip
import hashlib
import io
import json
import os
import re
from pathlib import Path
import subprocess
import tarfile
import tempfile

from manager_tui_artifact import verify as verify_frontend, verify_recipe
from upstream_release import load

REPO = 'humtr/codex'
SOURCE_BRANCH = 'rewrite/rust-core'


def gh(*args, payload=None):
    command = ['gh', *args]
    if payload is not None:
        command += ['--input', '-']
    result = subprocess.run(command, input=None if payload is None else json.dumps(payload).encode(),
                            capture_output=True, check=True)
    return result.stdout


def api(path, *, payload=None, method=None):
    args = ['api', 'repos/'+REPO+'/'+path]
    if method:args += ['--method', method]
    value = gh(*args, payload=payload)
    return json.loads(value) if value else None


def accepted_run(run, workflow, sha):
    expected = {'path': '.github/workflows/'+workflow, 'head_sha': sha,
                'head_branch': SOURCE_BRANCH, 'status': 'completed', 'conclusion': 'success'}
    return (all(run.get(k) == value for k, value in expected.items())
            and run.get('event') in ['push', 'workflow_dispatch']
            and run.get('repository', {}).get('full_name') == REPO)


def successful_run(workflow, sha):
    runs = api('actions/workflows/'+workflow+'/runs?head_sha='+sha+'&per_page=100')['workflow_runs']
    return next((run for run in runs if accepted_run(run, workflow, sha)), None), runs


def commit_files(branch, expected, files, message):
    if api('git/ref/heads/'+branch)['object']['sha'] != expected:
        raise ValueError('branch moved; re-evaluate on the next event')
    base = api('git/commits/'+expected)['tree']['sha']
    entries = []
    for path, data in files.items():
        blob = api('git/blobs', payload={'encoding':'base64', 'content':base64.b64encode(data).decode()})['sha']
        entries.append({'path':path, 'mode':'100644', 'type':'blob', 'sha':blob})
    tree = api('git/trees', payload={'base_tree':base, 'tree':entries})['sha']
    commit = api('git/commits', payload={'message':message, 'tree':tree, 'parents':[expected]})['sha']
    api('git/refs/heads/'+branch, method='PATCH', payload={'sha':commit, 'force':False})
    return commit


def dispatch(workflow, ref, inputs=None):
    api('actions/workflows/'+workflow+'/dispatches', method='POST',
        payload={'ref':ref, 'inputs':inputs or {}})


def bundle_inputs(artifact, proof):
    output = io.BytesIO()
    with gzip.GzipFile(fileobj=output, mode='wb', mtime=0) as compressed:
        with tarfile.open(fileobj=compressed, mode='w') as archive:
            for label, root in [('artifact', artifact), ('proof', proof)]:
                for path in sorted(root.rglob('*')):
                    if not path.is_file() or path.is_symlink():
                        continue
                    info = archive.gettarinfo(str(path), label+'/'+str(path.relative_to(root)))
                    info.uid=info.gid=0;info.uname=info.gname='';info.mtime=0
                    with path.open('rb') as stream:archive.addfile(info, stream)
    return output.getvalue()


def ensure_frontend(source, current, root):
    value = load(source/'release/upstream.json')
    pinned = value['frontend']
    native = None
    if pinned:
        observed = api('actions/runs/'+pinned['run_id'])
        if accepted_run(observed, 'manager-tui-native.yml', pinned['source']):
            native = observed
    # A newly changed recipe cannot reuse old output even if its old run passed.
    candidates = [native] if native else []
    recent = api('actions/workflows/manager-tui-native.yml/runs?branch='+SOURCE_BRANCH+'&per_page=20')['workflow_runs']
    # Output-only registration commits can share an earlier successful recipe.
    # Exact Git blobs and build_recipe still decide whether it is reusable.
    for run in recent:
        if (accepted_run(run, 'manager-tui-native.yml', run['head_sha'])
                and all(run['id'] != candidate['id'] for candidate in candidates)):
            candidates.append(run)
    for index, run in enumerate(candidates):
        directory=root/str(index);directory.mkdir()
        artifact, proof = directory/'artifact', directory/'proof'
        retained = pinned is not None and str(run['id']) == pinned['run_id']
        try:
            if retained:
                tag, name = pinned['input_url'].split('/releases/download/',1)[1].split('/')
                gh('release','download',tag,'--repo',REPO,'--pattern',name,'--dir',str(directory))
                bundle=directory/name
                if hashlib.sha256(bundle.read_bytes()).hexdigest()!=pinned['input_sha256']:
                    raise ValueError('retained immutable input digest differs')
                with tarfile.open(bundle,'r:gz') as archive:
                    members=archive.getmembers()
                    if sum(member.size for member in members)>1073741824:
                        raise ValueError('frontend input archive exceeds bound')
                    for member in members:
                        path=Path(member.name)
                        if path.is_absolute() or '..' in path.parts or not path.parts or path.parts[0] not in ['artifact','proof'] or not(member.isfile() or member.isdir()):
                            raise ValueError('unsafe frontend input archive')
                    archive.extractall(directory,filter='data')
            else:
                artifact.mkdir();proof.mkdir()
                gh('run','download',str(run['id']),'--repo',REPO,'--name','manager-tui-native','--dir',str(artifact))
                gh('run','download',str(run['id']),'--repo',REPO,'--name','manager-tui-native-proof','--dir',str(proof))
        except (subprocess.CalledProcessError, ValueError, OSError):
            continue
        subprocess.run(['git','-C',str(source),'fetch','--no-tags','--depth=1','origin',run['head_sha']],check=True,capture_output=True)
        run_json=directory/'run.json';run_json.write_text(json.dumps(run))
        try:
            binary=artifact/'codex'
            if binary.is_symlink() or binary.stat().st_size > 512*1024*1024:
                raise ValueError('invalid native artifact file')
            digest=hashlib.sha256(binary.read_bytes()).hexdigest()
            args=argparse.Namespace(source=source,source_sha=run['head_sha'],run_json=run_json,
                run_id=run['id'],version=value['upstream']['version'],sha256=digest,artifact=artifact,proof=proof)
            verify_frontend(args)
        except (ValueError,OSError,subprocess.CalledProcessError):
            continue
        if retained:
            if digest != pinned['sha256']:
                raise ValueError('retained frontend executable digest differs')
            return value
        tag='manager-tui-native-v'+value['upstream']['version']+'-'+run['head_sha'][:12]
        name='manager-tui-inputs-'+value['upstream']['version']+'-'+run['head_sha'][:12]+'.tar.gz'
        bundle=directory/name;bundle.write_bytes(bundle_inputs(artifact,proof))
        checksum=hashlib.sha256(bundle.read_bytes()).hexdigest()
        listed=api('releases?per_page=100')
        existing=next((release for release in listed if release['tag_name']==tag),None)
        if existing is None:
            gh('release','create',tag,str(bundle),'--repo',REPO,'--target',run['head_sha'],
               '--title','Qualified unsigned frontend '+value['upstream']['version'],
               '--notes','Exact successful native build and execution evidence; not a signed runtime release.',
               '--prerelease','--latest=false')
        readback=directory/'readback';readback.mkdir()
        gh('release','download',tag,'--repo',REPO,'--pattern',name,'--dir',str(readback))
        if (readback/name).read_bytes()!=bundle.read_bytes():
            raise ValueError('immutable frontend input readback differs; never overwrite it')
        value['frontend']={'source':run['head_sha'],'run_id':str(run['id']),'sha256':digest,
            'input_url':'https://github.com/'+REPO+'/releases/download/'+tag+'/'+name,'input_sha256':checksum}
        return value
    # Do not pretend stale frontend references qualify a changed recipe. Start
    # its independent actual build, while allowing the accepted Core/base ahead.
    _, runs = successful_run('manager-tui-native.yml', current)
    if not runs:
        dispatch('manager-tui-native.yml', SOURCE_BRANCH)
    value['frontend'] = None
    return value


# Documentation changes must not rebuild and publish an otherwise identical
# installation. Unknown product/workflow changes still require fresh proof.

def same_product_inputs(source, previous, current):
    if not re.fullmatch('[0-9a-f]{40}', previous):
        raise ValueError('invalid accepted product source pointer')
    known = subprocess.run(['git','-C',str(source),'cat-file','-e',previous+'^{commit}'],capture_output=True)
    if known.returncode:
        subprocess.run(['git','-C',str(source),'fetch','--no-tags','--depth=1','origin',previous],check=True,capture_output=True)
    changed = subprocess.check_output(['git','-C',str(source),'diff','--name-only','-z',previous,current])
    paths = changed.decode('utf-8', errors='surrogateescape').split('\0')
    return all(not path or path.endswith('.md') or path == '.gitignore' for path in paths)


def frontend_registration_needed(source):
    pinned=load(source/'release/upstream.json')['frontend']
    def matches(sha):
        try:
            subprocess.run(['git','-C',str(source),'fetch','--no-tags','--depth=1','origin',sha],check=True,capture_output=True)
            verify_recipe(source,sha)
            return True
        except (ValueError,OSError,subprocess.CalledProcessError):
            return False
    if pinned and matches(pinned['source']):
        return False
    recent=api('actions/workflows/manager-tui-native.yml/runs?branch='+SOURCE_BRANCH+'&per_page=20')['workflow_runs']
    return any(accepted_run(run,'manager-tui-native.yml',run['head_sha']) and matches(run['head_sha'])
               for run in recent)


def run(source):
    current=subprocess.check_output(['git','-C',str(source),'rev-parse','HEAD'],text=True).strip()
    if api('git/ref/heads/'+SOURCE_BRANCH)['object']['sha']!=current:
        return {'state':'obsolete-source-ignored'}
    main=api('git/ref/heads/main')['object']['sha']
    pointer=api('contents/release-source-v1?ref='+main)
    previous=base64.b64decode(pointer['content']).decode().strip()
    if (previous != current and same_product_inputs(source, previous, current)
            and not frontend_registration_needed(source)):
        return {'state':'no-product-change','source':current,'accepted_source':previous}
    for workflow in ['rald7-full-acceptance.yml','core-runtime-qualification.yml']:
        passed, runs=successful_run(workflow,current)
        if not passed:
            if not runs:dispatch(workflow,SOURCE_BRANCH,{'rebuild_current':'true'} if workflow.startswith('core-') else None)
            return {'state':'awaiting-exact-proof','workflow':workflow,'source':current}
    with tempfile.TemporaryDirectory(prefix='release-inputs-',dir=os.environ.get('TMPDIR')) as temporary:
        value=ensure_frontend(source,current,Path(temporary))
    before=load(source/'release/upstream.json')
    if value is not None and value!=before:
        updated=commit_files(SOURCE_BRANCH,current,{'release/upstream.json':(json.dumps(value,indent=2)+'\n').encode()},
                             'Bind successful immutable frontend inputs')
        # GitHub's own token commits do not trigger push workflows.
        dispatch('rald7-full-acceptance.yml',SOURCE_BRANCH)
        dispatch('core-runtime-qualification.yml',SOURCE_BRANCH,{'rebuild_current':'true'})
        return {'state':'frontend-inputs-bound-awaiting-source-proof','source':updated}
    if previous==current:
        return {'state':'already-pinned','source':current}
    public_runs=api('actions/workflows/auto-release-termux.yml/runs?branch=main&per_page=100')['workflow_runs']
    if any(run['status'] != 'completed' for run in public_runs):
        return {'state':'awaiting-publication','source':current}
    promoted=commit_files('main',main,{'release-source-v1':(current+'\n').encode()},
                          'Pin accepted base/full product source '+current[:12])
    dispatch('auto-release-termux.yml','main',{'publish':'true','rebuild_current':'true','variant':'both'})
    return {'state':'accepted-product-pinned-public-proof-started','source':current,'main':promoted}



if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source',type=Path,required=True)
    args=parser.parse_args()
    print(json.dumps(run(args.source.resolve())))
