"""Actual Android MCP operator/CLI proof; only owned synthetic Chat and signed fixtures."""
import argparse
import concurrent.futures
import fcntl
import hashlib
import json
import os
from pathlib import Path
import pty
import re
import select
import shlex
import signal
import socket
import struct
import subprocess
import tempfile
import termios
import time
import tomllib
import urllib.error
import urllib.request
from qualify_permission_lifecycle import fixture, server_paths, rpc as native_rpc
from qualify_provider_d3 import stop_owned_pid

TOKEN = '88c34e80b76d5b9289cc2719f6d43fb2d7d33f8c1f680a42'


def fixture_auth(path, provider_binary=None):
    """Use fixture HTTP auth but the actual installed CLI control protocol."""
    control = ''
    if provider_binary is not None:
        control = ('if [ "$1" = presentation ] || [ "$1" = work ]; then exec ' +
                   shlex.quote(str(provider_binary.resolve(strict=True))) + ' "$@"; fi\n')
    path.write_text('#!' + os.environ['PREFIX'] + '/bin/sh\n' + control +
                    'if [ "$1" = work ]; then echo "codex-chatgpt-provider: unsupported work action" >&2; exit 2; fi\n' +
                    'printf %s ' + shlex.quote(TOKEN) + '\n')
    path.chmod(0o700)


