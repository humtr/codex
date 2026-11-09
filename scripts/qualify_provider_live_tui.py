#!/usr/bin/env python3
"""Explicit live installed-TUI acceptance through /provider and the real app.

This submits bounded, labelled real model requests. It never prepares a mock
model, changes an account/profile, installs a generation, or replays an input.
The optional fault proxy drops one downstream SSE observation after a real text
delta, while consuming the original upstream response. Evidence stays private.
"""
import argparse
import datetime
import fcntl
import hashlib
import http.client
import http.server
import json
import os
from pathlib import Path
import pty
import re
import select
import socket
import struct
import subprocess
import termios
import threading
import time
import urllib.parse

from qualify_tasks import Ws


def stamp():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()


def save(path, value):
    with path.open('x') as output:
        json.dump(value, output, ensure_ascii=False, indent=2)
        output.write('\n')


def sha(path):
    with path.open('rb') as source:
        return hashlib.file_digest(source, 'sha256').hexdigest()


def plain(value):
    value = re.sub(r'\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)', '', value)
    return re.sub(r'\x1b\[[0-?]*[ -/]*[@-~]|\x1b[^[]', '', value)


class ObservationProxy:
    """One request forward, no transport retries and no replacement result."""
    def __init__(self, endpoint):
        parsed = urllib.parse.urlsplit(endpoint)
        assert parsed.scheme == 'http' and parsed.hostname == '127.0.0.1'
        assert parsed.port and parsed.path.rstrip('/') == '/v1'
        assert not parsed.query and not parsed.fragment and not parsed.username
        self.port = parsed.port
        self.records = []
        self.armed_thread = None
        self.can_cut = lambda: False
        self.fired = threading.Event()
        self.settled = threading.Event()
        self.lock = threading.Lock()
        owner = self

        class Handler(http.server.BaseHTTPRequestHandler):
            protocol_version = 'HTTP/1.1'

            def log_message(self, *_):
                pass

            def do_GET(self):
                self.forward()

            def do_POST(self):
                self.forward()

            def forward(self):
                assert self.path in ('/v1/models', '/v1/responses')
                size = int(self.headers.get('Content-Length', '0'))
                assert 0 <= size <= 16 * 1024 * 1024
                body = self.rfile.read(size)
                assert len(body) == size
                record = {'at': stamp(), 'method': self.command,
                          'path': self.path, 'body_sha256': hashlib.sha256(body).hexdigest(),
                          'thread': self.headers.get('session_id') or self.headers.get('x-codex-thread-id'),
                          'events': [], 'injected': False}
                if self.path == '/v1/responses':
                    record['model'] = json.loads(body)['model']
                with owner.lock:
                    owner.records.append(record)
                upstream = http.client.HTTPConnection('127.0.0.1', owner.port, timeout=650)
                observed = True
                try:
                    headers = {k: v for k, v in self.headers.items()
                               if k.lower() not in ('host', 'connection', 'transfer-encoding')}
                    upstream.request(self.command, self.path, body=body, headers=headers)
                    response = upstream.getresponse()
                    record['http_status'] = response.status
                    self.send_response(response.status)
                    for key, value in response.getheaders():
                        if key.lower() not in ('connection', 'transfer-encoding', 'content-length'):
                            self.send_header(key, value)
                    self.send_header('Connection', 'close')
                    self.end_headers()
                    self.close_connection = True
                    frame = []
                    while line := response.readline(16 * 1024 * 1024 + 1):
                        assert len(line) <= 16 * 1024 * 1024
                        if observed:
                            try:
                                self.wfile.write(line)
                                self.wfile.flush()
                            except (BrokenPipeError, ConnectionResetError):
                                observed = False
                        if line.strip():
                            frame.append(line)
                            continue
                        data = b'\n'.join(row[5:].strip() for row in frame if row.startswith(b'data:'))
                        frame = []
                        try:
                            event = json.loads(data)
                        except (ValueError, UnicodeDecodeError):
                            continue
                        kind = event.get('type')
                        if kind:
                            record['events'].append(kind)
                        if kind == 'response.created':
                            record['response_id'] = event['response']['id']
                        with owner.lock:
                            cut = (kind == 'response.output_text.delta' and observed
                                   and owner.armed_thread == record['thread'] and owner.can_cut())
                            if cut:
                                owner.armed_thread = None
                        if cut:
                            record.update(injected=True, cut_at=stamp())
                            observed = False
                            self.connection.shutdown(socket.SHUT_RDWR)
                            owner.fired.set()
                        if kind in ('response.completed', 'response.failed'):
                            record.update(terminal=kind, settled_at=stamp())
                            if record['injected']:
                                owner.settled.set()
                except Exception as error:
                    record['transport_error'] = type(error).__name__
                finally:
                    upstream.close()

        self.server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Handler)
        self.server.daemon_threads = True
        threading.Thread(target=self.server.serve_forever, daemon=True).start()

    @property
    def endpoint(self):
        return f'http://127.0.0.1:{self.server.server_port}/v1'

    def close(self):
        self.server.shutdown()
        self.server.server_close()


