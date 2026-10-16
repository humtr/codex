"""Real Android Core/frontend /model round trip in disposable, unauthenticated state.

No inference is submitted, including Pro. A private fixture signer admits only
this disposable generation; no production launcher, key or activation is changed.
"""
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
import tomllib

SOURCE = Path(os.environ.get('CODEX_CHAT_CORE_PROOF_SOURCE', str(Path(__file__).resolve().parents[2])))
sys.path.insert(0, str(SOURCE / 'scripts'))
from qualify_tasks import Ws
from qualify_permission_lifecycle import fixture as permission_fixture
from rald3_preflight import parse_descriptor
from rald5_publication import parse_manifest


def sha(path):
    with path.open('rb') as f:
        return hashlib.file_digest(f, 'sha256').hexdigest()


def fixture(root, core, frontend, named_permission=False, manager=None):
    home, prefix, env = permission_fixture(root, core, frontend, manager)
    native_path = home/'.codex/native-models.json'
    catalog = json.loads(native_path.read_text())
    catalog['models'][0].update(display_name='Owned Codex', visibility='list')
    for name in ['native-models.json','models_cache.json']:
        (home/'.codex'/name).write_text(json.dumps(catalog))
    if named_permission:
        (home/'.codex/owned-config.config.toml').write_text(
            'default_permissions = "owned"\n[permissions.owned.filesystem]\n":root" = "write"\n'
            '[permissions.owned.network]\nenabled = true\n')
        (home/'.codex/owned-config.config.toml').chmod(0o600)
    # The shared optional profile is provider metadata, not account auth.
    trial = Path.home()/'.local/share/codex-web-experiment/native-provider'
    selected_home = Path(os.environ.get('CODEX_HOME',Path.home()/'.codex'))
    profile = (selected_home/'chatgpt.config.toml').read_text()
    catalog_path=home/'.codex/chat-models.json'
    catalog_path.write_bytes((trial/'models.json').read_bytes());catalog_path.chmod(0o600)
    profile=re.sub(r'^model_catalog_json = .*$', 'model_catalog_json = '+json.dumps(str(catalog_path)), profile, flags=re.M)
    assert profile.startswith('# codex-web-experiment ChatGPT profile begin\n')
    registry=home/'.local/share/codex/core/config/profiles';registry.mkdir(mode=0o700)
    (registry/'chatgpt.config.toml').write_text(profile);(registry/'chatgpt.config.toml').chmod(0o600)
    return home,prefix,env


def backend_thread(home, thread, check_defaults=False):
    servers = [p for family in ['servers', 'catalogs']
        for p in (home / '.local/share/codex/core' / family).glob('*/s')]
    expected = home / '.local/lib/codex/core/generations/local-permission-fixture/runtime'
    found = []
    observed=[]
    for socket in servers:
        pid = int(socket.with_name('pid').read_text().strip())
        assert Path(f'/proc/{pid}/exe').resolve() == expected, 'official_backend_lost'
        wire = Ws(socket, uri_path='/rpc')
        try:
            loaded = wire.call('thread/loaded/list', {'limit':100})
            observed.append({'family':socket.parent.parent.name,'loaded':loaded['data']})
            assert not loaded.get('nextCursor'), 'owned_loaded_fixture_exceeded_page'
            if thread not in loaded['data']:
                continue
            current = wire.call('thread/read', {'threadId':thread,'includeTurns':False})['thread']
            if current['modelProvider']=='chatgpt_desktop_trial':
                assert socket.parent.parent.name=='catalogs', 'frontend_did_not_bind_resolved_catalog'
                models = json.loads(socket.with_name('models.json').read_text())['models']
                assert any(m['slug']=='chatgpt-desktop/gpt-6' for m in models), 'backend_chat_catalog_missing'
            if check_defaults:
                control = wire.call('thread/start', {'model':'gpt-6.1-sol','cwd':str(home.parent/'w'),
                    'ephemeral':True,'approvalPolicy':'never','sandbox':'danger-full-access'})
                assert control['thread']['modelProvider']=='openai', 'server_defaults_changed'
            found.append(current)
        finally:
            wire.close()
    if len(found)!=1:print(json.dumps({'ownedBackendObservation':observed,'expectedThread':thread,'matches':len(found)}))
    assert len(found)==1, 'official_backend_owner_missing_or_ambiguous'
    return found[0]