def probe(args):
    root = args.root if getattr(args, 'root', None) else Path(tempfile.mkdtemp(prefix='ob-', dir=os.environ['TMPDIR']))
    processes, masters = [], []
    pool = concurrent.futures.ThreadPoolExecutor(max_workers=8)
    screen = ''
    home = None
    try:
        home, prefix, env = fixture(root, args.core, args.frontend, generation=getattr(args, 'generation', None))
        config_path=home / '.codex/config.toml'
        config_path.write_text(config_path.read_text().replace('approval_policy = "never"','approval_policy = "on-request"')
            .replace('approvals_reviewer = "user"','approvals_reviewer = "auto_review"')
            .replace('memories = false','memories = false\nguardian_approval = true')
            + '\n[projects.' + json.dumps(str(root / 'w')) + ']\ntrust_level = "trusted"\n')
        reviewed=root / 'w/reviewed-fixture'
        reviewed.write_text('#!' + os.environ['PREFIX'] + '/bin/sh\nprintf reviewed >> reviewed-effects\n')
        reviewed.chmod(0o700)
        rules=home / '.codex/rules'
        rules.mkdir()
        (rules / 'fixture.rules').write_text('prefix_rule(pattern = [' + json.dumps(str(reviewed)) + '], decision = "prompt")\n')
        (root / 'owned-fixture').write_text('owned synthetic operator acceptance\n')
        env['CODEX_PROVIDER_OPERATOR_FIXTURE'] = str(root)
        if getattr(args, 'config', None):
            env['CODEX_PROVIDER_OPERATOR_CONFIG'] = str(args.config)
        auth = root / 'fixture-auth'
        fixture_auth(auth, getattr(args, 'provider_binary', None))
        output = (root / 'provider-test.log').open('wb')
        service = subprocess.Popen([str(args.service_test), '--ignored', '--exact',
            'synthetic_operator_service', '--nocapture'], env=env, stdout=output,
            stderr=subprocess.STDOUT, cwd=root / 'w')
        processes.append(service)

        def wait(predicate, timeout=35):
            until = time.monotonic() + timeout
            while time.monotonic() < until:
                value = predicate()
                if value:
                    return value
                time.sleep(.05)
            raise AssertionError('fixture timeout; inspect ' + str(root))

        wait(lambda: (root / 'provider-ready.json').exists() or service.poll() is not None)
        assert service.poll() is None, (root / 'provider-test.log').read_text()
        port = json.loads((root / 'provider-ready.json').read_text())['port']
        seq = 0

        def rpc(name, arguments, expect_error=False, authorized=True):
            nonlocal seq
            seq += 1
            row = {'jsonrpc': '2.0', 'id': seq, 'method': 'tools/call',
                   'params': {'name': name, 'arguments': arguments}}
            headers = {'Content-Type': 'application/json'}
            if authorized:
                headers['Authorization'] = 'Bearer ' + TOKEN
            request = urllib.request.Request(f'http://127.0.0.1:{port}/mcp',
                data=json.dumps(row).encode(), headers=headers)
            with urllib.request.urlopen(request, timeout=50) as response:
                data = json.load(response)
            assert 'error' not in data, data
            result = data['result']
            value = json.loads(result['content'][0]['text'])
            assert result['isError'] == expect_error, (name, value)
            return value

        def work_record(work_id):
            key = hashlib.sha256(json.dumps(['operator-work-v1',work_id],separators=(',',':')).encode()).hexdigest()
            return json.loads((root/'provider-state'/(key+'.json')).read_text())['payload']

        def restart_fixture():
            nonlocal service
            (root / 'stop-provider').touch()
            service.wait(timeout=20)
            assert service.returncode == 0
            (root / 'stop-provider').unlink()
            (root / 'provider-ready.json').unlink()
            closed_bridge_commands = json.loads((root / 'bridge-commands.json').read_text())
            env['CODEX_PROVIDER_OPERATOR_PORT'] = str(port)
            service = subprocess.Popen([str(args.service_test), '--ignored', '--exact',
                'synthetic_operator_service', '--nocapture'], env=env, stdout=output,
                stderr=subprocess.STDOUT, cwd=root / 'w')
            processes.append(service)
            wait(lambda: (root / 'provider-ready.json').exists() or service.poll() is not None)
            assert service.poll() is None
            assert json.loads((root / 'provider-ready.json').read_text())['port'] == port
            return closed_bridge_commands

        try:
            rpc('codex_sessions', {'action': 'list'}, authorized=False)
            raise AssertionError('unauthenticated bootstrap accepted')
        except urllib.error.HTTPError as error:
            assert error.code == 401
        created = rpc('codex_sessions', {'action': 'create', 'call_id': 'chat-entry-create'})
        assert rpc('codex_sessions', {'action': 'create', 'call_id': 'chat-entry-create'}) == created
        print('Native session created through qualified Core', flush=True)
        binding = {'work_id': created['work_id'], 'controller': created['controller']}
        rpc('codex_sessions', {**binding, 'action': 'start', 'task': 'OWNED_CHAT_ENTRY_DEVELOPMENT'})
        wait(lambda: rpc('codex_status', {'work_id': binding['work_id']})['phase'] == 'running')

        def attach(bound):
            observed = rpc('codex_attach', bound)
            accepted = rpc('codex_attach', {**bound, 'accept': observed['receipt']})
            assert accepted['accepted']
            return accepted

        attach(binding)
        catalog = rpc('codex_tools', binding)['tools']
        command_key = next(key for key in catalog if key.rsplit('.', 1)[-1] in ['exec_command', 'shell_command'])
        patch_key = next(key for key in catalog if key.rsplit('.', 1)[-1] == 'apply_patch')

        def execute(bound, call_id, name, value, report):
            return rpc('codex_execute', {**bound, 'call_id': call_id, 'name': name,
                'input': value if isinstance(value, str) else json.dumps(value), 'report': report})

        def command(bound, call_id, value):
            key = command_key.rsplit('.', 1)[-1]
            return execute(bound, call_id, command_key,
                {'cmd': value, 'login': False, 'max_output_tokens': 1000} if key == 'exec_command' else {'command': value},
                'OWNED_NATIVE_COMMAND_' + call_id)

        first = command(binding, 'first', 'printf original >> once.txt; printf NATIVE_RESULT_ONE')
        assert 'NATIVE_RESULT_ONE' in json.dumps(first), first
        assert command(binding, 'first', 'printf original >> once.txt; printf NATIVE_RESULT_ONE')['result'] == first['result']
        assert (root / 'w/once.txt').read_text() == 'original'
        print('Native command and exact retry returned once', flush=True)
        review_args={'cmd':str(reviewed),'login':False,'sandbox_permissions':'require_escalated',
            'justification':'Run only this owned disposable fixture program under independent native automatic review.'}
        execute(binding,'review-allow',command_key,review_args,'OWNED_INDEPENDENT_REVIEW_ALLOW')
        assert (root / 'w/reviewed-effects').read_text() == 'reviewed'
        (root / 'deny-review').touch()
        denied=execute(binding,'review-deny',command_key,review_args,'OWNED_INDEPENDENT_REVIEW_DENY')
        (root / 'deny-review').unlink()
        assert (root / 'w/reviewed-effects').read_text() == 'reviewed'
        assert 'denied' in json.dumps(denied).lower() or 'reject' in json.dumps(denied).lower(), denied
        review_commands=json.loads((root / 'bridge-commands.json').read_text())
        assert sum(row['op']=='submit' for row in review_commands) == 2
        assert all(row['messages'] for row in review_commands if row['op']=='submit')
        print('Independent native automatic reviewer allowed and prevented actual effects',flush=True)
        # Child sessions are independent work, never the primary Chat-renewal path.
        child = rpc('codex_sessions', {'action': 'fork', 'call_id': 'independent-child', 'thread_id': created['thread_id']})
        assert child['thread_id'] != created['thread_id']
        rpc('codex_sessions', {**{'work_id': child['work_id'], 'controller': child['controller']},
            'action': 'start', 'task': 'OWNED_INDEPENDENT_CHILD'})
        child_binding = {'work_id': child['work_id'], 'controller': child['controller']}
        wait(lambda: rpc('codex_status', {'work_id': child_binding['work_id']})['phase'] == 'running')
        attach(child_binding)
        command(child_binding, 'child-command', 'printf INDEPENDENT_CHILD_RESULT')
        rpc('codex_finish', {**child_binding, 'call_id': 'child-final', 'text': 'OWNED_INDEPENDENT_CHILD_COMPLETE'})
        assert not rpc('codex_status', {'work_id': binding['work_id']})['completed']
        execute(binding, 'patch', patch_key,
            '*** Begin Patch\n*** Add File: owned.py\n+def add(a, b):\n+    return a - b\n*** End Patch', 'OWNED_PATCH')
        failed = command(binding, 'failure', "python3 -c 'from owned import add; assert add(2, 3) == 5'")
        assert 'AssertionError' in json.dumps(failed), failed
        execute(binding, 'fix', patch_key,
            '*** Begin Patch\n*** Update File: owned.py\n@@\n-    return a - b\n+    return a + b\n*** End Patch', 'OWNED_FAILURE_CORRECTION')
        command(binding, 'test', "python3 -c 'from owned import add; assert add(2, 3) == 5; print(\"TEST_PASSED\")'")
        latest = rpc('codex_history', {'work_id': binding['work_id']})
        handoff = rpc('codex_handoff', {**binding, 'call_id': 'handoff-one', 'cursor': latest['cursor'], 'context_cursor':latest['context_cursor'],
            'summary': 'Implemented owned.py add after a real failed test and correction. Continue same thread; once.txt must remain original.', 'renew': False})
        new_binding = {'work_id': binding['work_id'], 'controller': handoff['controller']}
        assert handoff['thread_id'] == created['thread_id']
        assert rpc('codex_report', {**binding, 'call_id': 'stale', 'text': 'STALE_CONTROLLER'}, True)['error'] == 'operator_controller_fenced'
        attached = attach(new_binding)
        assert attached['summary'] and attached['thread_id'] == created['thread_id']
        # Upstream queues injection during an active turn; it becomes committed
        # context on the next native tool round, not at the injection RPC reply.
        native_rpc(home,'thread/inject_items',{'threadId':created['thread_id'],'items':[
            {'type':'message','role':'user','content':[{'type':'input_text','text':'OWNED_LATE_INSTRUCTION: preserve once.txt and continue from current history'}]}]})
        updated=command(new_binding,'late-context','printf LATE_CONTEXT_OBSERVED')
        assert 'OWNED_LATE_INSTRUCTION' in json.dumps(updated['context_updates'])
        assert rpc('codex_attach',new_binding)['receipt'] != attached['receipt']
        command(new_binding, 'after-handoff', "python3 -c 'from owned import add; assert add(3, 4) == 7; print(\"CONTINUED_SAME_THREAD\")'")
        rpc('codex_finish', {**new_binding, 'call_id': 'final', 'text': 'OWNED_CHAT_ENTRY_COMPLETE'})
        assert rpc('codex_finish', {**new_binding, 'call_id': 'final', 'text': 'OWNED_CHAT_ENTRY_COMPLETE'})['reused']
        assert rpc('codex_status', {'work_id': binding['work_id']})['completed']
        print('Incremental same-thread handoff and completion recorded', flush=True)

        # Same Provider now receives an actual native TUI-origin turn. Missing
        # explicit registration must fail before launching a default/API client.
        profile = tomllib.loads((home / '.codex/chatgpt.config.toml').read_text())
        assert profile['model_provider'] == 'chatgpt_desktop'
        route = profile['model_providers']['chatgpt_desktop']
        assert route['base_url'] == f'http://127.0.0.1:{port}/v1'
        assert route['requires_openai_auth'] is False
        assert route['auth']['command'] == str(root / 'fixture-auth')
        def launch(thread=None):
            master, slave = pty.openpty()
            masters.append(master)
            fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 42, 180, 0, 0))
            command = [str(prefix / 'bin/codex'), '--no-alt-screen', '--profile', 'chatgpt',
                       '-c', 'tui.animations=false']
            if thread is not None:
                command += ['resume', thread]
            terminal = subprocess.Popen(command, env=env, cwd=root / 'w',
                stdin=slave, stdout=slave, stderr=slave, start_new_session=True)
            os.close(slave)
            processes.append(terminal)
            return master, terminal

        master, terminal = launch()

        def drain(seconds=.1):
            nonlocal screen
            until = time.monotonic() + seconds
            while time.monotonic() < until:
                if select.select([master], [], [], .02)[0]:
                    try:
                        screen += os.read(master, 65536).decode(errors='replace')
                    except OSError:
                        break
            return screen

        wait(lambda: 'GPT-6' in drain())
        drain(1)
        os.write(master, b'OWNED_CODEX_ENTRY_DEVELOPMENT')
        drain(.8)
        os.write(master, b'\r')

        def driver_binding():
            drain()
            path = root / 'bridge-commands.json'
            if not path.exists():
                return None
            try:
                rows = json.loads(path.read_text())
            except json.JSONDecodeError:
                return None
            submissions = [v for v in rows if v['op'] == 'submit']
            if not submissions:
                return None
            text = '\n'.join(submissions[-1].get('instructions', []))
            match = re.search(r'work_id=([0-9a-f]{48}) and controller=([0-9a-f]{48})', text)
            if match:
                for message in submissions[-1]['messages']:
                    if message['role'] == 'user':
                        assert message['text'] in ['OWNED_CODEX_ENTRY_DEVELOPMENT','OWNED_CODEX_FOLLOWUP','OWNED_CODEX_AFTER_COMPACT','OWNED_AFTER_SERVICE_RESTART','OWNED_FAILED_HOST','OWNED_ENDED_HOST']
                return {'work_id': match[1], 'controller': match[2]}

        cli_binding = wait(driver_binding)
        attach(cli_binding)
        (root/'partial-host').touch()
        drain(1.5)
        assert rpc('codex_status',{'work_id':cli_binding['work_id']})['phase']=='running'
        (root/'partial-host').unlink()
        # App consent stays pending and visible. Reveal uses the same existing
        # authenticated HTTP surface, changes no inference and checks identity.
        (root/'host-confirmation').touch()
        wait(lambda:rpc('codex_status',{'work_id':cli_binding['work_id']})['host_state']=='waiting_confirmation')
        wait(lambda:'ChatGPT is waiting for tool permission' in drain())
        wait(lambda:'ChatGPT is waiting for tool permission' in json.dumps(rpc('codex_history',{'work_id':cli_binding['work_id']})))
        reveal_thread=rpc('codex_status',{'work_id':cli_binding['work_id']})['thread_id']
        def reveal(operation="reveal"):
            request=urllib.request.Request(f'http://127.0.0.1:{port}/__{operation}',data=b'',headers={
                'Authorization':'Bearer '+TOKEN,'X-Codex-Thread-Id':reveal_thread})
            with urllib.request.urlopen(request,timeout=15) as response:return json.load(response)
        revealed=reveal()
        assert revealed['app_reveal_requested'] and not revealed['inference_submitted'] and not revealed['physical_ui_confirmed']
        approved = reveal('approve')
        assert approved['app_approval_requested'] and not approved['window_open_requested'] and not approved['native_completion_confirmed']
        assert approved['scope']=='conversation' and approved['path']=='action'
        (root/'approval-ui-required').touch()
        approval_started=time.monotonic()
        fallback=reveal('approve')
        assert time.monotonic()-approval_started>=.3, 'UI approval returned before its real reply observation'
        assert fallback['scope']=='conversation' and fallback['path']=='ui' and fallback['window_open_requested']
        (root/'approval-ui-required').unlink()
        commands_before=len(json.loads((root/'bridge-commands.json').read_text()))
        (root/'approval-unknown').touch()
        try:
            reveal('approve')
            raise AssertionError('uncertain approval was admitted')
        except urllib.error.HTTPError as error:
            assert error.code==409 and json.load(error)['error']['code']=='app_approval_unconfirmed'
        after=json.loads((root/'bridge-commands.json').read_text())[commands_before:]
        assert not any(command['op']=='approve_ui' for command in after), 'uncertain approval triggered UI retry'
        (root/'approval-unknown').unlink()
        (root / 'changed-account').touch()
        for operation in ['reveal', 'approve', 'approve_ui']:
            try:
                reveal(operation)
                raise AssertionError('app control admitted another account')
            except urllib.error.HTTPError as error:
                assert error.code==409 and json.load(error)['error']['code']=='app_account_changed'
        denied = rpc('codex_execute', {**cli_binding, 'call_id': 'wrong-account', 'name': command_key,
            'input': json.dumps({'cmd': 'touch wrong-account-effect', 'login': False}), 'report': 'must refuse'}, True)
        assert denied['error'] == 'app_account_changed'
        (root / 'changed-account').unlink()
        assert not (root / 'w/wrong-account-effect').exists()
        (root/'host-confirmation').unlink()
        (root/'host-authentication').touch()
        wait(lambda:rpc('codex_status',{'work_id':cli_binding['work_id']})['host_state']=='waiting_authentication')
        wait(lambda:'ChatGPT is waiting for authentication' in drain())
        (root/'host-authentication').unlink()
        print('Host permission/authentication waits reached CLI/history; owned reveal made no inference and rejected another account',flush=True)
        (root / 'unobservable-host').touch()
        lost = rpc('codex_status', {'work_id': cli_binding['work_id']})
        assert not lost['completed'] and not lost['host_observation_available']
        assert lost['host_progress'] == 'unknown' and lost['host_observation'] is None
        (root / 'unobservable-host').unlink()
        rpc('codex_report', {**cli_binding, 'call_id': 'initial', 'text': 'OWNED_LIVE_INITIAL'})
        wait(lambda: 'OWNED_LIVE_INITIAL' in drain())
        command(cli_binding, 'cli-tool', 'printf CLI_TOOL_RESULT')
        dropped_args = {**cli_binding, 'call_id': 'lost-observer', 'name': command_key,
            'input': json.dumps({'cmd': 'printf once >> observer-once; sleep 2; printf OBSERVER_RESULT', 'login': False}),
            'report': 'OWNED_DROPPED_HTTP_OBSERVER'}
        body = json.dumps({'jsonrpc': '2.0', 'id': 'detached', 'method': 'tools/call',
            'params': {'name': 'codex_execute', 'arguments': dropped_args}}).encode()
        observer = socket.create_connection(('127.0.0.1', port))
        observer.sendall((f'POST /mcp HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {TOKEN}\r\n'
            f'Content-Type: application/json\r\nContent-Length: {len(body)}\r\nConnection: close\r\n\r\n').encode() + body)
        wait(lambda: (root / 'w/observer-once').exists())
        observer.close()
        result = rpc('codex_execute', dropped_args)
        assert 'OBSERVER_RESULT' in json.dumps(result)
        assert (root / 'w/observer-once').read_text() == 'once'
        rpc('codex_report', {**cli_binding, 'call_id': 'middle', 'text': 'OWNED_LIVE_INTERMEDIATE'})
        wait(lambda: 'OWNED_LIVE_INTERMEDIATE' in drain())
        latest = rpc('codex_history', {'work_id': cli_binding['work_id']})
        thread_before = rpc('codex_status', {'work_id': cli_binding['work_id']})['thread_id']
        rpc('codex_handoff', {**cli_binding, 'call_id': 'automatic-handoff', 'cursor': latest['cursor'], 'context_cursor':latest['context_cursor'],
            'summary': 'Native CLI work ran CLI_TOOL_RESULT; retain its return binding and finish after confirming continuation.', 'renew': True})
        successor = wait(lambda: (value if value != cli_binding else None) if (value := driver_binding()) else None)
        attach(successor)
        assert rpc('codex_status', {'work_id': successor['work_id']})['thread_id'] == thread_before
        command(successor, 'successor-tool', 'printf SUCCESSOR_RESULT')
        rpc('codex_finish', {**successor, 'call_id': 'cli-final', 'text': 'OWNED_LIVE_FINAL'})
        wait(lambda: 'OWNED_LIVE_FINAL' in drain())
        history = rpc('codex_history', {'work_id': successor['work_id']})
        for marker in ['OWNED_LIVE_INITIAL', 'OWNED_LIVE_INTERMEDIATE', 'OWNED_LIVE_FINAL', 'CLI_TOOL_RESULT', 'SUCCESSOR_RESULT']:
            assert marker in json.dumps(history), marker
        print('Codex entrance: live initial/middle/final, native tools and automatic synthetic Chat renewal passed', flush=True)
        commands_before = json.loads((root / 'bridge-commands.json').read_text())
        opens_before = sum(row['op'] == 'open' for row in commands_before)
        submissions_before = sum(row['op'] == 'submit' for row in commands_before)
        cold_proof = None
        cold_resume = getattr(args, 'cold_resume', False)
        if cold_resume:
            original_work = work_record(successor['work_id'])
            original = native_rpc(home, 'thread/read', {'threadId':thread_before, 'includeTurns':True})['thread']
            assert original['turns'] and all(turn['status']=='completed' for turn in original['turns'])
            (root / 'before-cold-native.json').write_text(json.dumps(original, indent=2) + '\n')
            executable = Path(f'/proc/{terminal.pid}/exe').resolve(strict=True)
            assert executable.is_relative_to(home / '.local/lib/codex/core/generations/local-permission-fixture') or executable == prefix / 'bin/codex'
            (root / 'before-cold-terminal.txt').write_text(screen)
            owners = [stop_owned_pid(terminal.pid, executable)]
            terminal.wait(timeout=10)
            for path in server_paths(home):
                owners.append(stop_owned_pid(int(path.with_name('pid').read_text()),
                    home / '.local/lib/codex/core/generations/local-permission-fixture/runtime'))
            assert len(owners)>1, 'cold proof requires an observed backend exit'
            screen = ''
            master, terminal = launch(thread_before)
            wait(lambda: 'GPT-6' in drain())
            drain(1)
            resumed_native = native_rpc(home, 'thread/read', {'threadId':thread_before, 'includeTurns':True})['thread']
            assert resumed_native['turns'] == original['turns'], 'resume changed completed native turns'
            assert sum(row['op']=='submit' for row in json.loads((root/'bridge-commands.json').read_text()))==submissions_before
            # Observe the replacement before the genuinely new turn. Completed
            # work stays completed; an unsettled phase requires reconciliation.
            observed = rpc('codex_status', {'work_id':successor['work_id']})
            assert observed['phase'] in ['completed','reconciliation_required'], observed
            assert work_record(successor['work_id'])['execution']==original_work['execution']
            cold_proof = {'kernel_exits':owners, 'same_native_thread':thread_before,
                          'resume_replayed_inputs':0, 'retained_turns':len(original['turns']),
                          'status_observed_replacement_before_admission':True,
                          'phase_before_admission':observed['phase']}
        else:
            (root / 'hold-host-completion').touch()
        os.write(master, b'OWNED_CODEX_FOLLOWUP')
        drain(.4)
        os.write(master, b'\r')
        if not cold_resume:
            wait(lambda: rpc('codex_status', {'work_id':successor['work_id']})['driver_phase']=='waiting_previous_completion')
            drain(.5)
            assert sum(row['op']=='submit' for row in json.loads((root/'bridge-commands.json').read_text()))==submissions_before
            (root / 'hold-host-completion').unlink()
        followup = wait(lambda: (value if value != successor else None) if (value := driver_binding()) else None)
        assert followup['work_id']==successor['work_id']
        assert rpc('codex_attach', successor, True)['error']=='operator_controller_fenced'
        attach(followup)
        rpc('codex_finish', {**followup, 'call_id': 'followup-final', 'text': 'OWNED_FOLLOWUP_COMPLETE'})
        wait(lambda: 'OWNED_FOLLOWUP_COMPLETE' in drain())
        commands_after = json.loads((root / 'bridge-commands.json').read_text())
        opens = [row['lane'] for row in commands_after if row['op'] == 'open']
        assert len(opens) == opens_before + 1
        if cold_resume:
            expected_reuse = observed['phase']=='completed'
            assert (opens[-1] == opens[-2]) == expected_reuse
            rebound = work_record(followup['work_id'])
            assert rebound['execution']!=original_work['execution']
            assert rebound['execution']['home']==original_work['execution']['home']
            assert rebound['generation']>original_work['generation']
            continued = native_rpc(home, 'thread/read', {'threadId':thread_before, 'includeTurns':True})['thread']
            assert continued['turns'][:-1] == original['turns']
            assert continued['turns'][-1]['status']=='completed'
            cold_proof.update(new_turn_completed=True, former_controller_rejected=True,
                              native_owner_rebound=True, settled_chat_reused=expected_reuse)
            (root / 'cold-resume-review.json').write_text(json.dumps(cold_proof, indent=2) + '\n')
            print('Cold original TUI resume preserved history, rebound its owner and completed a distinct turn; former controller fenced', flush=True)
        else:
            assert opens[-1] == opens[-2]
            print('Later native user turn reused the same settled Chat', flush=True)
        rpc('codex_sessions',{**followup,'action':'compact'})
        wait(lambda:rpc('codex_status',{'work_id':followup['work_id']})['phase']=='native_summary')
        compact_result=rpc('codex_native_summary',{**followup,'summary':'Preserve OWNED_CODEX_ENTRY_DEVELOPMENT, CLI_TOOL_RESULT, SUCCESSOR_RESULT and completed follow-up. Continue in the same primary thread.'})
        assert compact_result['native_summary_offered'] and not compact_result['chat_replaced']
        wait(lambda:rpc('codex_status',{'work_id':followup['work_id']})['native_status']['type']=='idle')
        os.write(master,b'OWNED_CODEX_AFTER_COMPACT')
        drain(.8)
        os.write(master,b'\r')
        after_compact=wait(lambda:(value if value!=followup else None) if (value:=driver_binding()) else None)
        attach(after_compact)
        rpc('codex_finish',{**after_compact,'call_id':'after-compact-final','text':'OWNED_NATIVE_COMPACT_CONTINUED'})
        wait(lambda:'OWNED_NATIVE_COMPACT_CONTINUED' in drain())
        print('Native inference compaction preserved the same Chat and subsequent task continuity',flush=True)

        (root / 'hold-host-completion').touch()
        submissions_before=sum(row['op']=='submit' for row in json.loads((root/'bridge-commands.json').read_text()))
        os.write(master,b'OWNED_CANCEL_WAITING_FOLLOWUP');drain(.4);os.write(master,b'\r')
        wait(lambda:rpc('codex_status',{'work_id':after_compact['work_id']})['driver_phase']=='waiting_previous_completion')
        queued_stop=rpc('codex_stop',{'work_id':after_compact['work_id']})
        assert queued_stop['native_stop_confirmed'] and queued_stop['host_stop_confirmed']
        (root/'hold-host-completion').unlink();drain(.5)
        assert sum(row['op']=='submit' for row in json.loads((root/'bridge-commands.json').read_text()))==submissions_before
        print('Terminal-only completion waits for app settlement; Stop fences the queued follow-up without a late dispatch',flush=True)

        stopped = rpc('codex_sessions', {'action': 'create', 'call_id': 'stop-entry-create'})
        stop_binding = {'work_id': stopped['work_id'], 'controller': stopped['controller']}
        rpc('codex_sessions', {**stop_binding, 'action': 'start', 'task': 'OWNED_INTERRUPT_RECOVERY'})
        wait(lambda: rpc('codex_status', {'work_id': stop_binding['work_id']})['phase'] == 'running')
        attach(stop_binding)
        execute(stop_binding, 'long-running', command_key,
            {'cmd': 'sleep 30; printf UNEXPECTED > never.txt', 'login': False, 'yield_time_ms': 1000},
            'OWNED_LONG_RUNNING_COMMAND')
        current = rpc('codex_history', {'work_id': stop_binding['work_id']})
        assert rpc('codex_handoff', {**stop_binding, 'call_id': 'busy-handoff',
            'summary': 'Must not transfer running work silently', 'cursor': current['cursor'],'context_cursor':current['context_cursor']}, True)['error'] == 'operator_background_jobs_unsettled'
        stopped_result = rpc('codex_stop', {'work_id': stop_binding['work_id']})
        assert stopped_result['fenced'] and stopped_result['native_stop_confirmed'], stopped_result
        assert not stopped_result['host_stop_requested'] and not stopped_result['host_stop_confirmed'], 'no recorded host is not a confirmed host Stop'
        assert not (root / 'w/never.txt').exists()
        assert rpc('codex_report', {**stop_binding, 'call_id': 'after-stop', 'text': 'BAD'}, True)['error'] == 'operator_controller_fenced'
        latest = rpc('codex_history', {'work_id': stop_binding['work_id']})
        recovered = rpc('codex_sessions', {'action': 'claim', 'work_id': stop_binding['work_id'], 'cursor': latest['cursor'], 'context_cursor': latest['context_cursor']})
        resumed = {'work_id': stop_binding['work_id'], 'controller': recovered['controller']}
        assert recovered['thread_id'] == stopped['thread_id']
        rpc('codex_sessions', {**resumed, 'action': 'start', 'task': 'Continue from the actual stopped state; do not replay the command'})
        wait(lambda: rpc('codex_status', {'work_id': resumed['work_id']})['phase'] == 'running')
        attach(resumed)
        command(resumed, 'resumed-test', 'test ! -e never.txt; printf RECOVERED_NATIVE_STATE')
        rpc('codex_finish', {**resumed, 'call_id': 'recovered-final', 'text': 'OWNED_STOP_RECOVERY_COMPLETE'})
        print('Owned job stop, stale-controller fence and same-thread explicit recovery passed', flush=True)

        released_thread = rpc('codex_status', {'work_id': after_compact['work_id']})['thread_id']
        closed_bridge_commands = restart_fixture()
        assert rpc('codex_attach', resumed, True)['error'] == 'operator_controller_fenced'
        latest = rpc('codex_history', {'work_id': resumed['work_id']})
        claimed = rpc('codex_sessions', {'action': 'claim', 'work_id': resumed['work_id'], 'cursor': latest['cursor'], 'context_cursor': latest['context_cursor']})
        assert claimed['thread_id'] == stopped['thread_id']
        released_history = rpc('codex_history', {'work_id': after_compact['work_id']})
        released_claim = rpc('codex_sessions', {'action': 'claim', 'work_id': after_compact['work_id'], 'cursor': released_history['cursor'], 'context_cursor': released_history['context_cursor']})
        assert released_claim['thread_id'] == released_thread and not released_claim['inference_submitted']
        restarted_stop = rpc('codex_stop', {'work_id': after_compact['work_id']})
        assert restarted_stop['native_stop_confirmed'] and restarted_stop['host_stop_confirmed'], restarted_stop
        assert not restarted_stop['host_stop_requested'] and not restarted_stop['remaining_jobs'], restarted_stop
        assert json.loads((root / 'bridge-commands.json').read_text()) == closed_bridge_commands, 'recorded released Stop must not recreate or address an app lane'
        assert (root / 'bridge-commands.json').exists()
        print('Same-endpoint restart, current-history claim and recorded released-host Stop passed without app dispatch', flush=True)
        os.write(master, b'OWNED_AFTER_SERVICE_RESTART')
        drain(.8)
        os.write(master, b'\r')
        restarted_binding = wait(lambda: (value if value['work_id'] == after_compact['work_id'] and value != after_compact else None) if (value := driver_binding()) else None)
        assert rpc('codex_status', {'work_id': restarted_binding['work_id']})['thread_id'] == released_thread
        attach(restarted_binding)
        command(restarted_binding, 'after-released-host', 'printf SAME_THREAD_AFTER_RELEASE')
        rpc('codex_finish', {**restarted_binding, 'call_id': 'after-released-host-final', 'text': 'OWNED_RELEASE_RESTART_CONTINUED'})
        wait(lambda: 'OWNED_RELEASE_RESTART_CONTINUED' in drain())
        print('New genuine user turn continued on the same thread after acknowledged host release', flush=True)
        start_failure_proof = None
        if cold_resume:
            # The synthetic app declares uncertainty before any real Chat call.
            # The native response must fail explicitly, with no replay or park.
            before_failure = native_rpc(home, 'thread/read', {'threadId':released_thread,'includeTurns':True})['thread']
            submits_before = sum(row['op']=='submit' for row in json.loads((root/'bridge-commands.json').read_text()))
            (root/'reject-submit').touch()
            failure_started = time.monotonic()
            os.write(master,b'OWNED_DECLARED_START_FAILURE');drain(.4);os.write(master,b'\r')
            def failed_native_turn():
                drain(.05)
                current = native_rpc(home, 'thread/read', {'threadId':released_thread,'includeTurns':True})['thread']
                if len(current['turns'])==len(before_failure['turns'])+1 and current['turns'][-1]['status']=='failed':
                    return current
            failed_native = wait(failed_native_turn, timeout=12)
            assert 'operator_app_start_unconfirmed' in json.dumps(failed_native['turns'][-1]), failed_native['turns'][-1]
            assert failed_native['turns'][:-1]==before_failure['turns']
            submissions = sum(row['op']=='submit' for row in json.loads((root/'bridge-commands.json').read_text()))
            assert submissions==submits_before+1, 'declared start failure was retried'
            (root/'reject-submit').unlink()
            stopped_failure = rpc('codex_stop', {'work_id':restarted_binding['work_id']})
            assert stopped_failure['native_stop_confirmed'] and stopped_failure['host_stop_confirmed'] and not stopped_failure['remaining_jobs']
            start_failure_proof = {'elapsed_seconds':time.monotonic()-failure_started,
                'original_native_turn_failed_explicitly':True,'synthetic_submit_attempts':1,
                'retries':0,'stop_confirmed':True}
            (root/'start-failure-native.json').write_text(json.dumps(failed_native,indent=2)+'\n')
            (root/'start-failure-review.json').write_text(json.dumps(start_failure_proof,indent=2)+'\n')
            print('Async driver-start uncertainty failed the original native response promptly with one synthetic attempt and confirmed Stop',flush=True)
        # Plan-mode question returns to its original native call, not a new user turn.
        question_work = rpc('codex_sessions', {'action': 'create', 'call_id': 'question-create'})
        question_binding = {'work_id': question_work['work_id'], 'controller': question_work['controller']}
        rpc('codex_sessions', {**question_binding, 'action': 'start', 'mode': 'plan', 'task': 'Ask the user the owned question'})
        wait(lambda: rpc('codex_status', {'work_id': question_binding['work_id']})['phase'] == 'running')
        attach(question_binding)
        question_tools = rpc('codex_tools', question_binding)['tools']
        question_key = next(key for key in question_tools if key.rsplit('.', 1)[-1] == 'request_user_input')
        pending_answer = pool.submit(execute, question_binding, 'owned-question', question_key,
            {'questions': [{'id': 'owned_choice', 'header': 'Fixture', 'question': 'Choose the owned fixture answer',
                'options': [{'label': 'Alpha (Recommended)', 'description': 'Use Alpha.'}, {'label': 'Beta', 'description': 'Use Beta.'}]}]},
            'OWNED_USER_QUESTION')
        question = wait(lambda: next(iter(rpc('codex_status', {'work_id': question_binding['work_id']})['human_requests']), None))
        rpc('codex_answer', {**question_binding, 'request_id': json.dumps(question['id'], separators=(',', ':')),
            'answers': {'owned_choice': {'answers': ['Alpha (Recommended)']}}})
        assert 'Alpha (Recommended)' in json.dumps(pending_answer.result(timeout=30))
        rpc('codex_finish', {**question_binding, 'call_id': 'question-final', 'text': 'OWNED_QUESTION_COMPLETE'})
        print('Native question returned its actual answer on the original tool call', flush=True)
        # A failed successor does not become a new primary thread or resend work.
        failed_work=rpc('codex_sessions',{'action':'create','call_id':'failed-renewal-create'})
        failed_binding={'work_id':failed_work['work_id'],'controller':failed_work['controller']}
        rpc('codex_sessions',{**failed_binding,'action':'start','task':'OWNED_FAILED_SUCCESSOR'})
        wait(lambda:rpc('codex_status',{'work_id':failed_binding['work_id']})['phase']=='running')
        attach(failed_binding)
        command(failed_binding,'before-failed-renewal','printf once > failed-renewal-once')
        current=rpc('codex_history',{'work_id':failed_binding['work_id']})
        (root / 'reject-submit').touch()
        rpc('codex_handoff',{**failed_binding,'call_id':'failed-renewal','cursor':current['cursor'],'context_cursor':current['context_cursor'],
            'summary':'The owned command ran once. Continue from recorded state without replay.','renew':True})
        wait(lambda:rpc('codex_status',{'work_id':failed_binding['work_id']})['driver_phase']=='handoff_start_unconfirmed')
        (root / 'reject-submit').unlink()
        current=rpc('codex_history',{'work_id':failed_binding['work_id']})
        takeover=rpc('codex_sessions',{'action':'takeover','work_id':failed_binding['work_id'],'cursor':current['cursor'],'context_cursor':current['context_cursor']})
        takeover_binding={'work_id':takeover['work_id'],'controller':takeover['controller']}
        assert takeover['thread_id']==failed_work['thread_id'] and not takeover['inference_submitted']
        attach(takeover_binding)
        rpc('codex_finish',{**takeover_binding,'call_id':'takeover-final','text':'OWNED_FAILED_SUCCESSOR_RECOVERED'})
        assert (root / 'w/failed-renewal-once').read_text()=='once'
        print('Uncertain successor preserved the original work and allowed explicit MCP takeover without replay',flush=True)
        # Stop cannot confirm a host while its in-flight start can still return.
        (root / 'hold-submit').write_text('hold owned synthetic dispatch')
        prior=json.loads((root / 'bridge-commands.json').read_text())
        drain()
        os.write(master,b'OWNED_START_STOP_RACE')
        time.sleep(.8)
        os.write(master,b'\r')
        wait(lambda:rpc('codex_status',{'work_id':after_compact['work_id']})['phase']=='running')
        wait(lambda:len([row for row in json.loads((root / 'bridge-commands.json').read_text()) if row['op']=='submit']) > len([row for row in prior if row['op']=='submit']))
        future=pool.submit(rpc,'codex_stop',{'work_id':after_compact['work_id']})
        wait(lambda:rpc('codex_status',{'work_id':after_compact['work_id']})['phase']=='stop_requested')
        time.sleep(.2)
        assert not future.done(),'host stop acknowledged while submit remained in flight'
        (root / 'hold-submit').unlink()
        stopped_race=future.result(timeout=25)
        assert stopped_race['native_stop_confirmed'] and stopped_race['host_stop_confirmed'],stopped_race
        after=json.loads((root / 'bridge-commands.json').read_text())
        time.sleep(.2)
        assert json.loads((root / 'bridge-commands.json').read_text())==after
        print('Concurrent host start/Stop serialized; no late dispatch after confirmation',flush=True)

        # An actual app-reported failure reaches the original native CLI and
        # durable history without a silent model wait or a successful final.
        (root / 'failed-host').touch()
        drain(.5)
        os.write(master,b'OWNED_FAILED_HOST')
        drain(.8)
        os.write(master,b'\r')
        wait(lambda:rpc('codex_status',{'work_id':after_compact['work_id']})['phase']=='reconciliation_required')
        wait(lambda:'operator_app_round_failed' in drain())
        failure_status=rpc('codex_status',{'work_id':after_compact['work_id']})
        assert failure_status['host_observation']['error']=='app_submit_failed'
        assert isinstance(failure_status['host_observation']['dispatches'],int)
        wait(lambda:'ChatGPT app reported a failure' in json.dumps(rpc('codex_history',{'work_id':after_compact['work_id']})))
        assert not rpc('codex_status',{'work_id':after_compact['work_id']})['completed']
        (root / 'failed-host').unlink()
        failed_stop=rpc('codex_stop',{'work_id':after_compact['work_id']})
        assert failed_stop['native_stop_confirmed']
        boundary=rpc('codex_history',{'work_id':after_compact['work_id']})
        failed_claim=rpc('codex_sessions',{'action':'claim','work_id':after_compact['work_id'],'cursor':boundary['cursor'],'context_cursor':boundary['context_cursor']})
        assert not failed_claim['inference_submitted']
        print('Observed host failure reached live CLI/native history; failed native state stopped/claimed without replay or false completion',flush=True)

        # Explicit host completion without the native control finish is a failed
        # round. Use a separate fresh native task for this failure scenario;
        # preserve the preceding stopped/claimed thread as recovery evidence.
        os.killpg(terminal.pid, signal.SIGTERM)
        terminal.wait(timeout=10)
        os.close(master)
        masters.remove(master)
        master,slave=pty.openpty()
        masters.append(master)
        fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',42,180,0,0))
        offset=len(screen)
        terminal=subprocess.Popen([str(prefix/'bin/codex'),'--no-alt-screen','--profile','chatgpt'],
            env=env,cwd=root/'w',stdin=slave,stdout=slave,stderr=slave,start_new_session=True)
        os.close(slave)
        processes.append(terminal)
        wait(lambda:'GPT-6' in drain()[offset:])
        drain(1)
        os.write(master,b'OWNED_ENDED_HOST')
        drain(.8)
        os.write(master,b'\r')
        def ended_binding():
            rows=json.loads((root/'bridge-commands.json').read_text())
            submissions=[row for row in rows if row['op']=='submit']
            return driver_binding() if any(message['text']=='OWNED_ENDED_HOST' for message in submissions[-1]['messages']) else None
        ended=wait(ended_binding)
        wait(lambda:rpc('codex_status',{'work_id':ended['work_id']})['phase']=='reconciliation_required')
        wait(lambda:'operator_app_round_incomplete' in drain())
        wait(lambda:'ChatGPT turn ended without completing' in json.dumps(rpc('codex_history',{'work_id':ended['work_id']})))
        assert not rpc('codex_status',{'work_id':ended['work_id']})['completed']
        assert rpc('codex_stop',{'work_id':ended['work_id']})['native_stop_confirmed']
        print('Explicit host completion without native finish reached CLI/history as incomplete, with no replay',flush=True)

        selected_proof = None
        if getattr(args, 'inputs', None) or getattr(args, 'selected_controls', False):
            # Additional new-control integration proof on real native Android
            # threads; synthetic reasoning never counts as F live agent proof.
            before_selected = len(json.loads((root / 'bridge-commands.json').read_text()))
            budget = {'max_parallel': 2, 'max_app_starts': 12,
                      'max_native_calls': 24, 'max_elapsed_seconds': 240}

            def selected(label, parent=None, dependencies=None, reasoning='direct', parallel=None):
                workspace = root / ('work-' + label)
                workspace.mkdir()
                request = {'action': 'create', 'call_id': 'selected-create-' + label,
                           'reasoning': reasoning, 'workspace': str(workspace),
                           'role': label, 'budget': {**budget, **({'max_parallel':parallel} if parallel is not None else {})}}
                if parent:
                    request.update(parent_work_id=parent['work_id'], parent_controller=parent['controller'])
                if dependencies:
                    request['dependencies'] = dependencies
                created = rpc('codex_sessions', request)
                return {'work_id': created['work_id'], 'controller': created['controller']}, created

            def begin(bound, label):
                started = rpc('codex_sessions', {**bound, 'action': 'start',
                              'call_id': 'selected-start-' + label, 'task': 'Owned selected task ' + label})
                bound = {**bound, 'controller': started['controller']}
                wait(lambda: rpc('codex_status', {'work_id': bound['work_id']})['phase'] == 'running')
                attach(bound)
                return bound

            # Project-free synthetic recipes retain their actual scope. The
            # project-configured service preflight separately proves managed
            # status/Stop before its first turn, without submitting inference.
            managed_status_proved = False
            configured_history = (tomllib.loads(args.config.read_text()).get('history', {})
                                  if getattr(args, 'config', None) else {})
            if configured_history.get('mode') == 'project':
                unstarted, _ = selected('managed-status', reasoning='provider')
                unstarted_status = rpc('codex_status', {'work_id': unstarted['work_id']})
                assert not unstarted_status['completed'] and unstarted_status['app_lane'] is None
                initial_events = rpc('codex_sessions', {'action': 'events', 'work_id': unstarted['work_id']})
                assert not initial_events['status']['completed'] and initial_events['events'] == []
                if initial_events['history_observation_available'] is False:
                    assert initial_events['history_observation_error'] == 'native_history_not_ready'
                    assert initial_events['delivery'] == 'pending_native_history'
                initial_wait = rpc('codex_sessions', {'action': 'wait', 'work_id': unstarted['work_id'], 'max_wait_seconds': 0})
                assert initial_wait['wait_reason'] == 'pending' and initial_wait['events'] == []
                assert initial_wait['cursor'] == initial_events['cursor']
                assert rpc('codex_stop', {'work_id': unstarted['work_id']})['native_stop_confirmed']
                managed_status_proved = True
            else:
                missing = rpc('codex_sessions', {'action': 'create', 'call_id': 'managed-without-project',
                    'reasoning': 'provider', 'role': 'managed-status', 'budget': budget}, True)
                assert missing['error'] == 'provider_project_required'

            independent_a, _ = selected('independent-a', parallel=1)
            independent_a = begin(independent_a, 'independent-a')
            independent_b, _ = selected('independent-b', parallel=1)
            independent_b = begin(independent_b, 'independent-b')
            assert rpc('codex_status', {'work_id':independent_a['work_id']})['phase'] == 'running'
            assert rpc('codex_stop', {'work_id':independent_a['work_id']})['native_stop_confirmed']
            assert rpc('codex_status', {'work_id':independent_b['work_id']})['phase'] == 'running'
            command(independent_b, 'independent-effect', 'printf independent > independent.txt')
            rpc('codex_finish', {**independent_b,'call_id':'independent-final','text':'INDEPENDENT_ROOT_COMPLETE'})
            restart_fixture()
            assert rpc('codex_status', {'work_id':independent_a['work_id']})['phase'] == 'stopped'
            assert rpc('codex_status', {'work_id':independent_b['work_id']})['phase'] == 'completed'
            # These old threads are unloaded after restart. Stop must observe
            # their original jobs without a new user turn or model submission.
            unloaded_stop = rpc('codex_stop', {'work_id':independent_b['work_id']})
            assert unloaded_stop['native_stop_confirmed'] and unloaded_stop['remaining_jobs'] == [], unloaded_stop
            assert rpc('codex_status', {'work_id':independent_b['work_id']})['phase'] == 'stopped'

            coordinator, original = selected('coordinator')
            coordinator = begin(coordinator, 'coordinator')
            command(coordinator, 'selected-source', 'printf source > source.txt')
            rpc('codex_finish', {**coordinator, 'call_id': 'selected-source-final', 'text': 'SELECTED_SOURCE_COMPLETE'})
            observed = rpc('codex_sessions', {'action': 'wait', 'work_id': coordinator['work_id'], 'max_wait_seconds': 1})
            assert observed['wait_reason'] == 'completed' and observed['status']['completed']
            restart_fixture()
            unloaded = rpc('codex_status', {'work_id': coordinator['work_id']})
            assert unloaded['native_status']['type'] == 'notLoaded', unloaded
            source = rpc('codex_history', {'work_id': coordinator['work_id']})
            extended = rpc('codex_sessions', {'work_id':coordinator['work_id'], 'action':'claim', 'call_id':'selected-budget-extension',
                'cursor':source['cursor'], 'context_cursor':source['context_cursor'], 'budget':{**budget,'max_elapsed_seconds':360,'max_native_calls':1000}})
            assert extended['thread_id']==original['thread_id'] and extended['assignment']['budget']['max_native_calls']==1000
            retained_start=extended['assignment']['started_at_unix_ms']
            retained_calls=extended['assignment']['admitted_calls']
            assert retained_start is not None and retained_calls
            coordinator={'work_id':coordinator['work_id'],'controller':extended['controller']}
            coordinator=begin(coordinator,'extended-time')
            command(coordinator,'selected-extended-check','test "$(cat source.txt)" = source')
            rpc('codex_finish',{**coordinator,'call_id':'selected-extended-final','text':'EXTENDED_BUDGET_COMPLETE'})
            source = rpc('codex_history', {'work_id': coordinator['work_id']})
            offered = rpc('codex_sessions', {**coordinator, 'action': 'transfer', 'call_id': 'selected-transfer',
                          'reasoning': 'direct', 'text': 'source.txt contains source; continue this same native work.',
                          'cursor': source['cursor'], 'context_cursor': source['context_cursor']})
            rpc('codex_tools', coordinator, True)
            successor = {'work_id': coordinator['work_id'], 'controller': offered['controller']}
            rpc('codex_sessions', {**successor, 'action': 'start', 'task': 'must remain fenced'}, True)
            accept = {**successor, 'action': 'transfer_accept', 'call_id': 'selected-transfer-accept',
                      'transfer_receipt': offered['offer']['receipt'], 'cursor': source['cursor'],
                      'context_cursor': source['context_cursor']}
            rpc('codex_sessions', {**accept, 'call_id': 'selected-bad-receipt', 'transfer_receipt': 'wrong'}, True)
            accepted = rpc('codex_sessions', accept)
            assert accepted['accepted'] and accepted['thread_id'] == original['thread_id']
            coordinator = begin(successor, 'successor')
            command(coordinator, 'selected-successor', 'test "$(cat source.txt)" = source; printf successor >> source.txt')
            rpc('codex_finish', {**coordinator, 'call_id': 'selected-successor-final', 'text': 'SELECTED_SUCCESSOR_COMPLETE'})
            a, _ = selected('writer-a', coordinator)
            b, _ = selected('writer-b', coordinator)
            with concurrent.futures.ThreadPoolExecutor(max_workers=2) as writers:
                fa = writers.submit(begin, a, 'writer-a')
                fb = writers.submit(begin, b, 'writer-b')
                a, b = fa.result(timeout=45), fb.result(timeout=45)
            queued, _ = selected('writer-queued', coordinator)
            with concurrent.futures.ThreadPoolExecutor(max_workers=1) as admission:
                waiting = admission.submit(begin, queued, 'writer-queued')
                wait(lambda: rpc('codex_status', {'work_id':queued['work_id']})['phase'] == 'queued')
                assert not waiting.done(), 'same-parent parallel limit failed'
                assert rpc('codex_stop', {'work_id': a['work_id']})['native_stop_confirmed']
                queued = waiting.result(timeout=45)
            assert rpc('codex_stop', {'work_id': queued['work_id']})['native_stop_confirmed']
            assert rpc('codex_status', {'work_id': b['work_id']})['phase'] == 'running'
            command(b, 'selected-writer-b', 'printf isolated > result.txt')
            rpc('codex_finish', {**b, 'call_id': 'selected-writer-b-final', 'text': 'ISOLATED_WRITER_COMPLETE'})
            h = rpc('codex_history', {'work_id': b['work_id']})
            evaluated = rpc('codex_sessions', {**b, 'action': 'evaluate', 'call_id': 'selected-evaluation',
                            'decision': 'accepted', 'text': 'Synthetic independent fixture inspection: isolated result.txt has expected bytes.',
                            'cursor': h['cursor']})
            assert evaluated['evaluation']['decision'] == 'accepted'
            dependent, _ = selected('dependent-review', coordinator, [b['work_id']])
            dependent = begin(dependent, 'dependent-review')
            tree = rpc('codex_sessions', {**coordinator, 'action': 'stop_tree'})
            assert tree['confirmed'] and len(tree['work']) == 5, tree
            assert (root / 'work-coordinator/source.txt').read_text() == 'sourcesuccessor'
            assert (root / 'work-writer-b/result.txt').read_text() == 'isolated'
            assert not (root / 'work-writer-a/result.txt').exists()
            after_selected = json.loads((root / 'bridge-commands.json').read_text())[before_selected:]
            assert not any(row['op'] == 'submit' for row in after_selected), 'direct selected work acquired managed reasoning'
            selected_proof = {'same_thread_successor': original['thread_id'], 'source_fenced_until_receipt': True,
                              'two_isolated_native_writers': True, 'same_parent_limit_queued_third_writer':True, 'independent_roots_progress_with_parallel_one':True, 'terminal_stop_survived_restart':True, 'per_task_stop_preserved_sibling': True,
                              'evaluated_dependency': True, 'tree_stop_confirmed_descendants': 5,
                              'claim_reloaded_observed_notLoaded_thread': True, 'explicit_time_extension_same_native_thread': True, 'extension_retained_call_count':len(retained_calls), 'extension_original_start':retained_start,
                              'managed_before_first_turn_observable': managed_status_proved,
                              'new_app_submissions': 0, 'live_reasoning_or_independent_review_qualified': False}
            print('Selected-work wait/receipt/scope/parallel/dependency/tree-Stop native integration passed, zero Chat', flush=True)

        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(json.dumps({'accepted': True, 'fixture': str(root), 'chat_submissions': 0,
            'same_thread': thread_before, 'both_entrances': True, 'physical_cli_markers': True,
            'terminal_with_pending_submission_did_not_dispatch': True,
            'settlement_resumed_same_chat': not cold_resume,
            'cold_native_resume': cold_proof,
            'async_start_failure': start_failure_proof,
            'stop_fenced_waiting_followup_without_late_dispatch': True,
            'selected_control_integration': selected_proof}, indent=2) + '\n')
    finally:
        if home:
            (root / 'stop-provider').touch()
            for path in server_paths(home):
                try:
                    pid = int((path.parent / 'pid').read_text())
                    expected = home / '.local/lib/codex/core/generations/local-permission-fixture/runtime'
                    if Path(f'/proc/{pid}/exe').resolve() == expected:
                        os.kill(pid, signal.SIGTERM)
                except (OSError, ValueError):
                    pass
        for process in reversed(processes):
            if process.poll() is None:
                process.terminate()
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
        for master in masters:
            os.close(master)
        pool.shutdown(wait=True)
        (root / 'terminal.txt').write_text(screen)
        print('Owned fixture/proof retained at ' + str(root), flush=True)


