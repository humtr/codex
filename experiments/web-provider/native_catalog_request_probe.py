"""Capture real official-backend request metadata in disposable Android state.

Both endpoints are deterministic local fixtures. No Chat or API inference, auth
copy, production state change, or private conversation content is involved.
"""
import argparse
import http.server
import json
import os
from pathlib import Path
import signal
import shlex
import subprocess
import sys
import tempfile
import threading
import time


def probe(source, runtime, bun, bridge, catalog, core=None, frontend=None, manager=None):
    sys.path.insert(0, str(source / 'scripts'))
    from qualify_tasks import Ws
    native = json.loads(Path(__file__).with_name('native_catalog_fixture.json').read_text())
    chat = next(m for m in json.loads(catalog.read_text())['models'] if m['slug']=='chatgpt-desktop/gpt-6')
    captured = []
    interrupt_fixture = threading.Event()
    stream_started = threading.Event()
    stream_closed = threading.Event()
    paths = []
    identities = []
    tool_command = None

    class Model(http.server.BaseHTTPRequestHandler):
        def log_message(self, *_):
            pass

        def do_POST(self):
            paths.append(self.path)
            raw = self.rfile.read(int(self.headers['Content-Length']))
            body = json.loads(raw)
            metadata = body.get('client_metadata', {}).get('x-codex-turn-metadata', {})
            if isinstance(metadata, str):
                metadata = json.loads(metadata)
            identities.append((metadata.get('thread_id'), metadata.get('turn_id')))
            captured.append({'model': body.get('model'), 'bytes': len(raw), 'serializedCharacters': len(json.dumps(body)),
                             'keys': sorted(body), 'inputTypes': sorted({str(i.get('type')) for i in body.get('input', [])}),
                             'toolTypes': sorted({str(t.get('type')) for t in body.get('tools', [])})})
            captured[-1]['nativeTurnIdentityPresent'] = bool(metadata.get('thread_id') and metadata.get('turn_id'))
            captured[-1]['classifiedEnvironmentParts'] = sum(
                kind=='environments.environment_context' for item in body.get('input', [])
                for kind in (item.get('internal_chat_message_metadata_passthrough') or {}).get('content_item_kinds', []))
            captured[-1]['userInputShapes'] = [
                {'parts':len(item.get('content',[])),
                 'environmentParts':sum(part.get('type')=='input_text' and part.get('text','').lstrip().startswith('<environment_context>')
                     for part in item.get('content',[]) if isinstance(part,dict)),
                 'metadataKeys':sorted((item.get('internal_chat_message_metadata_passthrough') or {}).keys())}
                for item in body.get('input',[]) if item.get('role')=='user' and isinstance(item.get('content'),list)]
            protocol = Path(__file__).with_name('desktop_protocol.mjs')
            code = 'const {parseRequest}=await import('+json.dumps(str(bridge))+');const {admitDesktopRequest}=await import('+json.dumps(str(protocol))+');const raw=JSON.parse(await Bun.stdin.text());try{const {parsed}=admitDesktopRequest(raw,parseRequest);console.log(JSON.stringify({status:"admitted",tools:parsed.context.tools.length}));}catch(e){console.log(JSON.stringify({status:e.message}));}'
            admission = subprocess.run([str(bun), '--eval', code], input=json.dumps(body),
                                       text=True, capture_output=True, check=True)
            admitted = json.loads(admission.stdout)
            captured[-1]['adapterAdmission'] = admitted['status']
            captured[-1]['adapterToolCount'] = admitted.get('tools')
            item = {'id': 'msg_owned', 'type': 'message', 'role': 'assistant',
                    'content': [{'type': 'output_text', 'text': 'OWNED_OK', 'annotations': []}]}
            if tool_command:
                item = {'id': 'fc_owned_wait', 'type': 'function_call', 'call_id': 'call_owned_wait',
                    'name': 'exec_command', 'namespace': 'functions',
                    'arguments': json.dumps({'cmd':tool_command, 'yield_time_ms':10000, 'max_output_tokens':100})}
            response = {'id': 'resp_owned', 'object': 'response', 'model': body['model'],
                        'status': 'completed', 'output': [item],
                        'usage': {'input_tokens': 10, 'output_tokens': 10, 'total_tokens': 20}}
            self.send_response(200)
            self.send_header('Content-Type', 'text/event-stream')
            self.end_headers()
            events = [('response.created', {'response': dict(response, status='in_progress', output=[])}),
                      ('response.output_item.added', {'output_index': 0, 'item': item}),
                      ('response.output_text.delta', {'item_id': item['id'], 'output_index': 0,
                                                      'content_index': 0, 'delta': 'OWNED_OK'}),
                      ('response.output_item.done', {'output_index': 0, 'item': item}),
                      ('response.completed', {'response': response})]
            if tool_command:
                events.pop(2) # A tool call has arguments, not an assistant text delta.
            for name, data in events[:3] if interrupt_fixture.is_set() else events:
                self.wfile.write(('event: ' + name + '\ndata: ' + json.dumps(dict(data, type=name)) + '\n\n').encode())
            self.wfile.flush()
            if interrupt_fixture.is_set():
                stream_started.set()
                deadline = time.monotonic()+10
                while time.monotonic()<deadline:
                    try:
                        self.wfile.write(b': owned fixture heartbeat\n\n');self.wfile.flush()
                    except (BrokenPipeError, ConnectionResetError):
                        stream_closed.set();break
                    time.sleep(.05)

    model = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Model)
    threading.Thread(target=model.serve_forever, daemon=True).start()
    report = {}
    try:
        with tempfile.TemporaryDirectory(prefix='c-', dir=os.environ['TMPDIR']) as tmp:
            root = Path(tmp)
            home, config, work = root/'h', root/'cfg', root/'w'
            core_env = None
            if core:
                from native_model_menu_probe import fixture
                home, prefix, core_env = fixture(root, core, frontend, manager=manager)
            for path in [home, config, work]:
                path.mkdir(mode=0o700, exist_ok=True)
            if core:
                home = home/'.codex'
            native_path, chat_path, merged_path = root/'native.json', root/'chat.json', root/'merged.json'
            native_path.write_text(json.dumps({'models': [native]}))
            chat_path.write_text(json.dumps({'models': [chat]}))
            merged_path.write_text(json.dumps({'models': [native, chat]}))
            (home/'config.toml').write_text('model = "gpt-6.1-sol"\nmodel_catalog_json = ' + json.dumps(str(native_path)) + '\ncheck_for_update_on_startup = false\n[features]\nmemories = false\nplugins = false\n')
            endpoint = f'http://127.0.0.1:{model.server_port}/v1'
            env = {k: v for k, v in os.environ.items() if not k.startswith(('CODEX_', 'OPENAI_', 'LD_'))}
            env.update(HOME=str(root), CODEX_HOME=str(home), OPENAI_BASE_URL=endpoint,
                       OPENAI_API_KEY='owned-inert-fixture', RUST_LOG='warn')
            descriptors = [os.open(p, os.O_RDONLY) for p in [Path(os.environ['PREFIX'])/'etc/resolv.conf', config, Path(os.environ['TMPDIR'])]]

            def fds():
                for fd, target in zip(descriptors, [33, 34, 35]):
                    os.dup2(fd, target, inheritable=True)

            modes = ['thread-override', 'startup-catalog', 'startup-merged-catalog']
            if core:
                modes.append('core-resolved-catalog')
                core_env['OPENAI_BASE_URL'] = endpoint
            for index, mode in enumerate(modes):
                interrupt_fixture.clear()
                stream_started.clear()
                stream_closed.clear()
                socket = root/('s'+str(index))
                argv = [str(runtime)]
                if mode == 'startup-catalog':
                    argv += ['-c', 'model_catalog_json='+json.dumps(str(chat_path))]
                elif mode == 'startup-merged-catalog':
                    argv += ['-c', 'model_catalog_json='+json.dumps(str(merged_path))]
                argv += ['app-server', '--listen', 'unix://'+str(socket)]
                owned_servers = []

                def core_catalog(value):
                    result = subprocess.run([str(prefix/'bin/codex'), '__app-server-catalog-v1'],
                        input=json.dumps(value).encode(), env=core_env, cwd=work,
                        capture_output=True, timeout=25)
                    assert result.returncode == 0, 'core_catalog_start_failed: '+result.stderr.decode()[-1500:]
                    prepared = Path(result.stdout.decode().strip())
                    assert prepared.is_absolute() and len(result.stdout)<=108, 'invalid_core_socket'
                    pid = int(prepared.with_name('pid').read_text())
                    expected = root/'h/.local/lib/codex/core/generations/local-chat-model-fixture/runtime'
                    assert Path(f'/proc/{pid}/exe').resolve() == expected
                    owned_servers.append((pid, expected))
                    return prepared, pid

                with (root/(mode+'.log')).open('w+') as log:
                    child = None
                    if mode == 'core-resolved-catalog':
                        socket, server_pid = core_catalog({'models': [chat]})
                        reused, reused_pid = core_catalog({'models': [chat]})
                        assert reused == socket and reused_pid == server_pid, 'resolved_catalog_not_reused'
                    else:
                        child = subprocess.Popen(argv, env=env, cwd=work, stdout=log, stderr=log,
                                                 close_fds=False, preexec_fn=fds, start_new_session=True)
                    wire = None
                    try:
                        deadline = time.monotonic()+20
                        while not socket.exists():
                            if child and child.poll() is not None:
                                log.flush();log.seek(0)
                                raise AssertionError('official_backend_start_failed: '+log.read()[-1500:])
                            assert time.monotonic()<deadline, 'official_backend_start_timeout'
                            time.sleep(.05)
                        wire = Ws(str(socket), uri_path='/rpc')
                        result = wire.call('thread/start', {'model': chat['slug'], 'modelProvider': 'owned',
                            'cwd': str(work), 'ephemeral': mode not in ['startup-merged-catalog', 'core-resolved-catalog'], 'approvalPolicy': 'never', 'sandbox': 'danger-full-access',
                            'config': {'model_catalog_json': str(chat_path), 'model_reasoning_effort': 'none', 'model_providers.owned': {
                                'name': 'Owned fixture', 'base_url': endpoint, 'wire_api': 'responses', 'requires_openai_auth': False}}})
                        thread = result['thread']['id']
                        initial_turn = wire.call('turn/start', {'threadId': thread, 'input': [{'type': 'text', 'text': 'Reply OWNED_OK.', 'text_elements': []}]})
                        while True:
                            event = wire.receive()
                            if event.get('method') == 'turn/completed':
                                assert event['params']['turn']['status']=='completed', 'owned_turn_failed'
                                break
                        log.flush();log.seek(0)
                        report[mode] = dict(captured[-1], fallbackMetadata=('Unknown model '+chat['slug']) in log.read())
                        if mode == 'thread-override':
                            capabilities = {}
                            for name, method, params in [
                                ('turnsList', 'thread/turns/list', {'threadId':thread,'limit':5}),
                                ('fullRead', 'thread/read', {'threadId':thread,'includeTurns':True})]:
                                try:
                                    value = wire.call(method, params)
                                    turns = value.get('data', value.get('thread', {}).get('turns', []))
                                    capabilities[name] = any(t['id']==initial_turn['turn']['id'] and t['status']=='completed' for t in turns)
                                except AssertionError as error:
                                    assert isinstance(error.args[0], tuple), 'unexpected_owned_rpc_failure'
                                    capabilities[name] = False
                            report['ephemeral-observer'] = capabilities
                        if mode == 'core-resolved-catalog':
                            report[mode]['fallbackMetadata'] = None # Core owns stderr; request shape is the proof here.
                            changed = dict(chat, display_name='Owned catalog revision')
                            other, other_pid = core_catalog({'models': [changed]})
                            assert other != socket and other_pid != server_pid, 'changed_catalog_reused_stale_server'
                            assert wire.call('thread/read', {'threadId': thread})['thread']['id']==thread
                            status = subprocess.run([str(prefix/'bin/codex'), 'termux', 'task', 'status'],
                                env=core_env, cwd=work, capture_output=True, timeout=20)
                            assert status.returncode==0, 'manager_catalog_discovery_failed: '+status.stderr.decode()[-1000:]
                            assert thread in status.stdout.decode(), 'catalog_writer_not_discovered'
                            snapshot = socket.with_name('models.json')
                            saved = snapshot.read_bytes()
                            try:
                                snapshot.write_bytes(b'{}')
                                rejected = subprocess.run([str(prefix/'bin/codex'), 'termux', 'task', 'status'],
                                    env=core_env, cwd=work, capture_output=True, timeout=20)
                                assert rejected.returncode!=0, 'manager_accepted_changed_catalog_identity'
                            finally:
                                snapshot.write_bytes(saved)
                            empty = subprocess.run([str(prefix/'bin/codex'), '__app-server-catalog-v1'],
                                input=b'', env=core_env, cwd=work, capture_output=True, timeout=25)
                            assert empty.returncode!=0, 'empty_catalog_accepted'
                            assert wire.call('thread/read', {'threadId': thread})['thread']['id']==thread
                            report[mode].update(sameCatalogReused=True, changedCatalogNewServer=True,
                                previousThreadReadable=True, managerDiscoversWriter=True,
                                changedCatalogIdentityRejected=True, emptyCatalogRejected=True, chatSubmissions=0)
                        if mode == 'startup-merged-catalog':
                            before = len(paths)
                            interrupt_fixture.set()
                            started = wire.call('turn/start', {'threadId': thread, 'input': [
                                {'type': 'text', 'text': 'Owned interrupt fixture.', 'text_elements': []}]})
                            assert stream_started.wait(5), 'provider_stream_not_started'
                            wire.call('turn/interrupt', {'threadId': thread, 'turnId': started['turn']['id']})
                            while True:
                                event = wire.receive()
                                if event.get('method') == 'turn/completed':
                                    outcome = event['params']['turn']['status'];break
                            assert outcome == 'interrupted'
                            closed = stream_closed.wait(3)
                            report['interrupt-wire'] = {'turnStatus': outcome, 'providerStreamClosed': closed,
                                'httpPaths': paths[before:], 'chatSubmissions': 0}
                            interrupt_fixture.clear()
                            marker = root/'owned-tool-pid'
                            tool_command = 'exec '+shlex.quote(sys.executable)+' -c '+shlex.quote(
                                'import os,time;from pathlib import Path;Path('+repr(str(marker))+').write_text(str(os.getpid()));time.sleep(60)')
                            before = len(paths)
                            observer = None
                            tool_identity = None
                            try:
                                started = wire.call('turn/start', {'threadId': thread, 'input': [
                                    {'type':'text','text':'Run the owned cancellable local fixture.','text_elements':[]}]})
                                deadline = time.monotonic()+8
                                while not marker.exists():
                                    assert time.monotonic()<deadline, 'owned_native_tool_not_started'
                                    time.sleep(.05)
                                pid = int(marker.read_text())
                                def process_identity(pid):
                                    stat = Path(f'/proc/{pid}/stat').read_text().rsplit(')',1)[1].split()
                                    return stat[19], Path(f'/proc/{pid}/exe').resolve()
                                tool_identity = (pid, process_identity(pid))
                                assert identities[-1]==(thread,started['turn']['id']), 'native_wire_turn_identity_mismatch'
                                observer = Ws(str(socket), uri_path='/rpc')
                                def observe_turn():
                                    page = observer.call('thread/turns/list', {'threadId':thread,'limit':5})
                                    return next((row for row in page['data'] if row['id']==started['turn']['id']), None)
                                active = observe_turn()
                                assert active and active['status']=='inProgress', 'read_only_observer_missed_active_turn'
                                wire.call('turn/interrupt', {'threadId':thread,'turnId':started['turn']['id']})
                                while True:
                                    event = wire.receive()
                                    if event.get('method')=='turn/completed':
                                        assert event['params']['turn']['status']=='interrupted'
                                        break
                                stopped = observe_turn()
                                assert stopped and stopped['status']=='interrupted', 'read_only_observer_missed_interrupt'
                                try:
                                    process_survives = process_identity(pid)==tool_identity[1]
                                except FileNotFoundError:
                                    process_survives = False
                                report['interrupt-during-tool'] = {'readOnlyObserver':True,
                                    'sameTurnBeforeAndAfter':True,'statusBefore':active['status'],
                                    'statusAfter':stopped['status'],'nativeTurnIdentityPresent':captured[-1]['nativeTurnIdentityPresent'],
                                    'nativeTurnIdentityMatches':True,
                                    'toolProcessSurvivesInterrupt':process_survives,
                                    'httpPaths':paths[before:],'chatSubmissions':0}
                            finally:
                                tool_command = None
                                if observer:
                                    observer.close()
                                if tool_identity:
                                    pid, identity = tool_identity
                                    try:
                                        if process_identity(pid)==identity:
                                            os.kill(pid, signal.SIGTERM)
                                    except FileNotFoundError:
                                        pass
                    finally:
                        if wire:
                            wire.close()
                        if child:
                            if child.poll() is None:
                                os.killpg(child.pid, signal.SIGTERM)
                            child.wait(timeout=10)
                        for pid, expected in set(owned_servers):
                            if Path(f'/proc/{pid}/exe').resolve() == expected:
                                os.kill(pid, signal.SIGTERM)
                        deadline = time.monotonic()+10
                        for pid, expected in set(owned_servers):
                            while Path(f'/proc/{pid}/exe').resolve()==expected:
                                assert time.monotonic()<deadline, 'owned_core_server_exit_timeout'
                                time.sleep(.05)
            for fd in descriptors:
                os.close(fd)
    finally:
        model.shutdown();model.server_close()
    assert report['thread-override']['fallbackMetadata']
    assert not report['startup-catalog']['fallbackMetadata']
    assert report['thread-override']['adapterAdmission']=='desktop_tool_declaration_unsupported'
    assert report['startup-catalog']['adapterAdmission']=='admitted'
    assert not report['startup-merged-catalog']['fallbackMetadata']
    assert report['startup-merged-catalog']['adapterAdmission']=='admitted'
    assert report['startup-merged-catalog']['toolTypes'] == report['startup-catalog']['toolTypes']
    assert report['startup-merged-catalog']['adapterToolCount'] == report['startup-catalog']['adapterToolCount'] > 0
    assert report['interrupt-wire']['providerStreamClosed']
    assert report['interrupt-wire']['httpPaths'] == ['/v1/responses']
    if core:
        assert report['core-resolved-catalog']['adapterAdmission']=='admitted'
        assert report['core-resolved-catalog']['adapterToolCount']==report['startup-catalog']['adapterToolCount']
    return report


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--source', type=Path, required=True)
    parser.add_argument('--runtime', type=Path, required=True)
    parser.add_argument('--report', type=Path, required=True)
    parser.add_argument('--bun', type=Path, required=True)
    parser.add_argument('--bridge', type=Path, required=True)
    parser.add_argument('--catalog', type=Path, required=True)
    parser.add_argument('--core', type=Path)
    parser.add_argument('--frontend', type=Path)
    parser.add_argument('--manager', type=Path)
    args = parser.parse_args()
    assert bool(args.core)==bool(args.frontend)==bool(args.manager), 'supply the complete disposable candidate trio'
    result = probe(args.source, args.runtime, args.bun, args.bridge, args.catalog, args.core, args.frontend, args.manager)
    args.report.write_text(json.dumps(result, indent=2)+'\n')
    args.report.chmod(0o600)
    print(json.dumps(result))
