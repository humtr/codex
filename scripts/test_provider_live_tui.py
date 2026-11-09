"""Exercise the live qualifier's observer fault, never a real model."""
import hashlib
import http.client
import http.server
import json
from pathlib import Path
import sys
import threading
import unittest

sys.path.insert(0, str(Path(__file__).parent))
from qualify_provider_live_tui import ObservationProxy, plain


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
                    {'type': 'response.completed', 'response': {'id': 'original-response'}},
                ]:
                    self.wfile.write(b'data: ' + json.dumps(value).encode() + b'\n\n')
                    self.wfile.flush()

        upstream = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Model)
        upstream.daemon_threads = True
        threading.Thread(target=upstream.serve_forever, daemon=True).start()
        proxy = ObservationProxy(f'http://127.0.0.1:{upstream.server_port}/v1')
        checks = []
        proxy.armed_thread = 'owned-native-thread'

        def effect_observed():
            checks.append(True)
            return len(checks) > 1

        proxy.can_cut = effect_observed
        client = http.client.HTTPConnection('127.0.0.1', proxy.server.server_port, timeout=5)
        body = b'{"model":"chatgpt-desktop/gpt-6","input":"owned fixture"}'
        try:
            client.request('POST', '/v1/responses', body, headers={
                'session_id': 'owned-native-thread', 'Authorization': 'Bearer private-fixture-token'})
            response = client.getresponse().read()
            self.assertIn(b'before effect', response)
            self.assertIn(b'after confirmed effect', response)
            self.assertNotIn(b'response.completed', response)
            self.assertTrue(proxy.fired.wait(5))
            self.assertTrue(proxy.settled.wait(5))
            self.assertEqual(requests, [body])
            record = proxy.records[0]
            self.assertEqual(record['terminal'], 'response.completed')
            self.assertEqual(record['response_id'], 'original-response')
            self.assertEqual(record['body_sha256'], hashlib.sha256(body).hexdigest())
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


if __name__ == '__main__':
    unittest.main()
