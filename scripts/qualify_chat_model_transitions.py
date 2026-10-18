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
import termios

from qualify_permission_lifecycle import fixture, rpc, server_paths


def probe(args):
    root = Path(tempfile.mkdtemp(prefix='chat-tui-', dir=os.environ['TMPDIR']))
    process = master = server = None
    accepted = False
    requests, failures = [], []
    screen = ''
    home = None
    try:
        home, prefix, env = fixture(root, args.core, args.frontend)
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

        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 40, 160, 0, 0))
        process = subprocess.Popen([str(prefix / 'bin/codex'), '--no-alt-screen'], env=env,
            cwd=root / 'w', stdin=slave, stdout=slave, stderr=slave, start_new_session=True)
        os.close(slave)

        def drain(seconds=.15):
            nonlocal screen
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
                    if str(error) != 'one owned thread server required':
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
            for byte in text.encode():
                os.write(master, bytes([byte]))
                drain(.04)
            drain(.8)
            os.write(master, b'\r')

        def choose_other_provider():
            nonlocal screen
            screen = ''
            # Whitespace closes slash-name completion before submitting the
            # command, so Enter cannot merely accept an autocomplete candidate.
            submit('/model ')
            wait(lambda: 'Select Model and Effort' in screen)
            assert 'ChatGPT · GPT-6' in screen and 'GPT-6.1-Sol' in screen
            os.write(master, b'\x1b[F')
            drain(.3)
            os.write(master, b'\r')

        wait(lambda: len(loaded()) == 1)
        thread = loaded()[0]
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
        # The provider's own model appears first; the opposite provider is the
        # appended final row in this deliberately two-model fixture.
        choose_other_provider()
        wait(lambda: public(thread)['modelProvider'] == 'openai')
        print('Terminal returned to native route', flush=True)
        assert len(requests) == 3, 'Chat to native requires exactly one source summary'
        submit('Continue the owned native fixture')
        wait(lambda: len(requests) == 4 and public(thread)['status']['type'] == 'idle')
        assert 'OWNED_PUBLIC_WORK_STATE_7K2Q' in json.dumps(requests[-1]['input'])
        assert not any('ocx1:' in json.dumps(v) for v in requests[-1]['input'])
        final = settings(thread)
        assert final['sandbox']['type'] == 'dangerFullAccess'
        assert final['approvalPolicy'] == 'never' and final['approvalsReviewer'] == 'user'
        assert all(v['id'] == thread for v in [public(thread)])
        accepted = True
        return {'accepted': True, 'actualAndroidTerminal': True, 'sameThread': True,
                'nativeToChatNoInference': True, 'chatToNativeSummaries': 1,
                'syntheticResponsesRequests': len(requests), 'chatSubmissions': 0,
                'providerCheckpointReal': True, 'chatRepliesSynthetic': True,
                'signedDisposableComposition': True, 'operatingRuntimeChanged': False,
                'input_sha256': {name: hashlib.file_digest(getattr(args, name).open('rb'),
                    'sha256').hexdigest() for name in ['core', 'frontend', 'provider']}}
    finally:
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
        if accepted:
            shutil.rmtree(root)
        else:
            (root / 'terminal.log').write_text(screen)
            (root / 'requests.json').write_text(json.dumps(requests))
            print('Owned synthetic diagnostic retained:', root)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ['core', 'frontend', 'provider', 'report']:
        parser.add_argument('--' + name, type=Path, required=True)
    args = parser.parse_args()
    for name in ['core', 'frontend', 'provider']:
        setattr(args, name, getattr(args, name).resolve(strict=True))
    report = probe(args)
    args.report.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report))
