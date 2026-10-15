"""Try a current experimental login snapshot without touching the working browser.

Only ChatGPT/OpenAI account cookies are copied from the experimental Chrome DB
using SQLite's read-only backup. Challenge cookies and other domains are removed
before Chrome is started. The original profile/browser is never stopped or driven.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import sqlite3
import struct
import subprocess
import tempfile
import time
import secrets
from live_core_probe import connector_config


def identity_matches(owner):
    try:
        p=Path('/proc')/str(owner['pid'])
        fields=p.joinpath('stat').read_text().rsplit(')',1)[1].split()
        return fields[0]!='Z' and fields[19]==str(owner['start']) and os.readlink(p/'exe')==owner['image']
    except OSError:
        return False


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ('gui','candidate','bun','storage','broker','proof'):
        parser.add_argument('--'+name,type=Path,required=True)
    parser.add_argument('--connector-name',required=True,help='Exact existing ChatGPT connector display name')
    args=parser.parse_args()
    connector_config(args.connector_name)
    prefix=Path(os.environ['PREFIX']);parent=Path(os.environ['TMPDIR']);source=Path(__file__).resolve().parent
    original=json.loads((args.gui/'account-view-process.json').read_text());assert identity_matches(original)
    manual=args.storage.parent/'manual-profile'
    assert not manual.is_symlink() and manual.stat().st_mode&0o777==0o700
    assert not args.broker.exists(),'another trial owns the execution broker'
    report={'currentExperimentalLoginSnapshot':True,'originalBrowserStopped':False,'challengeCookiesImported':False,'otherDomainsImported':False,'realModelReplyAccepted':False,'realModelToolLoopAccepted':False}
    processes=[]
    with tempfile.TemporaryDirectory(prefix='fc-',dir=parent) as temporary:
        root=Path(temporary);root.chmod(0o700)
        profile=root/'profile';(profile/'Default').mkdir(parents=True,mode=0o700)
        profile.chmod(0o700)
        # Keep native Chromium's cookie encryption configuration, never decrypt
        # or log cookie values. Nothing is imported into the user's live profile.
        local=manual/'Local State'
        assert local.is_file() and not local.is_symlink()
        shutil.copyfile(local,profile/'Local State');(profile/'Local State').chmod(0o600)
        relative=Path('Default/Network/Cookies') if (manual/'Default/Network/Cookies').is_file() else Path('Default/Cookies')
        db=manual/relative;assert not db.is_symlink()
        target=profile/relative;target.parent.mkdir(parents=True,exist_ok=True,mode=0o700)
        reader=sqlite3.connect(db.as_uri()+'?mode=ro',uri=True,timeout=2)
        writer=sqlite3.connect(target)
        try:
            reader.backup(writer)
            rows=writer.execute('select rowid,name,host_key from cookies').fetchall()
            removed=[];count=0
            for rowid,name,domain in rows:
                normalized=domain.lstrip('.').lower()
                allowed=normalized in ('chatgpt.com','openai.com') or normalized.endswith(('.chatgpt.com','.openai.com'))
                if not allowed or name.lower().startswith(('cf_','__cf','_cf')):removed.append((rowid,))
                else:count+=1
            writer.executemany('delete from cookies where rowid=?',removed);writer.commit()
            assert writer.execute('pragma quick_check').fetchone()[0]=='ok'
            report['freshAccountCookieCount']=count
        finally:writer.close();reader.close()
        target.chmod(0o600)
        assert count>0
        home=root/'h';home.mkdir(mode=0o700)
        display=next(n for n in range(128,256) if not (parent/f'.X{n}-lock').exists() and not (parent/'.X11-unix'/f'X{n}').exists())
        def field(x):return struct.pack('!H',len(x))+x
        authority=root/'xauthority';authority.write_bytes(struct.pack('!H',65535)+field(b'localhost')+field(str(display).encode())+field(b'MIT-MAGIC-COOKIE-1')+field(secrets.token_bytes(16)));authority.chmod(0o600)
        environment={'HOME':str(home),'PREFIX':str(prefix),'PATH':str(prefix/'bin'),'TMPDIR':str(root),'LANG':'C.UTF-8','DISPLAY':f':{display}','XAUTHORITY':str(authority)}
        def start(argv,log):
            with (root/log).open('w') as f:p=subprocess.Popen(argv,env=environment,stdout=f,stderr=f,start_new_session=True)
            processes.append(p);return p
        try:
            x=start([str(prefix/'bin/Xvnc'),f':{display}','-geometry','1280x800','-depth','24','-rfbunixpath',str(root/'rfb.sock'),'-rfbunixmode','0600','-rfbport','-1','-SecurityTypes','None','-auth',str(authority),'-nolisten','tcp'],'xvnc.log')
            for _ in range(50):
                if (root/'rfb.sock').exists():break
                assert x.poll() is None;time.sleep(.1)
            else:raise RuntimeError('Owned X11 readiness failed')
            start([str(prefix/'bin/openbox')],'openbox.log')
            descriptor=root/'host.json';host=start([str(args.bun),str(source/'native_chromium_host.mjs'),str(profile),str(descriptor)],'host.log')
            for _ in range(150):
                if descriptor.exists():break
                assert host.poll() is None;time.sleep(.1)
            else:raise RuntimeError('Owned Chromium readiness failed')
            result=subprocess.run([str(args.bun),str(source/'native_session_probe.mjs'),str(args.candidate),str(descriptor),str(root/'session.json'),"--retain-page"],env=environment,capture_output=True,timeout=45)
            assert result.returncode==0,'Session inspection failed; private details not logged'
            report['nativeSession']=json.loads((root/'session.json').read_text())
            if report['nativeSession'].get('composerReady'):
                result=subprocess.run([str(prefix/'bin/python3'),'-B',str(source/'live_core_probe.py'),'--connector-name',args.connector_name,'--candidate',str(args.candidate),'--bun',str(args.bun),'--storage',str(args.storage),'--broker',str(args.broker),'--proof',str(root/'core.json'),'--native-host','--existing-native-descriptor',str(descriptor),'--prepared-native-session',str(root/'session.json')],env=dict(environment,HOME=os.environ["HOME"],TMPDIR=str(parent)),capture_output=True,timeout=160)
                assert (root/'core.json').exists(),'Core proof absent'
                report['core']=json.loads((root/'core.json').read_text())
                report['realModelReplyAccepted']=report['core'].get('realModelReplyAccepted') is True
            else:report['blockedBeforeCoreRequest']=True
        finally:
            for p in reversed(processes):
                if p.poll() is None:
                    p.terminate()
                    try:p.wait(timeout=8)
                    except subprocess.TimeoutExpired:os.killpg(p.pid,signal.SIGKILL);p.wait()
            report['ownedProcessesExited']=all(p.poll() is not None for p in processes)
            report['originalBrowserPreserved']=identity_matches(original)
            assert report['originalBrowserPreserved'] and report['ownedProcessesExited']
            args.proof.write_text(json.dumps(report,indent=2)+'\n');args.proof.chmod(0o600)
    print(json.dumps(report))
    return 0 if report['realModelReplyAccepted'] else 1


if __name__=='__main__':raise SystemExit(main())
