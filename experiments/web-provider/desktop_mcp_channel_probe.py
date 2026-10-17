"""Bounded metadata proof; no execution. --ordinary-model permits one Chat start."""
import argparse
from contextlib import contextmanager
import hashlib
import json
import os
import re
from pathlib import Path
import shlex
import signal
import subprocess
import sys
import tempfile
import time
from urllib.request import urlopen
from urllib.parse import urlparse
import yaml
from desktop_local_core_probe import bounded_app

SOURCE=Path(__file__).resolve().parent
TUNNEL_SHA='65b1517bed8828de7a81c825a3a223cd400960b793099b8109cba06e2dcc1eb5'


def qualify_independent_discovery(result, connector_name):
    app=result.get('app',{})
    result['freshIndependentCatalog']=any(a.get('name')==connector_name and a.get('toolNames')==['native_catalog','native_tool'] for a in app.get('apps',[]))
    result['independentCatalogWire']=any(row.get('direction')=='response' and row.get('method')=='tools/list' and row.get('toolCount')==2 for row in result['wire'])
    return bool(all(result.get(k) for k in ['localReady','actualControlPlanePoll','ownedTunnelStopped','protectedProfilesUnchanged','freshIndependentCatalog','independentCatalogWire']) and result.get('appExitCode')==0 and result.get('metadataCalls')==0 and app.get('modelStarts')==0 and app.get('blockedAttempts')==0)


@contextmanager
def metadata_app(runtime_path):
    """Reuse an idle ready desktop for metadata only; never stop an existing app."""
    from desktop_lifecycle import live,desktop_ready
    runtime=json.loads(runtime_path.read_text());session=Path(runtime['session'])
    existing=live(session)
    if not existing:
        with bounded_app(runtime_path) as started:yield started
        return
    assert desktop_ready(session),'existing_desktop_not_ready'
    proof=json.loads(Path(runtime['binding']+'.proof').read_text())
    assert proof['requests']==proof['arrivals']==0,'operating_adapter_in_use_do_not_inspect'
    try:yield runtime
    finally:assert live(session)==existing,'existing_desktop_identity_changed'


