import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


class OwnedCatalogTests(unittest.TestCase):
    def test_discovery_and_exact_read_do_not_modify_the_marker(self):
        with tempfile.TemporaryDirectory(dir=os.environ['TMPDIR']) as temporary:
            root = Path(temporary)
            marker = root/'marker'
            marker.write_text('owned-proof')
            before = marker.stat()
            requests = [
                {'jsonrpc': '2.0', 'id': 1, 'method': 'initialize', 'params': {'protocolVersion': '2025-11-25'}},
                {'jsonrpc': '2.0', 'method': 'notifications/initialized'},
                {'jsonrpc': '2.0', 'id': 2, 'method': 'tools/list'},
                {'jsonrpc': '2.0', 'id': 3, 'method': 'tools/call', 'params': {'name': '_read_marker', 'arguments': {}}},
            ]
            result = subprocess.run([sys.executable, '-B', str(Path(__file__).with_name('owned_catalog_mcp.py')), str(marker), str(root/'evidence')],
                                    input=''.join(json.dumps(r)+'\n' for r in requests), text=True, capture_output=True, check=True)
            replies = [json.loads(r) for r in result.stdout.splitlines()]
            self.assertEqual([r['id'] for r in replies], [1, 2, 3])
            self.assertEqual(replies[1]['result']['tools'][0]['name'], '_read_marker')
            self.assertEqual(replies[2]['result']['content'][0]['text'], 'owned-proof')
            self.assertEqual(json.loads((root/'evidence').read_text())['actualCalls'], 1)
            self.assertEqual((marker.stat().st_ino, marker.stat().st_mtime_ns), (before.st_ino, before.st_mtime_ns))


if __name__ == '__main__':
    unittest.main()
