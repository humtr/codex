"""Unsigned exact-source Full composition from independently qualified bytes."""
import argparse
from contextlib import contextmanager, ExitStack
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import stat
import subprocess
import tarfile
import tempfile
import zipfile
from types import SimpleNamespace
from component_artifact import regular,unpack as unpack_component,verify as verify_component
from component_inputs import git
from component_revisions import selected,candidate_digest
from qualification_signal import api,gh,successful_run,accepted_run,REPO
from qualified_component import download as download_artifact
from release_components import candidate_from_base,unique
from release_preflight import verify_base_candidate,verify_candidate,verify_qualified_candidate,parse_descriptor
from release_publication import BASE_FILES,EXTENDED_FILES,sha256
from runtime_inputs import canonical

WORKFLOWS={'core':'core-runtime-qualification.yml','manager':'manager-qualification.yml','tui':'manager-tui-native.yml'}
INPUT_OWNERS={'android-core':'core','core-candidate':'core','android-manager':'manager',
              'manager-proof':'manager','tui':'tui','tui-proof':'tui'}
INPUT_NAMES={'android-core':'android-core','android-manager':'android-manager',
             'manager-proof':'manager-qualification','tui':'manager-tui-native','tui-proof':'manager-tui-native-proof'}
LIMIT=1024*1024*1024
PROOF_ARCHIVE_LIMIT=1024*1024
PROOF_FILES={'full-qualification.json':32768,'full-inputs/inputs.json':262144,
             'full-probes/execution.json':16384,
             **{'full-probes/'+name+suffix:4096 for name in ('core','manager','runtime','tui')
                for suffix in ('.out','.err')}}
PROOF_DIRECTORIES={'full-inputs/','full-probes/'}


def resolve(source,request=api,lookup=successful_run):
    sha=git(source,'rev-parse','HEAD').decode().strip()
    if re.fullmatch('[0-9a-f]{40}',sha) is None:raise ValueError('Full qualification needs an exact source commit')
    version=json.loads(git(source,'show','HEAD:release/upstream.json'))['upstream']['version']
    runs={};artifacts={}
    for name,workflow in WORKFLOWS.items():
        run,_=lookup(workflow,sha)
        if run is None or not accepted_run(run,workflow,sha):
            raise ValueError('no successful exact-source '+name+' qualification; rerun Full after its completion')
        page=request('actions/runs/'+str(run['id'])+'/artifacts?per_page=100')
        rows=page['artifacts']
        if page['total_count']!=len(rows) or len(rows)>100:raise ValueError('incomplete component artifact inventory')
        suffix=version.replace('.','-')+'-'+sha[:12]+'-s'
        wanted={'core':{'android-core':lambda v:v=='android-core','core-candidate':lambda v:v.startswith('base-qualified-'+suffix)},
                'manager':{'android-manager':lambda v:v=='android-manager','manager-proof':lambda v:v=='manager-qualification'},
                'tui':{'tui':lambda v:v=='manager-tui-native','tui-proof':lambda v:v=='manager-tui-native-proof'}}[name]
        for key,matches in wanted.items():
            found=[row for row in rows if matches(row.get('name','')) and row.get('expired') is False]
            if len(found)!=1:raise ValueError('missing, expired or ambiguous '+key+' artifact')
            artifacts[key]={'run_id':run['id'],'name':found[0]['name'],'id':found[0]['id']}
        runs[name]=run
    return {'schema':'codex-full-inputs-v1','source_sha':sha,'upstream_version':version,'runs':runs,'artifacts':artifacts}


def admit_plan(source,plan):
    return validate_plan(plan,git(source,'rev-parse','HEAD').decode().strip(),
                         json.loads(git(source,'show','HEAD:release/upstream.json'))['upstream']['version'])


def validate_plan(plan,sha,version):
    if (not isinstance(plan,dict) or set(plan)!={'schema','source_sha','upstream_version','runs','artifacts'}
            or plan['schema']!='codex-full-inputs-v1'
            or plan['source_sha']!=sha or plan['upstream_version']!=version
            or set(plan['runs'])!=set(WORKFLOWS)
            or set(plan['artifacts'])!={'android-core','core-candidate','android-manager','manager-proof','tui','tui-proof'}):
        raise ValueError('Full input authority differs')
    for name,workflow in WORKFLOWS.items():
        run=plan['runs'][name]
        if (type(run.get('id')) is not int or run['id']<=0
                or not accepted_run(run,workflow,plan['source_sha'])):
            raise ValueError('Full input run is unqualified: '+name)
    for key,entry in plan['artifacts'].items():
        expected=(re.escape('base-qualified-'+plan['upstream_version'].replace('.','-')+'-'+plan['source_sha'][:12]+'-s')+'[1-9][0-9]*'
                  if key=='core-candidate' else re.escape(INPUT_NAMES[key]))
        if (set(entry)!={'run_id','name','id'} or type(entry['id']) is not int or entry['id']<=0
                or type(entry['run_id']) is not int or entry['run_id']!=plan['runs'][INPUT_OWNERS[key]]['id']
                or not isinstance(entry['name'],str) or re.fullmatch(expected,entry['name']) is None):
            raise ValueError('Full artifact belongs to another run or role')
    return plan


