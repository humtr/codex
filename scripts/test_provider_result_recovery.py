import unittest

from qualify_provider_result_recovery import completed_command, executable_item, startup_settled


class ResultFaultBoundary(unittest.TestCase):
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
