"""Exercise the adapter's actual HTTP admission; never open or submit Chat."""
import argparse
import hashlib
import hmac
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time
import urllib.error
import urllib.request


def probe(bun, bridge):
    with tempfile.TemporaryDirectory(prefix='admit-', dir=os.environ['TMPDIR']) as tmp:
        root = Path(tmp)
        state = root/'server.json'
        script = Path(__file__).with_name('desktop_responses_server.mjs')
        with (root/'adapter.log').open('w') as log:
            child = subprocess.Popen([str(bun), str(script), str(bridge), str(root/'no-desktop'), str(state)], stdout=log, stderr=log)
            try:
                deadline = time.monotonic()+10
                while not state.exists():
                    assert child.poll() is None, 'adapter_start_failed'
                    assert time.monotonic()<deadline, 'adapter_start_timeout'
                    time.sleep(.05)
                binding = json.loads(state.read_text())
                base = binding['baseUrl'].removesuffix('/v1')

                def request(path, value=None, authenticated=True, headers=None):
                    headers = dict(headers or {})
                    if authenticated:
                        headers['Authorization'] = 'Bearer '+binding['token']
                    if value is not None:
                        headers['Content-Type'] = 'application/json'
                    req = urllib.request.Request(base+path, data=value, headers=headers)
                    try:
                        with urllib.request.urlopen(req, timeout=5) as response:
                            return response.status, response.read()
                    except urllib.error.HTTPError as error:
                        return error.code, error.read()

                nonce = 'a'*64
                status, ready = request('/__desktop_ready', headers={'x-termux-trial-challenge': nonce}, authenticated=False)
                assert status==200
                expected = hmac.new(binding['token'].encode(), ('desktop-ready-v1:'+nonce).encode(), hashlib.sha256).hexdigest()
                assert json.loads(ready)['proof']==expected
                status, _ = request('/v1/responses', b'{}', authenticated=False)
                assert status==401
                codes = []
                cases = [(b'{', 'invalid_desktop_json'),
                         (b'[]', 'invalid_desktop_request'),
                         (b'{"model":"native","stream":true}', 'unsupported_desktop_route'),
                         (b'{"model":"chatgpt-desktop/gpt-6","stream":false}', 'desktop_stream_required'),
                         (b'{"model":"chatgpt-desktop/gpt-6","stream":true,"previous_response_id":"private"}', 'desktop_response_cursor_unavailable')]
                for body, code in cases:
                    status, response = request('/v1/responses', body)
                    assert status==400
                    assert json.loads(response)['error']['message']==code
                    codes.append(code)
                extra_codes = []
                for body, headers, expected_status, code in [
                    (b'private', {'Content-Encoding': 'gzip'}, 415, 'desktop_encoding_unsupported'),
                    (b'private', {'Content-Encoding': 'zstd'}, 400, 'invalid_desktop_compression'),
                    (b'"\xff"', {}, 400, 'invalid_desktop_utf8'),
                    (json.dumps({'model': 'chatgpt-desktop/gpt-6', 'stream': True,
                                 'input': [{'type': 'compaction', 'encrypted_content': 'OWNED_OPAQUE'}]}).encode(), {}, 400, 'opaque_context_unavailable'),
                ]:
                    status, response = request('/v1/responses', body, headers=headers)
                    assert status == expected_status
                    assert json.loads(response)['error']['message'] == code
                    extra_codes.append(code)
                large = json.dumps({'model': 'native', 'stream': True, 'input': 'x'*(3*1024*1024)}).encode()
                status, response = request('/v1/responses', large)
                assert status == 400
                assert json.loads(response)['error']['message'] == 'unsupported_desktop_route'
                packed = subprocess.run([str(bun), '--eval',
                    'process.stdout.write(Bun.zstdCompressSync(await Bun.stdin.bytes()));'],
                    input=large, capture_output=True, check=True).stdout
                status, response = request('/v1/responses', packed, headers={'Content-Encoding': 'zstd'})
                assert status == 400
                assert json.loads(response)['error']['message'] == 'unsupported_desktop_route'
                bomb = subprocess.run([str(bun), '--eval',
                    'process.stdout.write(Bun.zstdCompressSync(new Uint8Array(33*1024*1024)));'],
                    capture_output=True, check=True).stdout
                status, response = request('/v1/responses', bomb, headers={'Content-Encoding': 'zstd'})
                assert status == 413
                assert json.loads(response)['error']['message'] == 'desktop_decoded_request_too_large'
                extra_codes.append('desktop_decoded_request_too_large')
                body_send_error = None
                try:
                    status, _ = request('/v1/responses', b'"'+b'x'*(16*1024*1024)+b'"')
                except urllib.error.URLError as error:
                    assert isinstance(error.reason, BrokenPipeError)
                    body_send_error = 'broken_pipe'
                status, _ = request('/v1/responses', b'', headers={'Content-Length': str(16*1024*1024+2)})
                assert status==413
                try:
                    status, response = request('/v1/responses', iter([b'x'*(1024*1024)]*17),
                                               headers={'Transfer-Encoding': 'chunked'})
                    assert status == 413
                    assert json.loads(response)['error']['message'] == 'desktop_request_too_large'
                    chunked_limit = 'typed_413'
                except urllib.error.URLError as error:
                    assert isinstance(error.reason, BrokenPipeError)
                    chunked_limit = 'broken_pipe'
                report = json.loads(state.with_suffix('.json.proof').read_text())
                assert report['requests']==report['submittedRequests']==report['contextPackets']==0
                assert report['observedChatSubmissions']==report['submittedContextRequests']==0
                assert report['submissionSources']=={'app_accepted': 0, 'ui_observed': 0}
                # Bun's outer body ceiling can reject before entering fetch.
                assert report['arrivals'] >= len(cases)+len(extra_codes)+2
                for code in codes+extra_codes:
                    expected_count = 3 if code == 'unsupported_desktop_route' else 1
                    assert report['admissionFailures'][code] == expected_count
                return {'ready': True, 'unauthorizedStatus': 401, 'rejectionCodes': codes,
                        'extraRejectionCodes': extra_codes, 'largeIdentityAndZstd': True,
                        'wireLimitBytes': 16*1024*1024, 'decodedLimitBytes': 32*1024*1024,
                        'chunkedWireLimit': chunked_limit,
                        'oversizeStatus': status, 'oversizeBodySendError': body_send_error,
                        'arrivals': report['arrivals'], 'admittedRequests': 0, 'chatSubmissions': 0}
            finally:
                child.terminate()
                try:
                    child.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    child.kill();child.wait()


if __name__=='__main__':
    parser = argparse.ArgumentParser()
    for name in ['bun', 'bridge', 'report']:
        parser.add_argument('--'+name, type=Path, required=True)
    args = parser.parse_args()
    result = probe(args.bun, args.bridge)
    args.report.write_text(json.dumps(result, indent=2)+'\n')
    args.report.chmod(0o600)
    print(json.dumps(result))
