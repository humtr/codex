#!/usr/bin/env python3
"""Actual foreground Termux copy/paste proof with an isolated Native/loopback task.

Run from a Termux terminal after explicit APK activation. This replaces clipboard
content with generated test text and checks actual Termux:API readback. No original
clipboard content is read or logged. Installed accounts/configuration remain untouched.
"""
import argparse
import fcntl
import http.server
import json
import os
from pathlib import Path
import pty
import re
import select
import shlex
import signal
import struct
import subprocess
import sys
import tempfile
import termios
import threading
import time

from qualify_permission_lifecycle import fixture, rpc, server_paths
from qualify_provider_d3 import stop_owned_pid


def terminal_request(operation, schema='termux-terminal-origin-v1'):
    client = Path(os.environ['PREFIX']) / 'bin/termux-am-socket'
    # The write's two two-second UI handoffs plus CLI return have a five-second
    # consumer budget; keep this outer subprocess bound outside that boundary.
    timeout = 6 if operation.startswith('clipboard-write-text ') else 4
    result = subprocess.run([str(client), 'termux-terminal-v1 ' + operation],
                            capture_output=True, timeout=timeout)
    assert result.returncode == 0, (
        'Termux terminal request failed: ' + operation + ': '
        + result.stderr.decode(errors='replace').strip()
    )
    assert len(result.stdout) <= 16384, 'terminal response exceeds bound'
    value = json.loads(result.stdout)
    assert value['schema'] == schema and value['result'] == 'ok'
    return value


