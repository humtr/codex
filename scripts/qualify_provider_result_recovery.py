"""Actual Native result/callback fault proof with the existing synthetic Operator.

The one-shot loopback proxy delivers Native's executable tool item and withholds
that response's terminal until Native has committed the command end. It then
disconnects only the response consumer. No model, service replacement, replay,
alternate executor or operating profile mutation is used. Run in a foreground
terminal outside a Native opaque command, whose claim correctly excludes this
independent writer. A failed run retains its original work and private evidence.
"""
import argparse
import concurrent.futures
import hashlib
import http.client
import http.server
import json
import os
from pathlib import Path
import socket
import struct
import subprocess
import tempfile
import threading
import time

from qualify_provider_native_compiler import Client, payload, process, save, sha, stamp, termux_path
from qualify_provider_operator import fixture_auth
from qualify_provider_native_trial import stop_owned_pid
from qualify_tasks import Ws


def executable_item(event):
    item = event.get('item', {})
    if event.get('type') != 'response.output_item.done' or item.get('type') not in (
            'function_call', 'custom_tool_call'):
        return None
    identity = item.get('call_id')
    assert isinstance(identity, str) and identity, 'Native tool item lacks its original call ID'
    return identity


def completed_command(thread, identity):
    items = [item for turn in thread.get('turns', []) for item in turn['items']
             if item['type'] == 'commandExecution']
    assert len(items) <= 1, 'original command executed more than once'
    if not items:
        return None
    item = items[0]
    assert item['id'] == identity, 'Native executed a different call'
    if item['status'] == 'inProgress':
        return None
    assert item['status'] == 'completed' and item['exitCode'] == 0, 'original command failed'
    return item


def startup_settled(value, work):
    assert value.get('work_id') == work and isinstance(value.get('thread_id'), str)
    assert value.get('selection', {}).get('inference') == 'attached'
    effects = value.get('native_effects', {})
    assert isinstance(effects.get('active'), list) and type(effects.get('unconfirmed_effects')) is int
    assert isinstance(value.get('pending_calls'), list) and isinstance(value.get('human_requests'), list)
    assert isinstance(value.get('call_observations'), list)
    return (not effects['active'] and effects['unconfirmed_effects'] == 0
            and not value['pending_calls'] and not value['human_requests']
            and not any(call.get('offered') is True and call.get('settled') is not True
                        for call in value['call_observations']))


def startup_barrier(args, installation, prefix, root):
    selected = (args.startup_provider, args.startup_state, args.startup_work)
    assert all(value is None for value in selected) or all(value is not None for value in selected)
    if args.startup_work is None:
        return None
    binary = termux_path(args.startup_provider, installation, prefix)
    state = termux_path(args.startup_state, installation, prefix)
    deadline = time.monotonic() + 45
    observed_thread = None
    while time.monotonic() < deadline:
        result = subprocess.run([str(binary), 'work', 'status', '--work-id', args.startup_work, '--state', str(state)],
                                capture_output=True, timeout=min(8, deadline - time.monotonic()))
        assert result.returncode == 0 and len(result.stdout) <= 4 * 1024 * 1024, 'original launch owner is not observable'
        value = json.loads(result.stdout)
        settled = startup_settled(value, args.startup_work)
        if observed_thread is None:
            observed_thread = value['thread_id']
        assert observed_thread == value['thread_id'], 'original launch Native thread changed'
        if settled:
            proof = {'work_id': args.startup_work, 'thread_id': observed_thread,
                     'native_effects_active': 0, 'unconfirmed_effects': 0, 'pending_calls': 0, **stamp()}
            save(root / 'startup-owner-settled.json', proof)
            return proof
        time.sleep(.1)
    raise AssertionError('foreground constructor still owns a Native effect; no fixture work was created')


