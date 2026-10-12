"""Bounded same-profile differential; always restore the working ordinary browser."""
import argparse
import json
import os
from pathlib import Path
import shutil
import shlex
import signal
import subprocess
import sys
import tempfile
import time
from fresh_login_probe import identity_matches
from fit_browser import fit_browser


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ('gui','candidate','bun','storage','broker','proof'):
        parser.add_argument('--'+name,type=Path,required=True)
    parser.add_argument("--human-check-seconds",type=int,default=0)
    args=parser.parse_args()
    if not 0<=args.human_check_seconds<=120:
        parser.error("human check must be bounded to 0..120 seconds")
    r=args.gui.resolve(strict=True);source=Path(__file__).resolve().parent
    env=json.loads((r/'environment.json').read_text());profile=args.storage.parent/'manual-profile'
    owner_file=r/'account-view-process.json';old=json.loads(owner_file.read_text());assert identity_matches(old)
    assert old['image']==str(Path(env['PREFIX'])/'lib/chromium/chrome')
    proc=Path('/proc')/str(old['pid']);argv=[x.decode() for x in (proc/'cmdline').read_bytes().split(b'\0') if x]
    assert '--user-data-dir='+str(profile) in argv and '--remote-debugging-pipe' not in argv
    assert profile.is_dir() and not profile.is_symlink() and profile.stat().st_mode&0o777==0o700
    assert not args.broker.exists()
    descriptor=r/'same-profile-host.json';assert not descriptor.exists()
    state={'sameExperimentalProfileReused':True,'accountCopiedToAnotherBrowserProfile':False,'realModelReplyAccepted':False,'realModelToolLoopAccepted':False}
    host=None;stopped=False
    recovery=Path(os.environ['HOME'])/'.cache/codex-web-experiment/profile-checkpoint'
    recovery.mkdir(parents=True,exist_ok=True,mode=0o700);recovery.chmod(0o700)
    checkpoint=Path(tempfile.mkdtemp(prefix='same-profile-',dir=recovery));checkpoint.chmod(0o700)
    try:
        # Never signal based on a stale PID or shut down unrelated browser jobs.
        assert identity_matches(old);os.kill(old['pid'],signal.SIGTERM)
        for _ in range(100):
            if not identity_matches(old):stopped=True;break
            time.sleep(.1)
        assert stopped,'ordinary experimental browser did not exit normally'
        state['ordinaryBrowserExitedNormally']=True
        shutil.copytree(profile,checkpoint/'profile',symlinks=True,ignore=shutil.ignore_patterns('Cache','Code Cache','GPUCache','ShaderCache','GrShaderCache','DawnGraphiteCache','DawnWebGPUCache','Crashpad','Singleton*','LOCK'))
        (checkpoint/'launch.json').write_text(json.dumps({'argv':argv,'environment':env}));(checkpoint/'launch.json').chmod(0o600)
        state['recoveryCheckpointCreated']=True
        with (r/'same-profile-host.log').open('w') as log:
            host=subprocess.Popen([str(args.bun),str(source/'native_chromium_host.mjs'),str(profile),str(descriptor),'--restore-session'],env=env,stdout=log,stderr=log,start_new_session=True)
        for _ in range(150):
            if descriptor.exists():break
            assert host.poll() is None;time.sleep(.1)
        else:raise RuntimeError('same-profile native host readiness failed')
        current=json.loads(descriptor.read_text())['browser'];assert identity_matches(current)
        owner_file.write_text(json.dumps(current));owner_file.chmod(0o600)
        for _ in range(100):
            try:fit_browser(r);break
            except RuntimeError:
                assert host.poll() is None;time.sleep(.1)
        else:raise RuntimeError('same-profile owned browser window readiness failed')
        proof=r/'same-profile-session.json'
        inspection=([str(args.bun),str(source/'native_interactive_prepare.mjs'),str(args.candidate),str(descriptor),str(proof)] if args.human_check_seconds else [str(args.bun),str(source/'native_session_probe.mjs'),str(args.candidate),str(descriptor),str(proof),'--retain-page'])
        ready=subprocess.run(inspection,env=env,capture_output=True,timeout=45)
        assert ready.returncode==0 and proof.exists(),'same-profile inspection failed'
        state['session']=json.loads(proof.read_text())
        if args.human_check_seconds:
            fit_browser(r)
            stop=r/'human-check-stop.json';assert not stop.exists()
            status=r/'human-check-active.json'
            status.write_text(json.dumps({'browser':current,'hardDeadline':time.time()+args.human_check_seconds,'stopPath':str(stop)}));status.chmod(0o600)
            action=shlex.join([env['PREFIX']+'/bin/python3','-B',str(source/'open_viewer.py'),str(r),'--refresh'])
            notification=subprocess.run([env['PREFIX']+'/bin/termux-notification','--id','codex-web-human-check','--group','codex-web-experiment','--title','ChatGPT 실제 전송 확인','--content','한 번 터치해 시험 화면을 열어 주세요. 2분 안에 원래 브라우저로 복원됩니다.','--action',action,'--priority','high'],env=dict(os.environ),capture_output=True,timeout=20)
            assert notification.returncode==0,'human-view navigation notification unavailable'
            print(json.dumps({'humanCheckWindowReady':True,'maximumWaitSeconds':args.human_check_seconds,'ordinaryBrowserAutomaticallyRestored':True}),flush=True)
            deadline=time.monotonic()+args.human_check_seconds
            try:
                while time.monotonic()<deadline and not stop.exists():
                    assert identity_matches(current);time.sleep(.2)
            finally:
                status.unlink(missing_ok=True);stop.unlink(missing_ok=True)
                subprocess.run([env['PREFIX']+'/bin/termux-notification-remove','codex-web-human-check'],env=dict(os.environ),capture_output=True,timeout=15)
            state['boundedHumanCheckCompleted']=True
        elif state['session'].get('composerReady'):
            target=r/'same-profile-core.json'
            result=subprocess.run([env['PREFIX']+'/bin/python3','-B',str(source/'live_core_probe.py'),'--candidate',str(args.candidate),'--bun',str(args.bun),'--storage',str(args.storage),'--broker',str(args.broker),'--proof',str(target),'--native-host','--existing-native-descriptor',str(descriptor),'--prepared-native-session',str(proof)],env=dict(env,HOME=os.environ['HOME'],TMPDIR=os.environ['TMPDIR']),capture_output=True,timeout=160)
            assert target.exists(),'actual Core proof absent'
            state['core']=json.loads(target.read_text());state['realModelReplyAccepted']=state['core'].get('realModelReplyAccepted') is True
        else:state['blockedBeforeCoreRequest']=True
    finally:
        if host and host.poll() is None:
            host.terminate()
            try:host.wait(timeout=10)
            except subprocess.TimeoutExpired:os.killpg(host.pid,signal.SIGKILL);host.wait()
        if stopped:
            # Restore the exact profile and ordinary startup flags; tabs are kept
            # by Chrome's own session restore, never edited by this script.
            normal=[arg for arg in argv if arg!='--new-window' and not arg.startswith(('https://','http://'))]
            normal.append('--restore-last-session')
            browser=subprocess.Popen(normal,env=env,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,start_new_session=True)
            for _ in range(100):
                p=Path('/proc')/str(browser.pid)
                if p.exists():
                    try:new={'pid':browser.pid,'image':os.readlink(p/'exe'),'start':p.joinpath('stat').read_text().rsplit(')',1)[1].split()[19]}
                    except OSError:time.sleep(.1);continue
                    if identity_matches(new):
                        owner_file.write_text(json.dumps(new));owner_file.chmod(0o600)
                        try:fit_browser(r);break
                        except RuntimeError:time.sleep(.1);continue
                time.sleep(.1)
            else:raise RuntimeError('ordinary browser restoration failed; private recovery checkpoint retained')
            state['ordinaryBrowserRestored']=True;state['profileAuthorityUnchanged']=True
        else:state['ordinaryBrowserUnchanged']=identity_matches(old)
        if state.get('ordinaryBrowserRestored') or state.get('ordinaryBrowserUnchanged'):
            shutil.rmtree(checkpoint)
            state['successfulRestorationCheckpointRemoved']=True
        state['trialBrokerRemoved']=not args.broker.exists();state['nativeHostExited']=host is None or host.poll() is not None
        args.proof.write_text(json.dumps(state,indent=2)+'\n');args.proof.chmod(0o600)
    print(json.dumps(state))
    return 0 if state['realModelReplyAccepted'] else 1


if __name__=='__main__':raise SystemExit(main())
