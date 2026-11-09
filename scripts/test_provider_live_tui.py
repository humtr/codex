"""Exercise the live qualifier's observer fault, never a real model."""
import hashlib
import copy
import http.client
import http.server
import io
import json
import os
from pathlib import Path
import stat
import subprocess
import sys
import threading
import tempfile
import unittest
from types import SimpleNamespace
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).parent))
from qualify_provider_live_tui import (
    MODEL, LiveTui, ObservationProxy, TurnBudget, admitted_prompt, assistant_marker,
    blank_history_error, blank_rollout, completed_fault, history, native_settled,
    main, one_pwd, plain, request_identity, require_terminal_owner,
    terminal_owner_preflight, terminal_response,
)


class ObserverFault(unittest.TestCase):
    def test_one_original_request_settles_after_only_downstream_is_cut(self):
        requests = []

        class Model(http.server.BaseHTTPRequestHandler):
            def log_message(self, *_):
                pass

            def do_GET(self):
                self.send_response(200)
                self.end_headers()
                self.wfile.write(b'{"data":[]}')

            def do_POST(self):
                requests.append(self.rfile.read(int(self.headers['Content-Length'])))
                self.send_response(200)
                self.send_header('Content-Type', 'text/event-stream')
                self.end_headers()
                for value in [
                    {'type': 'response.created', 'response': {'id': 'original-response'}},
                    {'type': 'response.output_text.delta', 'delta': 'before effect'},
                    {'type': 'response.output_text.delta', 'delta': 'after confirmed effect'},
                    {'type': 'response.completed', 'response': {'id': 'original-response', 'status': 'completed'}},
                ]:
                    self.wfile.write(b'data: ' + json.dumps(value).encode() + b'\n\n')
                    self.wfile.flush()

        upstream = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Model)
        upstream.daemon_threads = True
        threading.Thread(target=upstream.serve_forever, daemon=True).start()
        proxy = ObservationProxy(f'http://127.0.0.1:{upstream.server_port}/v1')
        checks = []
        proxy.armed_thread = 'owned-native-thread'

        def effect_observed(record):
            checks.append(True)
            self.assertEqual(record['thread'], 'owned-native-thread')
            return {'thread': record['thread'], 'turn': record['turn'],
                    'item': 'completed-pwd', 'observed_at': record['at']} if len(checks) > 1 else None

        proxy.can_cut = effect_observed
        client = http.client.HTTPConnection('127.0.0.1', proxy.server.server_port, timeout=5)
        body = json.dumps({'model': MODEL, 'input': 'owned unit fixture', 'client_metadata': {
            'x-codex-turn-metadata': json.dumps({'thread_id': 'owned-native-thread', 'turn_id': 'fault-turn'})}}).encode()
        try:
            client.request('POST', '/v1/responses', body, headers={
                'session_id': 'not-the-native-thread', 'Authorization': 'Bearer private-fixture-token'})
            response = client.getresponse().read()
            self.assertIn(b'before effect', response)
            self.assertNotIn(b'after confirmed effect', response, 'cut the first eligible delta before forwarding it')
            self.assertNotIn(b'response.completed', response)
            self.assertTrue(proxy.fired.wait(5))
            self.assertTrue(proxy.settled.wait(5))
            self.assertEqual(requests, [body])
            record = proxy.records[0]
            self.assertEqual(record['terminal'], 'response.completed')
            self.assertEqual(record['response_id'], 'original-response')
            self.assertEqual(record['body_sha256'], hashlib.sha256(body).hexdigest())
            self.assertTrue(record['upstream_eof'])
            self.assertEqual(completed_fault(proxy.records, 'owned-native-thread', 'fault-turn'), record)
            self.assertEqual(len(checks), 2)
            self.assertNotIn('private-fixture-token', json.dumps(proxy.records))
            client.close()
            client = http.client.HTTPConnection('127.0.0.1', proxy.server.server_port, timeout=5)
            client.request('GET', '/v1/models')
            self.assertEqual(json.loads(client.getresponse().read()), {'data': []})
            self.assertEqual(requests, [body], 'metadata after observer loss must not replay input')
            for attempted in (body, json.dumps(dict(json.loads(body), input='changed transient body')).encode()):
                client.close()
                client = http.client.HTTPConnection('127.0.0.1', proxy.server.server_port, timeout=5)
                client.request('POST', '/v1/responses', attempted)
                rejected = client.getresponse()
                self.assertEqual(rejected.status, 409)
                self.assertEqual(json.loads(rejected.read())['error']['code'], 'qualification_no_replay')
                self.assertEqual(requests, [body], 'a replay attempt must never reach the real model')
                self.assertEqual(proxy.records[-1]['rejected'], 'disconnected_turn_replay')
                with self.assertRaisesRegex(AssertionError, 'resubmitted|retried'):
                    completed_fault(proxy.records, 'owned-native-thread', 'fault-turn')
        finally:
            client.close()
            proxy.close()
            upstream.shutdown()
            upstream.server_close()

    def test_fault_proxy_cannot_target_an_unrelated_or_credentialled_endpoint(self):
        for endpoint in ['https://127.0.0.1:123/v1', 'http://example.com:123/v1',
                         'http://127.0.0.1:123/not-v1', 'http://user@127.0.0.1:123/v1',
                         'http://127.0.0.1:123/v1?secret=yes']:
            with self.subTest(endpoint=endpoint), self.assertRaises(AssertionError):
                ObservationProxy(endpoint)

    def test_terminal_normalization_keeps_visible_content(self):
        self.assertEqual(plain('\x1b]0;private title\x07\x1b[1mSelect Provider\x1b[0m'),
                         'Select Provider')