def input_artifact(entry,sha,request):
    row=request('actions/artifacts/'+str(entry['id']))
    if (not isinstance(row,dict) or type(row.get('id')) is not int or row['id']!=entry['id']
            or row.get('name')!=entry['name'] or row.get('expired') is not False
            or type(row.get('size_in_bytes')) is not int or not 0<row['size_in_bytes']<=LIMIT
            or not isinstance(row.get('digest'),str) or re.fullmatch('sha256:[0-9a-f]{64}',row['digest']) is None
            or not isinstance(row.get('workflow_run'),dict)
            or row['workflow_run'].get('id')!=entry['run_id'] or row['workflow_run'].get('head_sha')!=sha):
        raise ValueError('Full input artifact differs from the original selected ID, run or source')
    return row


def acquire_input(key,entry,sha,target,request=api,fetch=download_artifact):
    row=input_artifact(entry,sha,request)
    target.mkdir(mode=0o700)
    try:
        archive=target/'.download.zip';fetch(row,archive)
        with os.fdopen(os.open(archive,os.O_RDONLY|os.O_NOFOLLOW|os.O_NONBLOCK),'rb') as stream:
            before=os.fstat(stream.fileno())
            if (not stat.S_ISREG(before.st_mode) or before.st_size!=row['size_in_bytes']
                    or 'sha256:'+hashlib.file_digest(stream,'sha256').hexdigest()!=row['digest']):
                raise ValueError('Full input archive digest or byte count differs')
            stream.seek(0)
            with zipfile.ZipFile(stream) as zipped:
                entries=zipped.infolist();names=set();files=set();total=0
                if not 1<=len(entries)<=512:raise ValueError('Full input ZIP member count differs')
                for item in entries:
                    name=item.filename;path=PurePosixPath(name.rstrip('/') if item.is_dir() else name)
                    kind=stat.S_IFMT(item.external_attr>>16)
                    if (not name or path.is_absolute() or str(path)!=name.rstrip('/') or '..' in path.parts
                            or '\\' in name or str(path) in names or len(name.encode())>1024
                            or item.flag_bits&1 or item.compress_type not in (zipfile.ZIP_STORED,zipfile.ZIP_DEFLATED)):
                        raise ValueError('unsafe or duplicate Full input ZIP member')
                    names.add(str(path));total+=item.file_size
                    if not 0<=item.file_size<=LIMIT or total>LIMIT:raise ValueError('Full input ZIP payload exceeds bound')
                    if item.is_dir():
                        if item.file_size or kind not in (0,stat.S_IFDIR):raise ValueError('invalid Full input ZIP directory')
                    else:
                        if kind not in (0,stat.S_IFREG):raise ValueError('nonregular Full input ZIP file')
                        files.add(str(path))
                fixed={'android-core':{'component.tar'},'android-manager':{'component.tar'},
                       'core-candidate':{'qualified-candidate.tar'},'manager-proof':{'manager-qualification.json'}}
                if key=='tui':
                    from tui_artifact import FILES
                    fixed['tui']=FILES
                if key in fixed and files!=fixed[key]:raise ValueError('Full input ZIP role inventory differs')
                for name in names:
                    path=PurePosixPath(name.rstrip('/'))
                    if any(str(parent) in files for parent in path.parents):
                        raise ValueError('Full input ZIP file aliases a directory')
                for item in entries:
                    if item.is_dir():continue
                    relative=PurePosixPath(item.filename);parent=target
                    for part in relative.parts[:-1]:
                        parent/=part;parent.mkdir(mode=0o700,exist_ok=True)
                    with zipped.open(item) as incoming,(target/item.filename).open('xb') as output:
                        shutil.copyfileobj(incoming,output,1024*1024)
                        if output.tell()!=item.file_size:raise ValueError('Full input ZIP payload is truncated')
                after=os.fstat(stream.fileno())
                if (before.st_size,before.st_mtime_ns,before.st_mode)!=(after.st_size,after.st_mtime_ns,after.st_mode):
                    raise ValueError('Full input archive changed during verification')
        archive.unlink()
        return {'id':row['id'],'name':row['name'],'run_id':entry['run_id'],'source_sha':sha,
                'archive_digest':row['digest'],'archive_bytes':row['size_in_bytes']}
    except BaseException:
        shutil.rmtree(target)
        raise