if __name__ == '__main__':
    os.umask(0o077)
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--inputs', type=Path, help='explicit F source/native/fixture inputs; no historical defaults')
    parser.add_argument('--core', type=Path)
    parser.add_argument('--frontend', type=Path)
    parser.add_argument('--service-test', type=Path)
    parser.add_argument('--provider-binary', type=Path, help='actual Provider CLI used by the native client synchronization gate')
    parser.add_argument('--report', type=Path)
    parser.add_argument('--cold-resume', action='store_true', help='also qualify cold native owner replacement and asynchronous start failure; zero real Chat')
    parser.add_argument('--selected-controls', action='store_true', help='also exercise explicit assignment/controller/budget controls with legacy explicit fixture inputs; zero real Chat')
    args=parser.parse_args()
    if args.inputs:
        from provider_qualification import prepare, path, reference, require, digest
        require(not any((args.core,args.frontend,args.service_test,args.provider_binary,args.report)), '--inputs cannot be combined with implicit/override selectors')
        resolved=json.loads(prepare(args.inputs).read_text())
        inputs=resolved['inputs']
        require(inputs['limits']['mode']=='synthetic_android' and inputs['cases']==['F7-idempotency'], 'this driver qualifies only its declared synthetic F7 subset')
        args.root=path(inputs['environment']['tmp_root'],directory=True,private=True)
        require(not list(args.root.iterdir()), 'synthetic root is not empty; preserve and reconcile instead of restarting')
        require(inputs['native']['home']==str(args.root/'h/.codex') and inputs['native']['workspace']==str(args.root/'w'), 'fixture home/workspace must match explicit prepared roots')
        recipe=json.loads(reference(inputs['fixture']['manifest']).read_text())
        require(recipe['schema']=='termux-codex-f-native-fixture-v1','unsupported native fixture recipe')
        args.generation=path(recipe['generation']['path'],directory=True)
        require(reference(recipe['generation']['manifest'])==args.generation/'release.manifest','generation manifest mismatch')
        require(inputs['fixture']['tasks']['sha256']==digest(Path(__file__)), 'synthetic tasks must bind this exact driver source')
        args.core=reference(inputs['native']['core'])
        args.config=reference(inputs['candidate']['config'])
        args.frontend=reference(inputs['native']['frontend'])
        args.service_test=reference(recipe['service_test'])
        args.report=Path(inputs['environment']['artifact_root'])/(inputs['run_id']+'.synthetic.json')
        # This is a distinct exclusive launch, never an automatic retry. The
        # retained driver report covers its actual cases, not all F7 boundaries.
        with (args.report.with_suffix('.launch.json')).open('x') as stream:
            json.dump({'resolved_inputs':str(prepare(args.inputs)),'inputs_sha256':resolved['inputs_sha256'],'pid':os.getpid(),'monotonic_ns':time.monotonic_ns(),'new_chat_requests':0},stream)
    elif not all((args.core,args.frontend,args.service_test,args.report)):
        parser.error('supply --inputs, or all legacy --core/--frontend/--service-test/--report options')
    probe(args)