class TuiObservation(unittest.TestCase):
    def test_launch_sets_only_the_observed_title_and_optional_fault_endpoint(self):
        for endpoint in (None, 'http://127.0.0.1:123/v1'):
            with self.subTest(endpoint=endpoint), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                codex_home = root / 'codex-home'
                codex_home.mkdir()
                settings = codex_home / 'config.toml'
                original = b'model = "retained-model"\nprofile = "retained-profile"\n[tui]\nterminal_title = []\n'
                settings.write_bytes(original)
                args = SimpleNamespace(core=root / 'codex', workspace=root, codex_home=codex_home)
                budget = TurnBudget(root)
                with patch('qualify_provider_live_tui.subprocess.Popen') as launch:
                    client = LiveTui(args, root, 'primary', budget, endpoint)
                try:
                    expected = [str(args.core), '--no-alt-screen', '-c', 'tui.terminal_title=["thread"]']
                    if endpoint:
                        expected += ['-c', 'model_providers.chatgpt_desktop.base_url=' + json.dumps(endpoint)]
                    launch.assert_called_once()
                    self.assertEqual(launch.call_args.args, (expected,))
                    self.assertEqual(launch.call_args.kwargs['cwd'], args.workspace)
                    self.assertEqual(launch.call_args.kwargs['env']['CODEX_HOME'], str(codex_home))
                    self.assertEqual(settings.read_bytes(), original)
                    self.assertEqual(budget.submissions, [])
                    self.assertFalse(list(root.glob('submission-*.json')))
                finally:
                    os.close(client.master)
                    client.log.close()

    def test_split_osc_waits_for_the_full_title_and_retains_the_same_thread(self):
        thread = '550e8400-e29b-41d4-a716-446655440000'
        other = '550e8400-e29b-41d4-a716-446655440001'
        client = object.__new__(LiveTui)
        client.master, writer = os.pipe()
        self.addCleanup(os.close, client.master)
        self.addCleanup(os.close, writer)
        client.log = io.BytesIO()
        client.screen, client.thread = '', None

        def feed(data):
            os.write(writer, data)
            with patch('qualify_provider_live_tui.time.monotonic', side_effect=[0, 0, 1]):
                client.drain()

        parts = [
            f'{thread}\n\x1b]0;project\x07\x1b]0;{thread[:29]}...\x07'.encode(),
            b'\x1b', b']0;', thread[:17].encode(), thread[17:].encode(),
        ]
        for part in parts:
            feed(part)
            self.assertIsNone(client.thread, 'visible text, default/truncated titles, and incomplete OSC are not readiness')
        feed(b'\x07')
        self.assertEqual(client.thread, thread)
        self.assertEqual(client.log.getvalue(), b''.join(parts) + b'\x07')
        feed(f'\x1b]0;{thread}\x07'.encode())
        self.assertEqual(client.thread, thread)
        with self.assertRaisesRegex(AssertionError, 'silently changed Native thread'):
            feed(f'\x1b]0;{other}\x07'.encode())


