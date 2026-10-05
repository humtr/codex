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
import struct
import subprocess
import sys
import tempfile
import termios
import time

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'scripts'))
from qualify_tasks import fixture_activation_state, materialize_signed_generation

ANSI = re.compile(r'\x1b\[[0-?]*[ -/]*[@-~]|\x1b\].*?(?:\x07|\x1b\\)', re.S)

def qualify(binary, generation, public_key, parent):
    results=[]
    for available in [True,False]:
        with tempfile.TemporaryDirectory(prefix='native-profile-',dir=parent) as temporary:
            root=Path(temporary);home=root/'h';prefix=root/'p'
            identity=dict(line.split('\t',1) for line in (generation/'generation.meta').read_text().splitlines()[1:])['generation_id']
            native=home/'.local/lib/codex/core/generations'/identity
            materialize_signed_generation(generation,native)
            for path in [home/'.codex',home/'.local/share/codex/core/config',prefix/'bin',prefix/'etc/tls']:
                path.mkdir(parents=True,exist_ok=True,mode=0o700)
            core=prefix/'bin/codex';shutil.copy2(generation/'core',core)
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
[model_providers.fixture]
name = "Owned fixture"
base_url = "http://127.0.0.1:9/v1"
wire_api = "responses"
requires_openai_auth = false
'''
            env={'HOME':str(home),'PREFIX':str(prefix),'TMPDIR':str(parent),'PATH':str(Path(shutil.which('sh')).parent),'TERM':'xterm-256color'}
            def manager(*args):
                result=subprocess.run([str(core),'termux',*args],env=env,cwd=root,capture_output=True,timeout=15)
                assert result.returncode==0,(args,result.stderr.decode())
                return result.stdout
            for id in ['external','work']:manager('profile','create',id)
            manager('profile','default','work')
            account=home/'.local/share/codex/manager/profiles/work/home'
            (account/'config.toml').write_text(config)
            env['CODEX_HOME']=str(account)
            snapshot=json.loads(manager('__profile-snapshot-v1'))
            assert snapshot=={'schema':'codex-manager-profiles-v1','profiles':['default','external','work'],'current':'work','current_source':'inherited','saved_default':'work'},snapshot
            if available:env['CODEX_PROFILE_CORE']=str(core)
            master,slave=pty.openpty();fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',40,180,0,0))
            original=termios.tcgetattr(slave)
            process=subprocess.Popen([str(binary),'--no-daemon','--no-alt-screen'],env=env,cwd=root,stdin=slave,stdout=slave,stderr=slave,start_new_session=True)
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
            def status():
                output=until('Token usage:',command('/status'))
                assert re.search(r'Token usage:\s+0 total',output),output[-2500:]
                assert re.search(r'Model provider:\s+fixture',output),output[-2500:]
                match=re.search(r'Session:\s+([0-9a-f-]{36})',output);assert match,output[-2500:]
                return match[1]
            try:
                until('fixture-model');drain(3)
                thread=status()
                if available:
                    output=until('Profiles',command('/profile'))
                    assert 'Current: work' in output and 'Default: work' in output,output[-2500:]
                    assert 'external' in output and 'default' in output,output[-2500:]
                    os.write(master,b'work');filtered=drain(1)
                    assert 'work' in filtered and 'external' not in filtered,filtered[-2500:]
                    os.write(master,b'\x1b');drain(.3);os.write(master,b'\x1b');drain(.3)
                    assert status()==thread,'profile cancellation changed conversation'
                    until('Profiles',command('/profile'))
                    os.write(master,b'\x03');drain(.5)
                    assert status()==thread,'profile interrupt changed conversation'
                    results.append('native-profile/signed-core-manager/list-current-default/search/Esc/Ctrl-C/same-thread')
                else:
                    until('Profiles are unavailable.',command('/profile'))
                    assert status()==thread,'unavailable bridge broke current chat'
                    results.append('native-profile/unavailable-bridge/current-chat-usable')
                command('/quit');process.wait(timeout=10)
                assert process.returncode==0
                assert termios.tcgetattr(slave)==original
                assert not (account/'auth.json').exists()
            except Exception:
                (parent/'native-profile-owned-failure.txt').write_bytes(transcript);raise
            finally:
                if process.poll() is None:
                    process.terminate()
                    try:process.wait(timeout=3)
                    except subprocess.TimeoutExpired:process.kill();process.wait()
                os.close(master);os.close(slave)
    return {'native_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'manager_sha256':hashlib.sha256((generation/'manager').read_bytes()).hexdigest(),'core_sha256':hashlib.sha256((generation/'core').read_bytes()).hexdigest(),'proof':results+['zero-tokens/no-auth/exact-termios-restoration']}

if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('binary',type=Path);parser.add_argument('--generation',type=Path,required=True);parser.add_argument('--public-key',type=Path,required=True);parser.add_argument('--parent',type=Path,required=True)
    a=parser.parse_args();print(json.dumps(qualify(a.binary.resolve(),a.generation.resolve(),a.public_key.resolve(),a.parent.resolve()),sort_keys=True))
