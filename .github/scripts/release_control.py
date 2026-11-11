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

from tui_artifact import verify as verify_frontend, verify_recipe
from upstream_release import load, build_recipe, bind_frontend
from release_components import base_fingerprint, verified_published_base
from release_preflight import stable_version
from full_qualification import read_qualified_full

from qualification_signal import (REPO, SOURCE_BRANCH, GitHubFailure, gh, api,
                                  owned_run, accepted_run, successful_run)


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
                    # Actions transport strips executable modes. Restore only
                    # the admitted binary's mode in the immutable input bundle.
                    info.mode=0o755 if label=='artifact' and path.relative_to(root)==Path('codex') else 0o644
                    with path.open('rb') as stream:archive.addfile(info, stream)
    return output.getvalue()


def frontend_recipe_matches(source, sha):
    if re.fullmatch('[0-9a-f]{40}',sha) is None:
        return False
    try:
        subprocess.run(['git','-C',str(source),'fetch','--no-tags','--depth=1','origin',sha],check=True,capture_output=True)
        verify_recipe(source,sha)
        return True
    except (ValueError,OSError,subprocess.CalledProcessError):
        return False


def native_build_inflight(source, runs):
    return any(owned_run(run,'manager-tui-native.yml',run['head_sha'])
               and run.get('status') in ['queued','pending','in_progress','waiting','requested']
               and frontend_recipe_matches(source,run['head_sha']) for run in runs)


def registered_frontend(value, product, main):
    try:
        content=api('contents/release-frontend-v1?ref='+main)['content']
    except GitHubFailure as error:
        if error.detail != 'HTTP 404':raise
        # Published callers before composition registration embed the result.
        return value['frontend']
    encoded=base64.b64decode(content).decode()
    if not encoded:raise ValueError('empty composition result')
    return bind_frontend(value,product,encoded)['frontend']


def publish_frontend_input(item):
    bundle,tag,name,directory=item['bundle'],item['tag'],item['name'],item['directory']
    listed=api('releases?per_page=100')
    existing=next((release for release in listed if release['tag_name']==tag),None)
    if existing is None:
        gh('release','create',tag,str(bundle),'--repo',REPO,'--target','main',
           '--title','Qualified unsigned frontend '+item['version'],
           '--notes','Exact native source '+item['source']+' and execution evidence; the tag is a container on main, not source authority.',
           '--prerelease','--latest=false')
    readback=directory/'readback';readback.mkdir()
    gh('release','download',tag,'--repo',REPO,'--pattern',name,'--dir',str(readback))
    if (readback/name).read_bytes()!=bundle.read_bytes():
        raise ValueError('immutable frontend input readback differs; never overwrite it')


def ensure_frontend(source, current, root, pinned, *, pending, required_run_id=None):
    value = load(source/'release/upstream.json')
    value['frontend'] = pinned
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
    if required_run_id is not None:
        if type(required_run_id) is not int or required_run_id<=0:raise ValueError('invalid Full-selected frontend run')
        exact=api('actions/runs/'+str(required_run_id))
        if not accepted_run(exact,'manager-tui-native.yml',current):
            raise ValueError('Full-selected frontend no longer qualifies for this source')
        candidates=[exact]
    for index, run in enumerate(candidates):
        if not frontend_recipe_matches(source,run['head_sha']):
            continue  # Reject stale recipes before downloading large outputs.
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
        publication={'bundle':bundle,'tag':tag,'name':name,'directory':directory,
                     'version':value['upstream']['version'],'source':run['head_sha']}
        pending.append(publication)
        value['frontend']={'source':run['head_sha'],'run_id':str(run['id']),'sha256':digest,
            'input_url':'https://github.com/'+REPO+'/releases/download/'+tag+'/'+name,'input_sha256':checksum}
        return value
    # Do not pretend stale frontend references qualify a changed recipe. Start
    # its independent actual build, while allowing the accepted Core/base ahead.
    _, runs = successful_run('manager-tui-native.yml', current)
    if not runs and not native_build_inflight(source,recent):
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
    return all(not path or (path.endswith('.md') and path != 'release/notes.md')
               or path == '.gitignore' for path in paths)


def frontend_registration_needed(source, pinned):
    if pinned and frontend_recipe_matches(source,pinned['source']):
        return False
    recent=api('actions/workflows/manager-tui-native.yml/runs?branch='+SOURCE_BRANCH+'&per_page=20')['workflow_runs']
    return any(accepted_run(run,'manager-tui-native.yml',run['head_sha']) and frontend_recipe_matches(source,run['head_sha'])
               for run in recent)


