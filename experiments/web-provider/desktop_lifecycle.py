"""On-demand owner-checked trial bootstrap; no boot service or second transcript."""
import argparse
import hashlib
import hmac
import secrets
import fcntl
import json
import os
from pathlib import Path
import socket
import subprocess
import sys
import time
import urllib.request
from open_dual_trial import record, owner
from linux_desktop_trial import private, write

DATA=Path.home()/'.local/share/codex-web-experiment/native-provider'
RUNTIME=DATA/'runtime.json'


def endpoint(value):
    from urllib.parse import urlparse
    import re
    u=urlparse(value['baseUrl'])
    assert u.scheme=='http' and u.hostname=='127.0.0.1' and u.port and u.path=='/v1'
    assert not (u.username or u.password or u.query or u.fragment)
    assert value['model']=='chatgpt-desktop/instant' and re.fullmatch('[0-9a-f]{64}',value['token'])
    return u.port


def live(path):
    try:
        value=record(path);owner({**value,'start':int(value['start'])})
        return value
    except (OSError,ValueError,KeyError,AssertionError,TypeError):return None


def desktop_ready(session):
    value=live(session)
    if not value:return False
    try:
        d=record(Path(value['root'])/'desktop-host.json')
        assert d['kind']=='termux-electron-pipe'
        for key in ['host','browser']:owner({**d[key],'start':int(d[key]['start'])})
        return True
    except (OSError,ValueError,KeyError,AssertionError,TypeError):return False


def port_free(port):
    # Never send a capability to a listener without first proving its owner.
    with socket.socket() as s:
        s.setsockopt(socket.SOL_SOCKET,socket.SO_REUSEADDR,1)
        try:s.bind(('127.0.0.1',port))
        except OSError:raise RuntimeError('provider_port_owned_by_another_process') from None


def wait_for(predicate,deadline,process=None):
    while time.monotonic()<deadline:
        if predicate():return
        if process is not None and process.poll() is not None:raise RuntimeError('trial_startup_failed')
        time.sleep(.2)
    raise RuntimeError('trial_startup_timeout')


def desktop(runtime,deadline):
    session=Path(runtime['session'])
    if desktop_ready(session):return
    if live(session):
        # Let a controller settle/exit; do not kill a live process or replace its GUI.
        wait_for(lambda: desktop_ready(session) or not live(session),deadline)
        if desktop_ready(session):return
    pending=session.parent/'bootstrap-controller.json'
    if live(pending):
        wait_for(lambda:desktop_ready(session),deadline)
        return
    command=[sys.executable,'-B',str(Path(runtime['source'])/'linux_desktop_trial.py'),'serve',
             '--proot-source',runtime['proot'],'--bun',runtime['bun'],'--novnc',runtime['novnc'],'--seconds','0','--inspect']
    log=Path(runtime['session']).parent/'controller.log'
    with log.open('a') as output:
        child=subprocess.Popen(command,stdin=subprocess.DEVNULL,stdout=output,stderr=output,start_new_session=True)
    for _ in range(50):
        try:
            raw=Path(f'/proc/{child.pid}/stat').read_text();image=os.readlink(f'/proc/{child.pid}/exe')
            if image==os.readlink(f'/proc/{os.getpid()}/exe'):
                write(pending,{'pid':child.pid,'start':int(raw[raw.rfind(')')+2:].split()[19]),'image':image});break
        except OSError:pass
        time.sleep(.01)
    wait_for(lambda:desktop_ready(session),deadline,child)


def server_matches(runtime,server):
    return bool(server and all(server.get(k)==runtime[k] for k in ['baseUrl','token','model'])
                and ('returnTransport' not in runtime or server.get('returnTransport')==runtime['returnTransport']))


def ready(runtime):
    value=live(Path(runtime['binding']))
    if not server_matches(runtime,value):return False
    # Android denies /proc/net/tcp. Prove possession with a fresh keyed challenge
    # instead of transmitting the bearer to an unproven local listener.
    nonce=secrets.token_hex(32)
    request=urllib.request.Request(runtime['baseUrl'].removesuffix('/v1')+'/__desktop_ready',headers={'X-Termux-Trial-Challenge':nonce})
    try:
        with urllib.request.urlopen(request,timeout=3) as r:
            result=json.loads(r.read(8192))
            expected=hmac.new(runtime['token'].encode(),('desktop-ready-v1:'+nonce).encode(),hashlib.sha256).hexdigest()
            return r.status==200 and result.get('model')==runtime['model'] and hmac.compare_digest(result.get('proof',''),expected)
    except (OSError,ValueError,KeyError,TypeError):return False



def ensure(path=RUNTIME,*,desktop_only=False,timeout=110):
    runtime=record(path);port=endpoint(runtime)
    data=private(path.parent)
    fd=os.open(data/'.bootstrap-lock',os.O_CREAT|os.O_RDWR|os.O_NOFOLLOW,0o600)
    deadline=time.monotonic()+timeout
    try:
        assert os.fstat(fd).st_uid==os.getuid() and not os.fstat(fd).st_mode&0o077
        while True:
            try:fcntl.flock(fd,fcntl.LOCK_EX|fcntl.LOCK_NB);break
            except BlockingIOError:
                if time.monotonic()>=deadline:raise RuntimeError('trial_bootstrap_busy')
                time.sleep(.1)
        if not desktop_only and not ready(runtime):
            if live(Path(runtime['binding'])):raise RuntimeError('live_adapter_not_ready')
            port_free(port)
        desktop(runtime,deadline)
        if desktop_only:return runtime
        if not ready(runtime):
            command=[runtime['bun'],str(Path(runtime['source'])/'desktop_responses_server.mjs'),runtime['bridge'],runtime['session'],runtime['binding'],str(path)]
            with (data/'adapter.log').open('a') as output:
                child=subprocess.Popen(command,stdin=subprocess.DEVNULL,stdout=output,stderr=output,start_new_session=True)
            try:wait_for(lambda:ready(runtime),deadline,child)
            except Exception:
                # Only the child created by this bootstrap, never an existing
                # adapter or the shared signed desktop.
                if child.poll() is None:child.terminate()
                try:child.wait(timeout=5)
                except subprocess.TimeoutExpired:child.kill();child.wait(timeout=5)
                raise
        return runtime
    finally:os.close(fd)


if __name__=='__main__':
    os.umask(0o077)
    p=argparse.ArgumentParser();p.add_argument('command',choices=['token','ensure','desktop']);p.add_argument('--runtime',type=Path,default=RUNTIME);a=p.parse_args()
    try:
        value=ensure(a.runtime,desktop_only=a.command=='desktop')
        if a.command=='token':print(value['token'])
        else:print(json.dumps({'ready':True,'desktopOnly':a.command=='desktop','production':False}))
    except Exception as e:
        # Never include credentials, private app contents or descriptor values.
        message=str(e) if isinstance(e,RuntimeError) else 'invalid_or_stale_trial_installation'
        print(message,file=sys.stderr);raise SystemExit(1)