class NativeEvidence(unittest.TestCase):
    def setUp(self):
        self.workspace = Path('/data/data/com.termux/files/home/owned-fixture')
        self.shell = Path('/data/data/com.termux/files/usr/bin/bash')
        self.prompt = 'Concatenate "UNIQUE_" and "ANSWER" without a separator.'
        self.marker = 'UNIQUE_ANSWER'
        self.command = {'type': 'commandExecution', 'id': 'pwd-item',
                        'command': str(self.shell) + ' -c pwd', 'cwd': str(self.workspace),
                        'exitCode': 0, 'aggregatedOutput': str(self.workspace) + '\n', 'status': 'completed'}
        self.turn = {'id': 'turn', 'status': 'completed', 'items': [
            {'type': 'userMessage', 'content': [{'type': 'text', 'text': self.prompt}]},
            {'type': 'agentMessage', 'phase': 'final_answer', 'text': self.marker}]}

    def test_final_answer_and_visible_new_token_are_both_required(self):
        admitted_prompt(self.turn, self.prompt)
        self.assertTrue(assistant_marker(self.turn, self.prompt, self.marker, self.marker))
        self.assertFalse(assistant_marker(self.turn, self.prompt, self.marker, self.prompt))
        self.assertFalse(assistant_marker(self.turn, self.prompt, self.marker, 'UNIQUE_OLDER'))
        with self.assertRaisesRegex(AssertionError, 'user prompt'):
            assistant_marker(self.turn, 'Reply with ' + self.marker, self.marker, self.marker)
        for changed in ({'status': 'failed'}, {'items': [self.turn['items'][0]]},
                        {'items': [{'type': 'agentMessage', 'phase': 'commentary', 'text': self.marker}]}):
            with self.subTest(changed=changed), self.assertRaises(AssertionError):
                assistant_marker(dict(self.turn, **changed), self.prompt, self.marker, self.marker)

    def test_native_admission_cannot_be_an_ack_or_another_input(self):
        for items in ([], [self.turn['items'][0], self.turn['items'][0]],
                      [{'type': 'userMessage', 'content': [{'type': 'text', 'text': 'other input'}]}]):
            with self.subTest(items=items), self.assertRaises(AssertionError):
                admitted_prompt(dict(self.turn, items=items), self.prompt)

    def test_pwd_uses_the_actual_camel_case_native_history(self):
        thread = {'id': 'thread', 'turns': [{'id': 'turn', 'items': [self.command]}]}
        proof = one_pwd(thread, self.workspace, self.shell, 'turn')
        self.assertEqual(proof['item'], 'pwd-item')
        self.assertEqual(proof['aggregatedOutput'], str(self.workspace) + '\n')
        changes = [
            {'command': 'pwd'}, {'command': str(self.shell) + ' -lc pwd'},
            {'command': str(self.shell) + " -c 'pwd; pwd'"}, {'command': '/bin/bash -c pwd'},
            {'command': str(self.shell) + " -c 'echo " + str(self.workspace) + "'"},
            {'exitCode': True}, {'exitCode': 1}, {'exitCode': None, 'exit_code': 0},
            {'status': 'inProgress'}, {'status': 'failed'}, {'cwd': '/another/workspace'},
            {'aggregatedOutput': 'TUI_LIVE_FAKE'}, {'aggregatedOutput': None, 'aggregated_output': str(self.workspace) + '\n'},
            {'id': ''},
        ]
        for change in changes:
            candidate = copy.deepcopy(thread)
            candidate['turns'][0]['items'][0].update(change)
            with self.subTest(change=change), self.assertRaises(AssertionError):
                one_pwd(candidate, self.workspace, self.shell, 'turn')
        for items in ([], [self.command, self.command]):
            with self.subTest(items=items), self.assertRaises(AssertionError):
                one_pwd({'id': 'thread', 'turns': [{'id': 'turn', 'items': items}]}, self.workspace, self.shell)
        with self.assertRaises(AssertionError):
            one_pwd(thread, self.workspace, self.shell, 'another-turn')

    def test_original_two_turns_and_fault_turn_are_immutable(self):
        first = copy.deepcopy(self.turn)
        second = dict(copy.deepcopy(self.turn), id='followup')
        fault = {'id': 'fault', 'status': 'failed', 'items': [self.command]}
        resumed = {'id': 'thread', 'turns': [first, second, fault, dict(self.turn, id='recovered')]}
        history(resumed, 4, [first, second, fault])
        for index in range(3):
            changed = copy.deepcopy(resumed)
            changed['turns'][index]['items'] = []
            with self.subTest(index=index), self.assertRaises(AssertionError):
                history(changed, 4, [first, second, fault])
        with self.assertRaises(AssertionError):
            history(resumed, 3)
        with self.assertRaises(AssertionError):
            history({'turns': [first, first]}, 2)

    def test_five_submission_attempts_are_counted_before_any_delivery(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            budget = TurnBudget(root)
            for index in range(5):
                budget.reserve('owned-fixture', 'thread', index + 1, 'input')
                self.assertTrue((root / f'submission-{index + 1}.json').is_file())
            with self.assertRaisesRegex(AssertionError, 'budget exhausted'):
                budget.reserve('owned-fixture', 'thread', 6, 'do not deliver')
            self.assertEqual(len(budget.submissions), 5)
            self.assertFalse((root / 'submission-6.json').exists())

    def test_failed_native_turn_can_settle_without_becoming_a_successful_answer(self):
        cases = [
            ('idle', [], True), ('idle', ['completed'], True),
            ('systemError', ['failed'], True), ('systemError', [], False),
            ('systemError', ['completed'], False), ('systemError', ['inProgress'], False),
            ('active', ['failed'], False), ('notLoaded', ['failed'], False),
            ('idle', ['inProgress'], False), ('systemError', ['inProgress', 'failed'], False),
        ]
        for status, turns, expected in cases:
            thread = {'status': {'type': status}, 'turns': [{'status': value} for value in turns]}
            with self.subTest(status=status, turns=turns):
                self.assertEqual(native_settled(thread, allow_failed=True), expected)
                if status == 'systemError':
                    self.assertFalse(native_settled(thread))

    def test_request_identity_is_native_metadata_not_session_header(self):
        for value in ({'thread_id': 'native', 'turn_id': 'turn'},
                      {'x-codex-turn-metadata': {'thread_id': 'native', 'turn_id': 'turn'}},
                      {'x-codex-turn-metadata': '{"thread_id":"native","turn_id":"turn"}'}):
            self.assertEqual(request_identity({'client_metadata': value}), {'thread': 'native', 'turn': 'turn'})
        for value in ({}, {'thread_id': 'native'}, {'thread_id': 'native', 'turn_id': ''},
                      {'thread_id': 'native', 'turn_id': 'turn',
                       'x-codex-turn-metadata': {'thread_id': 'another', 'turn_id': 'turn'}}):
            with self.subTest(value=value), self.assertRaises(AssertionError):
                request_identity({'client_metadata': value})

    def test_unmaterialized_error_is_exactly_bound_to_the_first_thread_read(self):
        message = 'thread owned is not materialized yet; includeTurns is unavailable before first user message'
        self.assertTrue(blank_history_error(AssertionError(('thread/read', {'code': -32600, 'message': message})), 'owned'))
        for method, code, text in [('thread/read', -32603, message), ('thread/list', -32600, message),
                                   ('thread/read', -32600, message.replace('owned', 'other')),
                                   ('thread/read', -32600, 'history temporarily unavailable')]:
            with self.subTest(method=method, code=code, text=text):
                self.assertFalse(blank_history_error(AssertionError((method, {'code': code, 'message': text})), 'owned'))
        self.assertFalse(blank_history_error(AssertionError('the exact installed Native owner is unavailable'), 'owned'))

    def test_submitted_first_input_waits_for_materialization_without_another_paste(self):
        client = object.__new__(LiveTui)
        client.thread = 'owned'

        def not_materialized():
            raise AssertionError(('thread/read', {'code': -32600, 'message':
                'thread owned is not materialized yet; includeTurns is unavailable before first user message'}))

        client.read = not_materialized
        self.assertIsNone(client.settled(1, []))
        with self.assertRaises(AssertionError):
            client.settled(2, [self.turn])

    def test_empty_metadata_needs_a_complete_owned_root_rollout(self):
        with tempfile.TemporaryDirectory() as temp:
            home = Path(temp)
            sessions = home / 'sessions'
            sessions.mkdir()
            path = sessions / 'blank.jsonl'
            row = {'type': 'session_meta', 'payload': {'id': 'owned', 'session_id': 'owned'}}
            snapshot = {'id': 'owned', 'status': {'type': 'idle'}, 'ephemeral': False,
                        'turns': [], 'path': str(path)}

            def write(value):
                path.write_text(json.dumps(value))
                path.chmod(0o600)

            write(row)
            self.assertEqual(blank_rollout(snapshot, 'owned', home), snapshot)
            for change in ({'id': 'other'}, {'ephemeral': True}, {'status': {'type': 'active'}},
                           {'turns': [self.turn]}):
                with self.subTest(change=change), self.assertRaises(AssertionError):
                    blank_rollout(dict(snapshot, **change), 'owned', home)
            for key in ('id', 'session_id', 'forked_from_id', 'parent_thread_id', 'history_base',
                        'forked_from_ordinal_exclusive', 'subagent_history_start_ordinal', 'attached_thread'):
                candidate = copy.deepcopy(row)
                candidate['payload'][key] = 'another'
                write(candidate)
                with self.subTest(key=key), self.assertRaises(AssertionError):
                    blank_rollout(snapshot, 'owned', home)
            write(row)
            with path.open('a') as output:
                output.write('\n' + json.dumps({'type': 'response_item', 'payload': {'role': 'user'}}))
            with self.assertRaises(ValueError):
                blank_rollout(snapshot, 'owned', home)
            write(row)
            path.chmod(0o666)
            with self.assertRaises(AssertionError):
                blank_rollout(snapshot, 'owned', home)
            path.unlink()
            outside = home / 'outside.jsonl'
            outside.write_text(json.dumps(row))
            outside.chmod(0o600)
            path.symlink_to(outside)
            with self.assertRaises(OSError):
                blank_rollout(snapshot, 'owned', home)


class TerminalOwnerEvidence(unittest.TestCase):
    def setUp(self):
        self.prefix = Path('/data/data/com.termux/files/usr')
        self.process = {'pid': 42, 'start': 901, 'parent_pid': 2, 'process_group': 42,
                        'session': 42, 'tty_device': 34820, 'foreground_process_group': 42,
                        'ttys': {'0': '/dev/pts/4', '1': '/dev/pts/4', '2': '/dev/pts/4'}, 'uid': os.getuid()}
        self.origin = {'schema': 'termux-terminal-origin-v1', 'result': 'ok', 'selected': True,
                       'origin': {'handle': '01a0fe94-2d11-79f3-8f5b-9c1958465dc2', 'pid': 42, 'start': 901}}

    def test_receiver_must_be_the_selected_native_terminal_root(self):
        require_terminal_owner(self.process, self.origin, self.origin)
        require_terminal_owner(self.process, self.origin, dict(self.origin, selected=None))
        with self.assertRaisesRegex(AssertionError, 'no longer selected'):
            require_terminal_owner(self.process, self.origin, dict(self.origin, selected=False))
        for change in ({'pid': 43}, {'start': 902}, {'session': 41}, {'process_group': 43},
                       {'foreground_process_group': 41}, {'ttys': {'0': None, '1': None, '2': None}},
                       {'ttys': {'0': '/dev/pts/4', '1': None, '2': '/dev/pts/4'}}):
            with self.subTest(change=change), self.assertRaises(AssertionError):
                require_terminal_owner(dict(self.process, **change), self.origin, self.origin)
        for change in ({'selected': False}, {'selected': None},
                       {'origin': dict(self.origin['origin'], pid=41)},
                       {'origin': dict(self.origin['origin'], start=902)}):
            with self.subTest(change=change), self.assertRaises(AssertionError):
                require_terminal_owner(self.process, dict(self.origin, **change), self.origin)
        changed = dict(self.origin, origin=dict(self.origin['origin'], handle='02a0fe94-2d11-79f3-8f5b-9c1958465dc2'))
        with self.assertRaisesRegex(AssertionError, 'identity changed'):
            require_terminal_owner(self.process, self.origin, changed)

    def test_ack_is_insufficient_and_only_public_identity_enters_evidence(self):
        metadata = SimpleNamespace(st_mode=stat.S_IFSOCK | 0o600, st_uid=os.getuid())
        secret = 'PRIVATE_AUTH_MUST_NOT_ENTER_RECEIPTS'
        value = dict(self.origin, access_token=secret, arbitrary_private_field=secret)
        result = subprocess.CompletedProcess([], 0, json.dumps(value).encode(), b'')
        with patch.object(Path, 'lstat', return_value=metadata), \
                patch('qualify_provider_live_tui.subprocess.run', return_value=result) as run:
            observed = terminal_response(self.prefix, 'origin')
        self.assertEqual(observed, self.origin)
        self.assertNotIn(secret, json.dumps(observed))
        self.assertEqual(run.call_args.args[0], [str(self.prefix / 'bin/termux-am-socket'), 'termux-terminal-v1 origin'])
        candidates = [
            b'Starting service: Intent {}\n', b'{"schema":"termux-terminal-origin-v1","result":"ok"}',
            json.dumps(dict(self.origin, schema='another-schema')).encode(),
            json.dumps(dict(self.origin, result='pending')).encode(),
            json.dumps(dict(self.origin, origin=dict(self.origin['origin'], pid=True))).encode(),
            json.dumps(dict(self.origin, origin=dict(self.origin['origin'], start=0))).encode(),
            json.dumps(dict(self.origin, origin=dict(self.origin['origin'], handle='visible title'))).encode(),
            json.dumps(dict(self.origin, origin=dict(self.origin['origin'], private=secret))).encode(),
            b'x' * 4097,
        ]
        for data in candidates:
            with self.subTest(data=data[:100]), patch.object(Path, 'lstat', return_value=metadata), \
                    patch('qualify_provider_live_tui.subprocess.run', return_value=subprocess.CompletedProcess([], 0, data, b'')), \
                    self.assertRaises((AssertionError, ValueError)):
                terminal_response(self.prefix, 'origin')
        with patch.object(Path, 'lstat', return_value=metadata), \
                patch('qualify_provider_live_tui.subprocess.run', return_value=subprocess.CompletedProcess([], 0, result.stdout, secret.encode())), \
                self.assertRaisesRegex(AssertionError, '^terminal owner observation failed$'):
            terminal_response(self.prefix, 'origin')

    def test_preflight_validates_once_and_records_timing_without_inherited_secrets(self):
        with patch('qualify_provider_live_tui.process_identity', return_value=self.process), \
                patch('qualify_provider_live_tui.sha', return_value='a' * 64), \
                patch('qualify_provider_live_tui.terminal_response', return_value=self.origin) as request:
            receipt = terminal_owner_preflight(self.prefix, True)
        self.assertTrue(receipt['verified'])
        self.assertEqual(receipt['receiver'], self.process)
        self.assertLessEqual(receipt['started_at'], receipt['completed_at'])
        self.assertEqual([call.args for call in request.call_args_list], [
            (self.prefix, 'origin'),
            (self.prefix, 'validate 01a0fe94-2d11-79f3-8f5b-9c1958465dc2 42 901'),
        ])
        with patch.dict(os.environ, {'OWNED_AUTH': 'SECRET_MUST_NOT_BE_RECORDED'}), \
                patch('qualify_provider_live_tui.terminal_response') as request:
            optional = terminal_owner_preflight(Path('/unused'), False)
        self.assertFalse(optional['verified'])
        self.assertEqual(optional['receiver']['pid'], os.getpid())
        self.assertGreater(optional['receiver']['start'], 0)
        self.assertNotIn('SECRET_MUST_NOT_BE_RECORDED', json.dumps(optional))
        request.assert_not_called()
        with patch('qualify_provider_live_tui.process_identity', return_value=self.process), \
                patch('qualify_provider_live_tui.sha', return_value='a' * 64), \
                patch('qualify_provider_live_tui.terminal_response', side_effect=ValueError('PRIVATE_RAW_RESPONSE')):
            failed = terminal_owner_preflight(self.prefix, True)
        self.assertFalse(failed['verified'])
        self.assertNotIn('PRIVATE_RAW_RESPONSE', json.dumps(failed))

    def test_required_preflight_failure_precedes_all_tui_or_provider_work(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            home = root / 'home'
            (home / '.codex').mkdir(parents=True)
            runtime = home / '.local/lib/codex/core/current/runtime'
            runtime.parent.mkdir(parents=True)
            runtime.write_bytes(b'owned runtime fixture')
            core = root / 'core'
            core.write_bytes(b'owned core fixture')
            prefix = root / 'usr'
            (prefix / 'bin').mkdir(parents=True)
            (prefix / 'bin/bash').write_bytes(b'owned shell fixture')
            report = root / 'report'
            arguments = ['qualifier', '--core', str(core), '--workspace', str(root), '--home', str(home),
                         '--report-root', str(report), '--terminal-owner-proof', '--fault-endpoint', 'http://127.0.0.1:123/v1']
            output = io.StringIO()
            with patch.object(sys, 'argv', arguments), patch.dict(os.environ, {'PREFIX': str(prefix)}), \
                    patch('qualify_provider_live_tui.terminal_owner_preflight', return_value={'verified': False}), \
                    patch('qualify_provider_live_tui.LiveTui') as tui, \
                    patch('qualify_provider_live_tui.ObservationProxy') as proxy, patch('sys.stdout', output), \
                    self.assertRaisesRegex(AssertionError, 'foreground terminal owner was not verified'):
                main()
            tui.assert_not_called()
            proxy.assert_not_called()
            result = json.loads((report / 'result.json').read_text())
            self.assertFalse(result['accepted'])
            self.assertEqual(result['models_submitted'], 0)
            self.assertEqual(json.loads((report / 'terminal-owner.json').read_text()), {'verified': False})
            self.assertFalse(list(report.glob('submission-*.json')))


class StreamEvidence(unittest.TestCase):
    def setUp(self):
        self.original = {
            'injected': True, 'method': 'POST', 'path': '/v1/responses',
            'thread': 'thread', 'turn': 'fault-turn', 'model': MODEL, 'http_status': 200,
            'content_type': 'text/event-stream', 'upstream_eof': True,
            'events': ['response.created', 'response.output_text.delta', 'response.completed'],
            'terminal': 'response.completed', 'terminal_status': 'completed',
            'response_id': 'original', 'terminal_response_id': 'original',
            'cut_event': 1, 'cut_at': '02', 'settled_at': '03',
            'execution': {'thread': 'thread', 'turn': 'fault-turn', 'observed_at': '01'},
            'body_sha256': 'original-body', 'ordinal': 2,
        }

    def test_original_response_requires_exact_identity_and_completed_transport(self):
        self.assertEqual(completed_fault([self.original], 'thread', 'fault-turn'), self.original)
        for change in ({'http_status': 413}, {'upstream_eof': False}, {'transport_error': 'TimeoutError'},
                       {'terminal_response_id': 'replacement'}, {'terminal_status': 'failed'},
                       {'terminal': 'response.failed'}, {'thread': 'another-thread'}, {'turn': 'another-turn'},
                       {'cut_event': 0}, {'content_type': 'application/json'},
                       {'events': ['response.created', 'response.output_text.delta', 'response.completed', 'response.completed']},
                       {'events': ['response.created', 'response.output_text.delta', 'error', 'response.completed']}):
            with self.subTest(change=change), self.assertRaises(AssertionError):
                completed_fault([dict(self.original, **change)], 'thread', 'fault-turn')

    def test_changed_body_retry_is_rejected_but_prior_tool_round_and_new_turn_are_distinct(self):
        prior = dict(self.original, injected=False, ordinal=1, body_sha256='tool-round', response_id='tool-response')
        recovery = dict(self.original, injected=False, ordinal=3, turn='recovery-turn',
                        body_sha256='new-user-input', response_id='new-response')
        completed_fault([prior, self.original, recovery], 'thread', 'fault-turn')
        retry = dict(recovery, turn='fault-turn')
        with self.assertRaisesRegex(AssertionError, 'retried'):
            completed_fault([self.original, retry], 'thread', 'fault-turn')
        replay = dict(prior, body_sha256='original-body')
        with self.assertRaisesRegex(AssertionError, 'resubmitted'):
            completed_fault([replay, self.original], 'thread', 'fault-turn')


if __name__ == '__main__':
    unittest.main()
