"""Explicit bounded device faults, settled between actual installed CLI turns."""
import argparse
import concurrent.futures
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import time
from desktop_lifecycle import DATA,RUNTIME,live
from linux_desktop_trial import write
from open_dual_trial import record,owner
from native_desktop_cli_probe import probe


def install_cycle(report, previous):
    """Explicit idle-device installer/reinstall/rollback proof; zero inference."""
    from desktop_lifecycle import ready
    from install_desktop_provider import hashes
    runtime=record(RUNTIME);current=Path(runtime['installation'])
    assert previous.is_dir() and previous!=current
    homes=[Path.home()/'.codex',*list((Path.home()/'.local/share/codex/manager/profiles').glob('*/home'))]
    files=[p for h in homes for p in [h/'config.toml',h/'auth.json'] if p.is_file()]
    protected={str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in files}
    desktop=live(Path(runtime['session']));assert desktop
    steps=[]
    def stop_idle():
        binding=Path(runtime['binding']);identity=live(binding);assert identity,'owned_idle_adapter_required'
        p=record(Path(str(binding)+'.proof'))
        assert p['requests']==p['arrivals']==0,'adapter_has_activity_do_not_stop'
        assert live(binding)==identity,'adapter_owner_changed'
        stop_owned(binding)
    def activate(target,label):
        stop_idle()
        command=[sys.executable,'-B',str(target/'source/install_desktop_provider.py'),'--return-transport','local']
        p=subprocess.run(command,capture_output=True,timeout=180)
        assert p.returncode==0,'owned_install_cycle_failed_'+label
        selected=record(RUNTIME)
        assert selected['installation']==str(target) and ready(selected),label
        assert all(selected[k]==runtime[k] for k in ['baseUrl','token','model','binding','session']),label
        accepted=record(target/'.installation.json')
        expected={**accepted['files'],'.installation.json':hashlib.sha256((json.dumps(accepted)+'\n').encode()).hexdigest()}
        assert hashes(target)==expected,'installed_inventory_changed'
        steps.append({'step':label,'installation':target.name,'filesVerified':len(expected)})
    try:
        activate(current,'reinstall')
        activate(previous,'rollback')
        activate(current,'restore')
    finally:
        if record(RUNTIME)['installation']!=str(current):
            if live(Path(runtime['binding'])):stop_idle()
            restored=subprocess.run([sys.executable,'-B',str(current/'source/install_desktop_provider.py'),
                '--return-transport','local'],capture_output=True,timeout=180)
            assert restored.returncode==0 and ready(record(RUNTIME)),'owned_install_cycle_restore_failed'
    stop_idle()
    command=[sys.executable,'-B',str(current/'source/desktop_lifecycle.py'),'ensure','--runtime',str(RUNTIME)]
    def bootstrap(_):
        p=subprocess.run(command,capture_output=True,timeout=125)
        return p.returncode==0 and json.loads(p.stdout).get('ready') is True
    with concurrent.futures.ThreadPoolExecutor(max_workers=6) as pool:
        bootstrap_results=list(pool.map(bootstrap,range(6)))
    assert all(bootstrap_results) and ready(record(RUNTIME))
    assert live(Path(runtime['session']))==desktop
    assert all(hashlib.sha256(Path(p).read_bytes()).hexdigest()==d for p,d in protected.items())
    result={'accepted':True,'steps':steps,'concurrentBootstrapClients':6,'bootstrapResults':bootstrap_results,
        'protectedAuthConfigFiles':len(protected),'signedDesktopIdentityPreserved':True,
        'endpointCapabilityPreserved':True,'onlyUnusedAdaptersStopped':True,'actualChatSubmissions':0,
        'finalInstallation':current.name,'priorInstallationRetained':previous.is_dir()}
    write(report,result);print(json.dumps(result));return result


def stop_owned(path):
    value=live(path);assert value,'verified_owned_process_required'
    owner({**value,'start':int(value['start'])});os.kill(value['pid'],signal.SIGTERM)
    for _ in range(150):
        if not live(path):return value
        time.sleep(.1)
    raise RuntimeError('owned_trial_did_not_stop')


def run(report):
    runtime=record(RUNTIME)
    homes=[Path.home()/'.codex',*list((Path.home()/'.local/share/codex/manager/profiles').glob('*/home'))]
    files=[p for h in homes for p in [h/'config.toml',h/'auth.json',h/'chatgpt.config.toml'] if p.is_file()]
    desktop_auth=Path.home()/'.local/share/codex-web-experiment/linux-desktop/home/.codex/auth.json'
    if desktop_auth.exists():files.append(desktop_auth)
    before={str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in files}
    result={'production':False}
    # Every fault follows a settled successful native turn. No live user job is a fixture.
    first=probe();assert first['installedCommandAccepted'];result['initialNative']=first
    previous=stop_owned(Path(runtime['binding']))
    second=probe();assert second['installedCommandAccepted'];result['adapterRecoveryNative']=second
    current=record(Path(runtime['binding']));assert current['pid']!=previous['pid']
    assert all(current[k]==runtime[k] for k in ['baseUrl','token','model'])
    result['adapterRecoveredThroughProfile']=True
    stop_owned(Path(runtime['binding']))
    command=[sys.executable,'-B',str(Path(runtime['source'])/'desktop_lifecycle.py'),'ensure','--runtime',str(RUNTIME)]
    def invoke(_):
        p=subprocess.run(command,capture_output=True,text=True,timeout=125)
        return {'ready':p.returncode==0 and json.loads(p.stdout).get('ready') is True,'exit':p.returncode,'reason':p.stderr.strip() if p.returncode else None}
    with concurrent.futures.ThreadPoolExecutor(max_workers=6) as pool:accepted=list(pool.map(invoke,range(6)))
    result['bootstrapResults']=accepted;write(report,result)
    assert all(r['ready'] for r in accepted);result['concurrentBootstrapClients']=6;result['concurrentBootstrapAccepted']=True
    adapter=record(Path(runtime['binding']))
    previous_desktop=stop_owned(Path(runtime['session']))
    third=probe();assert third['installedCommandAccepted'];result['wholeDesktopRecoveryNative']=third
    current_desktop=record(Path(runtime['session']));assert current_desktop['pid']!=previous_desktop['pid'] and current_desktop['root']!=previous_desktop['root']
    assert record(Path(runtime['binding']))['pid']==adapter['pid']
    result['desktopRecoveredWithoutAdapterRestart']=True
    assert all(hashlib.sha256(Path(p).read_bytes()).hexdigest()==digest for p,digest in before.items())
    result['protectedFilesAndProfileUnchanged']=True
    assert all(not str(runtime[k]).startswith(os.environ['TMPDIR']) for k in ['source','bun','bridge','proot','novnc'])
    result['permanentDependencies']=True
    write(report,result);print(json.dumps(result));return result


if __name__=='__main__':
    os.umask(0o077)
    p=argparse.ArgumentParser();p.add_argument('--report',type=Path,default=DATA/'lifecycle-proof.json')
    p.add_argument('--idle-install-cycle',type=Path,help='Explicit zero-inference reinstall/rollback/restore of unused adapters, using this prior payload')
    args=p.parse_args()
    if args.idle_install_cycle:install_cycle(args.report,args.idle_install_cycle)
    else:run(args.report)
