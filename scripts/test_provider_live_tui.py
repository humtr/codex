"""Exercise the live qualifier's observer fault, never a real model."""
import hashlib
import copy
import http.client
import http.server
import json
from pathlib import Path
import sys
import threading
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).parent))
from qualify_provider_live_tui import (
    MODEL, ObservationProxy, TurnBudget, admitted_prompt, assistant_marker,
    completed_fault, history, native_settled, one_pwd, plain, request_identity,
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
