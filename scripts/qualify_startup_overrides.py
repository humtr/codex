#!/usr/bin/env python3
"""Actual native CLI startup with invocation-only provider/profile selection.

Uses disposable homes, inert fixture authentication and a loopback Responses
server. No installed profile, real account or remote model request is used.
Native records are retained; clean_development.py owns later payload archival.
"""
import argparse
import fcntl
import http.server
import json
import os
from pathlib import Path
import pty
import select
import signal
import struct
import subprocess
import tempfile
import termios
import threading
import time

from qualify_permission_lifecycle import fixture, rpc, server_paths
from qualify_provider_d3 import stop_owned_pid


def qualify(args):
    result = {'accepted': False, 'remote_model_requests': 0, 'cases': []}
    for mode in ('config', 'profile', 'fresh-auth'):
        root = Path(tempfile.mkdtemp(prefix='si-', dir=os.environ['TMPDIR']))
        case = {'mode': mode, 'fixture': str(root), 'primary_requests': 0, 'title_requests': 0}
        result['cases'].append(case)
        home, prefix, env = fixture(root, args.core, args.frontend, args.manager, args.generation)
        errors = []
        prompt = f'STARTUP_OVERRIDE_{mode.upper()}'
        answer = f'{prompt}_COMPLETE'

        class Model(http.server.BaseHTTPRequestHandler):
            def log_message(self, *_):
                pass

            def do_POST(self):
                try:
                    assert self.path == '/v1/responses', 'unexpected model route'
                    data = self.rfile.read(int(self.headers['Content-Length']))
                    if self.headers.get('Content-Encoding') == 'zstd':
                        data = subprocess.run(['zstd', '-dq', '-c'], input=data,
                                              capture_output=True, check=True).stdout
                    body = json.loads(data)
                    if ((body.get('text') or {}).get('format') or {}).get('type') == 'json_schema':
                        case['title_requests'] += 1
                        text = json.dumps({'title': 'Verify invocation provider'})
                    else:
                        case['primary_requests'] += 1
                        assert case['primary_requests'] == 1, 'duplicate primary submission'
                        user = [item for item in body['input'] if item.get('role') == 'user'][-1]
                        assert '\n'.join(v['text'] for v in user['content'] if v['type'] == 'input_text') == prompt
                        assert body['model'] == 'gpt-6.1-sol'
                        assert body['reasoning']['effort'] == 'low', 'invocation effort was lost'
                        text = answer
                    item = {'id': 'owned-message', 'type': 'message', 'role': 'assistant',
                            'status': 'completed', 'phase': 'final_answer',
                            'content': [{'type': 'output_text', 'text': text, 'annotations': []}]}
                    response = {'id': 'owned-response', 'status': 'completed', 'output': [item],
                                'usage': {'input_tokens': 1, 'output_tokens': 1, 'total_tokens': 2}}
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
                    errors.append(str(error))
                    self.close_connection = True

        model = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Model)
        threading.Thread(target=model.serve_forever, daemon=True).start()
        endpoint = f'http://127.0.0.1:{model.server_port}/v1'
        env['OPENAI_BASE_URL'] = endpoint
        config = home / '.codex/config.toml'
        config.write_text(config.read_text().replace('[features]', '[analytics]\nenabled = false\n[features]')
                          .replace('memories = false', 'memories = false\nplugins = false')
                          + '\n[model_providers.invocation]\nname = "Invocation fixture"\n'
                          + f'base_url = "{endpoint}"\nwire_api = "responses"\nrequires_openai_auth = false\n')
        original_config = config.read_bytes()
        if mode == 'fresh-auth':
            (home / '.codex/auth.json').unlink()  # Only the inert key created by fixture().
            env.pop('OPENAI_API_KEY')
            case['native_auth_present'] = False
        core = prefix / 'bin/codex'
        baseline = subprocess.run([str(core), '__app-server-default-v1'], env=env,
                                  capture_output=True, text=True, check=True, timeout=25)
        native = Path(baseline.stdout.strip())
        native_pid = native.with_name('pid').read_text()
        flags = ['--config', 'model_provider="invocation"', '--config', 'model_reasoning_effort="low"']
        if mode == 'profile':
            (home / '.codex/invocation.config.toml').write_text(
                'model_provider = "invocation"\nmodel_reasoning_effort = "low"\n')
            flags = ['--profile', 'invocation']
        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 40, 110, 0, 0))
        child = subprocess.Popen([str(core), '--no-alt-screen', *flags], env=env, cwd=root / 'w',
                                 stdin=slave, stdout=slave, stderr=slave, start_new_session=True)
        os.close(slave)
        os.set_blocking(master, False)
        screen = bytearray()

        def wait(predicate):
            deadline = time.monotonic() + 45
            while time.monotonic() < deadline:
                if select.select([master], [], [], .1)[0]:
                    screen.extend(os.read(master, 262144))
                assert not errors, errors
                assert child.poll() is None, 'native TUI exited'
                if predicate():
                    return
            raise AssertionError('actual native startup/input timed out; inspect retained screen')

        try:
            wait(lambda: len(rpc(home, 'thread/loaded/list', {'limit': 100})['data']) == 1)
            thread = rpc(home, 'thread/loaded/list', {'limit': 100})['data'][0]
            os.write(master, b'\x1b[200~' + prompt.encode() + b'\x1b[201~')
            wait(lambda: prompt.encode() in screen)
            os.write(master, b'\r')
            wait(lambda: answer.encode() in screen)
            settings = rpc(home, 'config/read', {'threadId': thread, 'includeLayers': False})['config']
            assert settings['model_provider'] == 'invocation', 'backend retained the default provider'
            assert settings['model_reasoning_effort'] == 'low', 'backend lost invocation effort'
            assert native.with_name('pid').read_text() == native_pid
            assert (home / '.codex/app-server-control/app-server-control.sock').resolve() == native.resolve()
            assert config.read_bytes() == original_config, 'invocation changed saved defaults'
            assert case['primary_requests'] == 1
            owners = subprocess.run([str(core), 'termux', 'task', 'status'], env=env,
                                    capture_output=True, text=True, timeout=15)
            assert owners.returncode == 0 and not owners.stderr and thread in owners.stdout, 'Manager did not discover the configured owner'
            case.update(accepted=True, visible_final=True, provider='invocation', effort='low',
                        default_sibling_preserved=True, saved_configuration_unchanged=True,
                        manager_owner_visible=True)
        except Exception as error:
            case['error'] = type(error).__name__ + ': ' + str(error)
            return result
        finally:
            screen_file = args.report.with_name(args.report.stem + '-' + mode + '.screen')
            screen_file.write_bytes(screen)
            screen_file.chmod(0o600)
            try:
                if child.poll() is None:
                    os.killpg(child.pid, signal.SIGTERM)
                    try:
                        child.wait(timeout=3)
                    except subprocess.TimeoutExpired:
                        os.killpg(child.pid, signal.SIGKILL)
                        child.wait()
                os.close(master)
                runtime = home / '.local/lib/codex/core/generations/local-permission-fixture/runtime'
                case['native_stops'] = [stop_owned_pid(int(path.with_name('pid').read_text()), runtime)
                                        for path in server_paths(home)]
            except Exception as error:
                case['cleanup_error'] = type(error).__name__ + ': ' + str(error)
            finally:
                model.shutdown()
                model.server_close()
        if 'cleanup_error' in case:
            return result
    result['accepted'] = True
    return result


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('core', 'manager', 'frontend', 'generation', 'report'):
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
