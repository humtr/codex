"""Explicit live signed-app/MCP/Android Core trial; two owned shell commands."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shlex
import signal
import subprocess
import sys
import tempfile
import time
from urllib.request import urlopen
from urllib.parse import urlparse
import yaml

from desktop_lifecycle import RUNTIME,ready,desktop_ready,live
from desktop_mcp_channel_probe import TUNNEL_SHA
from install_desktop_provider import RUNTIME_FILES
from siwc_core_probe import probe as core_probe

SOURCE=Path(__file__).resolve().parent


def stop(child):
    if child.poll() is None:child.terminate()
    try:child.wait(timeout=10)
    except subprocess.TimeoutExpired:child.kill();child.wait(timeout=5)


def run(args):
    runtime=json.loads(RUNTIME.read_text());assert ready(runtime) and desktop_ready(Path(runtime['session']))
    operating=json.loads(Path(runtime['binding']+'.proof').read_text())
    assert operating['requests']==operating['arrivals']==0,'operating_adapter_in_use'
    assert hashlib.sha256(args.binary.read_bytes()).hexdigest()==TUNNEL_SHA
    profiles=[args.profile,*list((Path.home()/'.config/tunnel-client').glob('*.yaml'))]
    protected={p:hashlib.sha256(p.read_bytes()).hexdigest() for p in profiles}
    config=yaml.safe_load(args.profile.read_text());trial=config['control_plane']['tunnel_id']
    # Never compete for a tunnel already owned by a running service.
    for proc in Path('/proc').iterdir():
        if not proc.name.isdigit():continue
        try:
            argv=(proc/'cmdline').read_bytes().decode().split('\0')
            if 'tunnel-client' not in str((proc/'exe').resolve()) or '--profile-dir' not in argv or '--profile' not in argv:continue
            path=Path(argv[argv.index('--profile-dir')+1])/(argv[argv.index('--profile')+1]+'.yaml')
            assert yaml.safe_load(path.read_text()).get('control_plane',{}).get('tunnel_id')!=trial,'existing_tunnel_owner_must_be_preserved'
        except (OSError,ValueError):pass
    args.output.mkdir(mode=0o700,parents=True,exist_ok=False)
    inputs={name:hashlib.sha256((SOURCE/name).read_bytes()).hexdigest() for name in RUNTIME_FILES if name!='README.md'}
    app=live(Path(runtime['session']));adapter_identity=live(Path(runtime['binding']))
    result={};tunnel=adapter=None
    with tempfile.TemporaryDirectory(prefix='mcp-core-',dir=os.environ['TMPDIR']) as temporary:
        root=Path(temporary);root.chmod(0o700)
        with (args.output/'adapter.log').open('w') as adapter_log,(args.output/'tunnel-launcher.log').open('w') as tunnel_log:
            try:
                adapter=subprocess.Popen([runtime['bun'],str(SOURCE/'desktop_responses_server.mjs'),runtime['bridge'],runtime['session'],str(root/'server.json')],
                    stdout=adapter_log,stderr=adapter_log,env={**os.environ,'CODEX_CHAT_RETURN_TRANSPORT':'mcp','CODEX_CHAT_MCP_CONNECTOR_NAME':args.connector_name})
                deadline=time.monotonic()+15
                while not (root/'server.json').exists():
                    assert adapter.poll() is None and time.monotonic()<deadline,'owned_mcp_adapter_not_ready';time.sleep(.05)
                binding=json.loads((root/'server.json').read_text());assert Path(binding['mcpSocket']).is_socket()
                target=root/'target.json';target.write_text(json.dumps([runtime['bun'],str(SOURCE/'desktop_mcp_stdio.mjs'),binding['mcpSocket']]));target.chmod(0o600)
                config['mcp']['commands'][0]['command']=shlex.join([sys.executable,'-B',str(SOURCE/'desktop_mcp_stdio_inspect.py'),str(target),str(args.output/'wire.jsonl'),'--native-core-results'])
                config['health']={'listen_addr':'127.0.0.1:0','url_file':str(root/'health.url')}
                config['log']={'file':str(root/'tunnel.log'),'format':'json','level':'info'}
                profile_root=root/'profiles';profile_root.mkdir(mode=0o700)
                profile=profile_root/'owned.yaml';profile.write_text(yaml.safe_dump(config));profile.chmod(0o600)
                tunnel=subprocess.Popen([str(args.binary),'run','--profile-dir',str(profile_root),'--profile','owned'],stdout=tunnel_log,stderr=tunnel_log)
                deadline=time.monotonic()+30
                while not (root/'health.url').exists():
                    assert tunnel.poll() is None and time.monotonic()<deadline,'owned_tunnel_not_ready';time.sleep(.1)
                url=urlparse((root/'health.url').read_text().strip());assert url.scheme=='http' and url.hostname=='127.0.0.1' and url.port and not url.username and not url.password
                health=f'{url.scheme}://{url.netloc}'
                assert urlopen(health.rstrip('/')+'/readyz',timeout=5).status==200
                deadline=time.monotonic()+35
                while True:
                    metrics=urlopen(health.rstrip('/')+'/metrics',timeout=3).read().decode()
                    stamp=re.search(r'\S*commands_poll_last_successful_timestamp_seconds(?:\{[^}]*\})?\s+([0-9.e+]+)',metrics)
                    if stamp and float(stamp[1])>0:break
                    assert time.monotonic()<deadline,'owned_tunnel_not_polling';time.sleep(.2)
                # Dedicated adapter/tunnel; existing operating descriptors remain intact.
                options=argparse.Namespace(desktop_server=root/'server.json',model='chatgpt-desktop/gpt-6',account_free_wire=False,
                    compact=False,cancel_resume=False,app_name='owned-native-mcp-core',effort='none',transcript_check=True,
                    mid_turn_compact=False,interrupt_tool=False,reject_tool=False,mcp_discovery=False,tui_trace_path=args.output/'tui.txt')
                started=time.monotonic()
                try:result=core_probe(options)
                except Exception as error:
                    result=getattr(error,'probe_report',{});result.update(realCoreToolLoopAccepted=False,failureClass=type(error).__name__)
                observed=json.loads((root/'server.json.proof').read_text())
                (args.output/'adapter-proof.json').write_text(json.dumps(observed,indent=2)+'\n')
                wire=[json.loads(line) for line in (args.output/'wire.jsonl').read_text().splitlines()] if (args.output/'wire.jsonl').exists() else []
                returns=[row for row in wire if row.get('direction')=='response' and row.get('nativeTool')=='native_tool']
                result.update(elapsedSeconds=round(time.monotonic()-started,3),actualChatUserObjects=observed['observedChatSubmissions'],
                    outerChatDispatches=observed.get('observedNativeDispatches',0),internalHostedSamplingCount=None,
                    nativeCoreHttpRequests=observed['requests'],exactNativeMcpReturns=sum(r.get('exactLogicalReturn') is True and not r.get('nativeError') and r.get('errorCode') is None for r in returns),
                    nativeCatalogCalls=sum(row.get('direction')=='request' and row.get('nativeTool')=='native_catalog' for row in wire),
                    returnedNativeResults=observed['returnedNativeResults'],adapterError=observed.get('errorCode'),automaticRetries=observed['retryAttempts'],proStarts=observed['submittedProRequests'],
                    productInputs=inputs,productInputsUnchanged=all(hashlib.sha256((SOURCE/n).read_bytes()).hexdigest()==v for n,v in inputs.items()))
                result['accepted']=bool(result.pop('realCoreToolLoopAccepted',False) and result['outerChatDispatches']==1 and result['actualChatUserObjects']==2 and result['exactNativeMcpReturns']==2
                    and result['nativeCatalogCalls']>=1 and result['returnedNativeResults']==2 and not result['adapterError'] and not result['automaticRetries'] and not result['proStarts']
                    and result['productInputsUnchanged'] and all(result.get(k) for k in ['persistedReportCheck','coldTranscriptRestored','resumeWithoutModelRequest','renderWithoutModelRequest'])
                    and all(result.get('renderedTranscript',{}).get(k) for k in ['initial','intermediate','final']))
            finally:
                if tunnel:stop(tunnel)
                if adapter:stop(adapter)
                result.update(ownedTunnelStopped=tunnel is not None and tunnel.poll() is not None,ownedAdapterStopped=adapter is not None and adapter.poll() is not None)
    result.update(temporaryRootRemoved=not root.exists(),protectedProfilesUnchanged=all(hashlib.sha256(p.read_bytes()).hexdigest()==v for p,v in protected.items()),
        operatingAppPreserved=live(Path(runtime['session']))==app,operatingAdapterPreserved=live(Path(runtime['binding']))==adapter_identity)
    result['accepted']=result.get('accepted',False) and all(result[k] for k in ['ownedTunnelStopped','ownedAdapterStopped','temporaryRootRemoved','protectedProfilesUnchanged','operatingAppPreserved','operatingAdapterPreserved'])
    (args.output/'result.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result));return result


if __name__=='__main__':
    os.umask(0o077);parser=argparse.ArgumentParser(description=__doc__)
    for name in ['binary','profile','output']:parser.add_argument('--'+name,type=Path,required=True)
    parser.add_argument('--connector-name',required=True)
    raise SystemExit(0 if run(parser.parse_args())['accepted'] else 1)
