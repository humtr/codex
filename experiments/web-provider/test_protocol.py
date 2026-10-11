import copy
import json
import unittest

from protocol import Transcript, result_nonce, inventory, declared_tools

NONCE = 'a1' * 16
TOOLS = [{'type': 'namespace', 'name': 'functions', 'tools': [
    {'type': 'function', 'name': 'exec_command', 'parameters': {'type': 'object'}},
    {'type': 'custom', 'name': 'apply_patch', 'format': {'type': 'grammar', 'syntax': 'lark', 'definition': 'start: /.+/'}}
]}]


def request(rows):
    return {'tools': copy.deepcopy(TOOLS), 'input': rows}


class ProtocolTests(unittest.TestCase):
    def pending(self):
        transcript = Transcript(NONCE)
        transcript.receive(request([]))
        call = transcript.next_command('/owned', {'HOME': '/owned/h', 'CODEX_HOME': '/owned/h/.codex', 'PREFIX': '/owned/p'})
        result = {'type': 'function_call_output', 'call_id': call['call_id'], 'output': NONCE}
        return transcript, call, result

    def test_two_real_correlated_returns_are_required_before_derived_reply(self):
        transcript, call, result = self.pending()
        with self.assertRaises(ValueError):
            transcript.final_reply()
        transcript.receive(request([call, result]))
        second = transcript.next_command('/owned', {'HOME': '/owned/h', 'CODEX_HOME': '/owned/h/.codex', 'PREFIX': '/owned/p'})
        with self.assertRaises(ValueError):
            transcript.final_reply()
        second_result = dict(result, call_id=second['call_id'])
        transcript.receive(request([call, result, second, second_result]))
        self.assertEqual(transcript.final_reply(), 'OWNED_RESULT_' + __import__('hashlib').sha256((NONCE * 2).encode()).hexdigest())

    def test_missing_wrong_duplicate_or_changed_call_and_result_refuse(self):
        for mutation in ['missing', 'wrong_id', 'duplicate', 'arguments', 'namespace', 'name', 'output', 'failed_command', 'result_type']:
            transcript, call, result = self.pending()
            rows = [copy.deepcopy(call), copy.deepcopy(result)]
            if mutation == 'missing': rows.pop()
            elif mutation == 'wrong_id': rows[1]['call_id'] = 'other'
            elif mutation == 'duplicate': rows.append(copy.deepcopy(result))
            elif mutation == 'arguments': rows[0]['arguments'] = json.dumps({'cmd': 'echo forged'})
            elif mutation == 'namespace': rows[0]['namespace'] = 'other'
            elif mutation == 'name': rows[0]['name'] = 'other'
            elif mutation == 'output': rows[1]['output'] = 'b2' * 16
            elif mutation == 'failed_command': rows[1]['output'] = 'Wall time: 1 seconds\nProcess exited with code 1\nOutput:\n' + NONCE
            else: rows[1]['type'] = 'custom_tool_call_output'
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                transcript.receive(request(rows))

    def test_namespace_and_custom_schema_are_preserved_without_flattening(self):
        routes = inventory(TOOLS)
        self.assertEqual(routes[('functions', 'apply_patch')], TOOLS[0]['tools'][1])
        self.assertEqual(routes[('functions', 'exec_command')], TOOLS[0]['tools'][0])
        for tools in [[], [dict(TOOLS[0], tools=[])], TOOLS + TOOLS, [{'type': 'unknown', 'name': 'shell'}]]:
            with self.subTest(tools=tools), self.assertRaises(ValueError):
                inventory(tools)

    def test_modified_inventory_or_previous_result_refuses(self):
        transcript, call, result = self.pending()
        transcript.receive(request([call, result]))
        changed = request([call, result]);changed['tools'][0]['tools'][0]['parameters'] = {'type': 'array'}
        with self.assertRaises(ValueError): transcript.receive(changed)
        result['output'] = 'Wall time: 1 seconds\nProcess exited with code 0\nOutput:\n' + NONCE
        with self.assertRaisesRegex(ValueError, 'previous result changed'):
            transcript.receive(request([call, result]))

    def test_lite_gateway_preserves_custom_input_and_correlated_text_parts(self):
        tools = [{'type': 'namespace', 'name': 'functions', 'tools': [
            {'type': 'custom', 'name': 'exec', 'description': 'declare const tools: { exec_command',
             'format': {'type': 'grammar', 'syntax': 'lark', 'definition': 'start: /.+/'}}]}]
        decl = {'type': 'additional_tools', 'role': 'developer', 'tools': tools}
        transcript = Transcript(NONCE)
        transcript.receive({'input': [decl]})
        first = transcript.next_command('/owned', {'HOME': '/owned/h', 'CODEX_HOME': '/owned/h/.codex', 'PREFIX': '/owned/p'})
        self.assertEqual(first['type'], 'custom_tool_call')
        self.assertEqual(first['namespace'], 'functions')
        output = [{'type': 'input_text', 'text': 'Script completed\nWall time 1.000 seconds (code-mode 0.500 seconds; overhead 0.500 seconds)\nOutput:\n'},
                  {'type': 'input_text', 'text': NONCE}]
        result = {'type': 'custom_tool_call_output', 'call_id': first['call_id'], 'output': output}
        transcript.receive({'input': [decl, first, result]})
        second = transcript.next_command('/owned', {'HOME': '/owned/h', 'CODEX_HOME': '/owned/h/.codex', 'PREFIX': '/owned/p'})
        second_result = dict(result, call_id=second['call_id'])
        transcript.receive({'input': [decl, first, result, second, second_result]})
        self.assertTrue(transcript.final_reply().startswith('OWNED_RESULT_'))
        altered = dict(first, input=first['input'] + 'text("forged");')
        with self.assertRaises(ValueError):
            transcript.receive({'input': [decl, altered, result, second, second_result]})
        for payload in ['Script failed\nWall time 1.0 seconds\nOutput:\n' + NONCE,
                        output + [{'type': 'input_image', 'image_url': 'owned'}]]:
            with self.assertRaises(ValueError): result_nonce(payload, 'custom_tool_call')

    def test_ambiguous_lite_or_unadvertised_nested_route_is_refused(self):
        decl = {'type': 'additional_tools', 'role': 'developer', 'tools': TOOLS}
        with self.assertRaises(ValueError): declared_tools({'tools': TOOLS, 'input': [decl]})
        with self.assertRaises(ValueError): inventory(declared_tools({'input': [decl, decl]}))
        for tools in [[{'type': 'function', 'name': 'other'}],
                      [{'type': 'namespace', 'name': 'functions', 'tools': [{'type': 'custom', 'name': 'exec'}]}]]:
            transcript = Transcript(NONCE);transcript.receive({'tools': tools, 'input': []})
            with self.assertRaises(ValueError): transcript.next_command('/owned', {'HOME': '/owned/h', 'CODEX_HOME': '/owned/h/.codex', 'PREFIX': '/owned/p'})
        with self.assertRaises(ValueError): Transcript(NONCE).receive({'input': [None]})

    def test_search_metadata_is_preserved_but_unselected_calls_refuse(self):
        search = {'type': 'tool_search', 'execution': 'client', 'parameters': {'type': 'object'}}
        self.assertEqual(inventory(TOOLS + [search])[(None, 'tool_search')], search)
        for bad in [dict(search, execution='server'), dict(search, name='custom'), search]:
            if bad is search:
                request_body = request([{'type': 'tool_search_call', 'call_id': 'unselected'}])
                with self.assertRaises(ValueError): Transcript(NONCE).receive(request_body)
            else:
                with self.assertRaises(ValueError): inventory([bad])

    def test_actual_shell_scope_guard_refuses_caller_home_leak(self):
        import os
        from pathlib import Path
        import subprocess
        import tempfile
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary);(root / 'nonce.txt').write_text(NONCE + '\n')
            roots = {'HOME': str(root / 'h'), 'CODEX_HOME': str(root / 'c'), 'PREFIX': str(root / 'p')}
            transcript = Transcript(NONCE);transcript.receive(request([]))
            command = json.loads(transcript.next_command(root, roots)['arguments'])
            env = {'PATH': '/data/data/com.termux/files/usr/bin', **roots}
            good = subprocess.run(['/data/data/com.termux/files/usr/bin/bash', '-c', command['cmd']], cwd=root, env=env, capture_output=True)
            self.assertEqual((good.returncode, good.stdout), (0, (NONCE + '\n').encode()))
            env['HOME'] = str(root / 'wrong')
            bad = subprocess.run(['/data/data/com.termux/files/usr/bin/bash', '-c', command['cmd']], cwd=root, env=env, capture_output=True)
            self.assertNotEqual(bad.returncode, 0)
            self.assertEqual(bad.stdout, b'')

    def test_only_exact_successful_nonce_output_is_accepted(self):
        self.assertEqual(result_nonce('Chunk ID: owned\nWall time: 1 seconds\nProcess exited with code 0\nOutput:\n' + NONCE), NONCE)
        for value in [NONCE + '\nextra', [{'type': 'image', 'text': NONCE}], {'output': NONCE}, NONCE + 'f']:
            with self.subTest(value=value), self.assertRaises(ValueError): result_nonce(value)


if __name__ == '__main__':
    unittest.main()
