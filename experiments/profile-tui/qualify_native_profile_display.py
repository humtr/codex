"""Owned real native /profile, signed Core/Manager data, cancellation and error recovery."""
import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import pty
import re
import select
import shutil
import signal
import struct
import subprocess
import sys
import tempfile
import termios
import time

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'scripts'))
from qualify_tasks import Ws, fixture_activation_state, materialize_signed_generation

ANSI = re.compile(r'\x1b\[[0-?]*[ -/]*[@-~]|\x1b\].*?(?:\x07|\x1b\\)', re.S)

def preview_assets(home):
    source = Path(__file__).with_name('live_preview.rs').read_text()
    directories = re.findall(r'^const DIRECTORY: &str = "([^"]+)";', source, re.M)
    assert len(directories) == 1, 'preview directory must have one production owner'
    relative = Path(directories[0])
    assert not relative.is_absolute() and '..' not in relative.parts
    return home / relative

def qualify(binary, generation, public_key, parent, preview_core=None, preview_manager=None):
    results=[]
    cases=['ready','persisted','no-manager','no-native','bad-native'] if preview_core else ['ready','no-manager']
    for case in cases:
        available=case in ['ready','persisted']
        fallback=case in ['no-native','bad-native']
        with tempfile.TemporaryDirectory(prefix='np',dir=parent) as temporary:
            root=Path(temporary);home=root/'h';prefix=root/'p'
            identity=dict(line.split('\t',1) for line in (generation/'generation.meta').read_text().splitlines()[1:])['generation_id']
            native=home/'.local/lib/codex/core/generations'/identity
            materialize_signed_generation(generation,native)
            for path in [home/'.codex',home/'.local/share/codex/core/config',prefix/'bin',prefix/'etc/tls']:
                path.mkdir(parents=True,exist_ok=True,mode=0o700)
            core=prefix/'bin/codex';shutil.copy2(preview_core or generation/'core',core)
            if preview_core:
                assert preview_manager is not None
                assets=preview_assets(home)
                assets.mkdir(parents=True,mode=0o700)
                for source,name in [(binary,'native'),(preview_manager,'manager'),(generation/'core','stable-core')]:
                    shutil.copy2(source,assets/name);os.chmod(assets/name,0o755)
                os.symlink(shutil.which('openssl'),prefix/'bin/openssl')
                curl=prefix/'bin/curl'
                curl.write_text('#!'+shutil.which('sh')+'\n: > "'+str(root/'public-discovery-attempted')+'"\nexit 1\n')
                os.chmod(curl,0o700)
            (prefix/'etc/resolv.conf').write_text('nameserver 127.0.0.1\n')
            (prefix/'etc/tls/cert.pem').write_text('owned\n')
            (home/'.local/share/codex/core/activation-state').write_text(fixture_activation_state(identity,public_key))
            config=f'''model = "fixture-model"
model_provider = "fixture"
sandbox_mode = "danger-full-access"
check_for_update_on_startup = false
[analytics]
enabled = false
[feedback]
enabled = false
[projects."{root}"]
trust_level = "trusted"
[features]
memories = false
guardian_approval = true
[model_providers.fixture]
name = "Owned fixture"
base_url = "http://127.0.0.1:9/v1"
wire_api = "responses"
requires_openai_auth = false
'''
            if preview_core:config=config.replace('sandbox_mode = "danger-full-access"\n','')
            env={'HOME':str(home),'PREFIX':str(prefix),'TMPDIR':str(parent),'PATH':str(Path(shutil.which('sh')).parent),'TERM':'xterm-256color'}
            def manager(*args):
                result=subprocess.run([str(core),'termux',*args],env=env,cwd=root,capture_output=True,timeout=15)
                assert result.returncode==0,(args,result.stderr.decode())
                return result.stdout
            for id in ['external','work']:manager('profile','create',id)
            manager('profile','default','work')
            account=home/'.local/share/codex/manager/profiles/work/home'
            for selected in [home/'.codex', account, home/'.local/share/codex/manager/profiles/external/home']:
                (selected/'config.toml').write_text(config)
            env['CODEX_HOME']=str(account)
            protected=[home/'.local/share/codex/manager/default-profile-v1']
            protected += [selected/'config.toml' for selected in [home/'.codex',account,home/'.local/share/codex/manager/profiles/external/home']]
            before={str(path):path.read_bytes() for path in protected}
            snapshot=json.loads(manager('__profile-snapshot-v1'))
            assert snapshot=={'schema':'codex-manager-profiles-v1','profiles':['default','external','work'],'current':'work','current_source':'inherited','saved_default':'work'},snapshot
            if available:env['CODEX_PROFILE_CORE']=str(core)
            if preview_core and case=='no-manager':(assets/'manager').unlink()
            if preview_core and case=='no-native':(assets/'native').unlink()
            if preview_core and case=='bad-native':(assets/'native').write_bytes(b'changed-owned-native')
            master,slave=pty.openpty();fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',40,180,0,0))
            original=termios.tcgetattr(slave)
            launch=[str(core),'--no-alt-screen'] if preview_core else [str(binary),'--no-daemon','--no-alt-screen']
            process=subprocess.Popen(launch,env=env,cwd=root,stdin=slave,stdout=slave,stderr=slave,start_new_session=True)
            transcript=bytearray()
            def drain(seconds):
                output=bytearray();deadline=time.monotonic()+seconds
                while time.monotonic()<deadline:
                    if not select.select([master],[],[],.05)[0]:continue
                    try:data=os.read(master,65536)
                    except OSError:break
                    if b'\x1b[6n' in data:os.write(master,b'\x1b[1;1R')
                    if b'\x1b[c' in data:os.write(master,b'\x1b[?1;2c')
                    output.extend(data);transcript.extend(data)
                assert len(transcript)<4*1024*1024,'unbounded owned TUI output'
                return ANSI.sub('',output.decode(errors='replace'))
            def command(text):
                os.write(master,text.encode());drain(.3);os.write(master,b'\r');return drain(1)
            def until(text,initial=''):
                output=initial;deadline=time.monotonic()+15
                while text not in output:
                    assert process.poll() is None,'owned TUI exited'
                    assert time.monotonic()<deadline,'missing '+text+': '+output[-2500:]
                    output+=drain(.25)
                return output
            expected_cwd=root
            def status():
                output=until('Token usage:',command('/status'))
                assert re.search(r'Token usage:\s+0 total',output),output[-2500:]
                assert re.search(r'Model provider:\s+fixture',output),output[-2500:]
                match=re.search(r'Session:\s+([0-9a-f-]{36})',output);assert match,output[-2500:]
                if preview_core:assert str(expected_cwd) in output,'native working directory changed'
                return match[1]
            try:
                until('fixture-model');drain(3)
                thread=status()
                if preview_core and available:
                    command('/quit');process.wait(timeout=10)
                    assert process.returncode==0 and termios.tcgetattr(slave)==original
                    expected_cwd=root/'next';expected_cwd.mkdir(mode=0o700)
                    process=subprocess.Popen(launch,env=env,cwd=expected_cwd,stdin=slave,stdout=slave,stderr=slave,start_new_session=True)
                    until('fixture-model');drain(3);thread=status()
                    results.append('existing-backend/new-client-CWD/same-terminal/native-exec')
                if preview_core:
                    expected_frontend=native/'runtime' if fallback else assets/'native'
                    assert Path(f'/proc/{process.pid}/exe').resolve()==expected_frontend,'public Core did not exec expected frontend'
                    records=list((home/'.local/share/codex/core/servers').glob('*/pid'))
                    assert len(records)==1
                    server=int(records[0].read_text())
                    assert Path(f'/proc/{server}/exe').resolve()==native/'runtime','wrong backend executable'
                    menu=until('Full Access',command('/permissions'))
                    assert 'Read Only' not in menu,'unsupported Read Only exposed'
                    assert 'without a sandbox' in menu,'misleading permission description'
                    os.write(master,b'\x1b');drain(.3)
                    results.append('public-preview-Core/actual-source-frontend/qualified-signed-backend/permission-menu')
                    if case=='persisted':
                        rpc=Ws(records[0].parent/'s')
                        try:
                            stored=rpc.call('thread/read',{'threadId':thread,'includeTurns':True})
                            assert stored['thread']['id']==thread
                            assert stored['thread']['turns']==[],'fixture unexpectedly has model turns'
                        finally:rpc.close()
                        assert list((home/'.codex/sessions').rglob('*'+thread+'*')),'owned read did not persist conversation'
                if available:
                    output=until('Profiles',command('/profile'))
                    assert 'Current: work' in output and 'Default: work' in output,output[-2500:]
                    assert 'external' in output and 'default' in output,output[-2500:]
                    os.write(master,b'work');filtered=drain(1)
                    assert 'work (current)' in filtered and 'external' not in filtered,'profile search did not render the matching row'
                    os.write(master,b'\x1b');drain(.3);os.write(master,b'\x1b');drain(.3)
                    assert status()==thread,'profile cancellation changed conversation'
                    until('Profiles',command('/profile'))
                    os.write(master,b'\x03');drain(.5)
                    assert status()==thread,'profile interrupt changed conversation'
                    results.append('native-profile/signed-core-manager/list-current-default/search/Esc/Ctrl-C/same-thread')
                    if preview_core:
                        until('Profiles',command('/profile'))
                        os.write(master,b'work');drain(.3);os.write(master,b'\r');drain(.5)
                        assert status()==thread,'current-profile Enter changed conversation'
                        initial_pid=process.pid
                        for destination in ['external','work','default','work']:
                            until('Profiles',command('/profile'))
                            # Exercise the actual arrow selection, not a private handoff command.
                            steps={'default':0,'external':1,'work':2}[destination]
                            os.write(master,b'\x1b[B'*steps);drain(.3)
                            os.write(master,b'\r')
                            until('fixture-model');drain(2)
                            assert process.pid==initial_pid and process.poll() is None,'profile switch replaced terminal process'
                            assert status()==thread,'profile switch did not preserve fresh conversation'
                            view=until('Current: '+destination,command('/profile'))
                            assert 'Default: work' in view,'profile switch changed saved default'
                            os.write(master,b'\x1b');drain(.3)
                            tasks=json.loads(manager('__task-snapshot-v1'))['tasks']
                            owners=[item['owner_profile'] for item in tasks if item['id']==thread]
                            assert owners==[destination],'profile switch did not transfer native writer'
                            assert Path(f'/proc/{server}/exe').resolve()==native/'runtime','previous account backend changed'
                        results.append('direct-arrow-Enter/'+('fresh-unseeded-thread' if case=='ready' else 'persisted-thread')+'/external-work-default/same-UUID-PID-TTY-CWD/native-writer/default-preserved')
                elif fallback:
                    assert status()==thread,'installed fallback broke current chat'
                    results.append('public-preview-Core/'+case+'/usable-installed-fallback')
                else:
                    until('Profiles are unavailable.',command('/profile'))
                    assert status()==thread,'unavailable bridge broke current chat'
                    results.append('native-profile/unavailable-bridge/current-chat-usable')
                command('/quit');process.wait(timeout=10)
                assert process.returncode==0
                assert termios.tcgetattr(slave)==original
                assert not (account/'auth.json').exists()
                assert all(path.read_bytes()==before[str(path)] for path in protected if str(path) in before),'profile configuration/default changed'
                assert not list(home.rglob('auth.json')),'owned transition wrote authentication'
                if preview_core:assert not (root/'public-discovery-attempted').exists(),'preview requested public discovery'
            except Exception:
                (parent/'native-profile-owned-failure.txt').write_bytes(transcript);raise
            finally:
                if process.poll() is None:
                    process.terminate()
                    try:process.wait(timeout=3)
                    except subprocess.TimeoutExpired:process.kill();process.wait()
                if preview_core:
                    for record in (home/'.local/share/codex/core/servers').glob('*/pid'):
                        pid=int(record.read_text())
                        try:
                            if Path(f'/proc/{pid}/exe').resolve()!=native/'runtime':continue
                            os.kill(pid,signal.SIGTERM)
                            deadline=time.monotonic()+5
                            while Path(f'/proc/{pid}/exe').exists() and time.monotonic()<deadline:time.sleep(.05)
                            if Path(f'/proc/{pid}/exe').resolve()==native/'runtime':os.kill(pid,signal.SIGKILL)
                            assert not Path(f'/proc/{pid}/exe').exists(),'owned server teardown incomplete'
                        except (FileNotFoundError,ProcessLookupError):pass
                os.close(master);os.close(slave)
    return {'native_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'manager_sha256':hashlib.sha256((preview_manager or generation/'manager').read_bytes()).hexdigest(),'core_sha256':hashlib.sha256((preview_core or generation/'core').read_bytes()).hexdigest(),'proof':results+['zero-tokens/no-auth/exact-termios-restoration']}

if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('binary',type=Path);parser.add_argument('--generation',type=Path,required=True);parser.add_argument('--public-key',type=Path,required=True);parser.add_argument('--parent',type=Path,required=True)
    parser.add_argument('--preview-core',type=Path);parser.add_argument('--preview-manager',type=Path)
    a=parser.parse_args();print(json.dumps(qualify(a.binary.resolve(),a.generation.resolve(),a.public_key.resolve(),a.parent.resolve(),a.preview_core.resolve() if a.preview_core else None,a.preview_manager.resolve() if a.preview_manager else None),sort_keys=True))
