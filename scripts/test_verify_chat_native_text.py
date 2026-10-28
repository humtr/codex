import copy
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from scripts.verify_chat_native_text import compare


def evidence():
    def message(id, parent, role, text, channel=None, **fields):
        return dict(id=id, parent=parent, role=role, channel=channel, recipient='all',
                    content={'content_type': 'text', 'parts': [text]}, unavailable=None, **fields)
    progress = '전체 공개 진행입니다.\n\n```text\n정확한 원문 🙂\n```'
    messages = [message('old', None, 'assistant', 'old', 'final', end_turn=True,
                        status='finished_successfully'),
                message('user', 'old', 'user', 'Original request'),
                message('progress1', 'user', 'assistant', progress, 'commentary'),
                message('progress2', 'progress1', 'assistant', progress, 'commentary'),
                message('final', 'progress2', 'assistant', 'Complete final\n완료', 'final',
                        end_turn=True, status='finished_successfully'),
                message('hidden', 'final', 'assistant', '', 'final', end_turn=True,
                        status='finished_successfully')]
    messages[-1].update(unavailable='hidden', content=None)
    pages = []
    for start, end in [(0, 3), (3, 6)]:
        cursor = dict(account='account', conversation='conversation', branch='hidden',
                      count=end, prefix='source-owned-opaque-prefix')
        pages.append(dict(account='account', conversation_id='conversation', branch='hidden',
                          total=6, messages=messages[start:end], complete=end == 6, cursor=cursor,
                          input_cursor=dict(cursor, count=5)))
    baseline = {'items': [{'turn_id': 'old', 'item': {'type': 'agentMessage', 'text': 'old'}}]}
    native = copy.deepcopy(baseline)
    for item in [dict(type='userMessage', content=[{'type': 'text', 'text': 'Original request'}]),
                 dict(type='agentMessage', phase='commentary', text=progress),
                 dict(type='commandExecution', status='completed', exitCode=0),
                 dict(type='agentMessage', phase='commentary', text=progress),
                 dict(type='agentMessage', phase='final_answer', text='Complete final\n완료')]:
        native['items'].append({'turn_id': 'new-turn', 'item': item})
    status = dict(work_id='work', thread_id='thread', phase='completed',
                  pending_calls=[], native_status={'type': 'idle'})
    return pages, native, baseline, status


def verify(parts):
    return compare(*parts, 'Original request', conversation='conversation', work='work', thread='thread')