def acquire(source,inputs,plan=None,request=api,lookup=successful_run,fetch=download_artifact):
    # A saved Full receipt plan selects immutable IDs; never replace it with
    # newer successful runs or reinterpret the artifact names during transfer.
    plan=admit_plan(source,resolve(source,request,lookup) if plan is None else plan)
    def current_runs():
        for name,run in plan['runs'].items():
            current=request('actions/runs/'+str(run['id']))
            if current.get('id')!=run['id'] or not accepted_run(current,WORKFLOWS[name],plan['source_sha']):
                raise ValueError('selected Full input run no longer qualifies: '+name)
    current_runs()
    inputs.mkdir(mode=0o700)
    try:
        origins={}
        for key,entry in plan['artifacts'].items():
            origins[key]=acquire_input(key,entry,plan['source_sha'],inputs/key,request,fetch)
        current_runs()
        (inputs/'inputs.json').write_bytes(canonical(plan))
        (inputs/'artifact-origins.json').write_bytes(canonical(
            {'schema':'codex-full-input-artifacts-v1','source_sha':plan['source_sha'],'artifacts':origins}))
        return plan
    except BaseException:
        shutil.rmtree(inputs)
        raise


def unpack_candidate(archive_path,output):
    """Admit fixed regular-file inventory before any extraction; never tar -xf."""
    if output.exists() or output.is_symlink():raise ValueError('candidate extraction destination exists')
    with os.fdopen(os.open(archive_path,os.O_RDONLY|os.O_NOFOLLOW|os.O_NONBLOCK),'rb') as stream:
        info=os.fstat(stream.fileno())
        if not stat.S_ISREG(info.st_mode) or info.st_size>LIMIT:raise ValueError('candidate transfer exceeds bound')
        with tarfile.open(fileobj=stream,mode='r:') as archive:
            entries=[];offset=0;total=0;names=set()
            for entry in archive:
                if len(entries)>=8 or entry.offset!=offset or entry.offset_data!=offset+512:
                    raise ValueError('unexpected candidate transfer header or entry count')
                name=entry.name
                if name in names or entry.pax_headers:raise ValueError('duplicate or extended candidate transfer entry')
                names.add(name);entries.append(entry)
                if name in {'candidate','candidate/helpers'}:
                    if not entry.isdir() or entry.size!=0:raise ValueError('invalid candidate transfer directory')
                elif name.startswith('candidate/') and name[10:] in BASE_FILES:
                    if not entry.isfile() or not 0<=entry.size<=536870912:raise ValueError('invalid candidate transfer payload')
                    if entry.mode!=int(BASE_FILES[name[10:]],8):raise ValueError('candidate transfer mode differs')
                else:raise ValueError('unexpected candidate transfer path')
                total+=entry.size
                if total>LIMIT-32768:raise ValueError('candidate transfer payload exceeds bound')
                offset=entry.offset_data+((entry.size+511)//512)*512
            if names!={'candidate','candidate/helpers'}|{'candidate/'+name for name in BASE_FILES}:
                raise ValueError('candidate transfer inventory differs')
            # Two EOF blocks can cross a record boundary: valid padding reaches10752 bytes.
            expected_ending=1024+(-(offset+1024)%tarfile.RECORDSIZE)
            stream.seek(offset);tail=stream.read(expected_ending+1)
            if len(tail)!=expected_ending or any(tail):raise ValueError('candidate transfer trailing bytes differ')
            output.mkdir(mode=0o700);(output/'helpers').mkdir(mode=0o700)
            try:
                for entry in entries:
                    if entry.isfile():
                        target=output/entry.name[10:]
                        with archive.extractfile(entry) as incoming,target.open('xb') as target_stream:
                            shutil.copyfileobj(incoming,target_stream,1024*1024)
                        target.chmod(int(BASE_FILES[entry.name[10:]],8))
                after=os.fstat(stream.fileno())
                if (info.st_size,info.st_mtime_ns)!=(after.st_size,after.st_mtime_ns):raise ValueError('candidate transfer changed')
            except BaseException:
                shutil.rmtree(output);raise


def manager_proof(source,plan,inputs,manager):
    value=json.loads(regular(inputs/'manager-proof/manager-qualification.json',32768),object_pairs_hook=unique)
    expected_probe=hashlib.sha256(b'codex-manager-artifact-v1\ncore_api=codex-manager-core-v1\n').hexdigest()
    if (set(value)!={'schema','component','qualification_source','run_id','checks','probe_sha256'}
            or value['schema']!='codex-component-qualification-v1' or value['component']!=manager
            or value['qualification_source']!=plan['source_sha']
            or value['run_id']!=str(plan['runs']['manager']['id'])
            or value['checks']!=['android-interface-probe','manager-behavior-regressions']
            or value['probe_sha256']!=expected_probe):
        raise ValueError('Manager proof does not bind selected bytes and behavior')


def assemble(source,inputs,builder,output):
    plan=admit_plan(source,json.loads(regular(inputs/'inputs.json',262144),object_pairs_hook=unique))
    core_dir=inputs/'core';manager_dir=inputs/'manager'
    unpack_component(source,'core',core_dir,inputs/'android-core')
    unpack_component(source,'manager',manager_dir,inputs/'android-manager')
    core=verify_component(source,'core',core_dir);manager=verify_component(source,'manager',manager_dir)
    manager_proof(source,plan,inputs,manager)
    base=inputs/'base';unpack_candidate(inputs/'core-candidate/qualified-candidate.tar',base)
    descriptor=verify_base_candidate(base,plan['upstream_version'])
    recipe=json.loads(git(source,'show','HEAD:release/upstream.json'))
    if (descriptor['source_artifact_digest']!=recipe['upstream']['archive_sha256']
            or descriptor['core_artifact_digest']!=core['artifact_sha256']
            or descriptor['core_api_identity']!='core-api-v1' or core['interface']!=manager['interface']):
        raise ValueError('Core/runtime and Manager interface or exact bytes differ')
    from tui_artifact import verify
    run_json=inputs/'tui-run.json';run_json.write_bytes(canonical(plan['runs']['tui']))
    frontend=inputs/'tui';digest=sha256(frontend/'codex')
    verify(SimpleNamespace(source=source,source_sha=plan['source_sha'],run_json=run_json,
        run_id=plan['runs']['tui']['id'],version=plan['upstream_version'],sha256=digest,
        artifact=frontend,proof=inputs/'tui-proof'))
    (frontend/'codex').chmod(0o755)  # Actions loose-file transport omits executable mode.
    generation='full-qualification-'+plan['upstream_version'].replace('.','-')+'-'+plan['source_sha'][:12]
    candidate_from_base(base,manager_dir/'codex-manager',frontend/'codex',builder,generation,
                        plan['upstream_version'],output,qualified=True)
    return plan


def verify_full(candidate,version, *, deferred=False):
    descriptor=(verify_candidate if deferred else verify_qualified_candidate)(candidate,version)
    if not descriptor['generation_id'].startswith('full-qualification-') or descriptor['helper_count']!='3':
        raise ValueError('Full proof requires an unpublishable exact three-helper composition')
    candidate_digest(candidate,deferred=deferred)  # Complete regular-file inventory and modes.
    names=['termux-browser-open-v1','termux-browser-manual-v1','termux-manager-tui-v1:'+version]
    if {k for k in descriptor if k.startswith('helper:')}!={'helper:'+name for name in names}:
        raise ValueError('Full helper declarations differ')
    for index,name in enumerate(names):
        if descriptor['helper:'+name]!=sha256(candidate/'helpers'/str(index)):raise ValueError('Full helper digest differs')
    report=dict(part.split('=',1) for part in descriptor['patch_report'].split(';')[1:])
    if report.get('code_mode_host_sha256')!=sha256(candidate/'codex-code-mode-host'):
        raise ValueError('Full companion digest differs')
    return descriptor


def execution_outputs(log_root,version):
    logs={}
    linker_warning=(b'linker: Warning: failed to find generated linker configuration from "/linkerconfig/ld.config.txt"\n'
                    b'WARNING: linker: Warning: failed to find generated linker configuration from "/linkerconfig/ld.config.txt"\n')
    for name,expected in {'manager':b'codex-manager-artifact-v1\ncore_api=codex-manager-core-v1\n',
            'runtime':('codex-cli '+version+'\n').encode(),
            'tui':('codex-cli '+version+'\n').encode()}.items():
        data=regular(log_root/(name+'.out'),4096)
        if data!=expected:raise ValueError('Full '+name+' execution result differs')
        logs[name]=hashlib.sha256(data).hexdigest()
    core=regular(log_root/'core.out',4096)
    if not all(token in core for token in (b'usage: codex update ',b'--wait',b'--no-wait')):
        raise ValueError('Full Core update interface proof differs')
    logs['core']=hashlib.sha256(core).hexdigest()
    for name in ('manager','core','runtime','tui'):
        stderr=regular(log_root/(name+'.err'),4096)
        if stderr not in ((b'',linker_warning) if name in ('manager','core') else (b'',)):
            raise ValueError('Full '+name+' produced unexpected diagnostic output')
    return logs


def owned_private_directory(path):
    info=path.lstat()
    return stat.S_ISDIR(info.st_mode) and info.st_uid==os.getuid() and stat.S_IMODE(info.st_mode)==0o700


@contextmanager
def probe_environment(log_root,home_parent):
    """Isolate upstream alias/config creation without inheriting a user's home."""
    log_root=log_root.resolve(strict=True)
    if not owned_private_directory(log_root):raise ValueError('Full probe log root must be owned and private')
    home_parent=home_parent.resolve(strict=True)
    parent=home_parent.lstat()
    if not stat.S_ISDIR(parent.st_mode) or parent.st_uid!=os.getuid() or parent.st_mode&0o022:
        raise ValueError('Full probe home parent must be owned and protected')
    temporary=Path(tempfile.gettempdir()).resolve(strict=True)
    with ExitStack() as cleanup:
        if not owned_private_directory(temporary):
            parent=temporary.stat()
            protected=(parent.st_uid==os.getuid() and not parent.st_mode&0o022) or (
                parent.st_uid==0 and parent.st_mode&stat.S_ISVTX)
            if not stat.S_ISDIR(parent.st_mode) or not protected:
                raise ValueError('Full probe temporary parent is not protected')
            temporary=Path(cleanup.enter_context(tempfile.TemporaryDirectory(prefix='fp-',dir=temporary))).resolve(strict=True)
        # A Termux private socket root may already consume almost all sun_path.
        # Borrow only that validated root; never remove it or create a long alias.
        if not owned_private_directory(temporary):raise ValueError('Full probe temporary root must be owned and private')
        if len(os.fsencode(temporary/str(os.getuid())/('0'*64)))>107:
            raise ValueError('Full probe temporary socket root is too long')
        # Release arg0 refuses alias creation beneath the actual TMPDIR. Resolve
        # both paths before admission; a short symlink must not bypass this guard.
        if home_parent.is_relative_to(temporary):
            raise ValueError('Full probe home parent must be outside canonical temporary root')
        probe_home=Path(cleanup.enter_context(tempfile.TemporaryDirectory(prefix='full-probe-',dir=home_parent))).resolve(strict=True)
        if not owned_private_directory(probe_home) or probe_home.is_relative_to(temporary):
            raise ValueError('Full probe home must be private and outside canonical temporary root')
        yield {'HOME':str(probe_home),'TMPDIR':str(temporary)}


def probe(source,inputs,candidate,log_root,run_id,*,home_parent):
    plan=admit_plan(source,json.loads(regular(inputs/'inputs.json',262144),object_pairs_hook=unique))
    verify_full(candidate,plan['upstream_version'],deferred=True)
    before=candidate_digest(candidate,deferred=True)
    if not re.fullmatch('[1-9][0-9]*',str(run_id)):raise ValueError('Full probe requires its run identity')
    log_root.mkdir(mode=0o700)
    with probe_environment(log_root,home_parent) as runtime_environment:
        for name,binary,args in [('manager','manager',['--artifact-probe']),('core','core',['update','--help']),
                                  ('runtime','runtime',['--version']),('tui','helpers/2',['--version'])]:
            environment=({'CODEX_MANAGER_ARTIFACT_PROBE':'1'} if name=='manager' else
                         runtime_environment if name in ('runtime','tui') else {})
            with (log_root/(name+'.out')).open('xb') as stdout,(log_root/(name+'.err')).open('xb') as stderr:
                result=subprocess.run([str(candidate/binary),*args],stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,
                                      env=environment,timeout=20)
            if result.returncode!=0:raise ValueError('Full '+name+' probe exited '+str(result.returncode))
    outputs=execution_outputs(log_root,plan['upstream_version'])
    if candidate_digest(candidate,deferred=True)!=before:raise ValueError('Full candidate changed during probes')
    (candidate/'.manager-probe-deferred').unlink()
    verify_full(candidate,plan['upstream_version'])
    receipt={'schema':'codex-full-execution-v1','source_sha':plan['source_sha'],'run_id':str(run_id),
             'candidate_sha256':before,'exit_codes':{name:0 for name in ('manager','core','runtime','tui')},
             'outputs':outputs}
    (log_root/'execution.json').write_bytes(canonical(receipt))
    return receipt


def prove(source,inputs,candidate,log_root,run_id):
    plan=admit_plan(source,json.loads(regular(inputs/'inputs.json',262144),object_pairs_hook=unique))
    descriptor=verify_full(candidate,plan['upstream_version'])
    if not re.fullmatch('[1-9][0-9]*',str(run_id)):raise ValueError('Full proof requires its run identity')
    logs=execution_outputs(log_root,plan['upstream_version'])
    receipt=json.loads(regular(log_root/'execution.json',16384),object_pairs_hook=unique)
    expected={'schema':'codex-full-execution-v1','source_sha':plan['source_sha'],'run_id':str(run_id),
              'candidate_sha256':candidate_digest(candidate),'exit_codes':{name:0 for name in ('manager','core','runtime','tui')},
              'outputs':logs}
    if receipt!=expected:raise ValueError('Full execution receipt does not bind this candidate and run')
    version,components=selected(source,candidate)
    inventory={name:{'sha256':sha256(candidate/name),'mode':mode,'bytes':(candidate/name).stat().st_size}
               for name,mode in sorted(EXTENDED_FILES.items())}
    return {'schema':'codex-full-qualification-v1','source_sha':plan['source_sha'],
        'upstream_version':version,'generation_id':descriptor['generation_id'],'run_id':str(run_id),
        'component_runs':{name:run['id'] for name,run in plan['runs'].items()},
        'components':components,'inventory':inventory,'candidate_sha256':candidate_digest(candidate),
        'descriptor_sha256':hashlib.sha256(canonical({k:v for k,v in descriptor.items() if k!='generation_id'})).hexdigest(),
        'composition_sha256':hashlib.sha256(canonical({'components':components,
            'descriptor':{k:v for k,v in descriptor.items() if k!='generation_id'},
            'payloads':{name:value for name,value in inventory.items() if name!='generation.meta'}})).hexdigest(),
        'checks':['qualified-component-inputs','full-android-interface-probes','runtime-and-tui-version'],
        'execution_outputs':logs}


def admit_published_candidate(source,candidate,value,run):
    """A qualified unsigned selection can bind a future separately signed issue."""
    sha=git(source,'rev-parse','HEAD').decode().strip()
    validate_receipt(value,run,sha)
    version,components=selected(source,candidate)
    if (not accepted_run(run,'full-qualification.yml',sha)
            or not isinstance(value,dict) or set(value)!={'schema','source_sha','upstream_version','generation_id','run_id',
                'component_runs','components','inventory','candidate_sha256','descriptor_sha256','composition_sha256','checks','execution_outputs'}
            or value['schema']!='codex-full-qualification-v1' or value['source_sha']!=sha
            or value['upstream_version']!=version or value['run_id']!=str(run['id'])
            or value['generation_id']!='full-qualification-'+version.replace('.','-')+'-'+sha[:12]
            or set(components)!={'core','runtime','manager','tui'}
            or value['components']!=components or set(value['inventory'])!=set(EXTENDED_FILES)
            or set(value['component_runs'])!=set(WORKFLOWS)
            or any(type(i) is not int or i<=0 for i in value['component_runs'].values())
            or value['checks']!=['qualified-component-inputs','full-android-interface-probes','runtime-and-tui-version']):
        raise ValueError('Full qualification does not bind the selected release source and components')
    candidate_digest(candidate)  # Exact complete inventory, paths and modes.
    descriptor=parse_descriptor(candidate/'generation.meta')
    descriptor={k:v for k,v in descriptor.items() if k!='generation_id'}
    if value['descriptor_sha256']!=hashlib.sha256(canonical(descriptor)).hexdigest():
        raise ValueError('Full qualification descriptor contract differs')
    payloads={name:{'sha256':sha256(candidate/name),'mode':mode,'bytes':(candidate/name).stat().st_size}
              for name,mode in sorted(EXTENDED_FILES.items()) if name!='generation.meta'}
    if any(value['inventory'][name]!=entry for name,entry in payloads.items()):
        raise ValueError('Full qualification payload differs from selected release')
    expected=hashlib.sha256(canonical({'components':components,'descriptor':descriptor,'payloads':payloads})).hexdigest()
    if value['composition_sha256']!=expected:raise ValueError('Full composition binding differs')
    return value


def validate_receipt(value,run,sha):
    """Admit a completed source run's digest-bound receipt before publication."""
    fields={'schema','source_sha','upstream_version','generation_id','run_id','component_runs',
            'components','inventory','candidate_sha256','descriptor_sha256','composition_sha256',
            'checks','execution_outputs'}
    digest=lambda item:isinstance(item,str) and re.fullmatch('[0-9a-f]{64}',item) is not None
    if (not accepted_run(run,'full-qualification.yml',sha) or not isinstance(value,dict) or set(value)!=fields
            or value['schema']!='codex-full-qualification-v1' or value['source_sha']!=sha
            or not isinstance(value['upstream_version'],str)
            or re.fullmatch('[0-9]+[.][0-9]+[.][0-9]+',value['upstream_version']) is None
            or value['generation_id']!='full-qualification-'+value['upstream_version'].replace('.','-')+'-'+sha[:12]
            or value['run_id']!=str(run['id']) or not isinstance(value['component_runs'],dict)
            or set(value['component_runs'])!=set(WORKFLOWS)
            or any(type(number) is not int or number<=0 for number in value['component_runs'].values())
            or any(not digest(value[key]) for key in ('candidate_sha256','descriptor_sha256','composition_sha256'))
            or value['checks']!=['qualified-component-inputs','full-android-interface-probes','runtime-and-tui-version']):
        raise ValueError('Full qualification receipt source or execution boundary differs')
    components=value['components'];inventory=value['inventory'];outputs=value['execution_outputs']
    if (not isinstance(components,dict) or set(components)!={'core','runtime','manager','tui'}
            or not isinstance(inventory,dict) or set(inventory)!=set(EXTENDED_FILES)
            or not isinstance(outputs,dict) or set(outputs)!={'manager','core','runtime','tui'}
            or any(not digest(item) for item in outputs.values())):
        raise ValueError('Full receipt complete component inventory differs')
    expected_outputs={'manager':b'codex-manager-artifact-v1\ncore_api=codex-manager-core-v1\n',
                      'runtime':('codex-cli '+value['upstream_version']+'\n').encode(),
                      'tui':('codex-cli '+value['upstream_version']+'\n').encode()}
    if any(outputs[name]!=hashlib.sha256(output).hexdigest() for name,output in expected_outputs.items()):
        raise ValueError('Full receipt version or Manager execution output differs')
    for name,entry in inventory.items():
        if (not isinstance(entry,dict) or set(entry)!={'sha256','mode','bytes'} or not digest(entry['sha256'])
                or entry['mode']!=EXTENDED_FILES[name] or type(entry['bytes']) is not int
                or not 0<=entry['bytes']<=536870912):
            raise ValueError('Full receipt payload identity differs')
    for name,path in {'core':'core','runtime':'runtime','manager':'manager','tui':'helpers/2'}.items():
        entry=components[name]
        if (not isinstance(entry,dict) or set(entry)!={'inputs_sha256','artifact_sha256','interface'}
                or not digest(entry['inputs_sha256']) or entry['artifact_sha256']!=inventory[path]['sha256']
                or not isinstance(entry['interface'],str)
                or re.fullmatch('[A-Za-z0-9._:-]{1,96}',entry['interface']) is None):
            raise ValueError('Full receipt component digest differs')
    selected={name:{'sha256':entry['sha256'],'mode':int(entry['mode'],8),'size':entry['bytes']}
              for name,entry in inventory.items()}
    if value['candidate_sha256']!=hashlib.sha256(canonical(selected)).hexdigest():
        raise ValueError('Full receipt candidate digest differs')
    return value


def proof_artifact(row,run):
    if (not isinstance(row,dict) or type(row.get('id')) is not int or row['id']<=0
            or row.get('name')!='full-composition-qualification' or row.get('expired') is not False
            or type(row.get('size_in_bytes')) is not int or not 0<row['size_in_bytes']<=PROOF_ARCHIVE_LIMIT
            or not isinstance(row.get('digest'),str) or re.fullmatch('sha256:[0-9a-f]{64}',row['digest']) is None
            or not isinstance(row.get('workflow_run'),dict) or row['workflow_run'].get('id')!=run['id']
            or row['workflow_run'].get('head_sha')!=run['head_sha']):
        raise ValueError('Full proof artifact authority differs')
    return row


def read_proof_archive(row,directory,fetch):
    archive=directory/'download.zip';fetch(row,archive)
    data=regular(archive,PROOF_ARCHIVE_LIMIT)
    if len(data)!=row['size_in_bytes'] or 'sha256:'+hashlib.sha256(data).hexdigest()!=row['digest']:
        raise ValueError('Full proof archive digest or byte count differs')
    with zipfile.ZipFile(io.BytesIO(data)) as zipped:
        entries=zipped.infolist();seen=set();files=set()
        if len(entries)>len(PROOF_FILES)+len(PROOF_DIRECTORIES):raise ValueError('Full proof archive inventory exceeds bound')
        for entry in entries:
            name=entry.filename;kind=stat.S_IFMT(entry.external_attr>>16)
            if name in seen or entry.flag_bits&1 or entry.compress_type not in (zipfile.ZIP_STORED,zipfile.ZIP_DEFLATED):
                raise ValueError('duplicate or unsupported Full proof ZIP entry')
            seen.add(name)
            if entry.is_dir():
                if name not in PROOF_DIRECTORIES or entry.file_size!=0 or kind not in (0,stat.S_IFDIR):
                    raise ValueError('invalid Full proof ZIP directory')
            else:
                if name not in PROOF_FILES or not 0<=entry.file_size<=PROOF_FILES[name] or kind not in (0,stat.S_IFREG):
                    raise ValueError('Full proof ZIP inventory, kind or size differs')
                files.add(name)
        if files!=set(PROOF_FILES):raise ValueError('Full proof ZIP inventory is incomplete')
        with zipped.open('full-qualification.json') as incoming:
            payload=incoming.read(PROOF_FILES['full-qualification.json']+1)
        with zipped.open('full-inputs/inputs.json') as incoming:
            original_inputs=incoming.read(PROOF_FILES['full-inputs/inputs.json']+1)
        if (len(payload)!=zipped.getinfo('full-qualification.json').file_size
                or len(original_inputs)!=zipped.getinfo('full-inputs/inputs.json').file_size):
            raise ValueError('Full proof receipt or input plan length differs')
    value=json.loads(payload,object_pairs_hook=unique)
    plan=validate_plan(json.loads(original_inputs,object_pairs_hook=unique),value['source_sha'],value['upstream_version'])
    if {name:run['id'] for name,run in plan['runs'].items()}!=value['component_runs']:
        raise ValueError('Full proof contains a different component input plan')
    (directory/'full-qualification.json').write_bytes(payload)
    (directory/'inputs.json').write_bytes(original_inputs)
    archive.unlink()
    return value


def read_qualified_full(sha,directory,request=api,lookup=successful_run,fetch=download_artifact):
    """Read-only admission; absence grants no Full publication authority."""
    if not isinstance(sha,str) or re.fullmatch('[0-9a-f]{40}',sha) is None:
        raise ValueError('Full proof requires exact source')
    run,_=lookup('full-qualification.yml',sha)
    if run is None or not accepted_run(run,'full-qualification.yml',sha):return None
    if type(run.get('id')) is not int or run['id']<=0:raise ValueError('Full run identity is invalid')
    page=request('actions/runs/'+str(run['id'])+'/artifacts?per_page=100')
    if page['total_count']!=len(page['artifacts']) or len(page['artifacts'])>100:
        raise ValueError('incomplete Full proof artifact listing')
    matches=[entry for entry in page['artifacts'] if entry['name']=='full-composition-qualification' and entry['expired'] is False]
    if len(matches)!=1:raise ValueError('Full composition proof is missing, expired or ambiguous')
    selected=proof_artifact(matches[0],run)
    actual=proof_artifact(request('actions/artifacts/'+str(selected['id'])),run)
    if any(actual[key]!=selected[key] for key in ('id','digest','size_in_bytes')):
        raise ValueError('Full proof archive identity changed')
    directory=Path(directory);directory.mkdir(mode=0o700)
    try:
        value=read_proof_archive(actual,directory,fetch)
        validate_receipt(value,run,sha)
        current=request('actions/runs/'+str(run['id']))
        if current.get('id')!=run['id'] or not accepted_run(current,'full-qualification.yml',sha):
            raise ValueError('Full run no longer qualifies')
        for name,number in value['component_runs'].items():
            current=request('actions/runs/'+str(number))
            if current.get('id')!=number or not accepted_run(current,WORKFLOWS[name],sha):
                raise ValueError('Full component run no longer qualifies: '+name)
        origin={'schema':'codex-full-proof-origin-v1','source_sha':sha,'run_id':run['id'],
                'artifact_id':actual['id'],'archive_digest':actual['digest'],'archive_bytes':actual['size_in_bytes']}
        (directory/'artifact-origin.json').write_bytes(canonical(origin))
        return run,value
    except BaseException:
        shutil.rmtree(directory)
        raise


def require_qualified_full(source,candidate,directory,request=api,lookup=successful_run,fetch=download_artifact):
    sha=git(source,'rev-parse','HEAD').decode().strip()
    admitted=read_qualified_full(sha,directory,request,lookup,fetch)
    if admitted is None:raise ValueError('selected Full needs a successful exact-source composition qualification')
    run,value=admitted
    # The earlier controller receipt is not signing authority: compare every
    # actual candidate byte, normalized descriptor and input again here.
    admit_published_candidate(source,candidate,value,run)
    return value


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action',choices=('acquire','assemble','probe','prove'))
    parser.add_argument('--source',type=Path,required=True);parser.add_argument('--inputs',type=Path,required=True)
    parser.add_argument('--builder',type=Path);parser.add_argument('--candidate',type=Path)
    parser.add_argument('--logs',type=Path);parser.add_argument('--run-id');parser.add_argument('--output',type=Path)
    parser.add_argument('--home-parent',type=Path)
    parser.add_argument('--plan',type=Path,help='Reuse the original admitted Full input IDs; do not select newer runs')
    args=parser.parse_args()
    if args.action=='acquire':
        plan=(json.loads(regular(args.plan,262144),object_pairs_hook=unique) if args.plan else None)
        acquire(args.source,args.inputs,plan)
    elif args.action=='assemble':
        if args.builder is None or args.candidate is None:parser.error('assemble requires builder and candidate')
        assemble(args.source,args.inputs,args.builder,args.candidate)
    elif args.action=='probe':
        if any(getattr(args,key) is None for key in ('candidate','logs','run_id','home_parent')):
            parser.error('probe requires candidate, logs, run-id and home-parent')
        probe(args.source,args.inputs,args.candidate,args.logs,args.run_id,home_parent=args.home_parent)
    else:
        if any(getattr(args,key) is None for key in ('candidate','logs','run_id','output')):parser.error('prove requires candidate, logs, run-id and output')
        args.output.write_bytes(canonical(prove(args.source,args.inputs,args.candidate,args.logs,args.run_id)))

if __name__=='__main__':main()
