"""Owned, credential-free native baseline: version/help, actual TUI and terminal restore."""
import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import pty
import re
import select
import struct
import subprocess
import tempfile
import termios
import time

ANSI = re.compile(r'\x1b\[[0-?]*[ -/]*[@-~]|\x1b\].*?(?:\x07|\x1b\\)', re.S)

def qualify(binary, parent):
    with tempfile.TemporaryDirectory(prefix='profile-native-', dir=parent) as temporary:
        root = Path(temporary)
        home, account = root/'home', root/'home/.codex'
        account.mkdir(parents=True, mode=0o700)
        temporary_root = root/'tmp';temporary_root.mkdir(mode=0o700)
        config = f'''model = "fixture-model"
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
        (account/'config.toml').write_text(config)
        env = {'HOME':str(home),'CODEX_HOME':str(account),'TMPDIR':str(temporary_root),
               'PATH':str(Path('/data/data/com.termux/files/usr/bin')),'TERM':'xterm-256color'}
        version = subprocess.run([str(binary),'--version'],env=env,cwd=root,capture_output=True,timeout=15)
        assert version.returncode == 0 and version.stdout == b'codex-cli 0.160.0\n',version
        help_result = subprocess.run([str(binary),'--help'],env=env,cwd=root,capture_output=True,timeout=15)
        assert help_result.returncode == 0 and b'Codex CLI' in help_result.stdout,help_result
        master, slave = pty.openpty()
        fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',40,180,0,0))
        original = termios.tcgetattr(slave)
        process = subprocess.Popen([str(binary),'--no-daemon','--no-alt-screen'],env=env,cwd=root,
                                   stdin=slave,stdout=slave,stderr=slave,start_new_session=True)
        transcript = bytearray()
        def drain(seconds):
            output = bytearray();deadline=time.monotonic()+seconds
            while time.monotonic()<deadline:
                if not select.select([master],[],[],.05)[0]:continue
                try:data=os.read(master,65536)
                except OSError:break
                if b'\x1b[6n' in data:os.write(master,b'\x1b[1;1R')
                if b'\x1b[c' in data:os.write(master,b'\x1b[?1;2c')
                output.extend(data);transcript.extend(data)
            assert len(transcript)<2*1024*1024,'unexpected unbounded terminal output'
            return ANSI.sub('',output.decode(errors='replace'))
        try:
            boot='';deadline=time.monotonic()+60
            while 'fixture-model' not in boot:
                boot+=drain(.25)
                assert process.poll() is None,'baseline TUI exited: '+boot[-3000:]
                assert time.monotonic()<deadline,'baseline TUI did not initialize: '+boot[-3000:]
            assert 'Sign in with ChatGPT' not in boot,'unexpected real-auth onboarding'
            drain(3)
            os.write(master,b'/status');drain(.3);os.write(master,b'\r')
            status='';deadline=time.monotonic()+15
            while 'Token usage:' not in status:
                status+=drain(.25)
                assert process.poll() is None,'baseline status exited'
                assert time.monotonic()<deadline,'status missing: '+status[-3000:]
            assert re.search(r'Model provider:\s+fixture',status),status[-3000:]
            assert re.search(r'Token usage:\s+0 total',status),status[-3000:]
            os.write(master,b'/quit');drain(.3);os.write(master,b'\r')
            deadline=time.monotonic()+10
            while process.poll() is None and time.monotonic()<deadline:drain(.1)
            assert process.wait(timeout=1)==0,'native /quit failed'
            assert termios.tcgetattr(slave)==original,'terminal modes were not restored'
            assert not (account/'auth.json').exists(),'unexpected credential creation'
            return {'version':version.stdout.decode().strip(),'sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),
                    'proof':['version','help','native-PTY-status','zero-token-usage','native-quit','exact-terminal-restoration','no-auth-file']}
        except Exception:
            (parent/'native-baseline-owned-failure.txt').write_bytes(transcript)
            raise
        finally:
            if process.poll() is None:
                process.terminate()
                try:process.wait(timeout=3)
                except subprocess.TimeoutExpired:process.kill();process.wait()
            os.close(master);os.close(slave)

if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('binary',type=Path);parser.add_argument('--parent',type=Path,required=True)
    args=parser.parse_args();print(json.dumps(qualify(args.binary.resolve(),args.parent.resolve()),sort_keys=True))
