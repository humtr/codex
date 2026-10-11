"""Strict owned Responses proof, never a live web-service implementation."""
import hashlib
import json
import re
import shlex


def inventory(tools):
    if not isinstance(tools, list) or not tools:
        raise ValueError('missing tool inventory')
    routes = {}

    def visit(tool, namespace=None):
        if not isinstance(tool, dict):
            raise ValueError('malformed tool definition')
        kind, name = tool.get('type'), tool.get('name')
        if kind == 'namespace':
            if namespace is not None or not isinstance(name, str) or not name:
                raise ValueError('ambiguous namespace')
            children = tool.get('tools')
            if not isinstance(children, list) or not children:
                raise ValueError('empty namespace')
            for child in children:
                visit(child, name)
            return
        if kind == 'tool_search':
            if namespace is not None or name is not None or tool.get('execution') != 'client':
                raise ValueError('unsupported search execution')
            name = 'tool_search'
        elif kind not in ('function', 'custom') or not isinstance(name, str) or not name:
            raise ValueError('unsupported tool definition')
        key = (namespace, name)
        if key in routes:
            raise ValueError('duplicate tool route')
        routes[key] = tool

    for tool in tools:
        visit(tool)
    return routes


def declared_tools(body):
    classic = body.get('tools')
    additional = [row['tools'] for row in body.get('input', [])
                  if row.get('type') == 'additional_tools']
    if classic and additional:
        raise ValueError('ambiguous classic and Lite declarations')
    if additional:
        if any(not isinstance(tools, list) for tools in additional):
            raise ValueError('malformed Lite declarations')
        return [tool for group in additional for tool in group]
    return classic


def text_output(output):
    if isinstance(output, list) and output:
        if any(not isinstance(item, dict) or item.get('type') not in ('input_text', 'output_text')
               or not isinstance(item.get('text'), str) for item in output):
            raise ValueError('unsupported result representation')
        output = ''.join(item['text'] for item in output)
    if not isinstance(output, str):
        raise ValueError('unsupported result representation')
    return output


def result_nonce(output, kind='function_call'):
    text = text_output(output)
    if kind == 'function_call':
        if re.fullmatch(r'[0-9a-f]{32}\n?', text):
            return text.strip()
        pattern = (r'(?:Chunk ID: [A-Za-z0-9_-]+\n)?Wall time: [0-9.]+ seconds\n'
                   r'Process exited with code 0\n(?:Original token count: [0-9]+\n)?'
                   r'Output:\n([0-9a-f]{32})\n?')
    elif kind == 'custom_tool_call':
        pattern = (r'Script completed\nWall time [0-9.]+ seconds'
                   r'(?: \(code-mode [0-9.]+ seconds; overhead -?[0-9.]+ seconds\))?'
                   r'\nOutput:\n([0-9a-f]{32})\n?')
    else:
        raise ValueError('unsupported call type')
    match = re.fullmatch(pattern, text)
    if match is None:
        raise ValueError('command did not return exactly one successful nonce')
    return match[1]


