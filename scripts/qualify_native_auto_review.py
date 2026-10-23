"""Owned Android proof of native OnRequest/AutoReview approval routing.

Uses a local synthetic Responses server; preserves original native records and
does not change any operating account, service or selected generation.
"""
import argparse
import fcntl
import hashlib
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
import tomllib

from qualify_permission_lifecycle import fixture, rpc, server_paths, schema_value
from qualify_provider_chat_entry import check_native_socket_budget


def probe(args):
    root = Path(tempfile.mkdtemp(prefix='ar.', dir=os.environ['TMPDIR']))
    process = master = model = home = None
    result = {'accepted': False, 'root': str(root), 'intent': args.decision}
    counts = {'primary': 0, 'guardian': 0, 'nativeResults': 0}
    failures, requests = [], []
    screen = ''
    server_owners = []
    try:
        check_native_socket_budget(root / 'h')
        home, prefix, env = fixture(root, args.core, args.frontend, args.manager, args.generation)
        work = root / 'w'
        program = work / 'owned-effect'
        program.write_text('#!' + os.environ['PREFIX'] + '/bin/sh\nprintf effect > "${0}.effect"\nprintf "OWNED_NATIVE_REVIEW_EFFECT\\n"\n')
        program.chmod(0o700)
        # A single explicit native approval rule is a legitimate OnRequest
        # trigger. It exists only in this owned fixture, never in user config.
        assert not (home / '.codex/rules').exists()
        if args.trigger == 'prompt-rule':
            rules = home / '.codex/rules'
            rules.mkdir(mode=0o700)
            (rules / 'owned.rules').write_text('prefix_rule(pattern = [' + json.dumps(str(program)) + '], decision = "prompt")\n')
        for name in ['native-models.json', 'models_cache.json']:
            path = home / '.codex' / name
            catalog = json.loads(path.read_text())
            catalog['models'][0].update(multi_agent_version='v1', context_window=262144,
                max_context_window=262144, auto_compact_token_limit=200000)
            path.write_text(json.dumps(catalog))

        class Model(http.server.BaseHTTPRequestHandler):
            def log_message(self, *_):
                pass

            def do_POST(self):
                try:
                    assert self.path == '/v1/responses'
                    assert len(requests) < 16, 'owned local request budget exceeded'
                    body = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
                    requests.append(body)
                    fmt = (body.get('text') or {}).get('format') or {}
                    guardian = fmt.get('type') == 'json_schema' and 'outcome' in fmt.get('schema', {}).get('properties', {})
                    if guardian:
                        counts['guardian'] += 1
                        if args.decision == 'failure':
                            self.send_response(503)
                            self.send_header('Content-Length', '0')
                            self.end_headers()
                            return
                        text = json.dumps({'risk_level': 'low' if args.decision == 'allow' else 'high',
                            'user_authorization': 'high' if args.decision == 'allow' else 'low',
                            'outcome': args.decision,
                            'rationale': 'Owned fixture controls the reviewer decision for this one inert program.'})
                        item = {'id': 'review', 'type': 'message', 'role': 'assistant',
                            'content': [{'type': 'output_text', 'text': text, 'annotations': []}]}
                    elif fmt.get('type') == 'json_schema':
                        item = {'id': 'owned-auxiliary', 'type': 'message', 'role': 'assistant',
                            'content': [{'type': 'output_text', 'text': json.dumps(schema_value(fmt['schema'])), 'annotations': []}]}
                    else:
                        counts['primary'] += 1
                        outputs = [v for v in body.get('input', []) if v.get('type') == 'function_call_output']
                        if outputs:
                            counts['nativeResults'] += 1
                            item = {'id': 'final', 'type': 'message', 'role': 'assistant',
                                'content': [{'type': 'output_text', 'text': 'OWNED_NATIVE_REVIEW_SETTLED', 'annotations': []}]}
                        else:
                            tools = list(body.get('tools', []))
                            for entry in body.get('input', []):
                                if entry.get('type') == 'additional_tools':
                                    tools.extend(entry.get('tools', []))
                            tool = next(v for v in tools if v.get('name') in ['exec_command', 'functions'])
                            item = {'id': 'owned-call', 'type': 'function_call', 'call_id': 'owned-once',
                                'name': 'exec_command', 'arguments': json.dumps({'cmd': str(program),
                                    'login': False, 'yield_time_ms': 1000, 'max_output_tokens': 100})}
                            if tool.get('type') == 'namespace':
                                item['namespace'] = tool['name']
                    response = {'id': 'owned-response', 'object': 'response', 'model': body['model'],
                        'status': 'completed', 'output': [item],
                        'usage': {'input_tokens': 10, 'output_tokens': 10, 'total_tokens': 20}}
                    events = [
                        {'type': 'response.created', 'response': {**response, 'status': 'in_progress', 'output': []}},
                        {'type': 'response.output_item.added', 'output_index': 0, 'item': item},
                        {'type': 'response.output_item.done', 'output_index': 0, 'item': item},
                        {'type': 'response.completed', 'response': response}]
                    if item['type'] == 'message':
                        events.insert(2, {'type': 'response.output_text.delta', 'item_id': item['id'],
                            'output_index': 0, 'content_index': 0, 'delta': item['content'][0]['text']})
                    self.send_response(200)
                    self.send_header('Content-Type', 'text/event-stream')
                    self.end_headers()
                    for event in events:
                        self.wfile.write(('event: ' + event['type'] + '\ndata: ' + json.dumps(event) + '\n\n').encode())
                    self.wfile.flush()
                except Exception as error:
                    failures.append(type(error).__name__ + ': ' + str(error))
                    self.close_connection = True

        model = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Model)
        model.daemon_threads = True
        threading.Thread(target=model.serve_forever, daemon=True).start()
        env['OPENAI_BASE_URL'] = f'http://127.0.0.1:{model.server_port}/v1'
        config = home / '.codex/config.toml'
        text = config.read_text().replace('approval_policy = "never"', 'approval_policy = "on-request"')
        text = text.replace('approvals_reviewer = "user"', 'approvals_reviewer = "auto_review"')
        if args.decision == 'full-access':
            text = text.replace('approval_policy = "on-request"', 'approval_policy = "never"')
            text = text.replace('approvals_reviewer = "auto_review"', 'approvals_reviewer = "user"')
        text = text.replace('[features]\n', '[features]\nguardian_approval = true\nplugins = false\n')
        text += '\n[projects.' + json.dumps(str(work)) + ']\ntrust_level="trusted"\n'
        # Disable provider retries only in this owned fixture, to distinguish a
        # reviewer transport failure from a later successful request.
        text = 'model_provider="owned"\n' + text
        text += '\n[model_providers.owned]\nname="Owned local model"\nbase_url=' + json.dumps(env['OPENAI_BASE_URL'])
        text += '\nwire_api="responses"\nrequires_openai_auth=false\nrequest_max_retries=0\nstream_max_retries=0\n'
        config.write_text(text)
        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 40, 160, 0, 0))
        process = subprocess.Popen([str(prefix / 'bin/codex'), '--no-alt-screen'], env=env,
            cwd=work, stdin=slave, stdout=slave, stderr=slave, start_new_session=True)
        os.close(slave)

        def wait(predicate):
            nonlocal screen
            until = time.monotonic() + 60
            while time.monotonic() < until:
                assert process.poll() is None, 'owned TUI exited before settlement'
                assert not failures, failures
                if select.select([master], [], [], .1)[0]:
                    try:
                        block = os.read(master, 65536)
                    except OSError as error:
                        if error.errno != 5:
                            raise
                        block = b''
                    screen += block.decode(errors='replace')
                    if not block:
                        raise AssertionError('owned TUI closed its terminal before settlement')
                if predicate():
                    return
            raise AssertionError('owned native reviewer qualification timed out')

        wait(lambda: bool(server_paths(home)))
        wait(lambda: len(rpc(home, 'thread/loaded/list', {})['data']) == 1)
        thread = rpc(home, 'thread/loaded/list', {})['data'][0]
        for path in server_paths(home):
            pid = int(path.with_name('pid').read_text())
            server_owners.append((pid, Path(f'/proc/{pid}/exe').resolve(), Path(f'/proc/{pid}/stat').read_text().split(')')[1].split()[19]))
        assert server_owners, 'owned native server identity must be retained before the one turn'
        result['thread'] = thread
        rpc(home, 'turn/start', {'threadId': thread, 'input': [{'type': 'text',
            'text': 'Run the one owned-effect program in this disposable workspace once, then report its native result.',
            'text_elements': []}]})
        wait(lambda: counts['nativeResults'] == 1 and rpc(home, 'thread/read',
            {'threadId': thread, 'includeTurns': True})['thread']['status']['type'] == 'idle')
        effect = program.with_name(program.name + '.effect').exists()
        assert effect == (args.decision in ['allow', 'full-access']), (args.decision, counts, effect)
        reviewed = args.trigger == 'prompt-rule' and args.decision != 'full-access'
        assert (counts['guardian'] >= 1) if reviewed else (counts['guardian'] == 0), counts
        result.update(accepted=True, counts=counts, commandEffect=effect, promptRules=args.trigger == 'prompt-rule',
            trigger=args.trigger, approvalPolicy='never' if args.decision == 'full-access' else 'on-request',
            actualAndroidTerminal=True, realModelInference=False, productionChanged=False,
            maximumNativeSocketBytes=check_native_socket_budget(home),
            input_sha256={'core': hashlib.sha256(args.core.read_bytes()).hexdigest(),
                'generation': hashlib.sha256((args.generation / 'generation.meta').read_bytes()).hexdigest()})
        contexts = []
        for rollout in (home / '.codex/sessions').rglob('*.jsonl'):
            for line in rollout.read_text().splitlines():
                row = json.loads(line)
                if row.get('type') == 'turn_context' and row['payload'].get('thread_id', thread) == thread:
                    payload = row['payload']
                    if payload.get('cwd') == str(work):
                        contexts.append({key: payload.get(key) for key in ['approval_policy', 'approvals_reviewer', 'sandbox_policy', 'permission_profile']})
        expected_policy = 'never' if args.decision == 'full-access' else 'on-request'
        assert any(row['approval_policy'] == expected_policy and row['approvals_reviewer'] == ('user' if args.decision == 'full-access' else 'auto_review') and row['sandbox_policy']['type'] == 'danger-full-access' for row in contexts), contexts
        result['nativeContexts'] = contexts
        return result
    except BaseException as error:
        result['accepted'] = False
        result['error'] = type(error).__name__ + ': ' + str(error)
        raise
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
        for pid, executable, start in server_owners:
            try:
                if (Path(f'/proc/{pid}/exe').resolve() == executable
                    and Path(f'/proc/{pid}/stat').read_text().split(')')[1].split()[19] == start):
                    os.kill(pid, signal.SIGTERM)
                    until = time.monotonic() + 5
                    while Path(f'/proc/{pid}/exe').resolve() == executable and time.monotonic() < until:
                        time.sleep(.05)
                    if (Path(f'/proc/{pid}/exe').resolve() == executable
                        and Path(f'/proc/{pid}/stat').read_text().split(')')[1].split()[19] == start):
                        os.kill(pid, signal.SIGKILL)
            except (ProcessLookupError, FileNotFoundError):
                pass
        if master is not None:
            os.close(master)
        if model is not None:
            model.shutdown()
            model.server_close()
        result['counts'] = dict(counts)
        result['commandEffect'] = (root / 'w/owned-effect.effect').exists()
        result['nativeProcessIdentities'] = [{'pid': pid, 'executable': str(executable), 'start': start, 'alive': Path(f'/proc/{pid}/exe').resolve() == executable} for pid, executable, start in server_owners]
        result['terminalProcessExit'] = process.returncode if process is not None else None
        result['requests'] = len(requests)
        (root / 'terminal.log').write_text(screen)
        (root / 'requests.json').write_text(json.dumps(requests))
        (root / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
        args.report.write_text(json.dumps(result, indent=2) + '\n')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--core', type=Path, required=True)
    parser.add_argument('--generation', type=Path, required=True)
    parser.add_argument('--frontend', type=Path)
    parser.add_argument('--manager', type=Path)
    parser.add_argument('--trigger', choices=['prompt-rule', 'ordinary'], default='prompt-rule')
    parser.add_argument('--decision', choices=['allow', 'deny', 'failure', 'full-access'], required=True)
    parser.add_argument('--report', type=Path, required=True)
    args = parser.parse_args()
    if args.trigger == 'ordinary' and args.decision not in ['allow', 'full-access']:
        parser.error('ordinary control has no review decision')
    print(json.dumps(probe(args)))
