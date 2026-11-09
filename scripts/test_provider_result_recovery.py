import unittest

from qualify_provider_result_recovery import completed_command, executable_item


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


if __name__ == '__main__':
    unittest.main()
