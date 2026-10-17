"""Content-free tap; metadata by default, explicit bound Core gateways otherwise."""
import argparse
import asyncio
import hashlib
import json
from pathlib import Path
import sys


async def main(target, evidence, native_core=False):
    child=await asyncio.create_subprocess_exec(*json.loads(target.read_text()),
        stdin=asyncio.subprocess.PIPE,stdout=asyncio.subprocess.PIPE,stderr=asyncio.subprocess.DEVNULL)
    stdin=asyncio.StreamReader()
    await asyncio.get_running_loop().connect_read_pipe(lambda:asyncio.StreamReaderProtocol(stdin),sys.stdin.buffer)
    pending={}
    def note(value):
        with evidence.open('a') as out:out.write(json.dumps(value)+'\n')
    def identifier(value):return hashlib.sha256(json.dumps(value,sort_keys=True).encode()).hexdigest()[:16]
    async def receive():
        while line:=await stdin.readline():
            row=json.loads(line);method=row.get('method')
            rid=identifier(row['id']) if 'id' in row else None
            if rid:pending[rid]=method
            safe={'direction':'request','method':method,'id':rid}
            meta=row.get('params',{}).get('_meta',{})
            safe['requestProtocolVersion']=meta.get('io.modelcontextprotocol/protocolVersion')
            if method=='initialize':
                params=row.get('params',{})
                safe.update(protocolVersion=params.get('protocolVersion'),capabilities=list(params.get('capabilities',{})))
            metadata_call=method=='tools/call' and row.get('params',{}).get('name')=='codex_tool_inventory' and str(row.get('params',{}).get('arguments',{}).get('turn_token','')).startswith('owned_unbound_')
            native_call=native_core and method=='tools/call' and row.get('params',{}).get('name') in ['native_catalog','native_tool']
            if native_call:
                safe['nativeTool']=row['params']['name']
                logical=row.get('params',{}).get('arguments',{}).get('call_id')
                if isinstance(logical,str):safe['logicalCallDigest']=hashlib.sha256(logical.encode()).hexdigest()
                pending[rid]=(method,safe.get('nativeTool'),safe.get('logicalCallDigest'))
            if method=='tools/call':
                safe['metadataProbe']=metadata_call
                if metadata_call:safe['turnTokenSha256']=hashlib.sha256(row['params']['arguments']['turn_token'].encode()).hexdigest()
            note(safe)
            if method=='tools/call' and not(metadata_call or native_call):
                # A metadata probe must not start real development work.
                sys.stdout.write(json.dumps({'jsonrpc':'2.0','id':row.get('id'),'error':{'code':-32000,'message':'Metadata-only qualification'}})+'\n');sys.stdout.flush();continue
            child.stdin.write(line);await child.stdin.drain()
    async def publish():
        while line:=await child.stdout.readline():
            row=json.loads(line);rid=identifier(row['id']) if 'id' in row else None;binding=pending.pop(rid,None);method=binding[0] if isinstance(binding,tuple) else binding
            result=row.get('result',{});safe={'direction':'response','method':method,'id':rid,'errorCode':row.get('error',{}).get('code'),'isError':result.get('isError') is True}
            if isinstance(binding,tuple):
                safe['nativeTool']=binding[1]
                if binding[1]=='native_tool' and 'error' not in row:
                    try:
                        returned=json.loads(result['content'][0]['text']);logical=returned['call_id']
                        safe['exactLogicalReturn']=hashlib.sha256(logical.encode()).hexdigest()==binding[2]
                        safe['nativeError']=returned['output'].get('isError') is True
                    except (KeyError,IndexError,TypeError,ValueError):safe['exactLogicalReturn']=False
            if method=='initialize':safe.update(protocolVersion=result.get('protocolVersion'),capabilities=list(result.get('capabilities',{})))
            if method=='tools/list':safe['toolCount']=len(result.get('tools',[]))
            note(safe);sys.stdout.buffer.write(line);sys.stdout.buffer.flush()
    tasks=[asyncio.create_task(receive()),asyncio.create_task(publish())]
    try:await asyncio.wait(tasks,return_when=asyncio.FIRST_COMPLETED)
    finally:
        for task in tasks:task.cancel()
        await asyncio.gather(*tasks,return_exceptions=True)
        if child.returncode is None:
            child.terminate()
            try:await asyncio.wait_for(child.wait(),3)
            except asyncio.TimeoutError:child.kill();await child.wait()


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('target',type=Path);parser.add_argument('evidence',type=Path)
    parser.add_argument('--native-core-results',action='store_true',help='Forward only the independent gateways to the bound Core broker; never execute locally')
    args=parser.parse_args();asyncio.run(main(args.target,args.evidence,args.native_core_results))
