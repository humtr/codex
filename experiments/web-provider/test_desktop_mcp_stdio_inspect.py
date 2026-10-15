"""The metadata tap forwards only the explicit unbound inventory call."""
import json
import os
from pathlib import Path
import select
import subprocess
import sys
import tempfile
import unittest


class MetadataTapTests(unittest.TestCase):
    def probe(self, call):
        with tempfile.TemporaryDirectory(dir=os.environ['TMPDIR']) as temporary:
            root = Path(temporary)
            target = root / 'target.py'
            target.write_text('''import json, sys
for line in sys.stdin:
    row = json.loads(line)
    if 'id' not in row: continue
    result = ({'protocolVersion': '2025-11-25', 'capabilities': {'tools': {}}}
              if row.get('method') == 'initialize' else {'isError': True, 'content': []})
    print(json.dumps({'jsonrpc': '2.0', 'id': row['id'], 'result': result}), flush=True)
''')
            argv = root / 'argv.json'
            argv.write_text(json.dumps([sys.executable, '-B', str(target)]))
            wire = root / 'wire.jsonl'
            child = subprocess.Popen(
                [sys.executable, '-B', str(Path(__file__).with_name('desktop_mcp_stdio_inspect.py')),
                 str(argv), str(wire)], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                stderr=subprocess.PIPE, text=True,
            )
            replies = []
            try:
                for row in [
                    {'jsonrpc': '2.0', 'id': 'init', 'method': 'initialize', 'params': {
                        'protocolVersion': '2025-11-25', 'capabilities': {}}}, call,
                ]:
                    child.stdin.write(json.dumps(row) + '\n')
                    child.stdin.flush()
                    self.assertTrue(select.select([child.stdout], [], [], 5)[0], 'tap response timeout')
                    replies.append(json.loads(child.stdout.readline()))
            finally:
                child.stdin.close()
                try:
                    child.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    child.kill()
                    child.wait(timeout=5)
                child.stdout.close()
                error = child.stderr.read()
                child.stderr.close()
            self.assertEqual(child.returncode, 0, error)
            return replies, [json.loads(line) for line in wire.read_text().splitlines()]

    def test_unbound_metadata_preserves_exact_response_identity(self):
        call = {'jsonrpc': '2.0', 'id': 42, 'method': 'tools/call', 'params': {
            'name': 'codex_tool_inventory', 'arguments': {'turn_token': 'owned_unbound_test'}}}
        replies, wire = self.probe(call)
        self.assertEqual(replies[-1]['id'], 42)
        self.assertTrue(replies[-1]['result']['isError'])
        self.assertEqual(wire[0]['protocolVersion'], '2025-11-25')
        request, response = wire[-2:]
        self.assertTrue(request['metadataProbe'])
        self.assertEqual(request['id'], response['id'])
        self.assertEqual(response['method'], 'tools/call')
        self.assertNotIn('owned_unbound_test', json.dumps(wire))

    def test_bound_or_execution_calls_never_reach_target(self):
        for name, token in [('codex_tool_inventory', 'real-owner'),
                            ('codex_exec', 'owned_unbound_test')]:
            with self.subTest(name=name):
                call = {'jsonrpc': '2.0', 'id': 'private-call', 'method': 'tools/call',
                        'params': {'name': name, 'arguments': {'turn_token': token}}}
                replies, wire = self.probe(call)
                self.assertEqual(replies[-1]['error']['code'], -32000)
                self.assertFalse(wire[-1]['metadataProbe'])
                self.assertEqual(wire[-1]['direction'], 'request')
                self.assertEqual(sum(row['direction'] == 'response' for row in wire), 1)


if __name__ == '__main__':
    unittest.main()
