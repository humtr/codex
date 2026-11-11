"""Consume an accepted component across branch cache boundaries without building."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil
import stat
import subprocess
import zipfile

from component_artifact import LIMIT, regular, unpack
from component_inputs import COMPONENTS, git
from qualification_signal import REPO, accepted_run, api, successful_run

WORKFLOWS={'core':'core-runtime-qualification.yml','manager':'manager-qualification.yml'}
TRANSFER_LIMIT=LIMIT+32768
ARCHIVE_LIMIT=TRANSFER_LIMIT+65536


def artifact_record(row,component,run):
    if (not isinstance(row,dict) or type(row.get('id')) is not int or row['id']<=0
            or row.get('name')!='android-'+component or row.get('expired') is not False
            or type(row.get('size_in_bytes')) is not int or not 0<row['size_in_bytes']<=ARCHIVE_LIMIT
            or not isinstance(row.get('digest'),str) or re.fullmatch('sha256:[0-9a-f]{64}',row['digest']) is None
            or row.get('workflow_run',{}).get('id')!=run['id']
            or row['workflow_run'].get('head_sha')!=run['head_sha']):
        raise ValueError('qualified component artifact authority differs')
    return row


def download(row,path):
    with path.open('xb') as output:
        result=subprocess.run(['gh','api','repos/'+REPO+'/actions/artifacts/'+str(row['id'])+'/zip'],
                              stdout=output,stderr=subprocess.PIPE,timeout=180)
    if result.returncode:raise ValueError('qualified component archive download failed')


def acquire(source,component,artifact,transfer,request=api,lookup=successful_run,fetch=download):
    if component not in WORKFLOWS:raise ValueError('unknown qualified component')
    sha=git(source,'rev-parse','HEAD').decode().strip()
    if re.fullmatch('[0-9a-f]{40}',sha) is None:raise ValueError('exact component source required')
    workflow=WORKFLOWS[component]
    run,_=lookup(workflow,sha)
    if (run is None or not accepted_run(run,workflow,sha)
            or type(run.get('id')) is not int or run['id']<=0):
        raise ValueError('no successful exact-source '+component+' qualification; compilation is not a fallback')
    page=request('actions/runs/'+str(run['id'])+'/artifacts?per_page=100')
    rows=page['artifacts']
    if page['total_count']!=len(rows) or len(rows)>100:raise ValueError('incomplete qualified component artifact inventory')
    matches=[row for row in rows if row.get('name')=='android-'+component and row.get('expired') is False]
    if len(matches)!=1:raise ValueError('missing or ambiguous qualified component artifact')
    selected=artifact_record(matches[0],component,run)
    actual=artifact_record(request('actions/artifacts/'+str(selected['id'])),component,run)
    if any(actual[key]!=selected[key] for key in ('id','digest','size_in_bytes')):
        raise ValueError('qualified component archive identity changed')
    artifact=Path(artifact);transfer=Path(transfer)
    if artifact.exists() or artifact.is_symlink():raise ValueError('component destination exists')
    transfer.mkdir(mode=0o700)
    try:
        archive=transfer/'download.zip';fetch(actual,archive)
        data=regular(archive,ARCHIVE_LIMIT)
        if len(data)!=actual['size_in_bytes'] or 'sha256:'+hashlib.sha256(data).hexdigest()!=actual['digest']:
            raise ValueError('qualified component archive digest or byte count differs')
        with zipfile.ZipFile(archive) as zipped:
            entries=zipped.infolist()
            if (len(entries)!=1 or entries[0].filename!='component.tar' or entries[0].is_dir()
                    or not 0<entries[0].file_size<=TRANSFER_LIMIT
                    or stat.S_IFMT(entries[0].external_attr>>16) not in (0,stat.S_IFREG)):
                raise ValueError('qualified component ZIP inventory differs')
            with zipped.open(entries[0]) as incoming,(transfer/'component.tar').open('xb') as output:
                shutil.copyfileobj(incoming,output,1024*1024)
                if output.tell()!=entries[0].file_size:raise ValueError('qualified component transfer is truncated')
        if not accepted_run(request('actions/runs/'+str(run['id'])),workflow,sha):
            raise ValueError('component qualification changed before admission')
        production=unpack(source,component,artifact,transfer)
        archive.unlink()
        return {'schema':'codex-qualified-component-consumption-v1','consumer_source':sha,'component':component,
                'qualification_run_id':run['id'],'qualification_artifact_id':actual['id'],
                'qualification_archive_digest':actual['digest'],'qualification_archive_bytes':len(data),
                'production':production}
    except BaseException:
        shutil.rmtree(transfer)
        raise


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source',type=Path,required=True)
    parser.add_argument('--component',choices=COMPONENTS,required=True)
    parser.add_argument('--artifact',type=Path,required=True)
    parser.add_argument('--transfer',type=Path,required=True)
    args=parser.parse_args()
    print(json.dumps(acquire(args.source,args.component,args.artifact,args.transfer),sort_keys=True))


if __name__=='__main__':main()