def run(args):
    assert hashlib.sha256(args.binary.read_bytes()).hexdigest()==TUNNEL_SHA
    protected=[args.profile,*list((Path.home()/'.config/tunnel-client').glob('*.yaml'))]
    before={p:hashlib.sha256(p.read_bytes()).hexdigest() for p in protected}
    trial_id=yaml.safe_load(args.profile.read_text())['control_plane']['tunnel_id']
    for proc in Path('/proc').iterdir():
        if not proc.name.isdigit():continue
        try:
            if proc.stat().st_uid!=os.getuid() or 'tunnel-client' not in str((proc/'exe').resolve()):continue
            parts=(proc/'cmdline').read_bytes().decode().split('\0')
            if '--profile' in parts and '--profile-dir' in parts:
                path=Path(parts[parts.index('--profile-dir')+1])/(parts[parts.index('--profile')+1]+'.yaml')
                assert yaml.safe_load(path.read_text()).get('control_plane',{}).get('tunnel_id')!=trial_id,'existing_tunnel_owner_must_be_preserved'
        except (FileNotFoundError,PermissionError,ProcessLookupError):continue
    ordinary=getattr(args,'ordinary_model',False)
    independent=getattr(args,'independent_discovery',False)
    assert not(ordinary and independent),'choose_one_probe_mode'
    wakeup=getattr(args,'independent_metadata_call',False)
    refresh=getattr(args,'refresh_tools',False)
    assert not refresh or independent and not wakeup,'refresh_requires_independent_catalog_probe'
    assert not wakeup or independent,'metadata_wakeup_requires_independent_endpoint'
    result={'qualification':'ordinary-model-negative-delivery-only' if ordinary else 'metadata-negative-delivery-only','ordinaryChatModel':ordinary,'diagnosticOnly':wakeup,'modelStarts':None if ordinary else 0,'nativeToolExecutionsPermitted':False,'operatingProviderChanged':False,'profileSha256':before[args.profile],'tunnelBinarySha256':TUNNEL_SHA}
    with tempfile.TemporaryDirectory(prefix='mcp-channel-',dir=os.environ['TMPDIR']) as temporary:
        root=Path(temporary);root.chmod(0o700)
        profiles=root/'profiles';profiles.mkdir(mode=0o700)
        (root/'home').mkdir(mode=0o700)
        config=yaml.safe_load(args.profile.read_text());assert len(config['mcp']['commands'])==1
        original=shlex.split(config['mcp']['commands'][0]['command'])
        result['targetArgvSha256']=hashlib.sha256(json.dumps(original).encode()).hexdigest()
        if not independent:
            assert '--broker-socket' in original,'explicit_unbound_broker_required'
            assert not Path(original[original.index('--broker-socket')+1]).exists(),'existing_broker_must_be_preserved'
            result['targetBrokerAbsent']=True
        if independent:
            runtime=json.loads(args.runtime.read_text())
            original=[runtime['bun'],str(SOURCE/'desktop_mcp_stdio.mjs'),str(root/'unbound-socket')]
            result.update(qualification='independent-catalog-discovery-only',independentEndpoint=True,
                          targetArgvSha256=hashlib.sha256(json.dumps(original).encode()).hexdigest())
        target=root/'target.json';target.write_text(json.dumps(original));target.chmod(0o600)
        config['mcp']['commands'][0]['command']=shlex.join([sys.executable,'-B',str(SOURCE/'desktop_mcp_stdio_inspect.py'),str(target),str(root/'wire.jsonl')])
        config['health']={'listen_addr':'127.0.0.1:0','url_file':str(root/'health.url')}
        config['log']={'file':str(root/'tunnel.log'),'format':'json','level':'info'}
        profile=profiles/'owned-mcp-proof.yaml';profile.write_text(yaml.safe_dump(config));profile.chmod(0o600)
        with (root/'launcher.log').open('w') as log:
            child=subprocess.Popen([str(args.binary),'run','--profile-dir',str(profiles),'--profile','owned-mcp-proof'],
                stdout=log,stderr=log,start_new_session=True,env={**os.environ,'HOME':str(root/'home'),'LOG_HTTP_RAW_UNSAFE':'false'})
            try:
                deadline=time.monotonic()+30
                while not (root/'health.url').exists():
                    assert child.poll() is None,'trial_tunnel_start_failed'
                    assert time.monotonic()<deadline,'trial_tunnel_start_timeout'
                    time.sleep(.1)
                health=(root/'health.url').read_text().strip();u=urlparse(health)
                assert u.scheme=='http' and u.hostname=='127.0.0.1' and u.port
                assert not u.username and not u.password
                health=f'{u.scheme}://{u.netloc}'
                result['localReady']=urlopen(health.rstrip('/')+'/readyz',timeout=5).status==200
                deadline=time.monotonic()+35
                while time.monotonic()<deadline:
                    metrics=urlopen(health.rstrip('/')+'/metrics',timeout=2).read().decode()
                    found=re.search(r'\S*commands_poll_last_successful_timestamp_seconds(?:\{[^}]*\})?\s+([0-9.e+]+)',metrics)
                    if found and float(found[1])>0:result['actualControlPlanePoll']=True;break
                    time.sleep(.2)
                assert result.get('actualControlPlanePoll'),'actual_poll_required'
                with (metadata_app(args.runtime) if independent else bounded_app(args.runtime)) as runtime:
                    command=[runtime['bun'],str(SOURCE/('desktop_mcp_model_probe.mjs' if ordinary else 'desktop_mcp_inspect.mjs')),str(args.runtime),str(root/'app.json'),args.connector_name]
                    if not ordinary:
                        command+=['--tools-from-cache']
                        if not independent or wakeup:command+=['--channel-inventory']
                        if refresh:command+=['--refresh-tools']
                    proc=subprocess.run(command,capture_output=True,timeout=125)
                    result['appExitCode']=proc.returncode
            except Exception as error:
                result['failureClass']=type(error).__name__
                result['failureErrno']=getattr(getattr(error,'reason',None),'errno',None)
            finally:
                result['tunnelExitBeforeCleanup']=child.poll()
                if child.poll() is None:
                    os.killpg(child.pid,signal.SIGTERM)
                    try:child.wait(timeout=8)
                    except subprocess.TimeoutExpired:os.killpg(child.pid,signal.SIGKILL);child.wait(timeout=5)
                result['ownedTunnelStopped']=child.poll() is not None
                if (root/'app.json').exists():result['app']=json.loads((root/'app.json').read_text())
                rows=[json.loads(line) for line in (root/'wire.jsonl').read_text().splitlines()] if (root/'wire.jsonl').exists() else []
                result['wire']=rows
                result['actualChannelInitialized']=any(r['direction']=='response' and r['method']=='initialize' and r.get('protocolVersion') for r in rows)
                result['metadataCalls']=sum(r['direction']=='request' and r['method']=='tools/call' for r in rows)
                requests=[r for r in rows if r['direction']=='request' and r.get('metadataProbe')]
                result['metadataRequestValidated']=len(requests)==1
                result['actualMetadataRefusalReturned']=len(requests)==1 and any(r['direction']=='response' and r['method']=='tools/call' and r['id']==requests[0]['id'] and (r.get('isError') or r.get('errorCode') is not None) for r in rows)
                if (root/'tunnel.log').exists():
                    content=(root/'tunnel.log').read_text()
                    result['tunnelDiagnostics']={key:bool(re.search(pattern,content,re.I)) for key,pattern in [('authFailure',r'"status_code":\s*(?:401|403)'),('dnsFailure',r'no such host|DNS lookup|lookup.*failed'),('fileFailure',r'no such file|not found')]}
    result['protectedProfilesUnchanged']=all(hashlib.sha256(p.read_bytes()).hexdigest()==digest for p,digest in before.items())
    # A retained host connection need not repeat initialize on every new client.
    # This gate proves only the negative delivery, not new session negotiation.
    result['wireAccepted']=bool(all(result.get(k) for k in ['localReady','actualControlPlanePoll','metadataRequestValidated','actualMetadataRefusalReturned','ownedTunnelStopped','protectedProfilesUnchanged']) and result.get('metadataCalls')==1)
    app=result.get('app',{})
    if independent:
        result['accepted']=qualify_independent_discovery(result,args.connector_name)
    elif ordinary:
        result['modelStarts']=app.get('modelStarts')
        result['submissionUnknown']=app.get('submissionUnknown',result['modelStarts'] is None)
        result['exactModelTurnToken']=any(r.get('metadataProbe') and r.get('turnTokenSha256')==app.get('turnTokenSha256') for r in result['wire'])
        result['accepted']=bool(result['wireAccepted'] and result['exactModelTurnToken'] and result.get('appExitCode')==0 and app.get('accepted') and app.get('modelStarts')==1 and app.get('userMessages')==1 and app.get('toolMessages')==0 and not result['submissionUnknown'])
    else:
        result['accepted']=bool(result['wireAccepted'] and result.get('appExitCode')==0 and app.get('metadataCall') and app.get('errorFlags',{}).get('unbound') and app.get('modelStarts')==0 and app.get('blockedAttempts')==0)
    return result


if __name__=='__main__':
    os.umask(0o077);parser=argparse.ArgumentParser()
    for name in ['binary','profile','runtime','report']:parser.add_argument('--'+name,type=Path,required=True)
    parser.add_argument('--connector-name',required=True)
    parser.add_argument('--ordinary-model',action='store_true',help='Explicit live ordinary-Chat test, at most one app request start; no Pro/retry/native execution')
    parser.add_argument('--independent-discovery',action='store_true',help='Zero-inference catalog read against our unbound independent endpoint')
    parser.add_argument('--refresh-tools',action='store_true',help='With independent discovery, invoke the existing signed-app settings refresh for the one selected MCP link')
    parser.add_argument('--independent-metadata-call',action='store_true',help='With independent discovery, send only the known legacy inventory request with an invalid synthetic token; no execution')
    args=parser.parse_args()
    report=run(args);args.report.write_text(json.dumps(report,indent=2)+'\n');args.report.chmod(0o600)
    print(json.dumps(report));raise SystemExit(0 if report['accepted'] else 1)