class Transcript:
    def __init__(self, nonce):
        self.nonce = nonce
        self.calls = []
        self.outputs = {}
        self.tool_definitions = None
        self.routes = None

    def receive(self, body):
        if (not isinstance(body, dict) or not isinstance(body.get('input'), list)
                or any(not isinstance(row, dict) for row in body['input'])):
            raise ValueError('invalid Responses request')
        supported = {'function_call', 'custom_tool_call', 'function_call_output', 'custom_tool_call_output'}
        for row in body['input']:
            kind = row.get('type', '')
            if not isinstance(kind, str) or (kind.endswith(('_call', '_call_output')) and kind not in supported):
                raise ValueError('unsupported call/result type')
        definitions = declared_tools(body)
        routes = inventory(definitions)
        if self.tool_definitions is None:
            self.tool_definitions = definitions
            self.routes = routes
        elif definitions != self.tool_definitions:
            raise ValueError('advertised inventory changed during owned roundtrip')
        calls = [row for row in body['input'] if row.get('type') in ('function_call', 'custom_tool_call')]
        results = [row for row in body['input'] if row.get('type') in ('function_call_output', 'custom_tool_call_output')]
        if len(calls) != len(self.calls) or len(results) != len(self.calls):
            raise ValueError('missing or duplicate call/result')
        for expected in self.calls:
            selected = [row for row in calls if row.get('call_id') == expected['call_id']]
            outputs = [row for row in results if row.get('call_id') == expected['call_id']]
            if len(selected) != 1 or len(outputs) != 1:
                raise ValueError('wrong or duplicate call ID')
            actual, output = selected[0], outputs[0]
            for field in ('type', 'name', 'namespace', 'input'):
                if actual.get(field) != expected.get(field):
                    raise ValueError('call identity or input changed')
            if expected['type'] == 'function_call':
                if json.loads(actual.get('arguments', 'null')) != json.loads(expected['arguments']):
                    raise ValueError('function arguments changed')
            if output['type'] != expected['type'].replace('_call', '_call_output'):
                raise ValueError('result type mismatch')
            if result_nonce(output.get('output'), expected['type']) != self.nonce:
                raise ValueError('nonce/result mismatch')
            value = text_output(output.get('output'))
            previous = self.outputs.get(expected['call_id'])
            if previous is not None and value != previous:
                raise ValueError('previous result changed')
            self.outputs[expected['call_id']] = value
        return routes

    def next_command(self, cwd, roots):
        choices = [(namespace, name) for namespace, name in self.routes
                   if name == 'exec_command' and self.routes[(namespace, name)]['type'] == 'function']
        if set(roots) != {'HOME', 'CODEX_HOME', 'PREFIX'}:
            raise ValueError('owned execution roots required')
        guard = ' && '.join('test "$' + key + '" = ' + shlex.quote(roots[key])
                            for key in ['HOME', 'CODEX_HOME', 'PREFIX'])
        args = {'cmd': guard + ' && cat nonce.txt', 'workdir': str(cwd), 'max_output_tokens': 200,
                'login': False, 'shell': '/data/data/com.termux/files/usr/bin/bash', 'yield_time_ms': 1000}
        if len(choices) == 1:
            namespace, name = choices[0]
            call = {'type': 'function_call', 'name': name, 'arguments': json.dumps(args)}
        elif not choices and self.routes.get(('functions', 'exec'), {}).get('type') == 'custom':
            tool = self.routes[('functions', 'exec')]
            if 'declare const tools: { exec_command' not in tool.get('description', ''):
                raise ValueError('nested exec_command route is not advertised')
            namespace = 'functions'
            source = ('const result = await tools.exec_command(' + json.dumps(args) + '); '
                      'if (result.exit_code !== 0 || typeof result.output !== "string") '
                      'throw new Error("owned command failed"); text(result.output.trim());')
            call = {'type': 'custom_tool_call', 'name': 'exec', 'input': source}
        else:
            raise ValueError('exact exec_command route unavailable')
        call.update(id=f'owned_call_{len(self.calls)}', call_id=f'call_owned_{len(self.calls)}', status='completed')
        if namespace is not None:
            call['namespace'] = namespace
        self.calls.append(call)
        return call

    def final_reply(self):
        if len(self.calls) != 2 or len(self.outputs) != 2:
            raise ValueError('two correlated actual outputs required')
        proof = hashlib.sha256((''.join(result_nonce(self.outputs[c['call_id']], c['type'])
                                        for c in self.calls)).encode()).hexdigest()
        return 'OWNED_RESULT_' + proof


def events(item, number):
    response = {'id': f'resp_owned_{number}', 'object': 'response', 'status': 'completed',
                'output': [item], 'usage': {'input_tokens': 1, 'output_tokens': 1, 'total_tokens': 2}}
    initial = dict(item, status='in_progress')
    if item['type'] == 'function_call':
        initial['arguments'] = ''
    elif item['type'] == 'custom_tool_call':
        initial['input'] = ''
    else:
        initial['content'] = []
    output = [
        {'type': 'response.created', 'response': dict(response, status='in_progress', output=[])},
        {'type': 'response.output_item.added', 'output_index': 0, 'item': initial},
    ]
    if item['type'] == 'function_call':
        output.append({'type': 'response.function_call_arguments.delta', 'item_id': item['id'],
                       'output_index': 0, 'delta': item['arguments']})
    elif item['type'] == 'custom_tool_call':
        output.append({'type': 'response.custom_tool_call_input.delta', 'item_id': item['id'],
                       'output_index': 0, 'delta': item['input']})
    else:
        output += [
            {'type': 'response.content_part.added', 'item_id': item['id'], 'output_index': 0,
             'content_index': 0, 'part': {'type': 'output_text', 'text': '', 'annotations': []}},
            {'type': 'response.output_text.delta', 'item_id': item['id'], 'output_index': 0,
             'content_index': 0, 'delta': item['content'][0]['text']},
        ]
    output += [{'type': 'response.output_item.done', 'output_index': 0, 'item': item},
               {'type': 'response.completed', 'response': response}]
    return ''.join('event: ' + row['type'] + '\ndata: ' + json.dumps(row) + '\n\n'
                   for row in output).encode()
