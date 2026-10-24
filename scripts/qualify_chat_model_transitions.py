"""Actual Android Core/TUI provider transitions; owned synthetic Responses only.

This is terminal integration proof, not Chat inference or stable artifact admission.
The Provider binary supplies the real readable-history checkpoint operation.
"""
import argparse
import fcntl
import hashlib
import http.server
import json
import os
from pathlib import Path
import pty
import re
import select
import shlex
import shutil
import signal
import struct
import subprocess
import tempfile
import threading
import time
import urllib.request
import urllib.parse
import termios

from qualify_permission_lifecycle import fixture, rpc, server_paths
from qualify_provider_chat_entry import check_native_socket_budget
from qualify_provider_d3 import stop_owned_pid
from clean_development import fixture_cleanup


def probe(args):
    root = Path(tempfile.mkdtemp(prefix='ct.', dir=os.environ['TMPDIR']))
    process = master = server = queue_service = None
    accepted = False
    requests, failures = [], []
    screen = ''
    reentry_observations = []
    home = None
    report = None
    try:
        check_native_socket_budget(root / 'h')
        home, prefix, env = fixture(root, args.core, args.frontend)
        if args.shared_sessions:
            subprocess.run([str(prefix / 'bin/codex'), 'termux', 'profile', 'create', 'fixture'],
                           env=env, cwd=root / 'w', check=True, capture_output=True)
        if args.without_manager:
            # Verified generated fixture copy only; never the installed Manager.
            manager=home/'.local/lib/codex/core/generations/local-permission-fixture/manager'
            assert manager.is_file() and not manager.is_symlink()
            manager.unlink()
        env['XDG_CONFIG_HOME'] = str(root / 'config')
        for name in ['native-models.json', 'models_cache.json']:
            path = home / '.codex' / name
            catalog = json.loads(path.read_text())
            catalog['models'][0].update(default_reasoning_level='none',
                supported_reasoning_levels=[{'effort': 'none', 'description': 'Fixture'}])
            path.write_text(json.dumps(catalog))

        class Backend(http.server.BaseHTTPRequestHandler):
            def log_message(self, *_):
                pass

            def do_POST(self):
                try:
                    assert self.path.endswith('/responses'), self.path
                    payload = self.rfile.read(int(self.headers['Content-Length']))
                    if self.headers.get('Content-Encoding') == 'zstd':
                        payload = subprocess.run(['zstd', '-d', '-q', '-c'], input=payload,
                                                 capture_output=True, check=True).stdout
                    body = json.loads(payload)
                    requests.append(body)
                    text = 'OWNED_PUBLIC_WORK_STATE_7K2Q'
                    item = {'id': 'fixture-' + str(len(requests)), 'type': 'message',
                            'role': 'assistant', 'phase': 'final_answer',
                            'content': [{'type': 'output_text', 'text': text, 'annotations': []}]}
                    response = {'id': 'owned-' + str(len(requests)), 'object': 'response',
                                'model': body['model'], 'status': 'completed', 'output': [item],
                                'usage': {'input_tokens': 10, 'output_tokens': 10, 'total_tokens': 20}}
                    events = [
                        {'type': 'response.created', 'response': {**response, 'status': 'in_progress', 'output': []}},
                        {'type': 'response.output_item.added', 'output_index': 0, 'item': item},
                        {'type': 'response.output_text.delta', 'item_id': item['id'],
                         'output_index': 0, 'content_index': 0, 'delta': text},
                        {'type': 'response.output_item.done', 'output_index': 0, 'item': item},
                        {'type': 'response.completed', 'response': response}]
                    self.send_response(200)
                    self.send_header('Content-Type', 'text/event-stream')
                    self.end_headers()
                    for event in events:
                        self.wfile.write(('event: ' + event['type'] + '\ndata: ' +
                                          json.dumps(event) + '\n\n').encode())
                    self.wfile.flush()
                except Exception as error:
                    failures.append(type(error).__name__ + ': ' + str(error))
                    self.close_connection = True

        server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Backend)
        server.daemon_threads = True
        threading.Thread(target=server.serve_forever, daemon=True).start()
        address = f'http://127.0.0.1:{server.server_port}/v1'
        env['OPENAI_BASE_URL'] = address
        # A fixture-only auth command, not an alternate production Provider.
        auth = root / 'fixture-auth'
        auth.write_text('#!' + os.environ['PREFIX'] + '/bin/sh\n'
                        'case "$1" in\n'
                        'token) printf "owned-inert-fixture\\n";;\n'
                        'checkpoint) exec ' + shlex.quote(str(args.provider)) + ' "$@";;\n'
                        '*) exit 1;;\nesac\n')
        auth.chmod(0o700)
        catalog = json.loads(subprocess.check_output([str(args.provider), 'catalog'], env=env))
        model = catalog['models'][0]
        model.update(default_reasoning_level='none',
                     supported_reasoning_levels=[{'effort': 'none', 'description': 'Fixture'}])
        chat_catalog = root / 'chat-models.json'
        chat_catalog.write_text(json.dumps({'models': [model]}))
        chat_catalog.chmod(0o600)
        profile = home / '.codex/chatgpt.config.toml'
        profile.write_text('# codex-chatgpt-provider generated v1\n'
            'model="chatgpt-desktop/gpt-6"\nmodel_provider="chatgpt_desktop"\n'
            'model_catalog_json=' + json.dumps(str(chat_catalog)) + '\n'
            '[model_providers.chatgpt_desktop]\nname="Owned Chat fixture"\n'
            'base_url=' + json.dumps(address) + '\nwire_api="responses"\n'
            'requires_openai_auth=false\nsupports_websockets=false\n'
            'request_max_retries=0\nstream_max_retries=0\n'
            '[model_providers.chatgpt_desktop.auth]\ncommand=' + json.dumps(str(auth)) + '\n'
            'args=["token","--state",' + json.dumps(str(root / 'state')) + ']\n')
        profile.chmod(0o600)
        config = home / '.codex/config.toml'
        config.write_text('openai_base_url=' + json.dumps(address) + '\n' +
                          config.read_text() + '\n[projects.' + json.dumps(str(root / 'w')) +
                          ']\ntrust_level="trusted"\n')
        if args.shared_sessions:
            # Let Core prepare the same supported shared topology as a Manager
            # account, rather than substituting the canonical history root.
            account = home / '.local/share/codex/manager/profiles/fixture/home'
            assert account.is_dir() and not account.is_symlink()
            for path in (home / '.codex').iterdir():
                assert path.is_file() and not path.is_symlink()
                shutil.copy2(path, account / path.name)
            env['CODEX_HOME'] = str(account)

        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 40, 160, 0, 0))
        launch = [str(prefix / 'bin/codex')]
        if args.shared_sessions and not args.without_manager:
            launch += ['termux', 'profile', 'use', 'fixture', '--']
        process = subprocess.Popen(launch + ['--no-alt-screen'], env=env,
            cwd=root / 'w', stdin=slave, stdout=slave, stderr=slave, start_new_session=True)
        os.close(slave)

        def drain(seconds=.15):
            nonlocal screen
            if master is None:
                return
            until = time.monotonic() + seconds
            while time.monotonic() < until:
                if select.select([master], [], [], .03)[0]:
                    try:
                        screen += os.read(master, 65536).decode(errors='replace')
                    except OSError:
                        break

        def wait(predicate, seconds=35):
            until = time.monotonic() + seconds
            while time.monotonic() < until:
                drain()
                assert not failures, failures
                try:
                    if predicate():
                        return
                except AssertionError as error:
                    # Provider re-entry temporarily retires/replaces its owned
                    # native backend. Never choose one while identity is ambiguous.
                    if str(error) == 'one owned thread server required':
                        continue
                    # A newly re-entered backend may expose thread metadata
                    # before its public turns materialize. Retry this read only;
                    # never resend a user turn or another model switch.
                    detail = error.args[0] if error.args else None
                    if (isinstance(detail, tuple) and len(detail) == 2
                            and detail[0] == 'thread/read' and isinstance(detail[1], dict)
                            and detail[1].get('code') == -32600
                            and 'is not materialized yet; includeTurns is unavailable before first user message' in detail[1].get('message', '')):
                        reentry_observations.append('public_turns_not_materialized')
                        continue
                    raise
            raise AssertionError('terminal transition timeout: ' +
                re.sub(r'\x1b\[[0-?]*[ -/]*[@-~]', '', screen[-3000:]))

        def loaded():
            return rpc(home, 'thread/loaded/list', {})['data'] if server_paths(home) else []

        def public(thread):
            return rpc(home, 'thread/read', {'threadId': thread, 'includeTurns': True})['thread']

        def settings(thread):
            return rpc(home, 'thread/resume', {'threadId': thread})

        def submit(text):
            # Backend idle can precede the TUI's turn-completed notification.
            drain(1)
            # One bracketed paste is an atomic composer event. Byte-by-byte
            # startup input can leave trailing command text after a slow frame,
            # correctly tripping the pending-input guard rather than testing reentry.
            os.write(master, b'\x1b[200~' + text.encode() + b'\x1b[201~')
            drain(1)
            os.write(master, b'\r')

        def choose_other_provider():
            nonlocal screen
            screen = ''
            # Whitespace closes slash-name completion before submitting the
            # command, so Enter cannot merely accept an autocomplete candidate.
            submit('/model ')
            wait(lambda: 'Select Model and Effort' in screen and
                 'ChatGPT · GPT-6' in screen and 'GPT-6.1-Sol' in screen)
            os.write(master, b'\x1b[F')
            drain(.3)
            os.write(master, b'\r')

        wait(lambda: len(loaded()) == 1)
        thread = loaded()[0]
        diagnostics=[]
        for proc in Path('/proc').glob('[0-9]*'):
            try:
                if Path(os.readlink(proc/'exe'))!=home/'.local/lib/codex/core/generations/local-permission-fixture/helpers/2':
                    continue
                values=dict(v.split(b'=',1) for v in (proc/'environ').read_bytes().split(b'\0') if b'=' in v)
                diagnostics.append({k:values.get(k.encode(),b'').decode(errors='replace')
                    for k in ['HOME','CODEX_HOME','CODEX_PROFILE_CORE','CODEX_TERMUX_BACKEND_STARTUP']})
            except (OSError,ValueError):pass
        (root/'safe-runtime.json').write_text(json.dumps(diagnostics,indent=2)+'\n')
        assert len(diagnostics)==1, 'the selected Core did not execute the qualified frontend; use --build-core or verify the artifact source'
        if args.empty_roundtrip:
            def empty_metadata(provider):
                snapshot = rpc(home, 'thread/read', {'threadId': thread, 'includeTurns': False})['thread']
                return snapshot['id'] == thread and snapshot['status']['type'] == 'idle' and snapshot['modelProvider'] == provider

            # A genuine fresh thread: do not name it or query full history first.
            # Those operations can materialize history and mask this regression.
            choose_other_provider()
            wait(lambda: empty_metadata('chatgpt_desktop'))
            assert not requests, 'empty native to Chat selection must not infer'
            choose_other_provider()
            wait(lambda: empty_metadata('openai'))
            assert not requests, 'empty Chat to native selection must not infer'
            # The unchanged backend still rejects full-history reads on an
            # untouched thread; verify its actual rollout, not that unavailable API.
            snapshot = rpc(home, 'thread/read', {'threadId': thread, 'includeTurns': False})['thread']
            assert snapshot['turns'] == [], 'empty selection must not add a public turn'
            rollout = Path(snapshot['path'])
            if args.shared_sessions:
                assert (account / 'sessions').is_symlink()
                assert (account / 'sessions').resolve() == home / '.codex/sessions'
                assert rollout.resolve().is_relative_to(home / '.codex/sessions')
            records = [json.loads(line) for line in rollout.read_text().splitlines() if line.strip()]
            assert records and records[0]['type'] == 'session_meta' and records[0]['payload']['id'] == thread
            settings = records[1:]
            assert all(row['type'] == 'event_msg' and row['payload']['type'] == 'thread_settings_applied'
                       and row['payload']['thread_id'] == thread for row in settings), 'empty selection must not add conversation or checkpoint records'
            assert [row['payload']['thread_settings']['model_provider_id'] for row in settings] == ['chatgpt_desktop', 'openai'], 'native audit must retain the actual provider roundtrip'
            accepted = True
            report = {'accepted': True, 'actualAndroidTerminal': True, 'sameThread': True,
                    'emptyRoundtrip': True, 'syntheticResponsesRequests': 0, 'chatSubmissions': 0,
                    'checkpointsInjected': 0, 'managerAbsent': args.without_manager,
                    'sharedSessionsDirectory': args.shared_sessions,
                    'nativeSettingsAuditEvents': len(settings),
                    'maximumNativeSocketBytes': check_native_socket_budget(home),
                    'input_sha256': {name: hashlib.file_digest(getattr(args, name).open('rb'), 'sha256').hexdigest()
                                     for name in ['core', 'frontend', 'provider']}}
            return report
        rpc(home, 'thread/name/set', {'threadId': thread, 'name': 'Owned transition fixture'})
        drain(.5)
        submit('Keep OWNED_PUBLIC_WORK_STATE_7K2Q while changing providers')
        wait(lambda: len(requests) == 1 and public(thread)['status']['type'] == 'idle')
        print('Initial native terminal turn settled', flush=True)
        before = public(thread)['turns']
        choose_other_provider()
        wait(lambda: public(thread)['modelProvider'] == 'chatgpt_desktop')
        print('Terminal entered Chat route', flush=True)
        assert public(thread)['turns'] == before, 'handoff must not create a public user turn'
        assert len(requests) == 1, 'native to Chat preparation must not infer'
        submit('Continue the owned Chat fixture')
        wait(lambda: len(requests) == 2 and public(thread)['status']['type'] == 'idle')
        print('Chat terminal turn settled', flush=True)
        cold_stops = []
        if args.warm_chat_resume:
            saved_chat = public(thread)
            submit('/new')
            wait(lambda: any(value != thread for value in loaded()))
            choose_other_provider()
            wait(lambda: any(public(value)['modelProvider'] == 'openai' for value in loaded() if value != thread))
            submit('/resume ' + thread)
            def resumed_target():
                args = Path(f'/proc/{process.pid}/cmdline').read_bytes().split(b'\0')
                targeted = any(args[index:index + 2] == [b'resume', thread.encode()]
                               for index in range(len(args) - 1))
                return targeted and thread in loaded() and public(thread)['modelProvider'] == 'chatgpt_desktop'
            wait(resumed_target)
            assert public(thread)['turns'] == saved_chat['turns'], 'warm resume changed prior turns'
            submit('OWNED_WARM_CROSS_PROVIDER_RESUME')
            wait(lambda: len(requests) == 3 and public(thread)['status']['type'] == 'idle')
            assert 'tools' not in requests[-1], 'warm saved Chat got native fallback metadata'
            assert public(thread)['reasoningEffort'] == saved_chat['reasoningEffort']
            print('Warm cross-provider saved-Chat resume and follow-up settled', flush=True)
        if args.cold_chat_resume:
            saved_turns = public(thread)['turns']
            runtime = home / '.local/lib/codex/core/generations/local-permission-fixture/runtime'
            for socket in server_paths(home):
                cold_stops.append(stop_owned_pid(int(socket.with_name('pid').read_text()), runtime))
            try:
                os.killpg(process.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass  # The owned frontend can exit after its backend closes.
            process.wait(timeout=8)
            os.close(master)
            master = None
            # Leave the account's ordinary native defaults and generated Provider
            # reference, but remove the explicit fixture catalog. The selected
            # saved Chat must supply its metadata before backend attachment.
            execution_config = Path(env['CODEX_HOME']) / 'config.toml'
            execution_config.write_text('\n'.join(line for line in execution_config.read_text().splitlines()
                                                 if not line.startswith('model_catalog_json')) + '\n')
            master, slave = pty.openpty()
            fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 40, 160, 0, 0))
            process = subprocess.Popen([str(prefix / 'bin/codex'), '--no-alt-screen', 'resume', thread],
                env=env, cwd=root / 'w', stdin=slave, stdout=slave, stderr=slave,
                start_new_session=True)
            os.close(slave)
            wait(lambda: thread in loaded() and public(thread)['status']['type'] == 'idle')
            assert public(thread)['model'] == 'chatgpt-desktop/gpt-6'
            assert public(thread)['modelProvider'] == 'chatgpt_desktop'
            assert public(thread)['reasoningEffort'] == 'none', 'account default replaced the saved effort'
            assert public(thread)['turns'] == saved_turns, 'cold resume changed canonical history'
            assert len(requests) == 2, 'catalog restoration must not infer'
            submit('Continue the owned cold Chat fixture after resume')
            wait(lambda: len(requests) == 3 and public(thread)['status']['type'] == 'idle')
            assert 'tools' not in requests[-1], 'saved Chat got non-Lite fallback model metadata'
            assert 'Continue the owned Chat fixture' in json.dumps(requests[-1]['input'])
            print('Cold saved-Chat resume and follow-up settled', flush=True)
        # The provider's own model appears first; the opposite provider is the
        # appended final row in this deliberately two-model fixture.
        before_return = public(thread)['turns']
        before_return_requests = len(requests)
        choose_other_provider()
        wait(lambda: public(thread)['modelProvider'] == 'openai')
        print('Terminal returned to native route', flush=True)
        assert len(requests) == before_return_requests, 'ordinary Chat to native selection must not request a source summary'
        assert public(thread)['turns'] == before_return, 'model selection must preserve the existing public turns'
        submit('Continue the owned native fixture')
        wait(lambda: len(requests) == before_return_requests + 1 and public(thread)['status']['type'] == 'idle')
        assert 'OWNED_PUBLIC_WORK_STATE_7K2Q' in json.dumps(requests[-1]['input'])
        assert 'Continue the owned Chat fixture' in json.dumps(requests[-1]['input']), 'native request lost the preceding Chat task'
        assert not any('ocx1:' in json.dumps(v) for v in requests[-1]['input'])
        final = settings(thread)
        assert final['sandbox']['type'] == 'dangerFullAccess'
        assert final['approvalPolicy'] == 'never' and final['approvalsReviewer'] == 'user'
        assert all(v['id'] == thread for v in [public(thread)])
        queue_proof = None
        if args.queue:
            (root / 'owned-fixture').write_text('owned queue qualification; no real account\n')
            subprocess.run([str(args.provider), 'config', 'set',
                'execution.mode', 'operator', 'execution.core', str(prefix / 'bin/codex'),
                'execution.home', env['CODEX_HOME'], 'execution.workspace', str(root / 'w')],
                env=env, cwd=root / 'w', capture_output=True, check=True)
            with (root / 'queue-provider.log').open('wb') as output:
                queue_service = subprocess.Popen([str(args.provider), 'serve', '--state',
                    str(root / 'provider-state')], env=env, cwd=root / 'w',
                    stdout=output, stderr=subprocess.STDOUT)
            status = None
            def ready():
                nonlocal status
                assert queue_service.poll() is None, (root / 'queue-provider.log').read_text()
                observed = subprocess.run([str(args.provider), 'status', '--state', str(root / 'provider-state')],
                    env=env, capture_output=True, text=True, timeout=6)
                status = json.loads(observed.stdout) if observed.returncode == 0 else None
                return status is not None
            wait(ready)
            port = urllib.parse.urlsplit(status['base_url']).port
            queue_token = subprocess.check_output([str(args.provider), 'token', '--state',
                str(root / 'provider-state')], env=env, text=True, timeout=6).strip()

            def enqueue(arguments):
                request = urllib.request.Request(f'http://127.0.0.1:{port}/mcp',
                    data=json.dumps({'jsonrpc': '2.0', 'id': 1, 'method': 'tools/call',
                        'params': {'name': 'codex_sessions', 'arguments': arguments}}).encode(),
                    headers={'Content-Type': 'application/json', 'Authorization': 'Bearer ' + queue_token})
                with urllib.request.urlopen(request, timeout=35) as response:
                    row = json.load(response)
                assert 'error' not in row, row
                return json.loads(row['result']['content'][0]['text'])

            queue_args = {'action': 'queue', 'thread_id': thread,
                          'text': 'OWNED_NATIVE_QUEUE_CONSUMPTION', 'call_id': 'owned-native-queue'}
            original = public(thread)
            receipt = enqueue(queue_args)
            assert receipt['accepted'] and receipt['delivery'] == 'queued', receipt
            count = len(original['turns'])
            wait(lambda: len(public(thread)['turns']) == count + 1 and public(thread)['status']['type'] == 'idle')
            received = public(thread)
            assert received['model'] == original['model'] == 'gpt-6.1-sol'
            assert received['modelProvider'] == 'openai' and received['reasoningEffort'] == original['reasoningEffort']
            assert received['turns'][:-1] == original['turns']
            item = next(item for item in received['turns'][-1]['items'] if item['type'] == 'userMessage')
            assert item['clientId'] == receipt['queued_submission']['clientUserMessageId']
            assert 'OWNED_NATIVE_QUEUE_CONSUMPTION' in json.dumps(item)
            assert enqueue(queue_args) == receipt
            assert enqueue(dict(queue_args, text='changed'))['error'] == 'operator_call_changed'

            # Inject loss of only the enqueue receipt into this disposable
            # journal. Core has already committed the receiving turn.
            key = hashlib.sha256(json.dumps(['native-queue-v1', thread, queue_args['call_id']],
                                           separators=(',', ':')).encode()).hexdigest()
            record_path = root / 'provider-state' / (key + '.json')
            retained_receipt = record_path.read_bytes()
            (root / 'queue-receipt-before-fault.json').write_bytes(retained_receipt)
            record = json.loads(retained_receipt)
            del record['payload']['result']
            record_path.write_text(json.dumps(record))
            recovered = enqueue(queue_args)
            assert recovered['delivery'] == 'committed' and recovered['native_turn'] == received['turns'][-1]['id']
            assert public(thread)['turns'] == received['turns'], 'unknown receipt recovery retransmitted input'
            assert len(requests) == before_return_requests + 2
            # A retained receipt remains readable even when the exact fixture
            # owner temporarily cannot be discovered. Restore its PID verbatim.
            pid_paths = [socket.with_name('pid') for socket in server_paths(home)]
            saved_pids = [(path, path.read_bytes()) for path in pid_paths]
            try:
                for path, _ in saved_pids:
                    path.write_text(str(2**32 - 1))
                assert enqueue(queue_args) == recovered
            finally:
                for path, data in saved_pids:
                    path.write_bytes(data)
            submit('/quit')
            process.wait(timeout=10)
            os.close(master)
            master = None
            runtime = home / '.local/lib/codex/core/generations/local-permission-fixture/runtime'
            for path in server_paths(home):
                cold_stops.append(stop_owned_pid(int(path.with_name('pid').read_text()), runtime))
            cold_args = dict(queue_args, text='OWNED_UNLOADED_NATIVE_QUEUE', call_id='owned-unloaded-queue',
                             execution_home=env['CODEX_HOME'])
            cold_receipt = enqueue(cold_args)
            assert cold_receipt.get('delivery') == 'queued', cold_receipt
            assert len(requests) == before_return_requests + 2, 'unloaded enqueue inferred'
            cold_key = hashlib.sha256(json.dumps(['native-queue-v1', thread, cold_args['call_id']],
                                                separators=(',', ':')).encode()).hexdigest()
            cold_path = root / 'provider-state' / (cold_key + '.json')
            cold_record = json.loads(cold_path.read_text())
            (root / 'cold-queue-receipt-before-fault.json').write_text(json.dumps(cold_record))
            del cold_record['payload']['result']
            cold_path.write_text(json.dumps(cold_record))
            assert enqueue(cold_args) == cold_receipt, 'unknown queued delivery did not recover the original item'
            master, slave = pty.openpty()
            fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 40, 160, 0, 0))
            process = subprocess.Popen([str(prefix / 'bin/codex'), '--no-alt-screen', 'resume', thread],
                env=env, cwd=root / 'w', stdin=slave, stdout=slave, stderr=slave, start_new_session=True)
            os.close(slave)
            wait(lambda: thread in loaded() and len(public(thread)['turns']) == count + 2
                 and public(thread)['status']['type'] == 'idle')
            resumed = public(thread)
            assert resumed['turns'][:-1] == received['turns']
            assert resumed['model'] == original['model'] and resumed['reasoningEffort'] == original['reasoningEffort']
            assert any(item.get('clientId') == cold_receipt['queued_submission']['clientUserMessageId']
                       for item in resumed['turns'][-1]['items'])
            queue_proof = {'nativeModel': received['model'], 'sameEffort': True,
                           'canonicalReceivingTurn': received['turns'][-1]['id'],
                           'clientIdentityMatched': True, 'unknownReceiptRecoveredWithoutResend': True,
                           'receiptReadWithoutOwner': True, 'changedInputRejected': True,
                           'unloadedEnqueueWithoutInference': True, 'unknownQueuedReceiptRecovered': True,
                           'unloadedReceivingTurn': resumed['turns'][-1]['id']}
        accepted = True
        report = {'accepted': True, 'actualAndroidTerminal': True, 'sameThread': True,
                'nativeToChatNoInference': True, 'chatToNativeSummaries': 0,
                'syntheticResponsesRequests': len(requests), 'chatSubmissions': 0,
                'providerCheckpointReal': True, 'chatRepliesSynthetic': True,
                'signedDisposableComposition': True, 'operatingRuntimeChanged': False,
                'managerAbsent': args.without_manager,
                'coldChatResume': args.cold_chat_resume,
                'warmChatResume': args.warm_chat_resume,
                'queueProof': queue_proof,
                'coldNativeStops': cold_stops,
                'readOnlyReentryTransients': reentry_observations,
                'coreBuiltFromCurrentSource': args.build_core,
                'input_sha256': {name: hashlib.file_digest(getattr(args, name).open('rb'),
                    'sha256').hexdigest() for name in ['core', 'frontend', 'provider']}}
        return report
    finally:
        if queue_service is not None and queue_service.poll() is None:
            subprocess.run([str(args.provider), 'stop', '--state', str(root / 'provider-state')],
                           env=env, capture_output=True, check=True, timeout=25)
            queue_service.wait(timeout=20)
        if process is not None:
            try:
                os.killpg(process.pid, signal.SIGTERM)
                process.wait(timeout=8)
            except ProcessLookupError:
                pass
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
                process.wait()
        if home is not None:
            for socket in server_paths(home):
                pid = int(socket.with_name('pid').read_text())
                try:
                    os.kill(pid, signal.SIGTERM)
                except ProcessLookupError:
                    pass
        if master is not None:
            os.close(master)
        if server is not None:
            server.shutdown()
            server.server_close()
        # Preserve original native records and readback before reclaiming only
        # manifest-matched generated payloads. Success does not erase evidence.
        (root / 'terminal.log').write_text(screen)
        (root / 'requests.json').write_text(json.dumps(requests))
        if accepted:
            record = root / 'payload-cleanup.jsonl'
            try:
                retention = fixture_cleanup(record, True, root)
            except (OSError, ValueError, subprocess.SubprocessError) as error:
                retention = {'error': type(error).__name__, 'payloads_preserved': True}
            report.update(fixtureRoot=str(root), payloadRetention={'record': str(record), **retention})
            (root / 'result.json').write_text(json.dumps(report, indent=2) + '\n')
        else:
            print('Owned synthetic diagnostic retained:', root)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    core=parser.add_mutually_exclusive_group(required=True)
    core.add_argument('--core',type=Path,help='Explicit prequalified Core artifact; does not assert current source')
    core.add_argument('--build-core',action='store_true',help='Build this checkout Core into XDG cache before qualification')
    for name in ['frontend', 'provider', 'report']:
        parser.add_argument('--' + name, type=Path, required=True)
    parser.add_argument('--queue', action='store_true', help='run the supplied exact Provider executable for warm and unloaded native queue proof')
    parser.add_argument('--without-manager', action='store_true')
    scenario = parser.add_mutually_exclusive_group()
    scenario.add_argument('--empty-roundtrip', action='store_true',
                        help='switch a genuinely fresh thread native to Chat and back without any model turn')
    scenario.add_argument('--warm-chat-resume', action='store_true',
                        help='open a new native thread in the same TUI, then resume saved Chat without sticky internal overrides')
    scenario.add_argument('--cold-chat-resume', action='store_true',
                        help='stop owned fixture backends and resume saved Chat without explicit catalog or profile')
    parser.add_argument('--shared-sessions', action='store_true',
                        help='use the shared sessions directory symlink of real Manager accounts')
    args = parser.parse_args()
    if args.build_core:
        source=Path(__file__).resolve().parents[1]
        target=Path(os.environ.get('XDG_CACHE_HOME',str(Path.home()/'.cache')))/'codex/check-target'
        subprocess.run(['cargo','build','--locked','-p','codex','--target-dir',str(target)],cwd=source,check=True)
        args.core=target/'debug/codex'
    for name in ['core', 'frontend', 'provider']:
        setattr(args, name, getattr(args, name).resolve(strict=True))
    report = probe(args)
    args.report.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report))
