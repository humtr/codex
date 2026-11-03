"""Actual Android TUI -> MCP Broker -> Native tools; synthetic app, no inference."""
import argparse
import concurrent.futures
import fcntl
import json
import os
from pathlib import Path
import pty
import select
import struct
import subprocess
import tempfile
import termios
import time
import urllib.request

from qualify_permission_lifecycle import fixture, rpc as native_rpc, server_paths
from qualify_provider_d3 import stop_owned_pid
from qualify_provider_operator import TOKEN, fixture_auth


def probe(args):
    root = Path(tempfile.mkdtemp(prefix='at-', dir=os.environ['TMPDIR']))
    home, prefix, env = fixture(root, args.core, args.frontend)
    for name in ['owned-fixture', 'model-adapter-fixture', 'hold-adapter-final']:
        (root / name).touch()
    config = home / '.codex/config.toml'
    config.write_text(config.read_text() + '\n[projects.' + json.dumps(str(root / 'w'))
                      + ']\ntrust_level="trusted"\n')
    fixture_auth(root / 'fixture-auth', args.provider_binary)
    env['CODEX_PROVIDER_OPERATOR_FIXTURE'] = str(root)
    master = terminal = service = None
    screen = ''
    output = (root / 'provider-test.log').open('wb')
    try:
        service = subprocess.Popen([str(args.service_test), '--ignored', '--exact',
            'synthetic_operator_service', '--nocapture'], env=env, cwd=root / 'w',
            stdout=output, stderr=subprocess.STDOUT)

        def wait(predicate, description):
            nonlocal screen
            deadline = time.monotonic() + 40
            while time.monotonic() < deadline:
                if master is not None and select.select([master], [], [], .02)[0]:
                    try:
                        screen += os.read(master, 65536).decode(errors='replace')
                    except OSError:
                        pass
                assert service.poll() is None, (root / 'provider-test.log').read_text()
                if terminal is not None:
                    assert terminal.poll() is None, 'owned TUI exited'
                value = predicate()
                if value:
                    return value
                time.sleep(.03)
            raise AssertionError(description + '; inspect ' + str(root))

        wait(lambda: (root / 'provider-ready.json').exists(), 'service readiness')
        port = json.loads((root / 'provider-ready.json').read_text())['port']

        def rpc(name, arguments):
            request = urllib.request.Request(f'http://127.0.0.1:{port}/mcp',
                data=json.dumps({'jsonrpc': '2.0', 'id': name, 'method': 'tools/call',
                    'params': {'name': name, 'arguments': arguments}}).encode(),
                headers={'Content-Type': 'application/json', 'Authorization': 'Bearer ' + TOKEN})
            with urllib.request.urlopen(request, timeout=45) as response:
                result = json.load(response)['result']
            value = json.loads(result['content'][0]['text'])
            assert result['isError'] is False, value
            return value

        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 44, 160, 0, 0))
        terminal = subprocess.Popen([str(prefix / 'bin/codex'), '--no-alt-screen',
            '--profile', 'chatgpt', '-c', 'tui.animations=false'], env=env,
            cwd=root / 'w', stdin=slave, stdout=slave, stderr=slave, start_new_session=True)
        os.close(slave)
        wait(lambda: 'GPT-6' in screen, 'TUI readiness')
        os.write(master, b'Run two actual Native tool calls and retain their results.')
        time.sleep(.3)
        os.write(master, b'\r')
        work = wait(lambda: next(iter(rpc('codex_sessions', {'action': 'list'})['work']), None),
                    'actual TUI-origin Native work')
        thread = work['thread_id']

        def current_lease():
            # Observe only the fixture's Producer-owned lease; never synthesize one.
            for path in (root / 'provider-state').glob('*.json'):
                value = json.loads(path.read_text()).get('payload', {})
                if isinstance(value, dict) and value.get('token') and value.get('gateway') \
                        and 'offered' in value and value.get('tools') and not value.get('complete'):
                    return value['token']
            return None

        token = wait(current_lease, 'McpBridge offered lease')
        catalog = rpc('native_catalog', {'turn_token': token})
        name = next(name for name in catalog['tools'] if name.rsplit('.', 1)[-1] == 'exec_command')
        results = []
        with concurrent.futures.ThreadPoolExecutor(max_workers=2) as clients:
            for index in (1, 2):
                arguments = {'turn_token': token, 'call_id': f'opaque-{index}', 'name': name,
                    'input': json.dumps({'cmd': f'printf "{index}\\n" >> adapter-effects; printf ADAPTER_TOOL_{index}',
                                         'login': False, 'yield_time_ms': 1000, 'max_output_tokens': 1000}),
                    'report': f'Executing actual Native tool {index}.'}
                original = clients.submit(rpc, 'native_tool', arguments)
                retry = clients.submit(rpc, 'native_tool', arguments)
                wait(original.done, f'Native tool {index} result')
                result = original.result()
                assert retry.result(timeout=5) == result, 'exact retry changed original result'
                assert f'ADAPTER_TOOL_{index}' in json.dumps(result), result
                assert (root / 'w/adapter-effects').read_text() == ''.join(f'{n}\n' for n in range(1, index + 1))
                results.append(result)
        (root / 'hold-adapter-final').unlink()
        wait(lambda: rpc('codex_status', {'work_id': work['work_id']})['completed'],
             'completed actual Native round')
        history = native_rpc(home, 'thread/read', {'threadId': thread, 'includeTurns': True})
        assert all(f'ADAPTER_TOOL_{index}' in json.dumps(history) for index in (1, 2))
        (root / 'native-original-history.json').write_text(json.dumps(history, indent=2) + '\n')
        (root / 'original-tool-results.json').write_text(json.dumps(results, indent=2) + '\n')
        wait(lambda: 'OWNED_MANAGED_ROUND_COMPLETE' in screen, 'final reaches original TUI')
        return {'accepted': True, 'fixture': str(root), 'actual_native_tools': 2,
                'exact_retry_observers': 2, 'native_final': True, 'live_tui_final': True,
                'model_requests': 0}
    finally:
        (root / 'terminal.txt').write_text(screen)
        if terminal is not None and terminal.poll() is None:
            terminal.terminate()
            terminal.wait(timeout=15)
        if service is not None and service.poll() is None:
            (root / 'stop-provider').touch()
            service.wait(timeout=30)
        if master is not None:
            os.close(master)
        output.close()
        for socket in server_paths(home):
            stop_owned_pid(int(socket.with_name('pid').read_text()),
                           home / '.local/lib/codex/core/generations/local-permission-fixture/runtime')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('core', 'frontend', 'provider-binary', 'service-test', 'evidence'):
        parser.add_argument('--' + name, type=Path, required=True)
    args = parser.parse_args()
    result = probe(args)
    args.evidence.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result), flush=True)
