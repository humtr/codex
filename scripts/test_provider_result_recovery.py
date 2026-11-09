import concurrent.futures
import json
import threading
from types import SimpleNamespace
import unittest

from qualify_provider_result_recovery import completed_command, executable_item, fault_observed, native_request_identity, startup_settled


class ResultFaultBoundary(unittest.TestCase):
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
                        {'call_observations': [{'offered': True, 'settled': False}]}):
            self.assertFalse(startup_settled(dict(value, **changed), 'original'))
        for changed in ({'work_id': 'different'}, {'native_effects': {}}, {'selection': {'inference': 'submitted'}}):
            with self.assertRaises(AssertionError):
                startup_settled(dict(value, **changed), 'original')


if __name__ == '__main__':
    unittest.main()