class CutProxy:
    """Cut once after a real tool end but before its response.completed frame."""
    def __init__(self):
        owner = self
        self.port = None
        self.thread = None
        self.observe = None
        self.records = []
        self.fired = threading.Event()
        self.settled = threading.Event()

        class Handler(http.server.BaseHTTPRequestHandler):
            def log_message(self, *_):
                pass

            def do_GET(self):
                self.forward()

            def do_POST(self):
                self.forward()

            def forward(self):
                assert self.path in ('/v1/models', '/v1/responses')
                length = int(self.headers.get('Content-Length', '0'))
                assert 0 <= length <= 32 * 1024 * 1024
                body = self.rfile.read(length)
                assert len(body) == length
                row = {'method': self.command, 'path': self.path,
                       'thread': self.headers.get('session_id') or self.headers.get('x-codex-thread-id'),
                       'body_sha256': hashlib.sha256(body).hexdigest(), 'events': [], **stamp()}
                owner.records.append(row)
                upstream = http.client.HTTPConnection('127.0.0.1', owner.port, timeout=90)
                observed = True
                try:
                    headers = {key: value for key, value in self.headers.items()
                               if key.lower() not in ('host', 'connection', 'transfer-encoding')}
                    upstream.request(self.command, self.path, body=body, headers=headers)
                    response = upstream.getresponse()
                    self.send_response(response.status)
                    for key, value in response.getheaders():
                        if key.lower() not in ('connection', 'transfer-encoding', 'content-length'):
                            self.send_header(key, value)
                    self.send_header('Connection', 'close')
                    self.end_headers()
                    self.close_connection = True
                    frame = []
                    total = 0
                    sse = response.getheader('Content-Type', '').split(';')[0] == 'text/event-stream'
                    while line := response.readline(16 * 1024 * 1024 + 1):
                        frame.append(line)
                        total += len(line)
                        assert total <= 16 * 1024 * 1024, 'oversized response frame'
                        if sse and line.strip():
                            continue
                        data = b'\n'.join(part[5:].strip() for part in frame if part.startswith(b'data:'))
                        try:
                            event = json.loads(data)
                        except (ValueError, UnicodeDecodeError):
                            event = {}
                        kind = event.get('type')
                        if kind:
                            row['events'].append(kind)
                        if kind == 'response.created':
                            row['response_id'] = event['response']['id']
                        identity = executable_item(event)
                        if identity and row['thread'] == owner.thread:
                            assert 'call_id' not in row, 'fault response offered multiple effects'
                            row['call_id'] = identity
                        if (kind == 'response.completed' and row['thread'] == owner.thread
                                and row.get('call_id') and not owner.fired.is_set()):
                            # The previous tool frame has already been flushed to
                            # Native. The canonical terminal item, not that flush,
                            # proves the command was received and executed once.
                            row['execution'] = owner.observe(row['call_id'])
                            row['cut'] = stamp()
                            self.connection.shutdown(socket.SHUT_RDWR)
                            observed = False
                            owner.fired.set()
                        if observed:
                            self.wfile.write(b''.join(frame))
                            self.wfile.flush()
                        if kind == 'response.completed':
                            row['completed_response_id'] = event['response']['id']
                        frame, total = [], 0
                    assert not frame, 'upstream ended in a partial frame'
                    row['upstream_eof'] = True
                except Exception as error:
                    row['transport_error'] = type(error).__name__
                finally:
                    upstream.close()
                    if row.get('cut'):
                        owner.settled.set()

        self.server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Handler)
        self.server.daemon_threads = True
        self.endpoint = f'http://127.0.0.1:{self.server.server_port}/v1'
        self.worker = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.worker.start()

    def close(self):
        self.server.shutdown()
        self.server.server_close()
        self.worker.join(timeout=5)


