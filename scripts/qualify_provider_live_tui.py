#!/usr/bin/env python3
"""Explicit live installed-TUI acceptance through /provider and the real app.

This submits bounded, labelled real model requests. It never prepares a mock
model, changes an account/profile, installs a generation, or replays an input.
The optional fault proxy drops one downstream SSE observation at the first text
delta after a completed Native command, while consuming the original upstream
response. Evidence stays private. Unit fixtures do not qualify a live model.
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
import shlex
import socket
import stat
import struct
import subprocess
import termios
import threading
import time
import urllib.parse

from qualify_tasks import Ws

# /provider presents the current AppHost-visible Instant route. The historical
# chatgpt-desktop/gpt-6 alias remains usable for saved work but is hidden in the
# installed picker; it is not the model selected by this live UI acceptance.
MODEL = 'chatgpt-desktop/app/gpt-6'


def stamp():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()


def save(path, value):
    with path.open('x') as output:
        json.dump(value, output, ensure_ascii=False, indent=2)
        output.write('\n')


def sha(path):
    with path.open('rb') as source:
        return hashlib.file_digest(source, 'sha256').hexdigest()


def installed_runtime(home):
    """Observe Core's selected generation; Core still owns signed admission."""
    def directory(*parts):
        path = home
        for part in parts:
            path /= part
            metadata = path.lstat()
            assert stat.S_ISDIR(metadata.st_mode) and metadata.st_uid == os.getuid(), 'Core installation path is not an owned directory'
        return path

    state = directory('.local', 'share', 'codex', 'core') / 'activation-state'
    descriptor = os.open(state, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(descriptor, 'rb') as source:
        metadata = os.fstat(source.fileno())
        assert stat.S_ISREG(metadata.st_mode) and metadata.st_uid == os.getuid(), 'Core activation state is not an owned regular file'
        raw = source.read(16 * 1024 + 1)
    assert len(raw) <= 16 * 1024 and raw.endswith(b'\n'), 'Core activation state is incomplete or oversized'
    fields = [line.split('=', 1) for line in raw.decode().split('\n')[:-1]]
    assert all(len(field) == 2 for field in fields)
    assert [field[0] for field in fields] == [
        'format', 'update_key', 'current', 'current_key', 'previous_present', 'previous', 'previous_key'], 'Core activation state fields changed'
    values = dict(fields)
    assert values['format'] == 'codex-activation-state-v3', 'Core activation state format changed'
    identity = values['current']
    assert 0 < len(identity.encode()) <= 512 and identity not in ('.', '..')
    assert '/' not in identity and not any(ord(char) < 32 or ord(char) == 127 for char in identity), 'Core generation identity is not a path component'
    generation = directory('.local', 'lib', 'codex', 'core', 'generations', identity)
    runtime = generation / 'runtime'
    metadata = runtime.lstat()
    assert stat.S_ISREG(metadata.st_mode) and metadata.st_uid == os.getuid(), 'selected runtime is not an owned regular file'
    return runtime


def process_identity():
    """Kernel facts only; never read command arguments, environment or titles."""
    pid = os.getpid()
    fields = Path(f'/proc/{pid}/stat').read_text().rsplit(')', 1)[1].split()
    ttys = {}
    for fd in (0, 1, 2):
        try:
            ttys[str(fd)] = os.ttyname(fd)
        except OSError:
            ttys[str(fd)] = None
    try:
        foreground = os.tcgetpgrp(0)
    except OSError:
        foreground = None
    return {'pid': pid, 'start': int(fields[19]), 'parent_pid': int(fields[1]),
            'process_group': int(fields[2]), 'session': int(fields[3]),
            'tty_device': int(fields[4]), 'foreground_process_group': foreground,
            'ttys': ttys, 'uid': os.getuid()}


def terminal_response(prefix, operation):
    """Use the same AM endpoint and origin contract as Manager's native route."""
    endpoint = prefix.parent / 'apps/com.termux/termux-am/am.sock'
    metadata = endpoint.lstat()
    assert stat.S_ISSOCK(metadata.st_mode) and metadata.st_uid == os.getuid(), 'AM endpoint is not the owned socket'
    result = subprocess.run([str(prefix / 'bin/termux-am-socket'), 'termux-terminal-v1 ' + operation],
                            stdin=subprocess.DEVNULL, capture_output=True, timeout=3)
    assert result.returncode == 0 and not result.stderr, 'terminal owner observation failed'
    assert len(result.stdout) <= 4096, 'terminal owner response exceeds its contract'
    value = json.loads(result.stdout)
    assert value.get('schema') == 'termux-terminal-origin-v1' and value.get('result') == 'ok', 'terminal owner response is not confirmed'
    origin = value.get('origin')
    assert isinstance(origin, dict) and set(origin) == {'handle', 'pid', 'start'}, 'terminal owner identity is incomplete'
    assert isinstance(origin['handle'], str) and re.fullmatch(r'[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}', origin['handle']), 'terminal handle is not canonical'
    assert type(origin['pid']) is int and 0 < origin['pid'] <= 0xffffffff, 'terminal owner PID is invalid'
    assert type(origin['start']) is int and origin['start'] > 0, 'terminal owner kernel start is invalid'
    # Only these public identity fields enter evidence. Unknown response fields,
    # stdout, stderr, process arguments and inherited secrets are never recorded.
    selected = value.get('selected')
    assert selected is None or type(selected) is bool, 'terminal selection evidence is invalid'
    return {'schema': value['schema'], 'result': 'ok', 'origin': origin, 'selected': selected}


def require_terminal_owner(process, origin, validated):
    assert process['pid'] > 0 and process['start'] > 0, 'receiver kernel identity is unavailable'
    assert process['session'] == process['process_group'] == process['pid'], 'receiver does not own its terminal session'
    assert process['foreground_process_group'] == process['process_group'], 'receiver is not the foreground process group'
    tty = process['ttys']['0']
    assert isinstance(tty, str) and re.fullmatch(r'/dev/pts/[0-9]+', tty), 'receiver has no native PTY'
    assert all(value == tty for value in process['ttys'].values()), 'receiver standard streams do not share the terminal'
    assert origin['selected'] is True, 'receiver Android terminal is not selected'
    assert origin['origin']['pid'] == process['pid'] and origin['origin']['start'] == process['start'], 'Android terminal owns another process'
    assert validated['origin'] == origin['origin'], 'live terminal identity changed during validation'
    assert validated.get('selected') is not False, 'receiver Android terminal is no longer selected'


def terminal_owner_preflight(prefix, required):
    receipt = {'schema': 'codex-live-tui-terminal-owner-v1', 'started_at': stamp(),
               'required': required, 'verified': False}
    try:
        before = process_identity()
        receipt['receiver'] = before
        if required:
            assert prefix.is_absolute(), 'Termux prefix must be absolute'
            receipt['client_sha256'] = sha(prefix / 'bin/termux-am-socket')
            origin = terminal_response(prefix, 'origin')
            receipt['origin_observation'] = origin
            identity = origin['origin']
            validated = terminal_response(prefix, f"validate {identity['handle']} {identity['pid']} {identity['start']}")
            receipt['validation_observation'] = validated
            require_terminal_owner(before, origin, validated)
            assert process_identity() == before, 'receiver kernel or terminal identity changed during preflight'
            receipt['verified'] = True
    except Exception as error:
        # Keep failure diagnostics useful without printing untrusted helper
        # output (including JSON decode excerpts or arbitrary error messages).
        receipt['failure'] = {'type': type(error).__name__,
                              'reason': str(error) if isinstance(error, AssertionError)
                              else 'terminal owner observation unavailable'}
    receipt['completed_at'] = stamp()
    return receipt


def plain(value):
    value = re.sub(r'\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)', '', value)
    return re.sub(r'\x1b\[[0-?]*[ -/]*[@-~]|\x1b[^[]', '', value)


def history(thread, count, prefix=()):
    turns = thread['turns']
    assert len(turns) == count, 'unexpected Native turn count; no input will be replayed'
    assert turns[:len(prefix)] == list(prefix), 'original Native history changed'
    ids = [turn['id'] for turn in turns]
    assert all(ids) and len(set(ids)) == len(ids), 'Native turn identity is missing or duplicated'
    return turns


def native_settled(thread, allow_failed=False):
    turns = thread['turns']
    if any(turn['status'] == 'inProgress' for turn in turns):
        return False
    status = thread['status']['type']
    return status == 'idle' or (allow_failed and status == 'systemError'
                                and bool(turns) and turns[-1]['status'] == 'failed')


def blank_history_error(error, thread):
    if len(error.args) != 1 or not isinstance(error.args[0], tuple) or len(error.args[0]) != 2:
        return False
    method, rpc = error.args[0]
    return (method == 'thread/read' and isinstance(rpc, dict) and rpc.get('code') == -32600
            and rpc.get('message') == f'thread {thread} is not materialized yet; includeTurns is unavailable before first user message')


def blank_rollout(snapshot, thread, codex_home):
    """Prove a newly created Native root; a metadata-only read omits all turns."""
    assert snapshot['id'] == thread and snapshot['status']['type'] == 'idle'
    assert snapshot['ephemeral'] is False and snapshot['turns'] == []
    reported = Path(snapshot['path'])
    assert reported.is_absolute()
    sessions = (codex_home / 'sessions').resolve(strict=True)
    parent = reported.parent.resolve(strict=True)
    assert parent.is_relative_to(sessions)
    path = parent / reported.name
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(descriptor, 'rb') as source:
        before = os.fstat(source.fileno())
        assert stat.S_ISREG(before.st_mode) and before.st_uid == os.geteuid()
        assert before.st_mode & 0o022 == 0 and before.st_size <= 32 * 1024 * 1024
        data = source.read(32 * 1024 * 1024 + 1)
    after = path.lstat()
    version = lambda value: (value.st_dev, value.st_ino, value.st_mode, value.st_size,
                             value.st_mtime_ns, value.st_ctime_ns)
    assert len(data) == before.st_size and version(before) == version(after)
    assert reported.parent.resolve(strict=True) == parent
    # json.loads consumes the whole file, rejecting any second rollout record.
    row = json.loads(data)
    assert row['type'] == 'session_meta'
    meta = row['payload']
    assert meta['id'] == meta['session_id'] == thread
    assert not meta.get('attached_thread', False)
    assert all(meta.get(key) is None for key in (
        'forked_from_id', 'forked_from_ordinal_exclusive', 'parent_thread_id',
        'history_base', 'subagent_history_start_ordinal'))
    return snapshot


def admitted_prompt(turn, text):
    messages = [item for item in turn['items'] if item['type'] == 'userMessage']
    assert len(messages) == 1, 'expected exactly one admitted Native user message'
    content = messages[0]['content']
    assert content and all(item['type'] == 'text' for item in content)
    assert '\n'.join(item['text'] for item in content) == text, 'Native admitted a different input'


def assistant_marker(turn, prompt, marker, screen):
    # A token copied verbatim into the prompt is visible before any model result.
    # Every measured answer therefore has a fresh token assembled by the model.
    assert marker not in prompt, 'answer marker must not be present in the user prompt'
    assert turn['status'] == 'completed', 'Native turn failed; inspect retained evidence'
    answers = [item['text'] for item in turn['items']
               if item['type'] == 'agentMessage' and item.get('phase') == 'final_answer']
    assert any(marker in answer for answer in answers), 'Native final answer did not contain the marker'
    return marker in plain(screen)


def one_pwd(thread, workspace, shell, turn_id=None):
    commands = [(turn['id'], item) for turn in thread['turns'] for item in turn['items']
                if item['type'] == 'commandExecution']
    assert len(commands) == 1, 'missing or duplicated Native command execution'
    owner, item = commands[0]
    assert not turn_id or owner == turn_id, 'command belongs to another Native turn'
    assert item['id'], 'Native command has no identity'
    assert shlex.split(item['command']) == [str(shell), '-c', 'pwd'], 'Native did not execute the exact pwd command'
    assert item['cwd'] == str(workspace), 'Native command ran in a different workspace'
    assert item['status'] == 'completed', 'Native command has not completed'
    assert type(item.get('exitCode')) is int and item['exitCode'] == 0, 'Native command did not succeed'
    assert item.get('aggregatedOutput') == str(workspace) + '\n', 'Native pwd output does not match the workspace'
    return {'thread': thread['id'], 'turn': owner, 'item': item['id'],
            'command': item['command'], 'cwd': item['cwd'], 'exitCode': item['exitCode'],
            'aggregatedOutput': item['aggregatedOutput'], 'observed_at': stamp(),
            'startedAtMs': item.get('startedAtMs'), 'completedAtMs': item.get('completedAtMs')}


def request_identity(value):
    metadata = value.get('client_metadata', {})
    turn_metadata = metadata.get('x-codex-turn-metadata', {})
    if isinstance(turn_metadata, str):
        turn_metadata = json.loads(turn_metadata)
    assert isinstance(turn_metadata, dict)
    identity = {}
    for name in ('thread', 'turn'):
        key = name + '_id'
        nested, flat = turn_metadata.get(key), metadata.get(key)
        assert not (nested and flat) or nested == flat, 'conflicting Native request identity'
        value = nested or flat
        assert isinstance(value, str) and value and len(value) <= 256
        assert not any(ord(char) < 32 for char in value)
        identity[name] = value
    return identity


def completed_fault(records, thread, turn):
    fired = [row for row in records if row['injected']]
    assert len(fired) == 1, 'expected exactly one injected observer disconnection'
    row = fired[0]
    assert (row['method'], row['path'], row['thread'], row['turn'], row['model']) == (
        'POST', '/v1/responses', thread, turn, MODEL)
    assert row['http_status'] == 200 and row['content_type'].split(';')[0] == 'text/event-stream'
    assert row.get('upstream_eof') and 'transport_error' not in row, 'original upstream stream did not settle'
    assert row['events'].count('response.created') == row['events'].count('response.completed') == 1
    assert 'response.failed' not in row['events'] and 'error' not in row['events']
    assert row['terminal'] == 'response.completed' and row['terminal_status'] == 'completed'
    assert row['response_id'] and row['response_id'] == row['terminal_response_id'], 'completion belongs to another response'
    assert row['events'][row['cut_event']] == 'response.output_text.delta'
    assert row['execution']['thread'] == thread and row['execution']['turn'] == turn
    assert row['execution']['observed_at'] <= row['cut_at'] <= row['settled_at']
    posts = [value for value in records if value['method'] == 'POST' and value['path'] == '/v1/responses']
    assert sum(value['body_sha256'] == row['body_sha256'] for value in posts) == 1, 'original request was resubmitted'
    # A tool round may legitimately precede this response in the same user turn.
    # After the cut, another request for that turn is a replay, even if its body
    # (e.g. transient client metadata) changed and no longer has the same hash.
    assert not any(value['ordinal'] > row['ordinal'] and
                   (value['thread'], value['turn']) == (thread, turn) for value in posts), 'Native retried the disconnected turn'
    assert sum(value.get('response_id') == row['response_id'] for value in posts) == 1
    return row


class TurnBudget:
    """Count an attempted submission before PTY delivery; never replay uncertainty."""
    def __init__(self, root, maximum=5):
        self.root, self.maximum = root, maximum
        self.submissions = []

    def reserve(self, label, thread, count, text):
        assert len(self.submissions) < self.maximum, 'live model turn budget exhausted'
        entry = {'at': stamp(), 'label': label, 'thread': thread, 'native_turn_number': count,
                 'prompt_sha256': hashlib.sha256(text.encode()).hexdigest()}
        save(self.root / f'submission-{len(self.submissions) + 1}.json', entry)
        self.submissions.append(entry)


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
        self.can_cut = lambda record: None
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
                          'events': [], 'injected': False}
                if self.path == '/v1/responses':
                    value = json.loads(body)
                    record.update(request_identity(value), model=value['model'])
                with owner.lock:
                    record['ordinal'] = len(owner.records)
                    replay = self.command == 'POST' and any(
                        prior['injected'] and
                        (prior.get('thread'), prior.get('turn')) ==
                        (record.get('thread'), record.get('turn'))
                        for prior in owner.records)
                    if replay:
                        record['rejected'] = 'disconnected_turn_replay'
                    owner.records.append(record)
                if replay:
                    # Retain the attempt as a failed acceptance assertion without
                    # submitting another real model operation for the cut turn.
                    data = b'{"error":{"code":"qualification_no_replay","message":"The disconnected Native turn cannot be resubmitted."}}'
                    self.send_response(409)
                    self.send_header('Content-Type', 'application/json')
                    self.send_header('Content-Length', str(len(data)))
                    self.send_header('Connection', 'close')
                    self.end_headers()
                    self.close_connection = True
                    self.wfile.write(data)
                    return
                upstream = http.client.HTTPConnection('127.0.0.1', owner.port, timeout=650)
                observed = True
                try:
                    headers = {k: v for k, v in self.headers.items()
                               if k.lower() not in ('host', 'connection', 'transfer-encoding')}
                    upstream.request(self.command, self.path, body=body, headers=headers)
                    response = upstream.getresponse()
                    record['http_status'] = response.status
                    record['content_type'] = response.getheader('Content-Type', '')
                    self.send_response(response.status)
                    for key, value in response.getheaders():
                        if key.lower() not in ('connection', 'transfer-encoding', 'content-length'):
                            self.send_header(key, value)
                    self.send_header('Connection', 'close')
                    self.end_headers()
                    self.close_connection = True
                    frame = []
                    frame_size = 0
                    sse = record['content_type'].split(';')[0] == 'text/event-stream'
                    while line := response.readline(16 * 1024 * 1024 + 1):
                        assert len(line) <= 16 * 1024 * 1024
                        frame.append(line)
                        frame_size += len(line)
                        assert frame_size <= 16 * 1024 * 1024
                        if sse and line.strip():
                            continue
                        data = b'\n'.join(row[5:].strip() for row in frame if row.startswith(b'data:'))
                        try:
                            event = json.loads(data)
                        except (ValueError, UnicodeDecodeError):
                            event = {}
                        kind = event.get('type')
                        if kind:
                            record['events'].append(kind)
                        if kind == 'response.created':
                            record['response_id'] = event['response']['id']
                        if (kind == 'response.output_text.delta' and observed
                                and owner.armed_thread == record.get('thread')):
                            # Observe Native before disconnecting, without holding
                            # the proxy's record lock during a read-only RPC.
                            proof = owner.can_cut(record)
                            if proof:
                                with owner.lock:
                                    cut = owner.armed_thread == record['thread']
                                    if cut:
                                        owner.armed_thread = None
                                if cut:
                                    record.update(injected=True, cut_at=stamp(), execution=proof,
                                                  cut_event=len(record['events']) - 1)
                                    observed = False
                                    try:
                                        self.connection.shutdown(socket.SHUT_RDWR)
                                    except OSError:
                                        pass
                                    owner.fired.set()
                        if observed:
                            try:
                                self.wfile.write(b''.join(frame))
                                self.wfile.flush()
                            except (BrokenPipeError, ConnectionResetError):
                                observed = False
                        frame, frame_size = [], 0
                        if kind in ('response.completed', 'response.failed'):
                            record.update(terminal=kind, settled_at=stamp(),
                                          terminal_response_id=event['response']['id'],
                                          terminal_status=event['response']['status'])
                    assert not frame, 'original SSE ended in a partial event'
                    record['upstream_eof'] = True
                except Exception as error:
                    record['transport_error'] = type(error).__name__
                finally:
                    upstream.close()
                    if record['injected']:
                        owner.settled.set()

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
    def __init__(self, args, root, label, budget, endpoint=None):
        self.args, self.root, self.label = args, root, label
        self.budget = budget
        self.screen = ''
        self.thread = None
        self.backend = None
        self.log = (root / (label + '-terminal-private.log')).open('xb')
        self.master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 40, 160, 0, 0))
        env = dict(os.environ, TERM='xterm-256color', CODEX_HOME=str(args.codex_home))
        # The upstream default title omits unnamed thread IDs; "session-id"
        # truncates them. "thread" exposes the configured widget's full UUID
        # without changing its model, profile, or the user's saved settings.
        command = [str(args.core), '--no-alt-screen', '-c', 'tui.terminal_title=["thread"]']
        if endpoint:
            # An upstream partial provider definition requires a nonempty name.
            # These fields declare the observer transport; /provider still owns
            # selecting the model and the installed route supplies its auth.
            command += ['-c', 'model_providers.chatgpt_desktop.name="ChatGPT AppHost"',
                        '-c', 'model_providers.chatgpt_desktop.base_url=' + json.dumps(endpoint)]
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
                result = condition()
                if result:
                    return result
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
        assert settings.get('model') == MODEL, settings.get('model')
        save(self.root / (self.label + '-selection.json'),
             {'at': stamp(), 'thread': self.thread, 'command': '/provider',
              'provider': settings['modelProvider'], 'selected_model': settings.get('model')})

    def submit(self, text, count, marker, allow_failed=False):
        try:
            before = self.read()
        except AssertionError as error:
            if count != 1 or not blank_history_error(error, self.thread):
                raise
            before = blank_rollout(self.read(False), self.thread, self.args.codex_home)
        assert before['id'] == self.thread and native_settled(before, allow_failed)
        history(before, count - 1)
        assert marker not in text, 'answer marker must not be present in the user prompt'
        assert marker not in json.dumps(before['turns']), 'answer marker was already present in Native history'
        self.budget.reserve(self.label, self.thread, count, text)
        self.screen = ''
        self.paste(text)
        return before

    def settled(self, count, prefix, allow_failed=False):
        try:
            result = self.read()
        except AssertionError as error:
            if count == 1 and not prefix and blank_history_error(error, self.thread):
                # Delivery is already attempted. Await materialization of that
                # original input; this is not an empty-history or replay grant.
                return None
            raise
        assert result['id'] == self.thread
        assert len(result['turns']) <= count, 'unexpected extra Native turn'
        assert result['turns'][:len(prefix)] == list(prefix), 'original Native history changed'
        if len(result['turns']) == count and native_settled(result, allow_failed):
            history(result, count, prefix)
            return result
        return None

    def turn(self, text, count, marker, allow_failed=False):
        before = self.submit(text, count, marker, allow_failed)
        result = self.wait(lambda: self.settled(count, before['turns']))
        turn = result['turns'][-1]
        admitted_prompt(turn, text)
        self.wait(lambda: assistant_marker(turn, text, marker, self.screen))
        return result

    def contexts(self, thread):
        path = Path(thread['path']).resolve(strict=True)
        assert path.is_relative_to(self.args.home) and path.stat().st_size <= 16 * 1024 * 1024
        records = [json.loads(line) for line in path.read_text().splitlines()]
        contexts = [{'at': row['timestamp'], 'turn': row['payload'].get('turn_id'),
                     'model': row['payload'].get('model')}
                    for row in records if row.get('type') == 'turn_context']
        assert contexts and all(row['model'] == MODEL for row in contexts)
        assert {row['turn'] for row in contexts} == {turn['id'] for turn in thread['turns']}, 'model context does not cover the measured Native turns'
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
    parser.add_argument('--terminal-owner-proof', action='store_true',
                        help='require this receiver to own the selected foreground Android terminal before starting TUI work')
    parser.add_argument('--timeout', type=int, default=650)
    args = parser.parse_args()
    assert 60 <= args.timeout <= 900
    args.home = args.home.resolve(strict=True)
    args.core = args.core.resolve(strict=True)
    args.workspace = args.workspace.resolve(strict=True)
    args.codex_home = (args.codex_home or args.home / '.codex').resolve(strict=True)
    args.runtime = installed_runtime(args.home)
    args.shell = Path(os.environ['PREFIX']) / 'bin/bash'
    assert args.shell.is_absolute() and args.shell.is_file()
    os.umask(0o077)
    args.report_root.mkdir(mode=0o700)
    root = args.report_root
    marker = 'TUI_LIVE_' + hashlib.sha256(os.fsencode(root)).hexdigest()[:12].upper()
    parts = (marker[:len(marker) // 2], marker[len(marker) // 2:])
    build_marker = f'Concatenate the two strings "{parts[0]}" and "{parts[1]}" without a separator.'
    save(root / 'intent.json', {'at': stamp(), 'workspace': str(args.workspace),
         'core_sha256': sha(args.core), 'runtime_sha256': sha(args.runtime),
         'generation': args.runtime.parent.name, 'runtime': str(args.runtime),
         'marker': marker, 'fault_requested': bool(args.fault_endpoint),
         'maximum_user_turns': 5, 'submission_evidence': 'submission-<number>.json',
         'terminal_owner_required': args.terminal_owner_proof})
    proxy = None
    budget = TurnBudget(root)
    clients = []
    report = {'started_at': stamp(), 'accepted': False, 'fault_requested': bool(args.fault_endpoint)}
    failure = None
    try:
        terminal_receipt = terminal_owner_preflight(args.shell.parent.parent, args.terminal_owner_proof)
        save(root / 'terminal-owner.json', terminal_receipt)
        report['terminal_owner'] = {'required': args.terminal_owner_proof,
                                   'verified': terminal_receipt['verified'], 'receipt': 'terminal-owner.json'}
        assert not args.terminal_owner_proof or terminal_receipt['verified'], 'foreground terminal owner was not verified; inspect terminal-owner.json'
        proxy = ObservationProxy(args.fault_endpoint) if args.fault_endpoint else None
        client = LiveTui(args, root, 'primary', budget, proxy.endpoint if proxy else None)
        clients.append(client)
        client.start()
        client.select_chatgpt()
        first = client.turn(build_marker + ' Reply only with that combined string. Do not use tools.', 1, marker)
        save(root / 'first-native-private.json', first)
        second = client.turn('Append the literal suffix _FOLLOWUP to the exact marker from your first answer. '
                             'Reply only with that combined string. Do not use tools.', 2, marker + '_FOLLOWUP')
        save(root / 'followup-native-private.json', second)
        assert second['id'] == first['id'] == client.thread
        history(second, 2, first['turns'])
        assert not any(item['type'] == 'commandExecution' for turn in second['turns'] for item in turn['items'])
        report.update(thread=client.thread, first_turn=first['turns'][0]['id'],
                      followup_turn=second['turns'][-1]['id'], same_thread_history_preserved=True,
                      native_model_contexts=client.contexts(second))
        if proxy:
            proxy.armed_thread = client.thread

            def effect_observed(record):
                current = client.read()
                assert current['id'] == client.thread
                assert current['turns'][:2] == second['turns'], 'fault changed original Native history'
                assert len(current['turns']) <= 3, 'fault created another Native turn'
                if len(current['turns']) < 3:
                    return None
                turn = current['turns'][2]
                assert record['turn'] == turn['id'], 'SSE does not belong to the faulted Native turn'
                commands = [item for item in turn['items'] if item['type'] == 'commandExecution']
                if not commands or commands[0]['status'] == 'inProgress':
                    return None
                admitted_prompt(turn, fault_prompt)
                return one_pwd(current, args.workspace, args.shell, turn['id'])

            proxy.can_cut = effect_observed
            command_prompt = ('Use the Native functions.exec_command tool exactly once with cmd="pwd", '
                              f'login=false, shell={json.dumps(str(args.shell))} '
                              f'and workdir={json.dumps(str(args.workspace))}. ')
            fault_prompt = (command_prompt + 'Then append _FAULT to the marker from your first answer '
                            'and reply with that combined string followed by one sentence. Do not use other tools.')
            before_fault = client.submit(fault_prompt, 3, marker + '_FAULT')
            client.wait(proxy.fired.is_set)
            # Exercise another real TUI while the original SSE owner can still
            # be settling. This creates no replacement for the interrupted work.
            other = LiveTui(args, root, 'independent', budget, proxy.endpoint)
            clients.append(other)
            other.start()
            other.select_chatgpt()
            independent = other.turn(command_prompt + build_marker + ' Append _INDEPENDENT and reply only '
                                     'with that combined string. Do not use other tools.', 1, marker + '_INDEPENDENT')
            assert independent['id'] != client.thread
            save(root / 'independent-native-private.json', independent)
            independent_execution = one_pwd(independent, args.workspace, args.shell, independent['turns'][0]['id'])
            client.wait(proxy.settled.is_set)
            fault = client.wait(lambda: client.settled(3, before_fault['turns'], allow_failed=True))
            assert fault['turns'][2]['status'] in ('completed', 'failed', 'interrupted')
            admitted_prompt(fault['turns'][2], fault_prompt)
            fault_execution = one_pwd(fault, args.workspace, args.shell, fault['turns'][2]['id'])
            completed_fault(proxy.records, client.thread, fault['turns'][2]['id'])
            save(root / 'fault-native-private.json', fault)
            resumed = client.turn('Continue this same Native thread. Repeat the original marker followed by _RECOVERED. '
                                  'Do not repeat any command or tool execution.', 4, marker + '_RECOVERED', allow_failed=True)
            save(root / 'recovered-native-private.json', resumed)
            history(resumed, 4, fault['turns'])
            assert resumed['turns'][:2] == second['turns'], 'recovery changed original turns'
            one_pwd(resumed, args.workspace, args.shell, fault['turns'][2]['id'])
            fired = completed_fault(proxy.records, client.thread, fault['turns'][2]['id'])
            assert len(budget.submissions) == 5
            started, completed = independent_execution['startedAtMs'], independent_execution['completedAtMs']
            overlap = None
            if type(started) is int and type(completed) is int:
                cut_ms = datetime.datetime.fromisoformat(fired['cut_at']).timestamp() * 1000
                settled_ms = datetime.datetime.fromisoformat(fired['settled_at']).timestamp() * 1000
                overlap = cut_ms <= started < settled_ms and completed >= started
            report.update(disconnect_injected=True, original_response_settled=True,
                          original_response_id=fired['response_id'], original_request_posts=1,
                          original_tool_execution_count=1, original_execution=fault_execution,
                          independent_thread=independent['id'], independent_tool_execution_count=1,
                          independent_execution=independent_execution,
                          independent_execution_overlapped_original_stream=overlap,
                          same_thread_recovered=True,
                          native_model_contexts=client.contexts(resumed),
                          independent_model_contexts=other.contexts(independent))
        else:
            assert len(budget.submissions) == 2
        for client in clients:
            assert 'cdp_operation_timeout' not in Path(client.log.name).read_text(errors='replace'), 'TUI reported a CDP operation timeout'
        report['acceptance_scope'] = 'live_tui_followup_disconnect_and_independent_work' if proxy else 'live_tui_followup_only'
        report.update(accepted=True, completed_at=stamp())
    except BaseException as error:
        failure = error
        report['failure'] = {'type': type(error).__name__, 'message': str(error)}
    finally:
        for client in reversed(clients):
            try:
                client.close()
            except Exception as error:
                report.setdefault('cleanup_errors', []).append(type(error).__name__)
        if proxy:
            try:
                proxy.close()
            except Exception as error:
                report.setdefault('cleanup_errors', []).append(type(error).__name__)
            save(root / 'sse-observations.json', proxy.records)
        report['models_submitted'] = len(budget.submissions)
        if report.get('cleanup_errors'):
            report['accepted'] = False
        save(root / 'result.json', report)
        print(json.dumps(report, ensure_ascii=False), flush=True)
    if failure:
        raise failure
    assert report['accepted'], 'live TUI cleanup failed; inspect retained evidence'


if __name__ == '__main__':
    main()
