import concurrent.futures
import hashlib
import http.client
import http.server
import json
from pathlib import Path
import sys
import threading
from types import SimpleNamespace
import unittest

sys.path.insert(0, str(Path(__file__).parent))
from qualify_provider_result_recovery import CutProxy, NATIVE_RESPONSES_PATH, completed_command, executable_item, fault_observed, native_request_identity, startup_settled


class ResultFaultBoundary(unittest.TestCase):

    def test_versioned_native_request_keeps_original_call_and_consumes_cut_response(self):
        requests, observed = [], []
        metadata = {'thread_id': 'owned-thread', 'turn_id': 'owned-turn'}
        body = json.dumps({'client_metadata': metadata, 'input': 'one owned fixture'}).encode()
        events = [
            {'type': 'response.created', 'response': {'id': 'original-response'}},
            {'type': 'response.output_item.done', 'item': {
                'type': 'function_call', 'call_id': 'original-call',
                'name': 'exec_command', 'arguments': '{"cmd":"owned fixture"}'}},
            {'type': 'response.completed', 'response': {'id': 'original-response'}},
        ]

        class Upstream(http.server.BaseHTTPRequestHandler):
            def log_message(self, *_):
                pass

            def do_GET(self):
                requests.append((self.command, self.path, b''))
                self.send_response(200)
                self.end_headers()
                self.wfile.write(b'{"data":[]}')

            def do_POST(self):
                original = self.rfile.read(int(self.headers['Content-Length']))
                requests.append((self.command, self.path, original))
                self.send_response(200)
                self.send_header('Content-Type', 'text/event-stream')
                self.end_headers()
                for event in events:
                    self.wfile.write(b'data: ' + json.dumps(event).encode() + b'\n\n')
                    self.wfile.flush()

        upstream = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Upstream)
        upstream.daemon_threads = True
        thread = threading.Thread(target=upstream.serve_forever, daemon=True)
        thread.start()
        proxy = CutProxy()
        proxy.port, proxy.thread = upstream.server_port, 'owned-thread'

        def completed(identity):
            observed.append(identity)
            return {'id': identity, 'status': 'completed', 'exitCode': 0}

        proxy.observe = completed
        client = http.client.HTTPConnection('127.0.0.1', proxy.server.server_port, timeout=5)
        try:
            client.request('POST', NATIVE_RESPONSES_PATH, body,
                           headers={'thread-id': 'owned-thread'})
            response = client.getresponse()
            self.assertEqual(response.status, 200)
            delivered = response.read()
            self.assertIn(b'original-call', delivered)
            self.assertNotIn(b'response.completed', delivered)
            self.assertTrue(proxy.fired.wait(5))
            self.assertTrue(proxy.settled.wait(5))
            self.assertEqual(requests, [('POST', '/v1/native-v4/responses', body)])
            self.assertEqual(observed, ['original-call'])
            row = proxy.records[0]
            self.assertEqual((row['thread'], row['turn'], row['call_id']),
                             ('owned-thread', 'owned-turn', 'original-call'))
            self.assertEqual(row['body_sha256'], hashlib.sha256(body).hexdigest())
            self.assertEqual(row['response_id'], 'original-response')
            self.assertEqual(row['completed_response_id'], row['response_id'])
            self.assertEqual(row['events'], [event['type'] for event in events])
            self.assertTrue(row['upstream_eof'])
            self.assertNotIn('transport_error', row)
            client.close()
            client = http.client.HTTPConnection('127.0.0.1', proxy.server.server_port, timeout=5)
            client.request('GET', '/v1/models')
            self.assertEqual(json.loads(client.getresponse().read()), {'data': []})
            self.assertEqual(requests, [('POST', '/v1/native-v4/responses', body),
                                        ('GET', '/v1/models', b'')])
        finally:
            client.close()
            proxy.close()
            upstream.shutdown()
            upstream.server_close()
            thread.join(timeout=5)

    def test_fault_matches_actual_native_body_identity_and_thread_header(self):
        metadata = {'thread_id': 'original-thread', 'turn_id': 'original-turn'}
        body = json.dumps({'client_metadata': {**metadata, 'x-codex-turn-metadata': json.dumps(metadata)}}).encode()
        expected = {'thread': 'original-thread', 'turn': 'original-turn'}
        self.assertEqual(native_request_identity(body, {'thread-id': 'original-thread', 'session-id': 'cache-affinity'}), expected)
        self.assertEqual(native_request_identity(body, {}), expected)
        for name in ('thread-id', 'x-codex-thread-id'):
            with self.assertRaisesRegex(AssertionError, 'conflicting Native thread'):
                native_request_identity(body, {name: 'other-thread'})
        changed = json.loads(body)
        changed['client_metadata']['thread_id'] = 'other-thread'
        with self.assertRaisesRegex(AssertionError, 'conflicting Native request'):
            native_request_identity(json.dumps(changed).encode(), {})
        with self.assertRaises(AssertionError):
            native_request_identity(b'{}', {'thread-id': 'original-thread'})

    def test_early_original_reply_is_observed_without_reexecution_or_timeout(self):
        proxy = SimpleNamespace(fired=threading.Event(), records=[])
        pending = concurrent.futures.Future()
        self.assertFalse(fault_observed(proxy, pending))
        for result in ({'result': {'error': 'native_resource_busy', 'executed': False}},
                       {'result': {'type': 'function_call_output', 'output': 'original output'}}):
            returned = concurrent.futures.Future()
            returned.set_result(result)
            with self.assertRaisesRegex(AssertionError, 'returned before response fault'):
                fault_observed(proxy, returned)
            self.assertIs(returned.result(), result)
        failed = concurrent.futures.Future()
        original_error = RuntimeError('original request failure')
        failed.set_exception(original_error)
        with self.assertRaises(RuntimeError) as observed:
            fault_observed(proxy, failed)
        self.assertIs(observed.exception, original_error)
        proxy.records.append({'cut': {'monotonic_ns': 1}})
        self.assertFalse(fault_observed(proxy, returned), 'shutdown may precede Event publication')
        proxy.fired.set()
        self.assertTrue(fault_observed(proxy, returned))

    def test_only_complete_executable_item_arms_the_fault(self):
        tool = {'type': 'function_call', 'id': 'presentation', 'call_id': 'actual-call'}
        self.assertEqual(executable_item({'type': 'response.output_item.done', 'item': tool}), 'actual-call')
        self.assertIsNone(executable_item({'type': 'response.output_item.added', 'item': tool}))
        self.assertIsNone(executable_item({'type': 'response.output_item.done', 'item': {'type': 'message'}}))
        with self.assertRaises(AssertionError):
            executable_item({'type': 'response.output_item.done', 'item': {'type': 'function_call'}})

    def test_canonical_command_end_not_sse_delivery_is_required(self):
        item = {'type': 'commandExecution', 'id': 'actual-call', 'status': 'inProgress', 'exitCode': None}
        thread = {'turns': [{'items': [item]}]}
        self.assertIsNone(completed_command({'turns': []}, 'actual-call'))
        self.assertIsNone(completed_command(thread, 'actual-call'))
        item.update(status='completed', exitCode=0)
        self.assertIs(completed_command(thread, 'actual-call'), item)
        for changed in ({'id': 'other-call'}, {'status': 'failed'}, {'exitCode': -1}, {'exitCode': 124}):
            with self.assertRaises(AssertionError):
                completed_command({'turns': [{'items': [dict(item, **changed)]}]}, 'actual-call')
        with self.assertRaises(AssertionError):
            completed_command({'turns': [{'items': [item, item]}]}, 'actual-call')

    def test_foreground_ack_does_not_replace_original_effect_settlement(self):
        value = {'work_id': 'original', 'thread_id': 'native-thread', 'selection': {'inference': 'attached'},
                 'native_effects': {'active': [], 'unconfirmed_effects': 0},
                 'pending_calls': [], 'human_requests': [], 'call_observations': []}
        self.assertTrue(startup_settled(value, 'original'))
        for changed in ({'native_effects': {'active': [{}], 'unconfirmed_effects': 0}},
                        {'native_effects': {'active': [], 'unconfirmed_effects': 1}},
                        {'pending_calls': ['constructor']}, {'human_requests': [{}]},
                        {'call_observations': [{'offered': True, 'settled': False}]},
                        {'call_observations': [{'offered': False, 'settled': False}]}):
            self.assertFalse(startup_settled(dict(value, **changed), 'original'))
        self.assertTrue(startup_settled(dict(value,
            call_observations=[{'offered': False, 'settled': True}]), 'original'),
            'a committed pre-effect rejection is already settled')
        for changed in ({'work_id': 'different'}, {'native_effects': {}}, {'selection': {'inference': 'submitted'}},
                        {'call_observations': [{'settled': True}]},
                        {'call_observations': [{'offered': False, 'settled': 1}]}):
            with self.assertRaises(AssertionError):
                startup_settled(dict(value, **changed), 'original')


if __name__ == '__main__':
    unittest.main()