def probe(args):
    os.umask(0o077)
    installation = Path(os.environ['HOME']).resolve(strict=True)
    prefix = Path(os.environ['PREFIX']).resolve(strict=True)
    core = termux_path(args.core, installation, prefix)
    generation = termux_path(args.generation, installation, prefix)
    executable = termux_path(args.service_test, installation, prefix)
    assert core == (prefix / 'bin/codex').resolve(strict=True)
    assert generation.parent == installation / '.local/lib/codex/core/generations'
    activation = dict(line.split('=', 1) for line in
        (installation / '.local/share/codex/core/activation-state').read_text().splitlines() if '=' in line)
    assert activation['current'] == generation.name, 'use the explicit installed generation'
    runtime = generation / 'runtime'
    assert runtime.is_file() and not runtime.is_symlink()
    args.report = termux_path(args.report.absolute().parent, installation, prefix) / args.report.name
    assert not os.path.lexists(args.report), 'preserve the original report'
    with args.report.with_suffix('.launch.json').open('x') as stream:
        json.dump({'scope': 'native-result-callback-disconnect', 'model_requests': 0, **stamp()}, stream)
    root = Path(tempfile.mkdtemp(prefix='nr-', dir=termux_path(os.environ['TMPDIR'], installation, prefix)))
    workspace, home = root / 'w', root / 'h/.codex'
    for path in (workspace, home):
        path.mkdir(parents=True, mode=0o700)
    (root / 'owned-fixture').touch()
    (home / 'config.toml').write_text('approval_policy="never"\napprovals_reviewer="user"\n'
        'sandbox_mode="danger-full-access"\ncheck_for_update_on_startup=false\n'
        '[features]\nmemories=false\n[projects.' + json.dumps(str(workspace)) + ']\ntrust_level="trusted"\n')
    (home / 'auth.json').write_text(json.dumps({'OPENAI_API_KEY': 'owned-inert-fixture', 'auth_mode': 'apikey'}))
    configuration = root / 'operator.toml'
    configuration.write_text('version=1\n[transport]\nreturn_path="mcp"\n[execution]\nmode="operator"\n'
        + ''.join(key + '=' + json.dumps(str(value)) + '\n' for key, value in
                  [('core', core), ('home', home), ('workspace', workspace)]))
    fixture_auth(root / 'fixture-auth')
    proxy = CutProxy()
    env = dict(os.environ, CODEX_HOME=str(home), CODEX_PROVIDER_OPERATOR_FIXTURE=str(root),
               CODEX_PROVIDER_OPERATOR_CONFIG=str(configuration), CODEX_PROVIDER_OPERATOR_BASE_URL=proxy.endpoint,
               OPENAI_BASE_URL='http://127.0.0.1:9/v1', OPENAI_API_KEY='owned-inert-fixture')
    report = {'accepted': False, 'scope': 'native-result-callback-disconnect', 'fixture': str(root),
              'model_requests': 0, 'live_tui_accepted': False, 'operating_deployment_changed': False,
              'core_sha256': sha(core), 'runtime_sha256': sha(runtime), 'service_test_sha256': sha(executable),
              'driver_sha256': sha(Path(__file__)), **stamp()}
    save(args.report, report)
    service = client = None
    works, owners = {}, {}
    calls = concurrent.futures.ThreadPoolExecutor(max_workers=2)
    log = (root / 'provider-test.log').open('wb')
    try:
        report['startup_owner'] = startup_barrier(args, installation, prefix, root)
        save(args.report, report)
        service = subprocess.Popen([str(executable), '--ignored', '--exact', 'synthetic_operator_service', '--nocapture'],
                                   env=env, cwd=workspace, stdout=log, stderr=subprocess.STDOUT)

        def wait(predicate, description, seconds=35):
            deadline = time.monotonic() + seconds
            while time.monotonic() < deadline:
                assert service.poll() is None, 'fixture service exited; inspect its private log'
                value = predicate()
                if value:
                    return value
                time.sleep(.025)
            raise AssertionError(description)

        wait(lambda: (root / 'provider-ready.json').exists(), 'fixture readiness')
        proxy.port = json.loads((root / 'provider-ready.json').read_text())['port']
        client = Client(proxy.port, root)

        def record(work):
            return payload(root, 'operator-work-v1', work['work_id'])

        def binding(work):
            return {key: work[key] for key in ('work_id', 'controller')}

        def native(work):
            owner = record(work)['execution']
            assert process(owner) == owner['process']
            assert owner['home'] == str(home) and owner['process']['image'] == str(runtime)
            connection = Ws(str(Path(owner['directory']) / 's'), uri_path='/rpc')
            try:
                pid, uid, _ = struct.unpack('3i', connection.socket.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
                assert pid == owner['process']['pid'] and uid == os.geteuid()
                return connection.call('thread/read', {'threadId': work['thread_id'], 'includeTurns': True})['thread']
            finally:
                connection.close()

        for label in ('original', 'independent'):
            work = client.call('codex_sessions', {'action': 'create', 'inference': 'attached', 'call_id': 'result-create-' + label})
            works[label] = work
            if label == 'original':
                proxy.thread = work['thread_id']
            client.call('codex_sessions', {**binding(work), 'action': 'start', 'call_id': 'result-start-' + label,
                'task': 'Owned zero-model Native result delivery qualification ' + label + '. Execute each explicitly requested fixture command once; preserve its original output and failed turn.'})

            def context():
                value = client.call('codex_attach', binding(work))
                return None if value.get('delivery') in ('pending_native_history', 'pending_native_context') else value

            receipt = wait(context, 'attached context')
            assert client.call('codex_attach', {**binding(work), 'accept': receipt['receipt']})['accepted']
            tools = client.call('codex_tools', binding(work))['tools']
            work['exec'] = next(key for key in tools if key.rsplit('.', 1)[-1] == 'exec_command')
            owner = record(work)['execution']
            owners[owner['process']['pid']] = owner
        original = works['original']
        proxy.observe = lambda identity: wait(lambda: completed_command(native(original), identity), 'original command did not publish its terminal item')
        command = "printf 'once\\n' >> callback-effects; printf 'BEGIN\\n'; i=0; while [ \"$i\" -lt 128 ]; do printf 'original-native-output\\n'; i=$((i+1)); done; printf 'END\\n'"

        def execute(work, identity, command):
            return client.call('codex_execute', {**binding(work), 'call_id': identity, 'name': work['exec'],
                'input': json.dumps({'cmd': command, 'login': False, 'shell': str(prefix / 'bin/bash'),
                                     'workdir': str(workspace), 'yield_time_ms': 10000, 'max_output_tokens': 10000}),
                'report': 'Owned Native result delivery probe ' + identity})

        original_reply = calls.submit(execute, original, 'original-once', command)
        wait(proxy.fired.is_set, 'response fault did not reach actual Native command completion')
        wait(proxy.settled.is_set, 'original response producer did not settle')
        failed = wait(lambda: (value if (value := native(original))['turns'][-1]['status'] in ('failed', 'interrupted') else None), 'Native did not settle its disconnected turn')
        save(root / 'native-after-cut.json', failed)
        assert (workspace / 'callback-effects').read_text() == 'once\n'
        cut = next(row for row in proxy.records if row.get('cut'))
        assert cut['response_id'] == cut['completed_response_id'] and cut.get('upstream_eof')
        assert 'transport_error' not in cut
        assert sum(row['method'] == 'POST' and row['thread'] == original['thread_id'] for row in proxy.records) == 1, 'Native replayed the faulted round'
        expected = 'BEGIN\n' + 'original-native-output\n' * 128 + 'END\n'
        assert completed_command(failed, cut['call_id'])['aggregatedOutput'] == expected
        status = client.call('codex_status', {'work_id': original['work_id']})
        save(root / 'provider-after-cut.json', status)
        report.update(thread=original['thread_id'], native_turn=failed['turns'][-1]['id'],
                      native_call=cut['call_id'], original_history_count=len(failed['turns']),
                      original_command_completed=True, original_execution_count=1, producer_settled=True,
                      observed_phase=status['phase'], pending_calls=status.get('pending_calls'),
                      native_effects=status['native_effects'])
        save(args.report, report)
        independent = execute(works['independent'], 'independent-once', "printf 'independent\\n'")
        save(root / 'independent-result.json', independent)
        assert 'independent\n' in independent['result'].get('output', ''), 'completed original Native effect still blocks an independent work'
        # The retained call, not a repeated exec, supplies its terminal outcome.
        returned = original_reply.result(timeout=10)
        save(root / 'original-result.json', returned)
        assert expected in returned['result']['output'], 'original Native command output was lost'
        assert returned['result']['call_id'] == cut['call_id']
        history = client.call('codex_history', {'work_id': original['work_id']})
        recovered = client.call('codex_sessions', {'action': 'claim', 'call_id': 'result-recover-claim',
            'work_id': original['work_id'], 'cursor': history['cursor'], 'context_cursor': history['context_cursor']})
        assert recovered['thread_id'] == original['thread_id'] and recovered['inference_submitted'] is False
        original['controller'] = recovered['controller']
        client.call('codex_sessions', {**binding(original), 'action': 'start', 'call_id': 'result-continue',
            'task': 'Continue this same owned qualification after the failed response. Preserve all prior history and do not execute the original command again.'})
        current = native(original)
        assert current['turns'][:len(failed['turns'])] == failed['turns']
        assert (workspace / 'callback-effects').read_text() == 'once\n'
        report.update(accepted=True, independent_work_progressed=True, original_callback_recovered=True,
                      same_thread_continuation=True, original_history_preserved=True, automatic_reexecution=False)
    except Exception as error:
        report.update(error=type(error).__name__, detail=str(error))
        raise
    finally:
        save(root / 'proxy.json', proxy.records)
        # Stop is cleanup after the assertions, never the event that makes the
        # qualification pass or releases the tested claim before its check.
        if client is not None:
            for work in works.values():
                try:
                    client.call('codex_stop', {'work_id': work['work_id']})
                except Exception as error:
                    report.setdefault('cleanup_errors', []).append(type(error).__name__)
        if service is not None:
            (root / 'stop-provider').touch()
            try:
                service.wait(timeout=20)
            except subprocess.TimeoutExpired:
                report.setdefault('cleanup_errors', []).append('provider_stop_timeout')
        for owner in owners.values():
            try:
                if process(owner) == owner['process']:
                    stop_owned_pid(owner['process']['pid'], Path(owner['process']['image']), owner['process']['start'])
            except (OSError, AssertionError) as error:
                if Path('/proc', str(owner['process']['pid'])).exists():
                    report.setdefault('cleanup_errors', []).append(type(error).__name__)
        proxy.close()
        # Client HTTP calls have a 45-second socket deadline and no streaming
        # response body. Stop above wakes retained calls before this join.
        calls.shutdown(wait=True, cancel_futures=True)
        log.close()
        report['behavior_accepted'] = report['accepted']
        report['cleanup_confirmed'] = not report.get('cleanup_errors')
        report['accepted'] = report['behavior_accepted'] and report['cleanup_confirmed']
        save(args.report, report)
        print(json.dumps({'accepted': report['accepted'], 'fixture': str(root), 'report': str(args.report)}), flush=True)
    assert report['accepted'], 'fixture behavior or cleanup remains unconfirmed; inspect the original report'


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--core', type=Path, required=True)
    parser.add_argument('--generation', type=Path, required=True)
    parser.add_argument('--service-test', type=Path, required=True)
    parser.add_argument('--report', type=Path, required=True)
    parser.add_argument('--startup-provider', type=Path, help='existing Provider CLI for foreground-constructor settlement observation')
    parser.add_argument('--startup-state', type=Path)
    parser.add_argument('--startup-work', help='original attached work that launched this foreground terminal')
    probe(parser.parse_args())