def probe(core, frontend, named_permission=False, chat_profile=False, manager=None):
    runtime=json.loads((Path.home()/'.local/share/codex-web-experiment/native-provider/runtime.json').read_text())
    adapter_proof=Path(runtime['binding']+'.proof')
    before_requests={k:json.loads(adapter_proof.read_text())[k] for k in ['requests','arrivals']}
    with tempfile.TemporaryDirectory(prefix='mm-', dir=os.environ['TMPDIR']) as tmp:
        root = Path(tmp)
        home, prefix, env = fixture(root, core, frontend, named_permission, manager)
        preserved = {p: sha(p) for p in (home / '.codex').glob('*.toml')}
        preserved_values = {p:tomllib.loads(p.read_text()) for p in preserved}
        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 40, 130, 0, 0))
        def session():
            os.setsid()
            fcntl.ioctl(slave, termios.TIOCSCTTY, 0)
        launch = ['--profile', 'chatgpt'] if chat_profile else ['--profile', 'owned-config'] if named_permission else []
        process = subprocess.Popen([str(prefix / 'bin/codex'), *launch, '--no-alt-screen'],
            env=env, cwd=root / 'w', stdin=slave, stdout=slave, stderr=slave, preexec_fn=session)
        os.close(slave)
        observer=None
        def collect(seconds):
            output = b''
            deadline = time.monotonic() + seconds
            while time.monotonic() < deadline:
                if select.select([master], [], [], .1)[0]:
                    try:
                        chunk = os.read(master, 65536)
                    except OSError:
                        break
                    if not chunk:
                        break
                    for query, response in [(b'\x1b[6n', b'\x1b[1;1R'), (b'\x1b[c', b'\x1b[?1;2c'),
                        (b'\x1b]10;?', b'\x1b]10;rgb:ffff/ffff/ffff\x1b\\'), (b'\x1b]11;?', b'\x1b]11;rgb:0000/0000/0000\x1b\\')]:
                        if query in chunk:
                            os.write(master, response)
                    output = (output + chunk)[-131072:]
                elif process.poll() is not None:
                    break
            return re.sub(r'\x1b\[[0-?]*[ -/]*[@-~]', '', output.decode(errors='replace'))
        def model_menu():
            for byte in b'/model':
                os.write(master, bytes([byte]));time.sleep(.07)
            os.write(master, b'\r')
            return collect(3)+repaint()
        def repaint():
            # Ratatui emits only changed cells. Resize the owned PTY to obtain
            # complete current labels instead of parsing an incomplete diff.
            fcntl.ioctl(master,termios.TIOCSWINSZ,struct.pack('HHHH',40,131,0,0));out=collect(.3)
            fcntl.ioctl(master,termios.TIOCSWINSZ,struct.pack('HHHH',40,130,0,0));return out+collect(.3)
        def choose(menu, pattern):
            matches = re.findall(pattern, menu)
            assert matches, 'model_choice_index_absent'
            index = int(matches[-1])
            assert 1 <= index <= 9, 'model_choice_index_unbounded'
            os.write(master, b'\x1b[H' + b'\x1b[B' * (index - 1) + b'\r')
            return collect(2)+repaint()
        try:
            initial = collect(8)
            if 'trust' in initial.lower():
                os.write(master, b'\r');initial += collect(4)
            # Empty0.163 threads have no rollout yet. Seed only owned local
            # history so a provider round trip can resume without inference.
            loaded=[]
            for socket in (p for family in ['servers','catalogs'] for p in (home/'.local/share/codex/core'/family).glob('*/s')):
                wire=Ws(socket,uri_path='/rpc')
                try:
                    ids=wire.call('thread/loaded/list',{'limit':100})['data']
                    for thread_id in ids:
                        wire.call('thread/inject_items',{'threadId':thread_id,'items':[{'type':'message','role':'user',
                            'content':[{'type':'input_text','text':'Owned model selection marker. Do not execute.'}]}]})
                    loaded.extend(ids)
                finally:wire.close()
            assert len(loaded)==1,'primary_thread_not_unique'
            before = model_menu()
            if not ('ChatGPT' in before and 'GPT-6' in before):
                print(json.dumps({'startupKeywords': {k: (initial+before).lower().count(k) for k in ['error','trust','login','sign in','select model','disabled until startup','unavailable','permission','profile','failed','unexpected argument','invalid','manifest','signature','current generation','cannot link']}, 'running':process.poll() is None, 'exit':process.poll(), 'bytes':len(initial+before), 'ownedStartupFailure': (initial+before).replace(str(root), '<fixture>')[:2000] if process.poll() is not None else None, 'imageName':Path(f'/proc/{process.pid}/exe').resolve().name if process.poll() is None else None}))
                log = home / '.codex/log/codex-tui.log'
                if log.is_file():
                    print(json.dumps({'ownedCatalogDiagnostics':[line.split('DEBUG',1)[-1].strip() for line in log.read_text().splitlines() if 'termux_chat_models' in line][-10:]}))
                raise AssertionError('chat_choices_absent')
            reasoning = choose(before, r'(\d+)\.\s+ChatGPT\s*·\s*GPT-6(?!\s+Pro\b)')
            assert 'Instant' in reasoning, 'chat_reasoning_absent'
            os.write(master, b'\x1b[H\r');entered = collect(15)
            assert process.poll() is None, 'chat_reentry_failed'
            argv = Path(f'/proc/{process.pid}/cmdline').read_bytes().split(b'\0')
            if not chat_profile:
                assert b'chatgpt-desktop/gpt-6' in argv and b'model_provider="chatgpt_desktop_trial"' in argv, 'chat_reentry_failed'
                permission = b'default_permissions="owned"' if named_permission else b'sandbox_mode="danger-full-access"'
                assert permission in argv and b'approvals_reviewer="user"' in argv and b'approval_policy="never"' in argv, 'permission_selection_changed'
            assert Path(f'/proc/{process.pid}/exe').resolve().name == '2', 'qualified_frontend_lost'
            files = list((home / '.codex/sessions').rglob('*.jsonl'))
            assert len(files) == 1, 'same_thread_not_preserved'
            thread_file = files[0]
            thread = json.loads(thread_file.read_text().splitlines()[0])['payload']['id']
            actual_chat = backend_thread(home, thread, check_defaults=True)
            assert actual_chat['modelProvider'] == 'chatgpt_desktop_trial' and actual_chat['model'] == 'chatgpt-desktop/gpt-6', 'chat_backend_route_not_selected'
            assert actual_chat['reasoningEffort'] == 'none', 'chat_backend_effort_not_selected'
            initial_argv=Path(f'/proc/{process.pid}/cmdline').read_bytes()
            trace=root/'same-chat-exec.trace'
            observer=subprocess.Popen([shutil.which('strace'),'-f','-p',str(process.pid),'-e','trace=execve,execveat','-o',str(trace)],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
            for _ in range(50):
                if 'TracerPid:\t'+str(observer.pid) in Path(f'/proc/{process.pid}/status').read_text():break
                collect(.1)
            assert 'TracerPid:\t'+str(observer.pid) in Path(f'/proc/{process.pid}/status').read_text(),'same_chat_observer_not_attached'
            same_chat=[]
            for model,pattern,effort,index in [
                ('chatgpt-desktop/gpt-6',r'(\d+)\.\s*ChatGPT\s*·\s*GPT-6(?!\s+Pro\b)','none',0),
                ('chatgpt-desktop/gpt-6',r'(\d+)\.\s*ChatGPT\s*·\s*GPT-6(?!\s+Pro\b)','high',2),
                ('chatgpt-desktop/gpt-5.6-sol',r'(\d+)\.\s*ChatGPT\s*·\s*GPT-5\.6\s*Sol(?!\s+Pro\b)','none',0),
                ('chatgpt-desktop/gpt-6',r'(\d+)\.\s*ChatGPT\s*·\s*GPT-6(?!\s+Pro\b)','none',0),
            ]:
                menu=model_menu();reason=choose(menu,pattern)
                assert 'Instant' in reason,'same_chat_reasoning_absent'
                os.write(master,b'\x1b[H'+b'\x1b[B'*index+b'\r');collect(2)
                current=backend_thread(home,thread)
                assert current['model']==model and current['reasoningEffort']==effort,'same_chat_setting_not_applied'
                assert Path(f'/proc/{process.pid}/cmdline').read_bytes()==initial_argv,'same_chat_frontend_reentered'
                assert not re.search(r'\bexecve(?:at)?\(',trace.read_text()),'same_chat_frontend_exec_observed'
                assert list((home/'.codex/sessions').rglob('*.jsonl'))==[thread_file],'same_chat_thread_changed'
                same_chat.append({'model':model,'effort':effort,'sameProcessAndThread':True,'frontendExecs':0})
            observer.terminate();observer.wait(timeout=5);observer=None
            returning = model_menu()
            # Cursor-positioned terminal updates may omit literal spaces.
            assert re.search(r'\d+\.\s*Owned\s*Codex', returning), 'native_return_choice_absent'
            native_reasoning = choose(returning, r'(\d+)\.\s*Owned\s*Codex')
            assert re.search(r'\b(?:Instant|None)\b',native_reasoning), 'native_reasoning_absent'
            os.write(master, b'\x1b[H\r');returned = collect(15)
            assert process.poll() is None, 'native_reentry_failed'
            assert b'gpt-6.1-sol' in Path(f'/proc/{process.pid}/cmdline').read_bytes().split(b'\0'), 'native_reentry_failed'
            assert list((home / '.codex/sessions').rglob('*.jsonl')) == [thread_file], 'return_thread_changed'
            actual_native = backend_thread(home, thread)
            assert actual_native['modelProvider'] == 'openai' and actual_native['model'] == 'gpt-6.1-sol', 'native_backend_route_not_restored'
            state = dict(part.split(b'=', 1) for part in Path(f'/proc/{process.pid}/environ').read_bytes().split(b'\0') if b'=' in part)
            assert state[b'CODEX_HOME'] == os.fsencode(home / '.codex')
            assert b'CODEX_CHAT_RETURN_PROFILE' not in state
            def changed_keys(a,b,prefix=''):
                result=[]
                for key in a.keys()|b.keys():
                    path=prefix+key
                    if isinstance(b.get(key),dict) and (isinstance(a.get(key),dict) or key not in a):result+=changed_keys(a.get(key,{}),b[key],path+'.')
                    elif a.get(key)!=b.get(key):result.append(path)
                return result
            modified={p.name:changed_keys(preserved_values[p],tomllib.loads(p.read_text())) for p,digest in preserved.items() if sha(p)!=digest}
            if modified:print(json.dumps({'ownedConfigurationChanges':modified}))
            assert not modified, 'configuration_files_changed'
            assert (home/'.codex/chatgpt.config.toml').is_symlink(), 'shared_registration_not_projected'
            assert {k:json.loads(adapter_proof.read_text())[k] for k in before_requests}==before_requests,'provider_inference_unexpected'
            return {'samePidAndPty': True, 'chatEntered': True, 'nativeReturned': True,
                'sameHomeAndThread': True, 'qualifiedFrontendRetained': True,'officialBackendRetained':True,'backendProviderRoundTrip':True,'backendChatCatalogBound':True,'otherThreadKeepsServerDefaults':True,
                'sharedRegistrationUsed': (home/'.codex/chatgpt.config.toml').is_symlink(),
                'originalConfigurationPreserved': True, 'namedPermissionRequested': named_permission,
                'chatProfileRequested':chat_profile,
                'sameChatSessionUpdates':same_chat,
                'noAuthCopied': True, 'modelInferenceSubmitted': False, 'proInference': False, 'productionChanged': False}
        finally:
            if observer and observer.poll() is None:
                observer.terminate();observer.wait(timeout=5)
            if process.poll() is None:
                os.killpg(process.pid, signal.SIGINT)
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    os.killpg(process.pid, signal.SIGTERM);process.wait(timeout=5)
            # The official fixture backend has its own session. Retire only
            # processes proven to execute this disposable generation's runtime.
            expected_runtime = home / '.local/lib/codex/core/generations/local-permission-fixture/runtime'
            for record in (p for family in ['servers', 'catalogs']
                for p in (home / '.local/share/codex/core' / family).glob('*/pid')):
                try:
                    pid = int(record.read_text().strip())
                    if Path(f'/proc/{pid}/exe').resolve() == expected_runtime:
                        os.kill(pid, signal.SIGTERM)
                        deadline = time.monotonic() + 5
                        while time.monotonic() < deadline and Path(f'/proc/{pid}/exe').exists():
                            time.sleep(.05)
                except (OSError, ValueError):
                    pass
            os.close(master)


if __name__ == '__main__':
    os.umask(0o077)
    parser = argparse.ArgumentParser()
    parser.add_argument('--core', type=Path, required=True)
    parser.add_argument('--frontend', type=Path, required=True)
    parser.add_argument('--report', type=Path, required=True)
    parser.add_argument('--manager', type=Path)
    parser.add_argument('--named-permission', action='store_true')
    parser.add_argument('--chat-profile', action='store_true')
    args = parser.parse_args()
    try:
        assert not (args.chat_profile and args.named_permission), 'choose_one_profile_fixture'
        report = probe(args.core, args.frontend, args.named_permission, args.chat_profile, args.manager)
    except Exception as error:
        report = {'accepted': False, 'failureClass': type(error).__name__,
            'failureCode': str(error) if re.fullmatch('[a-z_]+', str(error)) else 'unclassified'}
    else:
        report['accepted'] = True
    from linux_desktop_trial import write
    write(args.report, report)
    print(json.dumps(report))
    raise SystemExit(0 if report['accepted'] else 1)
