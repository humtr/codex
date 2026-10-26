"""Codex-entry local-function Operator Host proof on disposable Android native Core.

The synthetic app offers local calls; it never invokes native tools itself.
The production local transport must return each original call and perform handoff.
No actual ChatGPT request, app credential or installed state is used.
"""
import argparse
import fcntl
import json
import os
from pathlib import Path
import pty
import re
import select
import signal
import struct
import subprocess
import tempfile
import termios
import time
import urllib.request

from qualify_permission_lifecycle import fixture, server_paths
from qualify_provider_operator import TOKEN, fixture_auth


def probe(args):
    root = Path(tempfile.mkdtemp(prefix='ol-', dir=os.environ['TMPDIR']))
    processes = []
    master = None
    home = None
    screen = ''
    try:
        home, prefix, env = fixture(root, args.core, args.frontend)
        (root / 'owned-fixture').touch()
        (root / 'local-function-fixture').touch()
        env['CODEX_PROVIDER_OPERATOR_FIXTURE'] = str(root)
        auth = root / 'fixture-auth'
        fixture_auth(auth, args.provider_binary)
        with (root / 'provider-test.log').open('wb') as output:
            service = subprocess.Popen([str(args.service_test), '--ignored', '--exact',
                'synthetic_operator_service', '--nocapture'], env=env, stdout=output,
                stderr=subprocess.STDOUT, cwd=root / 'w')
        processes.append(service)

        def drain():
            nonlocal screen
            if master is not None:
                while select.select([master], [], [], .01)[0]:
                    try:
                        screen += os.read(master, 65536).decode(errors='replace')
                    except OSError:
                        break
            return screen

        def wait(predicate, timeout=45):
            until = time.monotonic() + timeout
            while time.monotonic() < until:
                drain()
                assert service.poll() is None, (root / 'provider-test.log').read_text()
                value = predicate()
                if value:
                    return value
                time.sleep(.04)
            raise AssertionError('fixture timeout; retained at ' + str(root))

        wait(lambda: (root / 'provider-ready.json').exists())
        port = json.loads((root / 'provider-ready.json').read_text())['port']

        def rpc(name, arguments):
            body = {'jsonrpc': '2.0', 'id': 1, 'method': 'tools/call',
                'params': {'name': name, 'arguments': arguments}}
            request = urllib.request.Request(f'http://127.0.0.1:{port}/mcp',
                data=json.dumps(body).encode(), headers={'Authorization': 'Bearer ' + TOKEN,
                    'Content-Type': 'application/json'})
            with urllib.request.urlopen(request, timeout=15) as response:
                result = json.load(response)['result']
            assert not result['isError'], result
            return json.loads(result['content'][0]['text'])

        def rows():
            path = root / 'bridge-commands.json'
            return json.loads(path.read_text()) if path.exists() else []

        def submissions():
            return [row for row in rows() if row['op'] == 'submit']

        def binding():
            for row in reversed(submissions()):
                match = re.search(r'work_id=([0-9a-f]{48}) and controller=([0-9a-f]{48})',
                    '\n'.join(row.get('instructions', [])))
                if match:
                    return {'work_id': match[1], 'controller': match[2]}

        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 42, 180, 0, 0))
        terminal = subprocess.Popen([str(prefix / 'bin/codex'), '--no-alt-screen',
            '--profile', 'chatgpt'], env=env, cwd=root / 'w', stdin=slave, stdout=slave,
            stderr=slave, start_new_session=True)
        os.close(slave)
        processes.append(terminal)
        wait(lambda: 'GPT-6' in drain())
        os.write(master, b'\x1b[200~OWNED_LOCAL_OPERATOR_DEVELOPMENT\x1b[201~')
        time.sleep(.8)
        drain()
        os.write(master, b'\r')
        bound = wait(binding)
        original_thread = rpc('codex_sessions', {'action': 'list'})['work'][0]['thread_id']
        counter = 0

        def local(name, arguments, expect_error=False, input_override=None):
            nonlocal counter
            counter += 1
            call_id = 'local-app-' + str(counter)
            latest = submissions()[-1]
            offered = {'round': latest['round'], 'id': call_id, 'name': name,
                'arguments': arguments}
            if input_override is not None:
                offered['input_override'] = input_override
            temporary = root / 'local-request.tmp'
            temporary.write_text(json.dumps(offered))
            temporary.replace(root / 'local-request.json')

            def returned():
                for row in submissions():
                    for result in row.get('results', []):
                        if result['id'] == call_id:
                            return (result['result'],)
            value = wait(returned)[0]
            assert ('error' in value) == expect_error, value
            return value

        def attach(current):
            observed = local('codex_attach', current)
            assert local('codex_attach', {**current, 'accept': observed['receipt']})['accepted']

        attach(bound)
        catalog = local('codex_tools', bound)['tools']
        command_key = next(key for key in catalog if key.rsplit('.', 1)[-1] == 'exec_command')
        patch_key = next(key for key in catalog if key.rsplit('.', 1)[-1] == 'apply_patch')
        rejected = local('codex_execute', bound, expect_error=True,
            input_override='{"input":"a patch with "unescaped" quotes"}')
        assert rejected['error']['executed'] is False
        assert not rpc('codex_status', {'work_id': bound['work_id']})['call_observations']

        def command(call_id, text):
            return local('codex_execute', {**bound, 'call_id': call_id, 'name': command_key,
                'input': json.dumps({'cmd': text, 'login': False, 'max_output_tokens': 1000}),
                'report': 'OWNED_LOCAL_NATIVE_' + call_id})

        failed = command('initial-failure', "python3 -c 'from owned_local import add; assert add(2,3)==5'")
        assert 'ModuleNotFoundError' in json.dumps(failed)
        patch = '*** Begin Patch\n*** Add File: owned_local.py\n+def add(a, b):\n+    return a + b\n*** End Patch'
        local('codex_execute', {**bound, 'call_id': 'actual-patch', 'name': patch_key,
            'input': patch, 'report': 'OWNED_LOCAL_NATIVE_PATCH'})
        assert 'LOCAL_NATIVE_PASS' in json.dumps(command('verify',
            "python3 -c 'from owned_local import add; assert add(2,3)==5; print(\"LOCAL_NATIVE_PASS\")'"))
        before = rpc('codex_history', {'work_id': bound['work_id']})
        assert 'OWNED_LOCAL_NATIVE_PATCH' in json.dumps(before)
        handoff = local('codex_handoff', {**bound, 'call_id': 'handoff',
            'cursor': before['cursor'], 'context_cursor': before['context_cursor'],
            'summary': 'Created owned_local.py add(a,b); initial missing-module failed, patch and 2+3 assertion passed. Continue on the same native thread and verify 3+4.', 'renew': True})
        assert handoff['prepared'] and handoff['successor_requested']
        old = bound
        bound = wait(lambda: value if (value := binding()) != old else None)
        assert bound['work_id'] == old['work_id']
        successor = [row for row in submissions() if row.get('bootstrap')][-1]
        assert all(message['role'] == 'developer' for message in successor['messages'])
        attach(bound)
        assert 'LOCAL_SUCCESSOR_PASS' in json.dumps(command('successor-verify',
            "python3 -c 'from owned_local import add; assert add(3,4)==7; print(\"LOCAL_SUCCESSOR_PASS\")'"))
        local('codex_finish', {**bound, 'call_id': 'finish', 'text': 'OWNED_LOCAL_OPERATOR_COMPLETE'})
        wait(lambda: 'OWNED_LOCAL_OPERATOR_COMPLETE' in drain())
        status = rpc('codex_status', {'work_id': bound['work_id']})
        assert status['completed'] and status['thread_id'] == original_thread
        stopped = rpc('codex_stop', {'work_id': bound['work_id']})
        assert stopped['native_stop_confirmed'] and stopped['host_stop_confirmed']
        assert not stopped['remaining_jobs']
        count = len(submissions())
        time.sleep(.3)
        assert len(submissions()) == count
        history = rpc('codex_history', {'work_id': bound['work_id']})
        args.report.write_text(json.dumps({'accepted': True, 'fixture': str(root),
            'chat_submissions': 0, 'codex_entry': True, 'same_thread': original_thread,
            'local_original_control_returns': counter, 'native_failure_patch_correction': True,
            'system_only_successor': True, 'original_cli_final': True,
            'native_host_stop_confirmed': True, 'pending_jobs': 0,
            'invalid_local_input_returned_before_effects_and_corrected': True,
            'history_cursor': history['cursor']}, indent=2) + '\n')
        print('Local operator original returns, Android effects, handoff and Stop pass', flush=True)
    finally:
        (root / 'stop-provider').touch()
        for process in reversed(processes):
            if process.poll() is None:
                process.terminate()
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
        if home:
            for path in server_paths(home):
                try:
                    pid = int((path.parent / 'pid').read_text())
                    expected = home / '.local/lib/codex/core/generations/local-permission-fixture/runtime'
                    if Path(f'/proc/{pid}/exe').resolve() == expected:
                        os.kill(pid, signal.SIGTERM)
                except (OSError, ValueError):
                    pass
        if master is not None:
            os.close(master)
        (root / 'terminal.txt').write_text(screen)
        print('Owned local fixture retained at ' + str(root), flush=True)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--core', type=Path, required=True)
    parser.add_argument('--frontend', type=Path, required=True)
    parser.add_argument('--service-test', type=Path, required=True)
    parser.add_argument('--provider-binary', type=Path, help='actual Provider CLI used by the native client synchronization gate')
    parser.add_argument('--report', type=Path, required=True)
    probe(parser.parse_args())
