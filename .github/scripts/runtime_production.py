"""Raw Runtime production provenance; every consumer still qualifies its own SHA.

A successful producer job is reusable production evidence even when another job
of its parent run failed. It is never promoted to a successful qualification.
Legacy cache keys alone carry no provenance and are not accepted by this module.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import subprocess
import shutil
import zipfile
from qualification_signal import api,owned_run
from runtime_inputs import canonical,production_inputs

WORKFLOW='core-runtime-qualification.yml'
BUILD='Build and verify through the shared runtime producer'
UPLOAD='Retain runtime and preparation evidence'
RAW_FILES={'codex','version.txt','preparation.json','lock-audit.json','source.patch','sha256.txt'}
SCHEMA='codex-raw-runtime-production-v1'


def git(source,*args):
    return subprocess.check_output(['git','-C',str(source),*args],stderr=subprocess.DEVNULL)


def revision(source,sha):
    if not isinstance(sha,str) or re.fullmatch('[0-9a-f]{40}',sha) is None:
        raise ValueError('raw producer source must be immutable')
    if subprocess.run(['git','-C',str(source),'cat-file','-e',sha+'^{commit}'],
                      stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL).returncode:
        subprocess.run(['git','-C',str(source),'fetch','--no-tags','--depth=1',
                        'https://github.com/humtr/codex.git',sha],check=True)
    return sha


def pages(value,key):
    rows=value[key]
    if value['total_count']!=len(rows) or len(rows)>100:
        raise ValueError('incomplete raw producer '+key+' inventory')
    return rows


def producer(run,job,complete=True):
    sha=run.get('head_sha')
    if (not isinstance(sha,str) or not owned_run(run,WORKFLOW,sha)
            or job.get('run_id')!=run.get('id')
            or not job.get('name','').endswith('/ runtime-source / runtime')):
        return False
    steps=job.get('steps',[])
    def passed(name):
        rows=[step for step in steps if step.get('name')==name]
        return len(rows)==1 and rows[0].get('conclusion')=='success'
    if not passed(BUILD):return False
    if complete:
        return job.get('status')=='completed' and job.get('conclusion')=='success' and passed(UPLOAD)
    return job.get('status') in {'in_progress','completed'}


def matching_jobs(run,request,complete=True):
    return [job for job in pages(request('actions/runs/'+str(run['id'])+'/jobs?per_page=100'),'jobs')
            if producer(run,job,complete)]


def origin(source,run_id,job_id,sha,request=api,complete=True):
    if type(run_id) is not int or run_id<=0 or type(job_id) is not int or job_id<=0:
        raise ValueError('invalid raw producer run/job identity')
    run=request('actions/runs/'+str(run_id))
    job=request('actions/jobs/'+str(job_id))
    if run.get('head_sha')!=sha or not producer(run,job,complete):
        raise ValueError('raw producer job is not authenticated completed production')
    revision(source,sha)
    return run,job


def artifact_record(row,run_id,sha):
    digest=row.get('digest')
    if (type(row.get('id')) is not int or row['id']<=0
            or row.get('name')!='termux-runtime-source' or row.get('expired') is not False
            or type(row.get('size_in_bytes')) is not int or not 0<row['size_in_bytes']<=1073741824
            or not isinstance(digest,str) or re.fullmatch('sha256:[0-9a-f]{64}',digest) is None
            or row.get('workflow_run',{}).get('id')!=run_id
            or row['workflow_run'].get('head_sha')!=sha):
        raise ValueError('raw producer artifact authority differs')
    return row


def resolve(source,request=api):
    """Select a bounded recent original producer, never a guessed cache prefix."""
    consumer=git(source,'rev-parse','HEAD').decode().strip()
    wanted=production_inputs(source,consumer)
    runs=request('actions/workflows/'+WORKFLOW+'/runs?per_page=100')['workflow_runs']
    if len(runs)>100:raise ValueError('raw producer run listing exceeds bound')
    for run in runs:
        sha=run.get('head_sha')
        if not isinstance(sha,str) or not owned_run(run,WORKFLOW,sha):continue
        jobs=matching_jobs(run,request)
        if len(jobs)>1:raise ValueError('ambiguous original raw producer job')
        if not jobs:continue
        revision(source,sha)
        try:previous=production_inputs(source,sha)
        except (KeyError,ValueError,subprocess.CalledProcessError):continue
        if previous!=wanted:continue
        artifacts=pages(request('actions/runs/'+str(run['id'])+'/artifacts?per_page=100'),'artifacts')
        matches=[row for row in artifacts if row.get('name')=='termux-runtime-source' and row.get('expired') is False]
        if len(matches)>1:raise ValueError('ambiguous original raw producer artifact')
        if not matches:continue
        artifact=artifact_record(matches[0],run['id'],sha)
        return {'schema':'codex-raw-runtime-selection-v1','consumer_source':consumer,
                'producer_source':sha,'production_run_id':run['id'],
                'production_job_id':jobs[0]['id'],'production_artifact_id':artifact['id'],
                'production_artifact_digest':artifact['digest'],'production_inputs':wanted}
    return None


def download(selection,transfer,request=api):
    """Fetch one immutable Actions ZIP and verify its API-bound digest before use."""
    artifact=artifact_record(request('actions/artifacts/'+str(selection['production_artifact_id'])),
                             selection['production_run_id'],selection['producer_source'])
    if artifact['digest']!=selection['production_artifact_digest']:
        raise ValueError('original raw archive digest changed')
    transfer=Path(transfer);transfer.mkdir(mode=0o700)
    archive=transfer/'download.zip'
    try:
        with archive.open('xb') as output:
            result=subprocess.run(['gh','api','repos/humtr/codex/actions/artifacts/'+str(artifact['id'])+'/zip'],
                                  stdout=output,stderr=subprocess.PIPE,timeout=180)
        if result.returncode:raise ValueError('original raw archive download failed')
        if archive.stat().st_size!=artifact['size_in_bytes']:
            raise ValueError('original raw archive byte count differs')
        with archive.open('rb') as stream:
            digest=hashlib.file_digest(stream,'sha256').hexdigest()
        if 'sha256:'+digest!=artifact['digest']:
            raise ValueError('original raw archive digest differs')
        with zipfile.ZipFile(archive) as zipped:
            rows=zipped.infolist()
            if (len(rows)!=1 or rows[0].filename!='runtime.tar' or rows[0].is_dir()
                    or not 0<rows[0].file_size<=1073741824
                    or stat.S_ISLNK(rows[0].external_attr>>16)):
                raise ValueError('original raw archive inventory differs')
            with zipped.open(rows[0]) as incoming,(transfer/'runtime.tar').open('xb') as output:
                shutil.copyfileobj(incoming,output,1024*1024)
                if output.tell()!=rows[0].file_size:raise ValueError('raw transfer is truncated')
        archive.unlink()
    except BaseException:
        shutil.rmtree(transfer);raise


def inventory(artifact):
    root=Path(artifact)
    if not stat.S_ISDIR(root.lstat().st_mode) or {p.name for p in root.iterdir()}!=RAW_FILES:
        raise ValueError('raw producer file inventory differs')
    result={}
    for name in sorted(RAW_FILES):
        fd=os.open(root/name,os.O_RDONLY|os.O_NOFOLLOW|os.O_NONBLOCK)
        with os.fdopen(fd,'rb') as stream:
            before=os.fstat(stream.fileno())
            if not stat.S_ISREG(before.st_mode) or not 0<=before.st_size<=536870912:
                raise ValueError('raw producer file is not bounded regular data')
            digest=hashlib.sha256()
            for chunk in iter(lambda:stream.read(1024*1024),b''):digest.update(chunk)
            after=os.fstat(stream.fileno())
            if (before.st_size,before.st_mtime_ns,before.st_mode)!=(after.st_size,after.st_mtime_ns,after.st_mode):
                raise ValueError('raw producer file changed during verification')
            result[name]={'sha256':digest.hexdigest(),'bytes':before.st_size,'mode':stat.S_IMODE(before.st_mode)}
    return result


def record(source,artifact,selection=None,run_id=None,artifact_id=None,request=api):
    """Record only after the current Rust verify-runtime gate has succeeded."""
    consumer=git(source,'rev-parse','HEAD').decode().strip()
    wanted=production_inputs(source,consumer)
    if selection is not None:
        fields={'schema','consumer_source','producer_source','production_run_id','production_job_id',
                'production_artifact_id','production_artifact_digest','production_inputs'}
        if (set(selection)!=fields or selection['schema']!='codex-raw-runtime-selection-v1'
                or selection['consumer_source']!=consumer or selection['production_inputs']!=wanted):
            raise ValueError('raw producer selection changed')
        sha=selection['producer_source'];run_id=selection['production_run_id'];job_id=selection['production_job_id']
        origin(source,run_id,job_id,sha,request)
        if production_inputs(source,sha)!=wanted:raise ValueError('raw compiler inputs differ')
        original=artifact_record(request('actions/artifacts/'+str(selection['production_artifact_id'])),run_id,sha)
        if original['digest']!=selection['production_artifact_digest']:
            raise ValueError('raw producer artifact changed')
    else:
        sha=consumer
        run=request('actions/runs/'+str(run_id))
        if run.get('head_sha')!=consumer:raise ValueError('current raw build belongs to another source')
        jobs=matching_jobs(run,request,complete=False)
        if len(jobs)!=1:raise ValueError('current raw build job is ambiguous or incomplete')
        job_id=jobs[0]['id']
        original=artifact_record(request('actions/artifacts/'+str(artifact_id)),run_id,sha)
    return {'schema':SCHEMA,'producer_source':sha,'production_run_id':run_id,
            'production_job_id':job_id,'production_artifact_id':original['id'],
            'production_artifact_digest':original['digest'],'production_inputs':wanted,
            'files':inventory(artifact),'qualification':False}


def read(path):
    with Path(path).open('rb') as stream:
        data=stream.read(262145)
    if len(data)>262144:raise ValueError('raw production receipt exceeds bound')
    def unique(pairs):
        value={}
        for key,item in pairs:
            if key in value:raise ValueError('duplicate raw production receipt field')
            value[key]=item
        return value
    return json.loads(data,object_pairs_hook=unique)


def write(path,value):
    with Path(path).open('x') as stream:stream.write(canonical(value).decode())


def consumer_record(source,receipt,output):
    write(output,{'schema':'codex-raw-runtime-consumer-v1',
          'consumer_source':git(source,'rev-parse','HEAD').decode().strip(),
          'production_receipt_sha256':hashlib.sha256(canonical(receipt)).hexdigest(),
          'producer_source':receipt['producer_source'],'production_run_id':receipt['production_run_id'],
          'production_job_id':receipt['production_job_id'],
          'production_artifact_id':receipt['production_artifact_id'],
          'production_artifact_digest':receipt['production_artifact_digest'],'qualification':False})


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action',choices=['resolve','download','record'])
    parser.add_argument('--source',type=Path,required=True)
    parser.add_argument('--artifact',type=Path)
    parser.add_argument('--transfer',type=Path)
    parser.add_argument('--artifact-id',type=int)
    parser.add_argument('--receipt',type=Path)
    parser.add_argument('--selection',type=Path)
    parser.add_argument('--consumer',type=Path)
    parser.add_argument('--run-id',type=int)
    args=parser.parse_args()
    if args.action=='resolve':
        value=resolve(args.source)
        if value is not None:
            write(args.selection,value);print('run_id='+str(value['production_run_id']))
        return
    if args.action=='download':
        download(read(args.selection),args.transfer);return
    value=record(args.source,args.artifact,read(args.selection) if args.selection else None,
                 args.run_id,args.artifact_id)
    write(args.receipt,value)
    if args.consumer:consumer_record(args.source,value,args.consumer)

if __name__=='__main__':main()
