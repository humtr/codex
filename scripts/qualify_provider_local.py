"""Actual Provider MCP -> Core threadless I/O, in a disposable signed Android fixture.

No live host/model/service or user state is acquired. The caller supplies retained
qualified Core/generation inputs; this script never rebuilds or activates them.
"""
import argparse
import base64
import concurrent.futures
import json
import os
from pathlib import Path
import select
import signal
import subprocess
import tempfile
import time
import urllib.request
import urllib.error

from qualify_permission_lifecycle import fixture, server_paths
from qualify_tasks import Ws
from qualify_provider_d3 import open_pidfd, stop_owned_pid


def probe(args):
    root = Path(tempfile.mkdtemp(prefix='lc-', dir=os.environ['TMPDIR']))
    service = None
    port, token = None, None
    home = None
    result = {'accepted': False, 'fixture': str(root), 'external_model_requests': 0}
    owned_pids = set()
    handles = []
    try:
        home, prefix, env = fixture(root, args.core, args.frontend, generation=args.generation)
        for name in ['auth.json', 'native-models.json', 'models_cache.json']:
            (home / '.codex' / name).unlink()
        env.pop('OPENAI_API_KEY', None)
        env.pop('OPENAI_BASE_URL', None)
        notification = root / 'native-completed'
        shell = str(Path(os.environ['PREFIX']) / 'bin/bash')
        (home / '.codex/config.toml').write_text(
            'approval_policy="never"\nsandbox_mode="danger-full-access"\n'
            'check_for_update_on_startup=false\nnotify=' + json.dumps([
                shell, '-c', 'printf completion >> ' + str(notification)]) + '\n')
        cfg = home / '.config/codex/providers/chatgpt.toml'
        cfg.parent.mkdir(parents=True, mode=0o700)
        state = root / 'provider-state'

        def configure(lock='standalone', profile=':danger-full-access'):
            cfg.write_text('[transport]\nreturn_path="mcp"\n[execution]\nmode="operator"\n' +
                           'core=' + json.dumps(str(prefix / 'bin/codex')) + '\n' +
                           'home=' + json.dumps(str(home / '.codex')) + '\n' +
                           'workspace=' + json.dumps(str(root / 'w')) + '\n' +
                           (('standalone_permission_profile=' + json.dumps(profile) + '\n') if profile else '') +
                           (('workflow_lock=' + json.dumps(lock) + '\n') if lock else ''))
            cfg.chmod(0o600)

        def launch():
            nonlocal service, port, token
            output = (root / ('provider-' + str(len(handles)) + '.log')).open('wb')
            handles.append(output)
            service = subprocess.Popen([str(args.provider), 'serve', '--state', str(state)],
                                       env=env, cwd=root / 'w', stdout=output, stderr=subprocess.STDOUT)
            until = time.monotonic() + 15
            while time.monotonic() < until:
                assert service.poll() is None, output.name
                try:
                    row = json.loads(Path(output.name).read_text().splitlines()[0])
                    port = int(row['base_url'].rsplit(':', 1)[1].split('/')[0])
                    token = subprocess.check_output([str(args.provider), 'token', '--state', str(state)], env=env, text=True).strip()
                    return
                except (IndexError, KeyError, ValueError):
                    time.sleep(.05)
            raise AssertionError('Provider startup timeout: ' + str(root))

        def stop(kill=False):
            nonlocal service
            if service and service.poll() is None:
                service.kill() if kill else service.terminate()
                service.wait(timeout=20)
                assert kill or service.returncode == 0, service.returncode

        def rpc(name, arguments=None, error=None, method='tools/call'):
            request = {'jsonrpc': '2.0', 'id': 1, 'method': method,
                       'params': {'name': name, 'arguments': arguments or {}}}
            headers = {'Authorization': 'Bearer ' + token, 'Content-Type': 'application/json'}
            with urllib.request.urlopen(urllib.request.Request(f'http://127.0.0.1:{port}/mcp',
                    data=json.dumps(request).encode(), headers=headers), timeout=40) as response:
                row = json.load(response)
            if method == 'tools/list':
                return row['result']['tools']
            row = row['result']
            value = json.loads(row['content'][0]['text'])
            assert row['isError'] == (error is not None), (name, value)
            if error:
                assert value['error'] == error, value
            return value

        context = None

        def execute(call, command, **kwargs):
            return rpc('local_execute', {'context_id': context['context_id'], 'call_id': call,
                       'command': command, **kwargs})

        def read(process, out=0, err=0, wait=1000):
            return rpc('local_process', {'context_id': context['context_id'], 'process_id': process,
                       'action': 'read', 'stdout_offset': out, 'stderr_offset': err, 'wait_ms': wait})

        def drain(process, timeout=25):
            stdout, stderr = bytearray(), bytearray()
            until = time.monotonic() + timeout
            while time.monotonic() < until:
                row = read(process, len(stdout), len(stderr))
                stdout.extend(base64.b64decode(row['stdout']['base64']))
                stderr.extend(base64.b64decode(row['stderr']['base64']))
                if row['state'] != 'submitted' and not row['stdout']['more'] and not row['stderr']['more']:
                    return row, bytes(stdout), bytes(stderr)
            raise AssertionError(('local command timeout', process, root))

        configure()
        launch()
        tools = rpc('', method='tools/list')
        names = {tool['name'] for tool in tools}
        assert {'local_context', 'local_execute', 'local_process'} <= names
        assert not {'codex_sessions', 'native_tool', 'codex_execute', 'codex_finish'} & names
        for tool in tools:
            assert tool['annotations']['readOnlyHint'] is True
            assert tool['annotations']['destructiveHint'] is False
            assert tool['annotations']['openWorldHint'] is False
        context = rpc('local_context')
        assert context['workflow'] == 'standalone'
        assert 'AGENTS.md' in context['instructions']
        assert not server_paths(home), 'context-only lookup acquired Core'
        try:
            urllib.request.urlopen(urllib.request.Request(f'http://127.0.0.1:{port}/v1/responses',
                data=json.dumps({'model': 'unoffered-model'}).encode(),
                headers={'Authorization': 'Bearer ' + token, 'Content-Type': 'application/json'}), timeout=5)
            raise AssertionError('standalone profile admitted a Native model request')
        except urllib.error.HTTPError as error:
            assert error.code == 409 and json.load(error)['error']['code'] == 'workflow_tool_not_admitted'
        assert not (state / 'desktop').exists(), 'rejected model request acquired a host'
        rpc('codex_sessions', {'action': 'create', 'call_id': 'wrong-mode'}, error='workflow_tool_not_admitted')
        bad = {'context_id': 'wrong', 'call_id': 'wrong-context', 'command': [shell, '-c', 'touch forbidden']}
        rpc('local_execute', bad, error='standalone_context_changed')
        bad['context_id'] = context['context_id']
        bad['permission_profile'] = ':danger-full-access'
        rpc('local_execute', bad, error='operator_arguments_invalid')
        assert not (root / 'w/forbidden').exists()

        # Full file edit and a real test, with guidance read before effects.
        (root / 'w/AGENTS.md').write_text('Write complete-scope as the first line of note.txt.\n')
        command = [shell, '-c', 'cat AGENTS.md; printf "complete-scope\\n" > note.txt; '
                   'test "$(cat note.txt)" = complete-scope; printf once >> effects; printf "\\000\\377" >&2']
        with concurrent.futures.ThreadPoolExecutor(max_workers=5) as pool:
            admissions = list(pool.map(lambda _: execute('file-edit', command), range(5)))
        process = admissions[0]['process_id']
        assert all(row['process_id'] == process for row in admissions)
        row, out, err = drain(process)
        assert row['state'] == 'exited' and row['completion']['native_reply']['result']['exitCode'] == 0, row
        assert out == (root / 'w/AGENTS.md').read_bytes() and err == b'\x00\xff'
        assert (root / 'w/note.txt').read_bytes() == b'complete-scope\n'
        assert (root / 'w/effects').read_bytes() == b'once'
        assert execute('file-edit', command)['process_id'] == process
        rpc('local_execute', {'context_id': context['context_id'], 'call_id': 'file-edit',
            'command': [shell, '-c', 'printf twice >> effects']}, error='operator_call_changed')
        # Complete paged output, beyond the upstream default capture limit.
        row = execute('large-output', [shell, '-c', 'head -c 2097152 /dev/zero'])
        settled, out, err = drain(row['process_id'])
        assert settled['state'] == 'exited' and out == b'\0' * 2097152 and not err
        # Real streaming stdin, then EOF, and idempotent write controls.
        row = execute('stdin', [shell, '-c', 'printf ready; cat'], stream_stdin=True)
        pid = row['process_id']
        ready = read(pid)
        assert ready['stdout']['text'] == 'ready', ready
        write = {'context_id': context['context_id'], 'process_id': pid, 'action': 'write',
                 'call_id': 'write-one', 'delta_base64': base64.b64encode(b'exact input\n').decode(), 'close_stdin': True}
        ack = rpc('local_process', write)
        assert ack['native_reply']['result'] == {} and not ack['exit_confirmed'], ack
        assert rpc('local_process', write) == ack
        settled, out, err = drain(pid)
        assert settled['state'] == 'exited' and out == b'readyexact input\n' and not err

        # Two commands use the same Core process but independent connections.
        first = execute('stop-target', [shell, '-c', 'echo $$; exec sleep 90'], disable_timeout=True)
        sibling = execute('sibling', [shell, '-c', 'printf sibling; cat'], stream_stdin=True, disable_timeout=True)
        ready = read(first['process_id'])
        os_pid = int(ready['stdout']['text'].strip())
        owned_pids.add(os_pid)
        fd = open_pidfd(os_pid)
        try:
            ack = rpc('local_process', {'context_id': context['context_id'], 'process_id': first['process_id'],
                      'action': 'terminate', 'call_id': 'stop-one'})
            assert ack['native_reply']['result'] == {} and not ack['exit_confirmed'], ack
            settled, _, _ = drain(first['process_id'])
            assert settled['state'] == 'exited', settled
            assert select.select([fd], [], [], 3)[0], 'Core reply did not correspond to kernel exit'
        finally:
            os.close(fd)
        assert read(sibling['process_id'])['state'] == 'submitted'
        rpc('local_process', {'context_id': context['context_id'], 'process_id': sibling['process_id'],
            'action': 'write', 'call_id': 'sibling-close', 'close_stdin': True})
        assert drain(sibling['process_id'])[1] == b'sibling'
        # The original Core PTY is resized and reports its actual dimensions.
        row = execute('pty', [shell, '-c', 'echo ready; read line; stty size'], tty=True,
                      size={'rows': 20, 'cols': 60}, disable_timeout=True)
        assert 'ready' in read(row['process_id'])['stdout']['text']
        ack = rpc('local_process', {'context_id': context['context_id'], 'process_id': row['process_id'],
            'action': 'resize', 'call_id': 'resize-one', 'size': {'rows': 33, 'cols': 99}})
        assert ack['native_reply']['result'] == {}, ack
        rpc('local_process', {'context_id': context['context_id'], 'process_id': row['process_id'],
            'action': 'write', 'call_id': 'pty-newline', 'delta_base64': base64.b64encode(b'go\n').decode()})
        settled, out, err = drain(row['process_id'])
        assert settled['state'] == 'exited' and b'33 99' in out, (settled, out)
        # Silent commands can outlive the ordinary Native RPC's 30s deadline.
        row = execute('long-command', [shell, '-c', 'sleep 31; printf after-deadline'], disable_timeout=True)
        assert drain(row['process_id'], timeout=45)[1] == b'after-deadline'

        # No Native thread, durable conversation or completion callback was created.
        paths = server_paths(home)
        assert len(paths) == 1, paths
        ws = Ws(str(paths[0].resolve()), uri_path='/rpc')
        try:
            assert ws.call('thread/loaded/list', {'limit': 100})['data'] == []
            assert ws.call('thread/list', {'limit': 100})['data'] == []
        finally:
            ws.close()
        assert not list((home / '.codex/sessions').rglob('*.jsonl'))
        assert not notification.exists()
        assert not (home / '.codex/auth.json').exists()
        assert not (home / '.codex/native-models.json').exists()
        assert not list((home / '.local/share/codex/core/catalogs').glob('*/s'))
        result['standalone'] = {'file_read_write_test': True, 'exact_concurrent_and_settled_retry_once': True,
            'changed_input_rejected': True, 'binary_stderr_preserved': True, 'output_bytes': 2097152,
            'stdin_and_eof': True, 'pty_resize': True, 'stop_kernel_exit': True, 'stop_preserved_sibling': True,
            'beyond_rpc_deadline': True, 'native_threads': 0, 'native_notifications': 0,
            'native_auth_and_model_catalog_absent': True}

        # Abrupt Provider loss: old commands cannot be redispatched after restart.
        pending_command = [shell, '-c', 'echo $$; printf once >> pending-effects; exec sleep 90']
        pending = execute('pending', pending_command, disable_timeout=True)
        pending_pid = int(read(pending['process_id'])['stdout']['text'].strip())
        owned_pids.add(pending_pid)
        fd = open_pidfd(pending_pid)
        try:
            stop(kill=True)
            assert select.select([fd], [], [], 10)[0], 'Provider death left its command alive'
        finally:
            os.close(fd)
        launch()
        assert rpc('local_context') == context
        assert execute('file-edit', command)['completion']['native_reply']['result']['exitCode'] == 0
        assert execute('pending', pending_command, disable_timeout=True)['state'] == 'delivery_unknown'
        assert (root / 'w/pending-effects').read_bytes() == b'once'
        assert (root / 'w/effects').read_bytes() == b'once'
        rpc('local_process', {'context_id': context['context_id'], 'process_id': pending['process_id'],
            'action': 'terminate', 'call_id': 'old-handle'}, error='standalone_process_not_active')

        stop()
        configure(lock='integrated')
        launch()
        assert not any(tool['name'].startswith('local_') for tool in rpc('', method='tools/list'))
        rpc('local_context', error='workflow_tool_not_admitted')
        created = rpc('codex_sessions', {'action': 'create', 'call_id': 'explicit-integrated'})
        assert created['thread_id']
        stop()
        # The standalone lock must not rotate recovered Native controllers.
        before = {path: path.read_bytes() for path in state.glob('*.json') if created['controller'].encode() in path.read_bytes()}
        assert before
        configure()
        launch()
        assert all(path.read_bytes() == data for path, data in before.items())
        rpc('codex_status', {'work_id': created['work_id']}, error='workflow_tool_not_admitted')
        stop()
        configure(profile='nonexistent-owned-qualification-profile')
        launch()
        context = rpc('local_context')
        rejected, _, _ = drain(execute('bad-profile', [shell, '-c', 'touch forbidden'])['process_id'])
        assert rejected['state'] == 'rejected' and rejected['completion']['native_reply']['error'], rejected
        assert not (root / 'w/forbidden').exists()
        stop()
        configure(lock=None)
        launch()
        names = {tool['name'] for tool in rpc('', method='tools/list')}
        assert {'local_context', 'codex_sessions'} <= names
        stop()
        configure(lock=None, profile=None)
        launch()
        assert not any(tool['name'].startswith('local_') for tool in rpc('', method='tools/list'))
        rpc('local_context', error='workflow_tool_not_admitted')
        result.update(accepted=True, restart_unknown_not_redispatched=True, fixed_profile_enforced=True,
                      native_controller_preserved=True, invalid_permission_profile_rejected=True)
    finally:
        if service and service.poll() is None:
            service.terminate()
            service.wait(timeout=20)
        if home:
            for path in server_paths(home):
                pid = int((path.parent / 'pid').read_text())
                stop_owned_pid(pid, home / '.local/lib/codex/core/generations/local-permission-fixture/runtime')
        for handle in handles:
            handle.close()
        result['owned_command_pids_exited'] = all(not Path('/proc', str(pid)).exists() for pid in owned_pids)
        args.report.write_text(json.dumps(result, indent=2) + '\n')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ['core', 'frontend', 'generation', 'provider', 'report']:
        parser.add_argument('--' + name, type=Path, required=True)
    probe(parser.parse_args())
