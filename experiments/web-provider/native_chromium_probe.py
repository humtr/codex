"""Account-free native pipe host and real Playwright transport qualification."""
import json
import os
from pathlib import Path
import secrets
import struct
import subprocess
import sys
import tempfile
import time


def main():
    bun, candidate = (Path(v).resolve(strict=True) for v in sys.argv[1:])
    prefix, parent = Path(os.environ['PREFIX']), Path(os.environ['TMPDIR'])
    source = Path(__file__).resolve().parent
    processes = []
    with tempfile.TemporaryDirectory(prefix='ncp-',dir=parent) as temporary:
        root=Path(temporary);root.chmod(0o700)
        for name in ('h','t','profile'):(root/name).mkdir(mode=0o700)
        display=next(n for n in range(128,256) if not (parent/f'.X{n}-lock').exists() and not (parent/'.X11-unix'/f'X{n}').exists())
        def field(value):return struct.pack('!H',len(value))+value
        auth=root/'xauthority';auth.write_bytes(struct.pack('!H',65535)+field(b'localhost')+field(str(display).encode())+field(b'MIT-MAGIC-COOKIE-1')+field(secrets.token_bytes(16)));auth.chmod(0o600)
        env={'HOME':str(root/'h'),'TMPDIR':str(root/'t'),'PATH':str(prefix/'bin'),'PREFIX':str(prefix),'DISPLAY':f':{display}','XAUTHORITY':str(auth),'LANG':'C.UTF-8'}
        def start(argv,name):
            with (root/name).open('w') as f:p=subprocess.Popen(argv,env=env,stdout=f,stderr=f,start_new_session=True)
            processes.append(p);return p
        try:
            x=start([str(prefix/'bin/Xvnc'),f':{display}','-geometry','900x700','-depth','24','-rfbunixpath',str(root/'rfb.sock'),'-rfbunixmode','0600','-rfbport','-1','-SecurityTypes','None','-auth',str(auth),'-nolisten','tcp'],'xvnc.log')
            for _ in range(80):
                if (root/'rfb.sock').exists():break
                assert x.poll() is None,'owned X11 exited';time.sleep(.1)
            else:raise RuntimeError('owned X11 unavailable')
            for _ in range(50):
                ready=subprocess.run([str(prefix/'bin/xwininfo'),'-root'],env=env,capture_output=True,timeout=2)
                if ready.returncode==0:break
                time.sleep(.1)
            else:raise RuntimeError('native X11 handshake failed')
            start([str(prefix/'bin/openbox')],'openbox.log')
            host=start([str(bun),str(source/'native_chromium_host.mjs'),str(root/'profile'),str(root/'host.json')],'host.log')
            for _ in range(250):
                if (root/'host.json').exists():break
                if host.poll() is not None:
                    print('account_free_host_diagnostics',(root/'host.log').read_text()[-2000:])
                    raise RuntimeError('native pipe host failed before descriptor')
                time.sleep(.1)
            else:raise RuntimeError('native pipe host readiness failed')
            result=subprocess.run([str(bun),str(source/'native_chromium_probe.mjs'),str(candidate),str(root)],env=env,capture_output=True,text=True,timeout=45)
            if result.returncode:
                print('native_client_exit',result.returncode)
                print('native_client_failure_classes',[name for name in ('WebSocket','Unauthorized','Timeout','Persistent','reconnect','DOM','headers','404','401','Unsupported','Unexpected') if name in result.stderr])
                print('account_free_client_diagnostics',result.stderr[-1600:])
                raise RuntimeError('native pipe client proof failed')
            report=json.loads((root/'proof.json').read_text())
        finally:
            for p in reversed(processes):
                if p.poll() is None:
                    p.terminate()
                    try:p.wait(timeout=8)
                    except subprocess.TimeoutExpired:p.kill();p.wait()
        assert not (root/'host.json').exists(),'owned descriptor remains'
        report['descriptorRemoved']=True;report['ownedProcessesExited']=all(p.poll() is not None for p in processes)
        out=Path(os.environ['TMPDIR'])/'native-chromium-account-free-proof.json';out.write_text(json.dumps(report,indent=2)+'\n');out.chmod(0o600)
        print(json.dumps(report))


if __name__=='__main__':main()