def qualify(args):
    assert sys.stdin.isatty() and sys.stdout.isatty(), 'run this proof in an actual Termux terminal'
    caps = terminal_request('capabilities')
    assert 'clipboard-write-text' in caps['operations'].split(','), 'install the qualified clipboard APK first'
    assert terminal_request('origin')['selected'], 'run in the selected foreground Termux terminal'
    blocks = '한글🙂 원문\n\n    indented code\n\tkeep tab and  double spaces\n'
    marker = 'COPY_PROOF_ASCII_END'
    payloads = ['x' * (100000 - len(marker)) + marker, blocks * 1200 + 'COPY_PROOF_UNICODE_END']
    assert len(payloads[1].encode()) <= 100000
    result = {'accepted': False, 'remote_model_requests': 0, 'fixture_model_requests': 0,
              'temp_directory': os.environ['TMPDIR'], 'cases': []}
    root = Path(tempfile.mkdtemp(prefix='cp-', dir=os.environ['TMPDIR']))
    args.report.with_suffix('.fixture-path').write_text(str(root) + '\n')
    # Keep Native records on both outcomes. The existing clean_development.py
    # owns inactive, manifest-matched payload archival after the proof batch.
    try:
        home, prefix, env = fixture(root, args.core, args.frontend, args.manager, args.generation)
        # The private PREFIX supplies Core fixtures; these two existing Android
        # endpoints retain their real producer/peer credentials and app checks.
        (root / 'apps').symlink_to(Path(os.environ['PREFIX']).parent / 'apps')
        (prefix / 'bin/termux-am-socket').symlink_to(Path(os.environ['PREFIX']) / 'bin/termux-am-socket')
        received_requests = []
        failures = []

        class Backend(http.server.BaseHTTPRequestHandler):
            def log_message(self, *_):
                pass

            def do_POST(self):
                try:
                    assert self.path == '/v1/responses', 'unexpected model route'
                    body = self.rfile.read(int(self.headers['Content-Length']))
                    if self.headers.get('Content-Encoding') == 'zstd':
                        body = subprocess.run(['zstd', '-d', '-q', '-c'], input=body,
                                              capture_output=True, check=True).stdout
                    request = json.loads(body)
                    result['fixture_model_requests'] += 1
                    schema = (request.get('text') or {}).get('format') or {}
                    if schema.get('type') == 'json_schema':
                        assert set(schema['schema']['properties']) == {'title'}, 'unexpected auxiliary model request'
                        result['fixture_title_requests'] = result.get('fixture_title_requests', 0) + 1
                        text = json.dumps({'title': 'Verify clipboard copy'})
                    else:
                        index = len(received_requests)
                        assert index < len(payloads), 'unexpected extra primary inference request'
                        user = [item for item in request['input'] if item.get('role') == 'user'][-1]
                        prompt = '\n'.join(part['text'] for part in user['content'] if part.get('type') == 'input_text')
                        assert prompt == f'COPY_PROOF_{index}', 'duplicate or unexpected primary input'
                        received_requests.append(index)
                        result['fixture_primary_requests'] = len(received_requests)
                        text = payloads[index]
                    request_number = result['fixture_model_requests']
                    item = {'id': f'm{request_number}', 'type': 'message', 'role': 'assistant',
                            'status': 'completed', 'phase': 'final_answer',
                            'content': [{'type': 'output_text', 'text': text, 'annotations': []}]}
                    response = {'id': f'r{request_number}', 'status': 'completed', 'output': [item],
                                'usage': {'input_tokens': 1, 'output_tokens': 25000, 'total_tokens': 25001}}
                    events = [
                        {'type': 'response.created', 'response': {'id': response['id']}},
                        {'type': 'response.output_item.added', 'output_index': 0, 'item': {**item, 'content': []}},
                        {'type': 'response.output_text.delta', 'output_index': 0, 'content_index': 0, 'delta': text},
                        {'type': 'response.output_item.done', 'output_index': 0, 'item': item},
                        {'type': 'response.completed', 'response': response},
                    ]
                    data = ''.join(f"event: {v['type']}\ndata: {json.dumps(v)}\n\n" for v in events).encode()
                    self.send_response(200)
                    self.send_header('Content-Type', 'text/event-stream')
                    self.send_header('Content-Length', str(len(data)))
                    self.end_headers()
                    self.wfile.write(data)
                except Exception as error:
                    failures.append(type(error).__name__ + ': ' + str(error))
                    self.close_connection = True

        server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Backend)
        threading.Thread(target=server.serve_forever, daemon=True).start()
        env['OPENAI_BASE_URL'] = f'http://127.0.0.1:{server.server_port}/v1'
        # A custom provider prevents unrelated ChatGPT account/analytics requests.
        config = home / '.codex/config.toml'
        config.write_text('model_provider = \"copy_fixture\"\n' + config.read_text().replace('[features]', '[analytics]\nenabled = false\n[features]').replace('memories = false', 'memories = false\nplugins = false')
                          + '\n[model_providers.copy_fixture]\nname = "Copy fixture"\n'
                          + f'base_url = "{env["OPENAI_BASE_URL"]}"\nwire_api = "responses"\nrequires_openai_auth = false\n')
        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 40, 100, 0, 0))
        child = subprocess.Popen([str(prefix / 'bin/codex'), '--no-alt-screen'], env=env, cwd=root / 'w',
                                 stdin=slave, stdout=slave, stderr=slave, start_new_session=True)
        os.close(slave)
        os.set_blocking(master, False)
        screen = bytearray()

        def wait(predicate, seconds=40):
            deadline = time.monotonic() + seconds
            while time.monotonic() < deadline:
                if select.select([master], [], [], .1)[0]:
                    screen.extend(os.read(master, 262144))
                assert not failures, failures
                assert child.poll() is None, 'candidate native TUI exited'
                if predicate():
                    return
            raise AssertionError('native copy stage timed out')

        try:
            wait(lambda: bool(server_paths(home)))
            wait(lambda: len(rpc(home, 'thread/loaded/list', {'limit': 100})['data']) == 1)
            for index, text in enumerate(payloads):
                print(f'시험 {index + 1}/2: {len(text.encode())} UTF-8 bytes, 로컬 답변 준비 중', flush=True)
                # Enter through the real TUI composer. A provider-specific tip
                # or an external RPC turn is not a startup/input barrier.
                prompt = f'COPY_PROOF_{index}'.encode()
                os.write(master, b'\x1b[200~' + prompt + b'\x1b[201~')
                wait(lambda: prompt in screen)
                os.write(master, b'\r')
                marker = b'COPY_PROOF_ASCII_END' if index == 0 else b'COPY_PROOF_UNICODE_END'
                wait(lambda: marker in re.sub(rb'\s+|\x1b\[[0-?]*[ -/]*[@-~]', b'', screen))
                for action in ('ctrl-o', '/copy'):
                    # A no-op copy must not pass using the preceding action's text.
                    with tempfile.NamedTemporaryFile(prefix='copy-sentinel-', dir=os.environ['TMPDIR']) as seed:
                        seed.write(b'COPY_PROOF_SENTINEL'); seed.flush()
                        seeded = terminal_request(shlex.join(['clipboard-write-text', seed.name]),
                                                  schema='termux-terminal-clipboard-v1')
                        assert seeded['operation'] == 'clipboard-write-text' and seeded['confirmed'] is True
                    start = len(screen)
                    if action == 'ctrl-o':
                        os.write(master, b'\x0f')
                    else:
                        os.write(master, b'\x1b[200~/copy\x1b[201~')
                        wait(lambda: b'/copy' in screen[start:])
                        os.write(master, b'\r')
                        wait(lambda: re.search(rb'Copy(?:\s+to\s+clipboard|\s+whole\s+response)',
                             re.sub(rb'\x1b\[[0-?]*[ -/]*[@-~]', b'', screen[start:])))
                        start = len(screen)
                        os.write(master, b'\r')

                    def copied():
                        shown = re.sub(rb'\x1b\[[0-?]*[ -/]*[@-~]', b'', screen[start:])
                        words = re.sub(rb'\s+', b'', shown)
                        assert not any(v in words for v in (b'Copyunconfirmed', b'Copysenttoterminal', b'Copysent\xc2\xb7verifypaste')), 'Android copy was not confirmed'
                        assert b'Copyfailed' not in words, 'Android clipboard backend failed'
                        if action == 'ctrl-o':
                            return b'Copiedlastmessagetoclipboard' in words
                        return b'CopiedWholeresponsetoclipboard' in words or re.search(rb'Copied[0-9]+charstohostclipboard', words)

                    wait(copied, 15)
                    copied_text = subprocess.run(
                        [str(Path(os.environ['PREFIX']) / 'bin/termux-clipboard-get')],
                        capture_output=True, timeout=15,
                    )
                    assert copied_text.returncode == 0, 'Android clipboard readback failed'
                    assert copied_text.stdout == text.encode(), 'Android clipboard readback differs; no copy replay attempted'
                    result['cases'].append({'action': action, 'utf8_bytes': len(text.encode()),
                                            'confirmed_tui': True, 'clipboard_bytes': len(copied_text.stdout),
                                            'exact_clipboard_equal': True})
            assert received_requests == [0, 1], 'model fixture received duplicate submissions'
            result['accepted'] = True
            return result
        finally:
            args.report.with_suffix('.screen').write_bytes(screen)
            args.report.with_suffix('.screen').chmod(0o600)
            try:
                if child.poll() is None:
                    try:
                        os.killpg(child.pid, signal.SIGTERM)
                        child.wait(timeout=3)
                    except ProcessLookupError:
                        child.wait()
                    except subprocess.TimeoutExpired:
                        os.killpg(child.pid, signal.SIGKILL)
                        child.wait()
                os.close(master)
                runtime = home / '.local/lib/codex/core/generations/local-permission-fixture/runtime'
                result['native_stops'] = [
                    stop_owned_pid(int(path.with_name('pid').read_text()), runtime)
                    for path in server_paths(home)
                ]
            except Exception as error:
                # Keep the original operation failure as well as cleanup failure.
                result.update(accepted=False, cleanup_error=type(error).__name__ + ': ' + str(error))
            finally:
                server.shutdown()
                server.server_close()
    except (Exception, KeyboardInterrupt) as error:
        import traceback
        result.update(accepted=False, error=type(error).__name__ + ': ' + str(error),
                      trace=traceback.format_exc())
        return result
if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ['core', 'manager', 'frontend', 'generation', 'report']:
        parser.add_argument('--' + name, type=Path, required=True)
    options = parser.parse_args()
    try:
        outcome = qualify(options)
    except (Exception, KeyboardInterrupt) as error:
        import traceback
        outcome = {'accepted': False, 'error': type(error).__name__ + ': ' + str(error),
                   'trace': traceback.format_exc(), 'remote_model_requests': 0}
    options.report.write_text(json.dumps(outcome, indent=2) + '\n')
    options.report.chmod(0o600)
    print(json.dumps(outcome))
    raise SystemExit(0 if outcome['accepted'] else 1)
