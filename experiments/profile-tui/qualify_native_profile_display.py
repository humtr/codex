"""Owned real native /switch, signed Core/Manager data, cancellation and error recovery."""
import argparse
import contextlib
import http.server
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
import threading
import tomllib

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

def qualify(binary, generation, public_key, parent, preview_core=None, preview_manager=None, case_filter=None, source_return=False, generation_mode=False):
    public_core = preview_core is not None or generation_mode
    if source_return:
        assert public_core and case_filter == 'persisted', 'source return requires the actual persisted Core path'
    results=[]
    cases=['ready','persisted','no-manager','no-native','bad-native'] if public_core else ['ready','no-manager']
    if case_filter is not None:
        assert case_filter in cases
        cases=[case_filter]
    for case in cases:
        available=case in ['ready','persisted']
        fallback=case in ['no-native','bad-native'] or (generation_mode and case == 'no-manager')
        with tempfile.TemporaryDirectory(prefix='np',dir=parent) as temporary, contextlib.ExitStack() as fixture_cleanup:
            root=Path(temporary);home=root/'h';prefix=root/'p'
            identity=next(line.split('\t',1)[1] for line in (generation/'generation.meta').read_text().splitlines() if line.startswith('generation_id\t'))
            native=home/'.local/lib/codex/core/generations'/identity
            if generation_mode:shutil.copytree(generation,native)
            else:materialize_signed_generation(generation,native)
            for path in [home/'.codex',home/'.local/share/codex/core/config',prefix/'bin',prefix/'etc/tls']:
                path.mkdir(parents=True,exist_ok=True,mode=0o700)
            core=prefix/'bin/codex';shutil.copy2(preview_core or generation/'core',core)
            if preview_core:
                assert preview_manager is not None
                assets=preview_assets(home)
                assets.mkdir(parents=True,mode=0o700)
                for source,name in [(binary,'native'),(preview_manager,'manager'),(generation/'core','stable-core')]:
                    shutil.copy2(source,assets/name);os.chmod(assets/name,0o755)
            if public_core:
                os.symlink(shutil.which('openssl'),prefix/'bin/openssl')
                curl=prefix/'bin/curl'
                curl.write_text('#!'+shutil.which('sh')+'\n: > "'+str(root/'public-discovery-attempted')+'"\nexit 1\n')
                os.chmod(curl,0o700)
            (prefix/'etc/resolv.conf').write_text('nameserver 127.0.0.1\n')
            (prefix/'etc/tls/cert.pem').write_text('owned\n')
            (home/'.local/share/codex/core/activation-state').write_text(fixture_activation_state(identity,public_key))
            model=None
            if case=='persisted':
                class HistoryModel(http.server.BaseHTTPRequestHandler):
                    def log_message(self,*_):pass
                    def do_POST(self):
                        assert self.path=='/v1/responses'
                        self.rfile.read(int(self.headers['Content-Length']))
                        self.server.requests+=1
                        message={'id':'owned-message','type':'message','role':'assistant','status':'completed','content':[{'type':'output_text','text':'owned-profile-history','annotations':[]}]}
                        events=[
                            {'type':'response.created','response':{'id':'owned-response','status':'in_progress','output':[]}},
                            {'type':'response.output_item.added','output_index':0,'item':dict(message,status='in_progress',content=[])},
                            {'type':'response.content_part.added','item_id':'owned-message','output_index':0,'content_index':0,'part':{'type':'output_text','text':'','annotations':[]}},
                            {'type':'response.output_text.delta','item_id':'owned-message','output_index':0,'content_index':0,'delta':'owned-profile-history'},
                            {'type':'response.output_item.done','output_index':0,'item':message},
                            {'type':'response.completed','response':{'id':'owned-response','status':'completed','output':[message],'usage':{'input_tokens':0,'output_tokens':0,'total_tokens':0}}},
                        ]
                        body=''.join('event: '+event['type']+'\ndata: '+json.dumps(event)+'\n\n' for event in events).encode()
                        self.send_response(200);self.send_header('Content-Type','text/event-stream');self.send_header('Content-Length',str(len(body)));self.end_headers()
                        self.wfile.write(body);self.wfile.flush()
                model=http.server.ThreadingHTTPServer(('127.0.0.1',0),HistoryModel)
                model.requests=0;model.daemon_threads=True
                fixture_cleanup.callback(model.server_close)
                threading.Thread(target=model.serve_forever,daemon=True).start()
                fixture_cleanup.callback(model.shutdown)
            config=f'''model = "fixture-model"
model_provider = "fixture"
sandbox_mode = "danger-full-access"
check_for_update_on_startup = false
[tui]
screen_reader_detection_done = true
show_tooltips = false
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
            if model is not None:config=config.replace('127.0.0.1:9/v1',f'127.0.0.1:{model.server_port}/v1')
            if public_core:config=config.replace('sandbox_mode = "danger-full-access"\n','')
            env={'HOME':str(home),'PREFIX':str(prefix),'TMPDIR':str(Path(shutil.which('sh')).parent.parent/'tmp'),'PATH':str(Path(shutil.which('sh')).parent),'TERM':'xterm-256color',
                 'SSL_CERT_FILE':str(Path(shutil.which('sh')).parent.parent/'etc/tls/cert.pem')}
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
            frontend = native/'helpers/2' if generation_mode else (assets/'native' if preview_core else binary)
            manager_asset = native/'manager' if generation_mode else (assets/'manager' if preview_core else generation/'manager')
            if public_core and case=='no-manager':manager_asset.unlink()
            if public_core and case=='no-native':frontend.unlink()
            if public_core and case=='bad-native':frontend.write_bytes(b'changed-owned-native')
            master,slave=pty.openpty();fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',40,180,0,0))
            original=termios.tcgetattr(slave)
            launch=[str(core),'--no-alt-screen'] if public_core else [str(binary),'--no-daemon','--no-alt-screen']
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
            def redraw():
                expected={native/'runtime',frontend} if public_core else {binary}
                output=''
                for columns in [181,180]:
                    assert process.poll() is None and Path(f'/proc/{process.pid}/exe').resolve() in expected
                    fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',40,columns,0,0))
                    os.kill(process.pid,signal.SIGWINCH)
                    output+=drain(.4)
                return output
            def command(text):
                os.write(master,text.encode());drain(.3);os.write(master,b'\r');output=drain(1)
                if process.poll() is None:output+=redraw()
                return output
            def until(text,initial=''):
                output=initial;deadline=time.monotonic()+15
                pattern = re.escape(text).replace(r'\ ', r'\s*')
                while re.search(pattern, output) is None:
                    assert process.poll() is None,'owned TUI exited'
                    assert time.monotonic()<deadline,'missing '+text+': '+output[-2500:]
                    output+=drain(.25)
                return output
            expected_cwd=root
            def status():
                output=until('Token usage:',command('/status'))
                assert re.search(r'Token usage:\s+0\s*total',output),output[-2500:]
                assert re.search(r'Model provider:\s+fixture',output),output[-2500:]
                match=re.search(r'Session:\s+([0-9a-f-]{36})',output);assert match,output[-2500:]
                if public_core:assert str(expected_cwd) in output,'native working directory changed'
                return match[1]
            try:
                until('fixture-model');drain(3)
                thread=status()
                if public_core and available:
                    command('/quit');process.wait(timeout=10)
                    assert process.returncode==0 and termios.tcgetattr(slave)==original
                    expected_cwd=root/'next';expected_cwd.mkdir(mode=0o700)
                    process=subprocess.Popen(launch,env=env,cwd=expected_cwd,stdin=slave,stdout=slave,stderr=slave,start_new_session=True)
                    until('fixture-model');drain(3);thread=status()
                    results.append('existing-backend/new-client-CWD/same-terminal/native-exec')
                if public_core:
                    expected_frontend=native/'runtime' if fallback else frontend
                    assert Path(f'/proc/{process.pid}/exe').resolve()==expected_frontend,'public Core did not exec expected frontend'
                    records=list((home/'.local/share/codex/core/servers').glob('*/pid'))
                    assert len(records)==1
                    server=int(records[0].read_text())
                    assert Path(f'/proc/{server}/exe').resolve()==native/'runtime','wrong backend executable'
                    menu=until('Full Access',command('/permissions'))
                    assert 'Read Only' not in menu,'unsupported Read Only exposed'
                    assert 'without a sandbox' in menu,'misleading permission description'
                    os.write(master,b'\x1b');drain(.3)
                    results.append('public-Core/actual-source-frontend/qualified-signed-backend/permission-menu')
                    if case=='persisted':
                        expected_cwd=root/'changed';expected_cwd.mkdir(mode=0o700)
                        command('/cd '+str(expected_cwd));drain(2);thread=status()
                        until('owned-profile-history',command('owned prior conversation'));drain(3)
                        assert status()==thread,'owned fixture turn changed thread'
                        requests_before=model.requests
                        assert requests_before>=1,'owned local fixture was never called'
                        rpc=Ws(records[0].parent/'s')
                        try:
                            try:
                                stored=rpc.call('thread/read',{'threadId':thread,'includeTurns':True})
                            except AssertionError as error:
                                assert error.args[0]==('thread/read', {'code':-32601,'message':'list_turns is not supported yet'}), 'unexpected owned persistence error'
                                stored=rpc.call('thread/read',{'threadId':thread,'includeTurns':False})
                            assert stored['thread']['id']==thread
                        finally:rpc.close()
                        rollout=Path(stored['thread']['path'])
                        assert rollout.resolve().is_relative_to(root) and rollout.is_file()
                        prior_history=[line for line in rollout.read_text().splitlines() if 'owned-profile-history' in line]
                        assert prior_history,'owned completed fixture turn was not persisted'
                if available:
                    output=until('Profiles',command('/switch'))
                    assert 'Current: work' in output and 'Default: work' in output,output[-2500:]
                    assert 'external' in output and 'default' in output,output[-2500:]
                    os.write(master,b'work');filtered=drain(1)+redraw()
                    assert 'work (current)' in filtered and 'external' not in filtered,'profile search did not render the matching row'
                    os.write(master,b'\x1b');drain(.3);os.write(master,b'\x1b');drain(.3)
                    assert status()==thread,'profile cancellation changed conversation'
                    until('Profiles',command('/switch'))
                    os.write(master,b'\x03');drain(.5)
                    assert status()==thread,'profile interrupt changed conversation'
                    results.append('native-profile/signed-core-manager/list-current-default/search/Esc/Ctrl-C/same-thread')
                    if public_core:
                        if case=='ready':
                            # Fault only the owned blank rollout destination; never seed it.
                            rpc=Ws(records[0].parent/'s')
                            try: metadata=rpc.call('thread/read',{'threadId':thread,'includeTurns':False})['thread']
                            finally: rpc.close()
                            rollout=Path(metadata['path'])
                            assert rollout.resolve().is_relative_to(root) and not rollout.exists()
                            rollout.parent.mkdir(parents=True,exist_ok=True,mode=0o700)
                            original_mode=rollout.parent.stat().st_mode & 0o777
                            try:
                                os.chmod(rollout.parent,0o500)
                                until('Profiles',command('/switch'))
                                os.write(master,b'\x1b[B');drain(.3);os.write(master,b'\r')
                                until('Current conversation could not be saved for profile switching.')
                                assert process.poll() is None and not rollout.exists()
                            finally: os.chmod(rollout.parent,original_mode)
                            os.write(master,b'\x1b');drain(.3)
                            assert status()==thread,'persistence failure lost current chat'
                            results.append('native-profile/unwritable-blank-rollout/refusal/current-chat-preserved')
                        # A stalled owned backend must time out before native cleanup.
                        assert Path(f'/proc/{server}/exe').resolve()==native/'runtime'
                        until('Profiles',command('/switch'))
                        os.write(master,b'\x1b[B');drain(.3)
                        os.kill(server,signal.SIGSTOP)
                        try:
                            os.write(master,b'\r')
                            until('Current conversation could not be saved for profile switching.')
                            assert process.poll() is None
                        finally:
                            if Path(f'/proc/{server}/exe').resolve()==native/'runtime':
                                os.kill(server,signal.SIGCONT)
                        os.write(master,b'\x1b');drain(.3)
                        assert status()==thread,'persistence timeout lost current chat'
                        results.append('native-profile/owned-backend-timeout/refusal/current-chat-preserved')
                        until('Profiles',command('/switch'))
                        os.write(master,b'work');drain(.3);os.write(master,b'\r');drain(.5)
                        assert status()==thread,'current-profile Enter changed conversation'
                        initial_pid=process.pid
                        destinations=['external','work','default','work'] + (['external'] if source_return else [])
                        for destination in destinations:
                            until('Profiles',command('/switch'))
                            # Exercise the actual arrow selection, not a private handoff command.
                            steps={'default':0,'external':1,'work':2}[destination]
                            os.write(master,b'\x1b[B'*steps);drain(.3)
                            os.write(master,b'\r')
                            startup=until('fixture-model');startup+=drain(2)
                            if case=='persisted':
                                until('owned-profile-history',startup)
                                lines=rollout.read_text().splitlines()
                                assert all(line in lines for line in prior_history),'native prior history changed'
                                assert model.requests==requests_before,'profile handoff started a model request'
                            assert process.pid==initial_pid and process.poll() is None,'profile switch replaced terminal process'
                            assert status()==thread,'profile switch did not preserve fresh conversation'
                            view=until(destination+' (current)',command('/switch'))
                            assert re.search(r'Default:\s*work',view),'profile switch changed saved default'
                            os.write(master,b'\x1b');drain(.3)
                            tasks=json.loads(manager('__task-snapshot-v1'))['tasks']
                            owners=[item['owner_profile'] for item in tasks if item['id']==thread]
                            assert owners==[destination],'profile switch did not transfer native writer'
                            assert Path(f'/proc/{server}/exe').resolve()==native/'runtime','previous account backend changed'
                        results.append('direct-arrow-Enter/'+('fresh-unseeded-thread' if case=='ready' else 'persisted-thread')+'/external-work-default/same-UUID-PID-TTY-CWD/native-writer/default-preserved')
                        if case=='persisted':results.append('native-nonempty-completed-turn-history/public-profile-'+str(len(destinations))+'-transitions/visible-and-byte-records-preserved/no-external-model-call/no-model-request-during-handoff')
                        if source_return:
                            source=home/'.local/share/codex/manager/profiles/external/home'
                            source_records=[record for record in (home/'.local/share/codex/core/servers').glob('*/pid')
                                            if (record.parent/'owner').read_bytes().split(b'\0')[0] == os.fsencode(source)]
                            assert len(source_records)==1
                            source_record=source_records[0]
                            source_pid=int(source_record.read_text())
                            assert Path(f'/proc/{source_pid}/exe').resolve()==native/'runtime'
                            subscriber=Ws(source_record.parent/'s')
                            try:
                                # A real second connection retains the original upstream writer
                                # after native cleanup; no production injection or lock removal.
                                subscriber.call('thread/resume',{'threadId':thread,'cwd':str(expected_cwd)})
                                until('Profiles',command('/switch'))
                                os.write(master,b'\x1b[B'*2);drain(.3);os.write(master,b'\r')
                                refusal=until('Enter to return')
                                assert 'writer is still owned' in refusal
                                assert process.pid==initial_pid and Path(f'/proc/{process.pid}/exe').resolve()==manager_asset
                                assert termios.tcgetattr(slave)==original,'cleanup did not restore terminal before source choice'
                                tasks=json.loads(manager('__task-snapshot-v1'))['tasks']
                                assert [item['owner_profile'] for item in tasks if item['id']==thread]==['external']
                                count=len(list((home/'.local/share/codex/core/servers').glob('*/pid')))
                                os.write(master,b'\r')
                                returned=until('fixture-model');returned+=drain(2)
                                until('owned-profile-history',returned)
                                assert status()==thread and process.pid==initial_pid
                                view=until('external (current)',command('/switch'))
                                assert re.search(r'Default:\s*work',view)
                                os.write(master,b'\x1b');drain(.3)
                                assert int(source_record.read_text())==source_pid
                                assert len(list((home/'.local/share/codex/core/servers').glob('*/pid')))==count
                                assert model.requests==requests_before,'source return started model work'
                                assert all(line in rollout.read_text().splitlines() for line in prior_history)
                            finally:subscriber.close()
                            results.append('actual-native-arrow-Enter/post-cleanup-retained-writer-refusal/explicit-original-profile-return/same-UUID-PID-TTY-CWD/backend/default-history-preserved/no-model-request')
                elif fallback:
                    assert status()==thread,'installed fallback broke current chat'
                    results.append('public-Core/'+case+'/usable-installed-fallback')
                else:
                    until('Profiles are unavailable.',command('/switch'))
                    assert status()==thread,'unavailable bridge broke current chat'
                    results.append('native-profile/unavailable-bridge/current-chat-usable')
                command('/quit');process.wait(timeout=10)
                assert process.returncode==0
                assert termios.tcgetattr(slave)==original
                assert not (account/'auth.json').exists()
                changed=[]
                for path in protected:
                    if path.read_bytes()==before[str(path)]:continue
                    assert path.resolve().is_relative_to(root)
                    def leaves(value, prefix=''):
                        if isinstance(value,dict):
                            return {key:leaf for name,child in value.items() for key,leaf in leaves(child,prefix+name+'.').items()}
                        return {prefix.rstrip('.'):value}
                    if path.name=='config.toml':
                        old=leaves(tomllib.loads(before[str(path)].decode()))
                        new=leaves(tomllib.loads(path.read_text()))
                        fields=[key for key in sorted(set(old)|set(new)) if old.get(key)!=new.get(key)]
                    else:fields=['saved-default-bytes']
                    changed.append({'relative':str(path.relative_to(root)), 'fields':fields})
                if changed:
                    (parent/'native-profile-owned-config-diff.json').write_text(json.dumps({'case':case,'changes':changed,'completed_proof':results},sort_keys=True))
                assert not changed,'profile configuration/default changed'
                assert not list(home.rglob('auth.json')),'owned transition wrote authentication'
                if public_core:assert not (root/'public-discovery-attempted').exists(),'preview requested public discovery'
            except Exception:
                (parent/'native-profile-owned-failure.txt').write_bytes(transcript)
                raise
            finally:
                if process.poll() is None:
                    process.terminate()
                    try:process.wait(timeout=3)
                    except subprocess.TimeoutExpired:process.kill();process.wait()
                if public_core:
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
    parser.add_argument('--case',choices=['ready','persisted','no-manager','no-native','bad-native'])
    parser.add_argument('--source-return-proof',action='store_true')
    parser.add_argument('--generation-mode',action='store_true')
    a=parser.parse_args();print(json.dumps(qualify(a.binary.resolve(),a.generation.resolve(),a.public_key.resolve(),a.parent.resolve(),a.preview_core.resolve() if a.preview_core else None,a.preview_manager.resolve() if a.preview_manager else None,a.case,a.source_return_proof,a.generation_mode),sort_keys=True))