def release_lane(source, previous, current, main):
    # A newly admitted frontend need not republish an already proven base.
    # Any recipe/code change or missing/older base keeps both real lanes.
    try:
        if base_fingerprint(source,previous)==base_fingerprint(source,current):
            if verified_published_base(source,main) is not None:
                return 'full'
    except (ValueError,OSError,KeyError,subprocess.CalledProcessError):
        pass  # Unknown or unauthenticated input state requires both real lanes.
    changed=subprocess.check_output(['git','-C',str(source),'diff','--name-only',previous,current],text=True).splitlines()
    if changed != ['release/upstream.json']:
        return 'both'
    try:
        with tempfile.TemporaryDirectory(prefix='previous-recipe-',dir=os.environ.get('TMPDIR')) as temporary:
            path=Path(temporary)/'upstream.json'
            path.write_bytes(subprocess.check_output(['git','-C',str(source),'show',previous+':release/upstream.json']))
            before, after=load(path), load(source/'release/upstream.json')
        if build_recipe(before)!=build_recipe(after) or after['frontend'] is None:
            return 'both'
        content=api('contents/update-index-base-v1?ref='+main)['content']
        index=base64.b64decode(content).decode().splitlines()
        if (len(index)==4 and index[:2]==['codex-update-index-v1','channel\tstable']
                and re.fullmatch(r'generation_id\tlocal-base-hosted-[0-9-]+-'+previous[:12]+r'-s[1-9][0-9]*',index[2])):
            return 'full'
    except (ValueError,OSError,KeyError,subprocess.CalledProcessError):
        pass
    return 'both'


def caller_compatible(source, main):
    """Never run a new publication protocol through an older immutable caller."""
    try:
        content=api('contents/.github/workflows/auto-release-termux.yml?ref='+main)['content']
        text=base64.b64decode(content).decode()
        pins=re.findall(r'uses: humtr/codex/\.github/workflows/auto-release-termux\.yml@([0-9a-f]{40})',text)
        if len(pins)!=2 or len(set(pins))!=1:return False
        subprocess.run(['git','-C',str(source),'fetch','--no-tags','--depth=1','origin',pins[0]],check=True,capture_output=True)
        for name in ['.github/workflows/auto-release-termux.yml','.github/workflows/qualify-termux-runtime.yml',
                     '.github/workflows/termux-runtime-source.yml','.github/workflows/android-component.yml',
                     'scripts/pin_release.py']:
            identities=[subprocess.check_output(['git','-C',str(source),'rev-parse',revision+':'+name],stderr=subprocess.DEVNULL) for revision in ['HEAD',pins[0]]]
            if identities[0]!=identities[1]:return False
        return True
    except (ValueError,TypeError,OSError,KeyError,subprocess.CalledProcessError):
        return False


def git_head(source):
    return subprocess.check_output(['git','-C',str(source),'rev-parse','HEAD'],text=True).strip()


def release_authorized(source_sha, main_sha, version):
    """An immutable publication-owner decision, never inferred from green CI.

    The scheduled main controller reads this from main, not the candidate
    checkout. Missing intent is a normal development state; invalid records
    fail closed. Existing source/qualification/caller/signature gates still
    apply after authorization.
    """
    if not all(re.fullmatch('[0-9a-f]{40}', item) for item in (source_sha, main_sha)):
        raise ValueError('release intent requires exact source and main identities')
    try:
        record = api('contents/release-intent-v1?ref=' + main_sha)
    except GitHubFailure as error:
        if error.detail == 'HTTP 404':
            return False
        raise
    raw = base64.b64decode(record['content'], validate=False)
    if len(raw) > 4096:
        raise ValueError('release intent exceeds bound')
    from release_components import unique
    value = json.loads(raw, object_pairs_hook=unique)
    if (not isinstance(value, dict)
            or set(value) != {'schema', 'source_sha', 'upstream_version', 'publication'}
            or value['schema'] != 'codex-release-intent-v1'
            or not isinstance(value['source_sha'], str)
            or re.fullmatch('[0-9a-f]{40}', value['source_sha']) is None
            or value['publication'] is not True):
        raise ValueError('invalid explicit release intent')
    stable_version(value['upstream_version'])
    return value['source_sha'] == source_sha and value['upstream_version'] == version


