"""Private stdio MCP fixture: read one owned marker, never execute user commands."""
import json
import os
from pathlib import Path
import sys


def serve(marker, evidence):
    assert marker.parent == evidence.parent and marker.parent.stat().st_mode & 0o777 == 0o700
    calls = 0
    for line in sys.stdin:
        request = json.loads(line)
        if 'id' not in request:
            continue
        method = request.get('method')
        if method == 'initialize':
            result = {'protocolVersion': request['params']['protocolVersion'], 'capabilities': {'tools': {}},
                      'serverInfo': {'name': 'owned-catalog-fixture', 'version': '1'}}
        elif method == 'tools/list':
            result = {'tools': [{'name': '_read_marker', 'description': 'Read the owned proof marker. No shell or other file access.',
                                 'inputSchema': {'type': 'object', 'properties': {}, 'additionalProperties': False}}]}
        elif method == 'tools/call' and request.get('params', {}).get('name') == '_read_marker':
            calls += 1
            assert calls == 1, 'fixture_tool_replayed'
            fd = os.open(evidence, os.O_WRONLY | os.O_CREAT | os.O_TRUNC | os.O_NOFOLLOW, 0o600)
            with os.fdopen(fd, 'w') as output:
                json.dump({'actualCalls': calls, 'tool': '_read_marker'}, output)
            result = {'content': [{'type': 'text', 'text': marker.read_text().strip()}], 'isError': False}
        elif method == 'ping':
            result = {}
        else:
            print(json.dumps({'jsonrpc': '2.0', 'id': request['id'], 'error': {'code': -32601, 'message': 'Unknown fixture method'}}), flush=True)
            continue
        print(json.dumps({'jsonrpc': '2.0', 'id': request['id'], 'result': result}), flush=True)


if __name__ == '__main__':
    os.umask(0o077)
    serve(Path(sys.argv[1]), Path(sys.argv[2]))
