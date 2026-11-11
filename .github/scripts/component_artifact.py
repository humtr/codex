"""Bounded Android component transfer; source tests remain a separate proof."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import stat
import struct
import tarfile
from component_inputs import COMPONENTS, fingerprint, git, inputs
from runtime_inputs import canonical

LIMIT=64*1024*1024
INTERFACE='codex-manager-core-v1'


def regular(path, limit):
    with os.fdopen(os.open(path,os.O_RDONLY|os.O_NOFOLLOW|os.O_NONBLOCK),'rb') as stream:
        before=os.fstat(stream.fileno())
        if not stat.S_ISREG(before.st_mode) or before.st_size>limit:
            raise ValueError('invalid component file or size')
        data=stream.read(limit+1)
        after=os.fstat(stream.fileno())
        if len(data)>limit or len(data)!=before.st_size or (before.st_size,before.st_mtime_ns,before.st_mode)!=(after.st_size,after.st_mtime_ns,after.st_mode):
            raise ValueError('component changed during verification')
        return data


def android_elf(data):
    if (len(data)<64 or data[:6]!=b'\x7fELF\x02\x01'
            or struct.unpack_from('<H',data,18)[0]!=183):
        raise ValueError('component is not a little-endian AArch64 ELF')
    offset=struct.unpack_from('<Q',data,32)[0]
    size,count=struct.unpack_from('<HH',data,54)
    if size!=56 or not 1<=count<=128 or offset+size*count>len(data):
        raise ValueError('invalid Android ELF program table')
    interpreters=[]
    for index in range(count):
        header=offset+index*size
        if struct.unpack_from('<I',data,header)[0]==3:
            start=struct.unpack_from('<Q',data,header+8)[0]
            length=struct.unpack_from('<Q',data,header+32)[0]
            if length>128 or start+length>len(data):raise ValueError('invalid Android interpreter')
            interpreters.append(data[start:start+length])
    if interpreters!=[b'/system/bin/linker64\0']:
        raise ValueError('component does not use the qualified Android linker')


def unique(items):
    result={}
    for key,value in items:
        if key in result:raise ValueError('duplicate component receipt field')
        result[key]=value
    return result


def record(source, component, artifact, run_id):
    artifact=Path(artifact)
    binary=artifact/COMPONENTS[component][1]
    data=regular(binary,LIMIT);android_elf(data)
    source_sha=git(source,'rev-parse','HEAD').decode().strip()
    if not re.fullmatch('[0-9a-f]{40}',source_sha) or not re.fullmatch('[1-9][0-9]*',str(run_id)):
        raise ValueError('invalid component producer identity')
    value={'schema':'codex-android-component-v1','component':component,
           'producer_source':source_sha,'production_run_id':str(run_id),
           'inputs_sha256':fingerprint(source,component),'artifact_sha256':hashlib.sha256(data).hexdigest(),
           'artifact_bytes':len(data),'interface':INTERFACE,'target':'aarch64-linux-android',
           'rust':inputs(source,component)['rust']}
    (artifact/'component.json').write_bytes(canonical(value))
    os.chmod(binary,0o755);os.chmod(artifact/'component.json',0o644)
    return verify(source,component,artifact)


def verify(source, component, artifact):
    artifact=Path(artifact)
    if not stat.S_ISDIR(artifact.lstat().st_mode):raise ValueError('invalid component directory')
    name=COMPONENTS[component][1]
    if {p.name for p in artifact.iterdir()}!={name,'component.json'}:
        raise ValueError('component inventory differs')
    raw=regular(artifact/'component.json',16384)
    value=json.loads(raw,object_pairs_hook=unique)
    required={'schema','component','producer_source','production_run_id','inputs_sha256','artifact_sha256',
              'artifact_bytes','interface','target','rust'}
    if (set(value)!=required or value['schema']!='codex-android-component-v1'
            or value['component']!=component or value['interface']!=INTERFACE
            or value['target']!='aarch64-linux-android'
            or re.fullmatch('[0-9a-f]{40}',value['producer_source']) is None
            or re.fullmatch('[1-9][0-9]*',value['production_run_id']) is None
            or value['inputs_sha256']!=fingerprint(source,component)
            or value['rust']!=inputs(source,component)['rust'] or raw!=canonical(value)):
        raise ValueError('component receipt inputs or identity differ')
    data=regular(artifact/name,LIMIT);android_elf(data)
    if (type(value['artifact_bytes']) is not int or value['artifact_bytes']!=len(data)
            or value['artifact_sha256']!=hashlib.sha256(data).hexdigest()
            or stat.S_IMODE((artifact/name).lstat().st_mode)!=0o755):
        raise ValueError('component bytes, digest or mode differ')
    return value


def pack(source,component,artifact,transfer):
    verify(source,component,artifact)
    transfer=Path(transfer);transfer.mkdir(mode=0o700)
    with tarfile.open(transfer/'component.tar','w',format=tarfile.USTAR_FORMAT) as archive:
        for name in (COMPONENTS[component][1],'component.json'):
            archive.add(Path(artifact)/name,arcname=name,recursive=False)


def unpack(source,component,artifact,transfer):
    transfer=Path(transfer);artifact=Path(artifact)
    data=regular(transfer/'component.tar',LIMIT+32768)
    import io
    with tarfile.open(fileobj=io.BytesIO(data),mode='r:') as archive:
        entries=[]
        offset=0
        for entry in archive:
            if len(entries)>=2 or entry.offset!=offset or entry.offset_data!=offset+512:
                raise ValueError('unsafe component transfer member count or offsets')
            entries.append(entry)
            offset=entry.offset_data+(entry.size+511)//512*512
        ending=data[offset:]
        if len(ending)<1024 or len(ending)>10240 or any(ending):
            raise ValueError('invalid component transfer ending')
        expected={COMPONENTS[component][1]:0o755,'component.json':0o644}
        if (len(entries)!=2 or {e.name for e in entries}!=set(expected)
                or any(not e.isfile() or e.linkname or e.pax_headers or e.mode!=expected[e.name]
                       or e.size> (16384 if e.name=='component.json' else LIMIT) for e in entries)):
            raise ValueError('unsafe component transfer inventory')
        artifact.mkdir(mode=0o700)
        try:
            for entry in entries:
                with archive.extractfile(entry) as incoming, (artifact/entry.name).open('xb') as outgoing:
                    shutil.copyfileobj(incoming,outgoing)
                    os.fchmod(outgoing.fileno(),entry.mode)
            return verify(source,component,artifact)
        except BaseException:
            shutil.rmtree(artifact)
            raise


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action',choices=('record','verify','pack','unpack'))
    parser.add_argument('--source',type=Path,required=True)
    parser.add_argument('--component',choices=COMPONENTS,required=True)
    parser.add_argument('--artifact',type=Path,required=True)
    parser.add_argument('--transfer',type=Path)
    parser.add_argument('--run-id')
    args=parser.parse_args()
    if args.action=='record':result=record(args.source,args.component,args.artifact,args.run_id)
    elif args.action=='verify':result=verify(args.source,args.component,args.artifact)
    else:
        if args.transfer is None:parser.error('--transfer is required')
        result=globals()[args.action](args.source,args.component,args.artifact,args.transfer)
    if result is not None:print(json.dumps(result,sort_keys=True))

if __name__=='__main__':main()