def run(source, requested_upstream=None):
    current=subprocess.check_output(['git','-C',str(source),'rev-parse','HEAD'],text=True).strip()
    if api('git/ref/heads/'+SOURCE_BRANCH)['object']['sha']!=current:
        return {'state':'obsolete-source-ignored'}
    if requested_upstream is not None:
        stable_version(requested_upstream)
        if requested_upstream != load(source/'release/upstream.json')['upstream']['version']:
            raise ValueError('requested upstream differs from the selected exact source; build not started')
    main=api('git/ref/heads/main')['object']['sha']
    if not release_authorized(current, main, load(source/'release/upstream.json')['upstream']['version']):
        return {'state':'awaiting-release-authorization','source':current,'main':main}
    pointer=api('contents/release-source-v1?ref='+main)
    previous=base64.b64decode(pointer['content']).decode().strip()
    before=load(source/'release/upstream.json')
    registered=registered_frontend(before,previous,main)
    product=current
    registration_pending=False
    if previous != current and same_product_inputs(source, previous, current):
        registration_pending=frontend_registration_needed(source,registered)
        if not registration_pending and requested_upstream is None:
            return {'state':'no-product-change','source':current,'accepted_source':previous}
        product=previous
    for workflow in ['source-acceptance.yml','core-runtime-qualification.yml']:
        passed, runs=successful_run(workflow,product)
        if not passed:
            if product != current:
                raise ValueError('accepted product proof is missing; requested build not started')
            owned=[run for run in runs if owned_run(run,workflow,current)]
            if not owned:dispatch(workflow,SOURCE_BRANCH,{'rebuild_current':'true'} if workflow.startswith('core-') else None)
            if owned and all(run.get('status')=='completed' for run in owned):
                failed=owned[0]
                return {'state':'qualification-failed','workflow':workflow,'source':current,
                        'run_id':failed['id'],'conclusion':failed.get('conclusion'),
                        'url':'https://github.com/'+REPO+'/actions/runs/'+str(failed['id'])}
            return {'state':'awaiting-exact-proof','workflow':workflow,'source':current}
    with tempfile.TemporaryDirectory(prefix='release-inputs-',dir=os.environ.get('TMPDIR')) as temporary:
        root=Path(temporary);pending=[]
        value=None
        if product==current or registration_pending:
            selection=root/'frontend';selection.mkdir()
            value=ensure_frontend(source,product,selection,registered,pending=pending)
        selected = value if value is not None else {**before,'frontend':registered}
        binding = {'source': product, 'frontend': selected['frontend']}
        changed_binding = selected['frontend'] != registered
        if previous==product and not changed_binding and requested_upstream is None:
            return {'state':'already-pinned','source':current}
        public_runs=api('actions/workflows/auto-release-termux.yml/runs?branch=main&per_page=100')['workflow_runs']
        if any(run['status'] != 'completed' for run in public_runs):
            return {'state':'awaiting-publication','source':current}
        if not caller_compatible(source,main):
            return {'state':'awaiting-accepted-caller-repin','source':current,'main':main}
        variant=release_lane(source,previous,product,main)
        if selected['frontend'] is None:
            if variant=='full':
                return {'state':'awaiting-exact-proof','workflow':'manager-tui-native.yml','source':product}
            variant='base'
        if variant in ('full','both'):
            admitted=read_qualified_full(product,root/'full-proof')
            if admitted is None:
                if variant=='full':
                    return {'state':'awaiting-exact-proof','workflow':'full-qualification.yml','source':product}
                # Both lanes explicitly permit the independent Core first.
                # Do not publish an unqualified optional frontend along with it.
                variant='base';pending.clear()
                selected={**selected,'frontend':None}
                binding={'source':product,'frontend':None}
                changed_binding=registered is not None
            else:
                _,proof=admitted
                if proof['upstream_version']!=before['upstream']['version']:
                    raise ValueError('Full-qualified upstream version differs; publication not started')
                required=proof['component_runs']['tui']
                if selected['frontend']['run_id']!=str(required):
                    pending.clear()
                    selection=root/'qualified-frontend';selection.mkdir()
                    selected=ensure_frontend(source,product,selection,registered,
                                             pending=pending,required_run_id=required)
                    if selected['frontend'] is None or selected['frontend']['run_id']!=str(required):
                        raise ValueError('Full-qualified frontend bytes are unavailable; publication not started')
                    binding={'source':product,'frontend':selected['frontend']}
                    changed_binding=selected['frontend']!=registered
        if api('git/ref/heads/main')['object']['sha']!=main:
            raise ValueError('publication main moved before qualified selection')
        for publication in pending:publish_frontend_input(publication)
        if previous==product and not changed_binding:
            dispatch('auto-release-termux.yml','main',{'publish':'true','rebuild_current':'true','variant':variant,
                                                    'upstream_version':before['upstream']['version']})
            return {'state':'requested-build-started','source':product,'version':requested_upstream,'variant':variant}
        promoted=commit_files('main',main,{
            'release-source-v1':(product+'\n').encode(),
            'release-frontend-v1':(json.dumps(binding,sort_keys=True)+'\n').encode(),
        }, 'Pin accepted source and qualified composition results '+product[:12])
        dispatch('auto-release-termux.yml','main',{'publish':'true','rebuild_current':'true','variant':variant,
                                                'upstream_version':before['upstream']['version']})
        return {'state':'accepted-product-pinned-public-proof-started','source':product,'main':promoted}




if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source',type=Path,required=True)
    parser.add_argument('--requested-upstream')
    args=parser.parse_args()
    print(json.dumps(run(args.source.resolve(),args.requested_upstream or None)))