class OriginalPublicText(unittest.TestCase):
    def test_saved_input_suffix_extends_verified_original_prefix(self):
        pages, native, baseline, status = evidence()
        messages = [m for page in pages for m in page['messages']]
        cursor = dict(pages[-1]['cursor'], branch='old', count=1)
        prefix = [dict(pages[0], branch='old', total=1, messages=messages[:1],
                       complete=True, cursor=cursor, input_cursor=cursor)]
        result = dict(complete=True, input=dict(phase='completed', conversation_id='conversation',
                                               message_id='user'),
                      source_binding=dict(prefix_verified=True, resolution='exact',
                                          requested_parent='old', saved_parent='old',
                                          delivery_source_cursor=copy.deepcopy(cursor)),
                      cursor=copy.deepcopy(pages[-1]['cursor']), messages=messages[1:],
                      completed_message_id='final', terminal_message_id='hidden')
        for fault in [None, 'prefix', 'identity', 'input', 'parent', 'count', 'completion', 'overlap']:
            with self.subTest(fault=fault):
                value = copy.deepcopy(result)
                if fault == 'prefix':
                    value['source_binding']['delivery_source_cursor']['prefix'] = 'changed'
                elif fault == 'identity':
                    value['cursor']['account'] = 'other'
                elif fault == 'input':
                    value['input']['message_id'] = 'other'
                elif fault == 'parent':
                    value['messages'][0]['parent'] = 'other'
                elif fault == 'count':
                    value['cursor']['count'] += 1
                elif fault == 'completion':
                    value['completed_message_id'] = 'progress1'
                elif fault == 'overlap':
                    value['messages'][1]['id'] = 'old'
                def check():
                    return compare(prefix, native, baseline, status, 'Original request',
                                   conversation='conversation', work='work', thread='thread',
                                   saved_result=value)
                if fault is None:
                    self.assertTrue(check()['accepted_public_text'])
                else:
                    with self.assertRaises(ValueError):
                        check()

    def test_exact_multiline_progress_with_legitimate_repeated_text_and_hidden_suffix(self):
        for channel in ['commentary', None]:
            with self.subTest(channel=channel):
                parts = evidence()
                parts[0][0]['messages'][2]['channel'] = channel
                result = verify(parts)
                self.assertTrue(result['accepted_public_text'])
                self.assertEqual(result['source_user_id'], 'user')
                self.assertEqual(result['source_final_id'], 'final')
                self.assertEqual(result['source_progress_count'], 2)
                self.assertEqual(result['source_unavailable_messages'], 1)

    def test_summary_missing_duplicate_or_reordered_public_text_cannot_pass(self):
        for kind in ['summary', 'missing', 'duplicate', 'reordered', 'final', 'extra-final', 'early-final', 'late-input', 'unsettled']:
            with self.subTest(kind=kind):
                pages, native, baseline, status = evidence()
                if kind == 'summary':
                    native['items'][2]['item']['text'] = 'short summary'
                elif kind == 'missing':
                    del native['items'][2]
                elif kind == 'duplicate':
                    native['items'].insert(-1, copy.deepcopy(native['items'][2]))
                elif kind == 'reordered':
                    pages[1]['messages'][0]['content']['parts'] = ['Different later progress']
                    native['items'][4]['item']['text'] = 'Different later progress'
                    native['items'][2], native['items'][4] = native['items'][4], native['items'][2]
                elif kind == 'final':
                    native['items'][-1]['item']['text'] += '\nExtra summary'
                elif kind == 'early-final':
                    native['items'].insert(2, native['items'].pop())
                elif kind == 'late-input':
                    native['items'].insert(3, native['items'].pop(1))
                elif kind == 'extra-final':
                    native['items'].append(copy.deepcopy(native['items'][-1]))
                else:
                    status['native_status'] = {'type': 'active'}
                self.assertFalse(verify((pages, native, baseline, status))['accepted_public_text'])

    def test_source_identity_pagination_and_native_prefix_require_original_evidence(self):
        for fault in ['account', 'branch', 'gap', 'duplicate-id', 'incomplete', 'ancestry',
                      'later-input', 'source-order', 'unsupported-public', 'native-prefix', 'native-turn']:
            with self.subTest(fault=fault):
                pages, native, baseline, status = evidence()
                if fault in ['account', 'branch']:
                    pages[1][fault] = 'different'
                elif fault == 'gap':
                    pages[1]['cursor']['count'] += 1
                elif fault == 'duplicate-id':
                    pages[1]['messages'][0]['id'] = 'progress1'
                elif fault == 'incomplete':
                    pages[1]['complete'] = False
                elif fault == 'ancestry':
                    pages[1]['messages'][1]['parent'] = 'missing-parent'
                elif fault == 'later-input':
                    pages[1]['messages'][0]['role'] = 'user'
                elif fault == 'source-order':
                    pages[0]['messages'][2], pages[1]['messages'][0] = pages[1]['messages'][0], pages[0]['messages'][2]
                elif fault == 'unsupported-public':
                    pages[1]['messages'][0]['unavailable'] = 'unsupported_content'
                elif fault == 'native-prefix':
                    native['items'][0]['item']['text'] = 'changed old history'
                else:
                    native['items'][-1]['turn_id'] = 'another-turn'
                with self.assertRaises(ValueError):
                    verify((pages, native, baseline, status))

    def test_cli_keeps_original_prompt_line_endings(self):
        pages, native, baseline, status = evidence()
        prompt = 'Original\r\nrequest'
        pages[0]['messages'][1]['content']['parts'] = [prompt]
        native['items'][1]['item']['content'][0]['text'] = prompt
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            args = [sys.executable, str(Path(__file__).with_name('verify_chat_native_text.py')),
                    '--conversation', 'conversation', '--work', 'work', '--thread', 'thread']
            for index, page in enumerate(pages):
                path = root / f'page{index}.json'
                path.write_text(json.dumps(page))
                args += ['--source-page', str(path)]
            for flag, data in [('native-history', native), ('baseline-history', baseline),
                               ('native-status', status)]:
                path = root / (flag + '.json')
                path.write_text(json.dumps(data))
                args += ['--' + flag, str(path)]
            request = root / 'request.txt'
            request.write_bytes(prompt.encode('utf-8'))
            report = root / 'report.json'
            result = subprocess.run(args + ['--prompt', str(request), '--report', str(report)],
                                    capture_output=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertTrue(json.loads(report.read_text())['accepted_public_text'])
