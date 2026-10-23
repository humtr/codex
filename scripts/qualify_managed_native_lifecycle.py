"""Actual Android managed admission, cold owner replacement and Stop; zero Chat.

The real Provider/Operator/AppHost serves an explicitly armed synthetic app.
Every native input is a distinct TUI turn; observations never retransmit one.
"""
import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import pty
import select
import shlex
import shutil
import signal
import socket
import struct
import subprocess
import tempfile
import termios
import threading
import time
import urllib.request

from qualify_permission_lifecycle import fixture, rpc as native_rpc, server_paths
from qualify_provider_operator import TOKEN
from qualify_provider_d3 import stop_owned_pid
from qualify_provider_chat_entry import native_work
from qualification_diagnostics import native_thread_settled, process_fact


def probe(args):
    # The same 107-byte UDS limit applies before any Core process is prepared.
    frontend = args.frontend.read_bytes()
    for before, after, count in [
        (b'/etc/resolv.conf', b'/proc/self/fd/33', 2),
        (b'/etc/codex/config.toml', b'/dev/fd/34/config.toml', 1),
        (b'/etc/codex/requirements.toml', b'/dev/fd/34/requirements.toml', 1),
        (b'/etc/codex/managed_config.toml', b'/dev/fd/34/managed_config.toml', 1),
    ]:
        assert frontend.count(before) == count and not frontend.count(after), 'supply the original unpatched frontend input'
    del frontend
    root = Path(tempfile.mkdtemp(prefix='ml-', dir=os.environ['TMPDIR']))
    assert len(str(root / 'h/.local/share/codex/core/catalogs/0000000000000000/s').encode()) <= 107
    home = process = service = master = None
    screens, observations, steps = [], [], []
    stop_observer = threading.Event()
    observer = None
    started = time.monotonic()
    result = {'accepted': False, 'root': str(root), 'actual_Chat_requests': 0,
              'provider_restart': args.restart_provider,
              'account_roundtrip': args.account_roundtrip,
              'followup_turns': args.followup_turns, 'idle_seconds': args.idle_seconds}
    warm_submissions, provider_exit = [], None
    try:
        home, prefix, env = fixture(root, args.core, args.frontend)
        (root / 'owned-fixture').write_text('owned managed lifecycle; no account access\n')
        (root / 'model-adapter-fixture').touch()
        auth = root / 'fixture-auth'
        auth.write_text('#!' + os.environ['PREFIX'] + '/bin/sh\nprintf %s ' + shlex.quote(TOKEN) + '\n')
        auth.chmod(0o700)
        env['CODEX_PROVIDER_OPERATOR_FIXTURE'] = str(root)
        with (root / 'provider-test.log').open('wb') as output:
            service = subprocess.Popen([str(args.service_test), '--ignored', '--exact',
                'synthetic_operator_service', '--nocapture'], env=env, cwd=root / 'w',
                stdout=output, stderr=subprocess.STDOUT)

        def wait(predicate, seconds=40):
            deadline = time.monotonic() + seconds
            while time.monotonic() < deadline:
                if master is not None:
                    drain(.03)
                assert service.poll() is None, (root / 'provider-test.log').read_text()
                try:
                    value = predicate()
                except AssertionError as error:
                    detail = error.args[0] if error.args else None
                    if str(error) == 'one owned thread server required':
                        continue
                    if (isinstance(detail, tuple) and len(detail) == 2
                            and detail[0] == 'thread/read' and isinstance(detail[1], dict)
                            and ((detail[1].get('code') == -32601
                                  and detail[1].get('message') == 'list_turns is not supported yet')
                                 or (detail[1].get('code') == -32600
                                     and 'is not materialized yet' in detail[1].get('message', '')))):
                        time.sleep(.05)
                        continue  # Exact read-only startup gap; never resend input.
                    raise
                if value:
                    return value
                time.sleep(.03)
            raise AssertionError('owned managed lifecycle timeout; inspect ' + str(root))

        wait(lambda: (root / 'provider-ready.json').exists())
        port = json.loads((root / 'provider-ready.json').read_text())['port']
        (root / 'info.json').write_text(json.dumps({'state': str(root / 'provider-state')}) + '\n')
        selected_home = home / '.codex'
        selected_workspace = root / 'w'
        account_homes = []
        if args.account_roundtrip:
            for name in ['one', 'two']:
                subprocess.run([str(prefix / 'bin/codex'), 'termux', 'profile', 'create', name],
                               env=env, cwd=root / 'w', check=True, capture_output=True)
                account = home / '.local/share/codex/manager/profiles' / name / 'home'
                for source in (home / '.codex').iterdir():
                    if source.is_file():
                        shutil.copy2(source, account / source.name)
                account_homes.append(account)
            selected_home = account_homes[0]
            (root / 'other-workspace').mkdir(mode=0o700)

        def control(name, arguments):
            request = urllib.request.Request(f'http://127.0.0.1:{port}/mcp',
                data=json.dumps({'jsonrpc': '2.0', 'id': 1, 'method': 'tools/call',
                    'params': {'name': name, 'arguments': arguments}}).encode(),
                headers={'Content-Type': 'application/json', 'Authorization': 'Bearer ' + TOKEN})
            with urllib.request.urlopen(request, timeout=25) as response:
                row = json.load(response)
            assert 'error' not in row, row
            return json.loads(row['result']['content'][0]['text'])

        screen = ''

        def drain(seconds=.1):
            nonlocal screen
            until = time.monotonic() + seconds
            while time.monotonic() < until:
                if select.select([master], [], [], .01)[0]:
                    try:
                        screen += os.read(master, 65536).decode(errors='replace')
                    except OSError:
                        return

        def launch(thread=None):
            nonlocal process, master, screen
            screen = ''
            master, slave = pty.openpty()
            fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 42, 180, 0, 0))
            command = [str(prefix / 'bin/codex'), '--no-alt-screen', '--profile', 'chatgpt',
                '-c', 'model_reasoning_effort="none"', '-c', 'tui.animations=false',
                '-C', str(selected_workspace)]
            if thread is not None:
                command += ['resume', thread]
            launch_env = dict(env, CODEX_HOME=str(selected_home))
            process = subprocess.Popen(command, env=launch_env, cwd=selected_workspace, stdin=slave,
                stdout=slave, stderr=slave, start_new_session=True)
            os.close(slave)
            def ready():
                assert process.poll() is None, 'owned TUI exited before readiness: ' + screen[-1500:]
                return 'GPT-6' in screen
            wait(ready)
            drain(.8)

        def submit(text):
            drain(.5)
            os.write(master, text.encode())
            drain(.8)
            os.write(master, b'\r')

        def public(thread):
            return native_rpc(home, 'thread/read', {'threadId': thread, 'includeTurns': True})['thread']

        def settled(thread, count):
            state = public(thread)
            return native_thread_settled(state, count)

        def record():
            rows = native_work(root)
            assert len(rows) == 1, 'one authoritative work record required; archived controllers are history'
            return rows[0]

        launch()
        thread = wait(lambda: native_rpc(home, 'thread/loaded/list', {})['data'])[0]
        submit('OWNED_WARM_MANAGED_LIFECYCLE')
        wait(lambda: settled(thread, 1))
        first = record()
        assert first['execution']['home'] == str(selected_home)
        assert first['workspace'] == str(selected_workspace)
        binding = {'work_id': first['id']}
        wait(lambda: control('codex_status', binding).get('completed'))
        (root / 'warm-native.json').write_text(json.dumps(public(thread), indent=2) + '\n')
        steps.append('warm managed TUI turn completed through real Operator/AppHost')

        # Arm the cold boundary before the observer starts: its first request
        # may be in flight when this controller closes the original backend.
        observation_stage = 'cold_owner_replacement'

        def observe():
            while not stop_observer.is_set():
                stage = observation_stage
                try:
                    observations.append(dict(control('codex_status', binding), observation_stage=stage))
                except Exception as error:
                    observations.append({'transport_error': type(error).__name__})
                stop_observer.wait(.08)

        observer = threading.Thread(target=observe)
        observer.start()
        screens.append(screen)
        os.killpg(process.pid, signal.SIGTERM)
        process.wait(timeout=8)
        os.close(master)
        master = None
        owners = []
        for path in server_paths(home):
            pid = int(path.with_name('pid').read_text())
            owners.append(stop_owned_pid(pid,
                home / '.local/lib/codex/core/generations/local-permission-fixture/runtime'))
        assert owners, 'cold proof requires an observed backend exit'
        if args.restart_provider:
            # Both the native owner and Provider disappeared in the real crash.
            # Retiring a recorded old token must not reactivate that capability.
            stop_observer.set()
            observer.join(timeout=30)
            assert not observer.is_alive()
            observer = None
            prior_commands = json.loads((root / 'bridge-commands.json').read_text())
            (root / 'warm-bridge-commands.json').write_text(json.dumps(prior_commands, indent=2))
            warm_submissions = [row for row in prior_commands if row['op'] == 'submit']
            assert len(warm_submissions) == 1
            provider_exit = stop_owned_pid(service.pid, args.service_test)
            service.wait(timeout=8)
            # This is the owned expired readiness output, not an execution input.
            (root / 'provider-ready.json').rename(root / 'provider-ready-before-restart.json')
            env['CODEX_PROVIDER_OPERATOR_PORT'] = str(port)
            with (root / 'provider-test-after-restart.log').open('wb') as output:
                service = subprocess.Popen([str(args.service_test), '--ignored', '--exact',
                    'synthetic_operator_service', '--nocapture'], env=env, cwd=root / 'w',
                    stdout=output, stderr=subprocess.STDOUT)
            wait(lambda: (root / 'provider-ready.json').exists())
            assert json.loads((root / 'provider-ready.json').read_text())['port'] == port
            stop_observer.clear()
            observer = threading.Thread(target=observe)
            observer.start()
            steps.append('owned Provider also exits and restarts on the same endpoint/state')
        if args.account_roundtrip:
            selected_home = account_homes[1]
            selected_workspace = root / 'other-workspace'
        launch(thread)
        assert len(public(thread)['turns']) == 1, 'resume must not infer'
        # A status read intentionally observes the new owner before admission.
        # The subsequent new canonical TUI turn must reconcile it itself.
        assert control('codex_status', binding)['phase'] in ['completed', 'reconciliation_required']
        observation_stage = 'post_cold'
        submit('OWNED_DISTINCT_POST_COLD_MANAGED_LIFECYCLE')
        wait(lambda: settled(thread, 2))
        wait(lambda: control('codex_status', binding).get('completed'))
        rebound = record()
        assert rebound['execution'] != first['execution']
        assert rebound['generation'] == first['generation'] + 1
        assert rebound['capability'] != first['capability']
        assert rebound['execution']['home'] == str(selected_home)
        assert rebound['workspace'] == str(selected_workspace)
        assert public(thread)['turns'][0] == json.loads((root / 'warm-native.json').read_text())['turns'][0]
        steps.append('same-thread cold TUI resume rebinds owner under concurrent status reads')

        (root / 'hold-host-completion').touch()
        submit('OWNED_DISTINCT_MANAGED_STOP')
        wait(lambda: len(public(thread)['turns']) == 3 and record()['phase'] == 'running')
        wait(lambda: len([row for row in json.loads((root / 'bridge-commands.json').read_text())
            if row['op'] == 'submit']) == 3 - len(warm_submissions))
        stopped = control('codex_stop', binding)
        assert stopped['native_stop_confirmed'] and stopped['host_stop_confirmed'], stopped
        (root / 'hold-host-completion').unlink()
        wait(lambda: control('codex_status', binding).get('phase') == 'stopped')
        drain(.8)
        assert record()['phase'] == 'stopped', 'old managed finisher resurrected stopped work'
        steps.append('managed Stop remains stopped after original exchange exits')
        submit('OWNED_DISTINCT_NEW_TURN_AFTER_CONFIRMED_STOP')
        wait(lambda: settled(thread, 4))
        wait(lambda: control('codex_status', binding).get('completed'))
        retained = public(thread)['turns']
        followups = []

        def resource_snapshot():
            pids = {process.pid, service.pid}
            pids.update(int(path.with_name('pid').read_text()) for path in server_paths(home))
            return [process_fact(Path('/proc') / str(pid)) for pid in sorted(pids)]

        resources_before = resource_snapshot()
        for index in range(args.followup_turns):
            began = time.monotonic()
            submit(f'OWNED_DISTINCT_RETAINED_FOLLOWUP_{index + 1}')
            wait(lambda: settled(thread, 5 + index))
            wait(lambda: control('codex_status', binding).get('completed'))
            current = public(thread)['turns']
            assert current[:-1] == retained, 'a later turn changed earlier canonical items'
            retained = current
            followups.append({'index': index + 1, 'elapsed_seconds': time.monotonic() - began,
                              'turn_id': current[-1]['id'], 'resources': resource_snapshot()})
            (root / 'followups.json').write_text(json.dumps(followups, indent=2) + '\n')
        idle_started = time.monotonic()
        while time.monotonic() - idle_started < args.idle_seconds:
            drain(.2)
            assert process.poll() is None and service.poll() is None, 'owned idle process exited'
            time.sleep(.3)
        assert public(thread)['turns'] == retained, 'idle observation created or changed a turn'
        assert control('codex_status', binding).get('completed')
        resources_after = resource_snapshot()
        rss_growth = sum(row['rss_kib'] for row in resources_after) - sum(row['rss_kib'] for row in resources_before)
        assert rss_growth <= args.max_rss_growth_mib * 1024, 'declared fixture RSS growth bound exceeded'
        (root / 'retention-resources.json').write_text(json.dumps({
            'before': resources_before, 'after': resources_after,
            'rss_growth_kib': rss_growth, 'max_rss_growth_mib': args.max_rss_growth_mib,
            'idle_elapsed_seconds': time.monotonic() - idle_started}, indent=2) + '\n')
        expected_turns = 4 + args.followup_turns
        failed_index = None
        if args.account_roundtrip:
            # Normal client cleanup releases its writer; leave unrelated servers
            # running, then resume the same thread back in the first account.
            submit('/quit')
            process.wait(timeout=15)
            os.close(master)
            master = None
            selected_home = account_homes[0]
            selected_workspace = root / 'w'
            launch(thread)
            assert len(public(thread)['turns']) == expected_turns
            submit('OWNED_DISTINCT_ACCOUNT_RETURN')
            expected_turns += 1
            wait(lambda: settled(thread, expected_turns))
            wait(lambda: control('codex_status', binding).get('completed'))
            assert record()['execution']['home'] == str(selected_home)
            assert record()['workspace'] == str(selected_workspace)
            assert public(thread)['turns'][:-1] == retained
            steps.append('account one → two with workspace change → one, same native thread and original history')

            # Reproduce the reported pre-app admission failure in this owned
            # binding only. Pause observers so no control attempts owner recovery
            # during the fault. Restore the original PID even on failure.
            stop_observer.set()
            observer.join(timeout=30)
            assert not observer.is_alive()
            observer = None
            pid_file = Path(record()['execution']['directory']) / 'pid'
            assert pid_file.is_relative_to(home / '.local/share/codex/core')
            original_pid = pid_file.read_bytes()
            error_screen_start = len(screen)
            try:
                pid_file.write_text(str(2**32 - 1))
                submit('OWNED_DISTINCT_EXPECTED_ADMISSION_FAILURE')
                wait(lambda: 'native_thread_owner_unavailable' in screen[error_screen_start:])
            finally:
                pid_file.write_bytes(original_pid)
            expected_turns += 1
            failed_index = expected_turns - 1
            def failed_settled():
                state = public(thread)
                (root / 'failed-native-read.json').write_text(json.dumps(state, indent=2) + '\n')
                return (state['status']['type'] in ['idle', 'systemError']
                        and len(state['turns']) == expected_turns
                        and state['turns'][-1]['status'] == 'failed')
            wait(failed_settled)
            submit('/model gpt-6.1-sol')
            wait(lambda: public(thread)['modelProvider'] == 'openai')
            assert len(public(thread)['turns']) == expected_turns
            submit('/model chatgpt-desktop/gpt-6 none')
            wait(lambda: public(thread)['modelProvider'] == 'chatgpt_desktop')
            submit('OWNED_DISTINCT_AFTER_ACCOUNT_AND_MODEL_RETURN')
            expected_turns += 1
            wait(lambda: settled(thread, expected_turns))
            wait(lambda: control('codex_status', binding).get('completed'))
            steps.append('exact native_thread_owner_unavailable → readable history → native model → Chat model → new prompt')
            result['account_homes'] = [str(value) for value in account_homes]
            result['provider_default_home'] = str(home / '.codex')
            result['workspace_rebound'] = True
            result['failed_request_native_return'] = True
            result['admission_fault'] = 'owned PID restored byte-for-byte; no app submission for the failed input'
        final = public(thread)
        assert len(final['turns']) == expected_turns and len({turn['id'] for turn in final['turns']}) == expected_turns
        for index, turn in enumerate(final['turns']):
            if index in [2, failed_index]:  # Confirmed Stop or deliberately failed admission.
                continue
            assert any(item.get('type') == 'agentMessage' and item.get('phase') == 'final_answer'
                and item.get('text') == 'OWNED_MANAGED_ROUND_COMPLETE' for item in turn['items'])
        submissions = warm_submissions + [row for row in json.loads((root / 'bridge-commands.json').read_text()) if row['op'] == 'submit']
        expected_submissions = expected_turns - int(failed_index is not None)
        assert len(submissions) == expected_submissions and len({row['round'] for row in submissions}) == expected_submissions
        assert all(row['nativeThread'] == thread for row in submissions)
        assert not any('transport_error' in row for row in observations)
        read_errors = [row for row in observations if 'error' in row]
        assert all(row['error'] == 'native_observation_superseded'
            or row.get('observation_stage') == 'cold_owner_replacement'
            and row['error'] in ['native_delivery_unknown', 'native_thread_owner_unavailable',
                'native_socket_unavailable'] for row in read_errors), read_errors
        (root / 'final-native.json').write_text(json.dumps(final, indent=2) + '\n')
        steps.append('a genuinely new TUI turn works after confirmed Stop')
        result.update(accepted=True, thread_id=thread, actual_Android_TUI=True,
            native_inputs=expected_turns, kernel_confirmed_backend_exits=owners,
            kernel_confirmed_provider_exit=provider_exit,
            followup_timings=followups, rss_growth_kib=rss_growth,
            original_turn_preserved=True, controller_rotated=True,
            synthetic_app_submissions=len(submissions), read_only_transition_errors=read_errors, steps=steps)
    except Exception as error:
        result['failure'] = {'type': type(error).__name__, 'message': str(error)}
        raise
    finally:
        stop_observer.set()
        if observer is not None:
            observer.join(timeout=30)
        if master is not None:
            screens.append(screen)
            os.close(master)
        if process is not None and process.poll() is None:
            os.killpg(process.pid, signal.SIGTERM)
            process.wait(timeout=8)
        if home is not None:
            for path in server_paths(home):
                try:
                    os.kill(int(path.with_name('pid').read_text()), signal.SIGTERM)
                except ProcessLookupError:
                    pass
        if service is not None and service.poll() is None:
            (root / 'stop-provider').touch()
            service.wait(timeout=20)
        (root / 'terminal.txt').write_text('\n'.join(screens))
        (root / 'observations.json').write_text(json.dumps(observations, indent=2) + '\n')
        result.update(elapsed_seconds=time.monotonic() - started, status_observations=len(observations),
            input_sha256={name: hashlib.file_digest(getattr(args, name).open('rb'), 'sha256').hexdigest()
                for name in ['core', 'frontend', 'service_test']})
        args.report.write_text(json.dumps(result, indent=2) + '\n')
        print(json.dumps(result), flush=True)


if __name__ == '__main__':
    os.umask(0o077)
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--account-roundtrip', action='store_true',
                        help='different Provider default, two Manager accounts/workspaces, failed-turn model return')
    parser.add_argument('--restart-provider', action='store_true')
    parser.add_argument('--followup-turns', type=int, default=0, choices=range(0, 65), metavar='0..64')
    parser.add_argument('--idle-seconds', type=int, default=0, choices=range(0, 601), metavar='0..600')
    parser.add_argument('--max-rss-growth-mib', type=int, default=512, choices=range(1, 2049), metavar='1..2048')
    for name in ['core', 'frontend', 'service-test', 'report']:
        parser.add_argument('--' + name, type=Path, required=True)
    args = parser.parse_args()
    for name in ['core', 'frontend', 'service_test']:
        setattr(args, name, getattr(args, name).resolve(strict=True))
    probe(args)