class LiveTui:
    def __init__(self, args, root, label, endpoint=None):
        self.args, self.root, self.label = args, root, label
        self.screen = ''
        self.thread = None
        self.backend = None
        self.log = (root / (label + '-terminal-private.log')).open('xb')
        self.master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 40, 160, 0, 0))
        env = dict(os.environ, TERM='xterm-256color', CODEX_HOME=str(args.codex_home))
        command = [str(args.core), '--no-alt-screen']
        if endpoint:
            command += ['-c', 'model_providers.chatgpt_desktop.base_url=' + json.dumps(endpoint)]
        self.process = subprocess.Popen(command, cwd=args.workspace, env=env,
                                        stdin=slave, stdout=slave, stderr=slave,
                                        start_new_session=True)
        os.close(slave)

    def start(self):
        self.wait(lambda: self.thread is not None, 90)
        self.wait(lambda: self.read(False)['status']['type'] == 'idle', 60)

    def drain(self, seconds=.2):
        until = time.monotonic() + seconds
        while time.monotonic() < until:
            if select.select([self.master], [], [], .03)[0]:
                try:
                    data = os.read(self.master, 65536)
                except OSError:
                    break
                if not data:
                    break
                self.log.write(data)
                self.log.flush()
                self.screen = (self.screen + data.decode(errors='replace'))[-8 * 1024 * 1024:]
                titles = re.findall(r'\x1b\]0;([^\x07]*)\x07', self.screen)
                for title in titles:
                    ids = re.findall(r'\b[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}\b', title)
                    if ids:
                        if self.thread is not None:
                            assert self.thread == ids[-1], 'TUI silently changed Native thread'
                        self.thread = ids[-1]

    def wait(self, condition, seconds=None):
        until = time.monotonic() + (seconds or self.args.timeout)
        while time.monotonic() < until:
            self.drain()
            assert self.process.poll() is None, 'installed TUI exited; private terminal evidence retained'
            try:
                if condition():
                    return
            except AssertionError as error:
                if str(error) != 'the exact installed Native owner is unavailable':
                    raise
        raise AssertionError('live TUI deadline; original input was not replayed')

    def call(self, method, params):
        if self.backend:
            paths = [self.backend]
        else:
            paths = [p for family in ('servers', 'catalogs')
                     for p in (self.args.home / '.local/share/codex/core' / family).glob('*/s')]
        for path in paths:
            try:
                pid = int(path.with_name('pid').read_text())
                if Path(f'/proc/{pid}/exe').resolve() != self.args.runtime:
                    continue
                client = Ws(str(path), uri_path='/rpc')
                try:
                    loaded = client.call('thread/loaded/list', {'limit': 100})['data']
                    if self.thread not in loaded:
                        continue
                    value = client.call(method, params)
                    self.backend = path
                    return value
                finally:
                    client.close()
            except (OSError, ConnectionError):
                continue
        self.backend = None
        raise AssertionError('the exact installed Native owner is unavailable')

    def read(self, turns=True):
        return self.call('thread/read', {'threadId': self.thread, 'includeTurns': turns})['thread']

    def paste(self, text):
        self.drain(.5)
        os.write(self.master, b'\x1b[200~' + text.encode() + b'\x1b[201~')
        self.drain(.5)
        os.write(self.master, b'\r')

    def select_chatgpt(self):
        self.screen = ''
        self.paste('/provider ')
        self.wait(lambda: 'Select Provider' in plain(self.screen))
        assert 'ChatGPT AppHost' in plain(self.screen)
        os.write(self.master, b'\x1b[F')
        self.drain(.3)
        self.screen = ''
        os.write(self.master, b'\r')
        self.wait(lambda: 'Select Model and Effort' in plain(self.screen))
        assert 'GPT-6' in plain(self.screen)
        os.write(self.master, b'\x1b[H')
        self.drain(.3)
        self.screen = ''
        os.write(self.master, b'\r')
        self.drain(1)
        if 'Select Reasoning Effort' in plain(self.screen):
            os.write(self.master, b'\x1b[H\r')
        self.wait(lambda: self.read(False)['modelProvider'] == 'chatgpt_desktop')
        settings = self.read(False)
        assert settings.get('model') == 'chatgpt-desktop/gpt-6', settings.get('model')
        save(self.root / (self.label + '-selection.json'),
             {'at': stamp(), 'thread': self.thread, 'command': '/provider',
              'provider': settings['modelProvider'], 'selected_model': settings.get('model')})

    def turn(self, text, count, marker):
        self.screen = ''
        self.paste(text)
        self.wait(lambda: len(self.read()['turns']) >= count
                  and self.read()['status']['type'] == 'idle')
        result = self.read()
        turn = result['turns'][-1]
        assert turn['status'] == 'completed', 'Native turn failed; inspect retained evidence'
        answers = [item.get('text', '') for item in turn['items'] if item['type'] == 'agentMessage']
        assert any(marker in answer for answer in answers), 'real Native answer did not contain the marker'
        self.wait(lambda: marker in plain(self.screen))
        return result

    def contexts(self, thread):
        path = Path(thread['path']).resolve(strict=True)
        assert path.is_relative_to(self.args.home) and path.stat().st_size <= 16 * 1024 * 1024
        records = [json.loads(line) for line in path.read_text().splitlines()]
        contexts = [{'at': row['timestamp'], 'turn': row['payload'].get('turn_id'),
                     'model': row['payload'].get('model')}
                    for row in records if row.get('type') == 'turn_context']
        assert contexts and all(row['model'] == 'chatgpt-desktop/gpt-6' for row in contexts)
        return contexts

    def close(self):
        if self.process.poll() is None:
            os.write(self.master, b'\x04')
            until = time.monotonic() + 15
            while self.process.poll() is None and time.monotonic() < until:
                self.drain()
            # Terminate only this owned observer. Native work is not cancelled.
            if self.process.poll() is None:
                self.process.terminate()
                self.process.wait(timeout=10)
        os.close(self.master)
        self.log.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--core', type=Path, required=True)
    parser.add_argument('--workspace', type=Path, required=True)
    parser.add_argument('--home', type=Path, default=Path.home())
    parser.add_argument('--codex-home', type=Path)
    parser.add_argument('--report-root', type=Path, required=True)
    parser.add_argument('--fault-endpoint', help='actual loopback Provider /v1 endpoint; enables one SSE cut')
    parser.add_argument('--timeout', type=int, default=650)
    args = parser.parse_args()
    assert 60 <= args.timeout <= 900
    args.home = args.home.resolve(strict=True)
    args.core = args.core.resolve(strict=True)
    args.workspace = args.workspace.resolve(strict=True)
    args.codex_home = (args.codex_home or args.home / '.codex').resolve(strict=True)
    args.runtime = (args.home / '.local/lib/codex/core/current/runtime').resolve(strict=True)
    os.umask(0o077)
    args.report_root.mkdir(mode=0o700)
    root = args.report_root
    marker = 'TUI_LIVE_' + hashlib.sha256(os.fsencode(root)).hexdigest()[:12].upper()
    save(root / 'intent.json', {'at': stamp(), 'workspace': str(args.workspace),
         'core_sha256': sha(args.core), 'runtime_sha256': sha(args.runtime),
         'marker': marker, 'fault_requested': bool(args.fault_endpoint),
         'maximum_user_turns': 5, 'models_submitted': 0})
    proxy = ObservationProxy(args.fault_endpoint) if args.fault_endpoint else None
    clients = []
    report = {'started_at': stamp(), 'accepted': False, 'fault_requested': bool(proxy)}
    try:
        client = LiveTui(args, root, 'primary', proxy.endpoint if proxy else None)
        clients.append(client)
        client.start()
        client.select_chatgpt()
        first = client.turn('Reply with exactly ' + marker + '. Do not use tools.', 1, marker)
        save(root / 'first-native-private.json', first)
        second = client.turn('Repeat the exact marker from your previous answer. Do not use tools.', 2, marker)
        save(root / 'followup-native-private.json', second)
        assert second['id'] == first['id'] == client.thread
        assert second['turns'][0] == first['turns'][0], 'follow-up changed original Native history'
        report.update(thread=client.thread, first_turn=first['turns'][0]['id'],
                      followup_turn=second['turns'][-1]['id'], same_thread_history_preserved=True,
                      native_model_contexts=client.contexts(second))
        if proxy:
            proxy.armed_thread = client.thread
            proxy.can_cut = lambda: any(item['type'] == 'commandExecution' and item.get('exitCode') == 0
                                        for turn in client.read()['turns'] for item in turn['items'])
            client.screen = ''
            client.paste('Use the Native functions.exec_command tool exactly once with cmd="pwd" and login=false. '
                         'Then reply with ' + marker + '_FAULT followed by one sentence. Do not use other tools.')
            client.wait(proxy.fired.is_set)
            client.wait(proxy.settled.is_set)
            fault = client.read()
            save(root / 'fault-native-private.json', fault)
            other = LiveTui(args, root, 'independent', proxy.endpoint)
            clients.append(other)
            other.start()
            other.select_chatgpt()
            independent = other.turn('Use Native functions.exec_command exactly once with cmd="pwd" and login=false. '
                                     'Then reply with ' + marker + '_INDEPENDENT.', 1, marker + '_INDEPENDENT')
            assert independent['id'] != client.thread
            save(root / 'independent-native-private.json', independent)
            resumed = client.turn('Continue this same Native thread. Repeat the original marker followed by _RECOVERED. '
                                  'Do not repeat any command or tool execution.', 4, marker + '_RECOVERED')
            save(root / 'recovered-native-private.json', resumed)
            commands = [item for turn in resumed['turns'] for item in turn['items']
                        if item['type'] == 'commandExecution']
            other_commands = [item for turn in independent['turns'] for item in turn['items']
                              if item['type'] == 'commandExecution']
            assert len(commands) == 1 and len(other_commands) == 1, 'missing or duplicated Native execution'
            assert all(item.get('exitCode') == 0 for item in commands + other_commands)
            assert resumed['turns'][:2] == second['turns'], 'recovery changed original turns'
            fired = [row for row in proxy.records if row['injected']]
            assert len(fired) == 1 and fired[0]['terminal'] == 'response.completed'
            assert sum(row['body_sha256'] == fired[0]['body_sha256'] for row in proxy.records) == 1
            report.update(disconnect_injected=True, original_response_settled=True,
                          original_tool_execution_count=len(commands), independent_thread=independent['id'],
                          independent_tool_execution_count=len(other_commands), same_thread_recovered=True,
                          native_model_contexts=client.contexts(resumed),
                          independent_model_contexts=other.contexts(independent))
        report.update(accepted=True, completed_at=stamp())
    finally:
        for client in reversed(clients):
            client.close()
        if proxy:
            proxy.close()
            save(root / 'sse-observations.json', proxy.records)
        save(root / 'result.json', report)
        print(json.dumps(report, ensure_ascii=False), flush=True)


if __name__ == '__main__':
    main()
