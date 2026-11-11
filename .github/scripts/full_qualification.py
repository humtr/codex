"""Unsigned exact-source Full composition from independently qualified bytes."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import stat
import subprocess
import tarfile
from types import SimpleNamespace
from component_artifact import regular,unpack as unpack_component,verify as verify_component
from component_inputs import git
from component_revisions import selected,candidate_digest
from qualification_signal import api,gh,successful_run,accepted_run,REPO
from release_components import candidate_from_base,unique
from release_preflight import verify_base_candidate,verify_candidate,verify_qualified_candidate,parse_descriptor
from release_publication import BASE_FILES,EXTENDED_FILES,sha256
from runtime_inputs import canonical

WORKFLOWS={'core':'core-runtime-qualification.yml','manager':'manager-qualification.yml','tui':'manager-tui-native.yml'}
LIMIT=1024*1024*1024


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
    if (not isinstance(plan,dict) or set(plan)!={'schema','source_sha','upstream_version','runs','artifacts'}
            or plan['schema']!='codex-full-inputs-v1'
            or plan['source_sha']!=git(source,'rev-parse','HEAD').decode().strip()
            or plan['upstream_version']!=json.loads(git(source,'show','HEAD:release/upstream.json'))['upstream']['version']
            or set(plan['runs'])!=set(WORKFLOWS)
            or set(plan['artifacts'])!={'android-core','core-candidate','android-manager','manager-proof','tui','tui-proof'}):
        raise ValueError('Full input authority differs')
    for name,workflow in WORKFLOWS.items():
        if not accepted_run(plan['runs'][name],workflow,plan['source_sha']):raise ValueError('Full input run is unqualified: '+name)
    owners={'android-core':'core','core-candidate':'core','android-manager':'manager','manager-proof':'manager','tui':'tui','tui-proof':'tui'}
    for key,entry in plan['artifacts'].items():
        if (set(entry)!={'run_id','name','id'} or type(entry['id']) is not int or entry['id']<=0
                or entry['run_id']!=plan['runs'][owners[key]]['id']):
            raise ValueError('Full artifact belongs to another run')
    return plan


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
            stream.seek(offset);tail=stream.read(10241)
            if len(tail)>10240 or len(tail)<1024 or any(tail):raise ValueError('candidate transfer trailing bytes differ')
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


def probe(source,inputs,candidate,log_root,run_id):
    plan=admit_plan(source,json.loads(regular(inputs/'inputs.json',262144),object_pairs_hook=unique))
    verify_full(candidate,plan['upstream_version'],deferred=True)
    before=candidate_digest(candidate,deferred=True)
    if not re.fullmatch('[1-9][0-9]*',str(run_id)):raise ValueError('Full probe requires its run identity')
    log_root.mkdir(mode=0o700)
    for name,binary,args in [('manager','manager',['--artifact-probe']),('core','core',['update','--help']),
                              ('runtime','runtime',['--version']),('tui','helpers/2',['--version'])]:
        with (log_root/(name+'.out')).open('xb') as stdout,(log_root/(name+'.err')).open('xb') as stderr:
            result=subprocess.run([str(candidate/binary),*args],stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,
                                  env={'CODEX_MANAGER_ARTIFACT_PROBE':'1'} if name=='manager' else {},timeout=20)
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


def require_qualified_full(source,candidate,directory,request=api,lookup=successful_run,fetch=gh):
    sha=git(source,'rev-parse','HEAD').decode().strip()
    run,_=lookup('full-qualification.yml',sha)
    if run is None or not accepted_run(run,'full-qualification.yml',sha):
        raise ValueError('selected Full needs a successful exact-source composition qualification')
    page=request('actions/runs/'+str(run['id'])+'/artifacts?per_page=100')
    if page['total_count']!=len(page['artifacts']):raise ValueError('incomplete Full proof artifact listing')
    matches=[entry for entry in page['artifacts'] if entry['name']=='full-composition-qualification' and entry['expired'] is False]
    if len(matches)!=1:raise ValueError('Full composition proof is missing, expired or ambiguous')
    directory.mkdir(mode=0o700)
    fetch('run','download',str(run['id']),'--repo',REPO,'--name','full-composition-qualification','--dir',str(directory))
    value=json.loads(regular(directory/'full-qualification.json',32768),object_pairs_hook=unique)
    admit_published_candidate(source,candidate,value,run)
    for name,number in value['component_runs'].items():
        actual=request('actions/runs/'+str(number))
        if not accepted_run(actual,WORKFLOWS[name],sha):raise ValueError('Full component run no longer qualifies: '+name)
    return value


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action',choices=('acquire','assemble','probe','prove'))
    parser.add_argument('--source',type=Path,required=True);parser.add_argument('--inputs',type=Path,required=True)
    parser.add_argument('--builder',type=Path);parser.add_argument('--candidate',type=Path)
    parser.add_argument('--logs',type=Path);parser.add_argument('--run-id');parser.add_argument('--output',type=Path)
    args=parser.parse_args()
    if args.action=='acquire':
        plan=resolve(args.source);args.inputs.mkdir(mode=0o700)
        (args.inputs/'inputs.json').write_bytes(canonical(plan))
        for name,entry in plan['artifacts'].items():
            target=args.inputs/name;target.mkdir()
            gh('run','download',str(entry['run_id']),'--repo',REPO,'--name',entry['name'],'--dir',str(target))
    elif args.action=='assemble':
        if args.builder is None or args.candidate is None:parser.error('assemble requires builder and candidate')
        assemble(args.source,args.inputs,args.builder,args.candidate)
    elif args.action=='probe':
        if any(getattr(args,key) is None for key in ('candidate','logs','run_id')):parser.error('probe requires candidate, logs and run-id')
        probe(args.source,args.inputs,args.candidate,args.logs,args.run_id)
    else:
        if any(getattr(args,key) is None for key in ('candidate','logs','run_id','output')):parser.error('prove requires candidate, logs, run-id and output')
        args.output.write_bytes(canonical(prove(args.source,args.inputs,args.candidate,args.logs,args.run_id)))

if __name__=='__main__':main()
