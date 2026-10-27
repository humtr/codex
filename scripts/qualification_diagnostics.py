"""Bounded, durable local evidence for interrupted Android qualification.

One sampler thread, no model calls, argv or environment collection. Optional
Android logging uses one owned logcat child with bounded private output files.
An absent final event proves an unclosed recorder, not a particular kill reason.
"""
import datetime
import fcntl
import json
import os
from pathlib import Path
import re
import secrets
import select
import subprocess
import sys
import threading
import time
from urllib.parse import urlsplit


def native_thread_settled(thread, expected_turns):
    """Accept only completed/idle; a sampled noncompletion is never a pass.

    Public history may be observed between its final item and terminal receipt.
    Callers keep their bounded wait and retain failure/timeout evidence instead
    of treating a single reconstructed interrupted status as an execution receipt.
    """
    turns = thread.get('turns', [])
    return (len(turns) == expected_turns and bool(turns)
            and turns[-1].get('status') == 'completed'
            and thread.get('status', {}).get('type') == 'idle')


def terminal_contains(output, phrase):
    """Recognize a label split by ANSI cursor/style updates.

    This is a text-presence check, not a terminal emulator or a model-selection
    receipt. Callers still verify the native model/thread after selecting.
    """
    plain = re.sub(r'\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)|\x1b\[[0-?]*[ -/]*[@-~]', '', output)
    return ''.join(phrase.split()) in ''.join(plain.split())


def terminal_screen_lines(screen, character_width):
    """Render observed cells, including pyte's orphaned wide-character stubs.

    pyte 0.8.2 can leave an empty continuation cell after a wide glyph's first
    cell is overwritten. It is then a blank column, not a missing glyph. Keep
    normal wide/combining glyphs intact and do not mutate the observed buffer.
    """
    lines = []
    for y in range(screen.lines):
        row = screen.buffer[y]
        parts = []
        continuation = False
        for x in range(screen.columns):
            if continuation:
                continuation = False
                continue
            text = row[x].data or ' '
            parts.append(text)
            continuation = character_width(text[0]) == 2
        lines.append(''.join(parts))
    return lines


def listener_port(ready):
    """Return only an explicitly reported loopback listener port."""
    value = ready.get('base_url') if isinstance(ready, dict) else None
    parsed = urlsplit(value or '')
    if parsed.scheme != 'http' or parsed.hostname != '127.0.0.1' or parsed.port is None:
        raise ValueError('candidate Provider did not report an HTTP loopback listener')
    return parsed.port


def candidate_info(original, ready, *, binary, state, pid, desktop_session):
    """Copy original settings while replacing every candidate-owned endpoint."""
    result = dict(original)
    result.update(binary=str(binary), state=str(state), pid=int(pid),
                  port=listener_port(ready), desktop_session=str(desktop_session))
    if result['port'] != listener_port(ready):
        raise ValueError('candidate endpoint mismatch')
    return result


class DispatchLedger:
    """Keep local-source calls separate from managed Chat app dispatches.

    Missing either measurement makes the combined total unknown; callers must
    never turn an unobserved managed dispatch into a zero-cost pass.
    """
    def __init__(self):
        self._managed_highwater = {}
        self._managed_unknown = set()
        self.local_source_requests = None

    def observe(self, local_source_requests, managed_work_dispatches):
        self.local_source_requests = self._count(local_source_requests)
        for work_id, count in managed_work_dispatches:
            if not work_id:
                raise ValueError('managed dispatch must be bound to a work ID')
            if count is None:
                self._managed_unknown.add(work_id)
                continue
            self._managed_highwater[work_id] = max(
                self._managed_highwater.get(work_id, 0), self._count(count))
            self._managed_unknown.discard(work_id)
        managed_known = not self._managed_unknown
        known = self.local_source_requests is not None and managed_known
        managed = sum(self._managed_highwater.values()) if managed_known else None
        return {
            'local_source_requests': self.local_source_requests,
            'managed_app_dispatches': managed,
            'actual_chat_dispatches': (self.local_source_requests + managed) if known else None,
            'measurement_complete': known,
        }

    @staticmethod
    def _count(value):
        if value is None:
            return None
        if isinstance(value, bool) or not isinstance(value, int) or value < 0:
            raise ValueError('dispatch counts must be nonnegative integers or unknown')
        return value


def observer_keeper(root, *, terminal_fd, terminal_log, thread_id,
                    input_stdout_fd=None, input_stdin_fd=None, ready_timeout=5):
    """Transfer observer pipe/PTY handles to a private, independently owned keeper.

    The keeper drains terminal bytes to the existing private CLI log and
    discards input-client replies. It does not send requests or control native
    work. Handles remain open until an exact nonce-bound release receipt exists.
    """
    root = Path(root)
    info = root.lstat()
    if root.resolve() != root or info.st_uid != os.getuid() or info.st_mode & 0o077:
        raise ValueError('private canonical evidence directory required')
    if not isinstance(thread_id, str) or not thread_id:
        raise ValueError('observer lease requires the exact native thread')
    if (input_stdout_fd is None) != (input_stdin_fd is None):
        raise ValueError('input-client descriptors must be supplied together')
    fds = (int(terminal_fd),)
    if input_stdout_fd is not None:
        fds += (int(input_stdout_fd), int(input_stdin_fd))
    if len(set(fds)) != len(fds) or min(fds) < 0:
        raise ValueError('distinct live observer descriptors required')
    for fd in fds:
        os.fstat(fd)
    nonce = secrets.token_hex(24)
    argv = [sys.executable, '-B', str(Path(__file__).resolve()), '--observer-keeper',
            str(root), json.dumps(fds), str(Path(terminal_log)), nonce]
    process = subprocess.Popen(argv, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL, pass_fds=fds, start_new_session=True, umask=0o077)
    if not select.select([process.stdout], [], [], ready_timeout)[0]:
        process.terminate()
        process.wait(timeout=3)
        raise RuntimeError('observer keeper did not acknowledge inherited handles')
    receipt = json.loads(process.stdout.readline())
    process.stdout.close()
    if receipt.get('ready') is not True or receipt.get('pid') != process.pid:
        process.terminate()
        process.wait(timeout=3)
        raise RuntimeError('observer keeper identity receipt invalid')
    release = root / 'observer-release.json'
    receipt_path = root / 'observer-lease-owner.json'
    payload = {'pid': process.pid, 'nonce': nonce, 'thread_id': thread_id,
               'terminal_fd': fds[0], 'input_stdout_fd': input_stdout_fd,
               'input_stdin_fd': input_stdin_fd, 'release_file': str(release),
               'terminal_log': str(Path(terminal_log))}
    temp = receipt_path.with_suffix('.tmp')
    with temp.open('x') as stream:
        json.dump(payload, stream)
        stream.flush()
        os.fsync(stream.fileno())
    os.chmod(temp, 0o600)
    temp.replace(receipt_path)
    return process, payload


def release_observer_keeper(root, nonce, settlement):
    """Release only after an observed native idle/completed, no-pending boundary."""
    root = Path(root)
    receipt = json.loads((root / 'observer-lease-owner.json').read_text())
    if not secrets.compare_digest(str(nonce), receipt['nonce']):
        raise ValueError('observer release nonce mismatch')
    if (not isinstance(settlement, dict)
            or settlement.get('thread_id') != receipt['thread_id']
            or settlement.get('thread_status') != 'idle'
            or settlement.get('last_turn_status') != 'completed'
            or settlement.get('pending_calls') != []):
        raise ValueError('native work is not confirmed idle/completed with no pending calls')
    release = Path(receipt['release_file'])
    if release.exists():
        raise FileExistsError(release)
    temp = release.with_suffix('.tmp')
    with temp.open('x') as stream:
        json.dump({'release': True, 'nonce': receipt['nonce'],
                   'settlement': settlement}, stream)
        stream.flush()
        os.fsync(stream.fileno())
    os.chmod(temp, 0o600)
    temp.replace(release)


def _run_observer_keeper(root, fds, terminal_log, nonce):
    terminal_fd = fds[0]
    input_stdout_fd = fds[1] if len(fds) == 3 else None
    release = root / 'observer-release.json'
    # The stdout line is the transfer acknowledgment consumed by the parent.
    print(json.dumps({'ready': True, 'pid': os.getpid()}), flush=True)
    max_bytes = 8 * 1024 * 1024
    log = Path(terminal_log)
    log_bytes = log.stat().st_size if log.exists() else 0
    terminal_open = True
    input_stdout_open = input_stdout_fd is not None
    while True:
        try:
            marker = json.loads(release.read_text())
            if marker.get('release') is True and secrets.compare_digest(
                    str(marker.get('nonce', '')), nonce):
                return 0
        except FileNotFoundError:
            pass
        watch = ([terminal_fd] if terminal_open else []) + ([input_stdout_fd] if input_stdout_open else [])
        if watch:
            readable, _, _ = select.select(watch, [], [], .25)
            for descriptor in readable:
                try:
                    data = os.read(descriptor, 65536)
                except OSError:
                    data = b''
                if not data:
                    if descriptor == terminal_fd:
                        terminal_open = False
                    else:
                        input_stdout_open = False
                    continue
                if descriptor == terminal_fd and log_bytes < max_bytes:
                    chunk = data[:max_bytes - log_bytes]
                    fd = os.open(log, os.O_WRONLY | os.O_CREAT | os.O_APPEND, 0o600)
                    try:
                        os.write(fd, chunk)
                    finally:
                        os.close(fd)
                    log_bytes += len(chunk)
        else:
            time.sleep(.25)


if __name__ == '__main__' and len(sys.argv) == 6 and sys.argv[1] == '--observer-keeper':
    raise SystemExit(_run_observer_keeper(Path(sys.argv[2]), tuple(json.loads(sys.argv[3])),
                                          Path(sys.argv[4]), sys.argv[5]))


class AndroidLogCapture:
    """Retain otherwise overwritten crash/toast evidence, without full logcat.

    Android may withhold OS kill reasons. An empty crash buffer is not proof
    that the app exited normally. This child shares Termux's lifetime.
    """
    def __init__(self, root, executable=None):
        root = Path(root)
        info = root.lstat()
        if root.resolve() != root or info.st_uid != os.getuid() or info.st_mode & 0o077:
            raise ValueError('private canonical evidence directory required')
        target = root / 'android.log'
        descriptor = os.open(target, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        os.close(descriptor)
        executable = executable or str(Path(os.environ['PREFIX']) / 'bin/logcat')
        self.process = subprocess.Popen(
            [executable, '-T', '1', '-v', 'epoch', '-b', 'main', '-b', 'crash',
             '-f', str(target), '-r', '256', '-n', '2',
             'Toast:I', 'AndroidRuntime:E', 'libc:F', '*:S'],
            stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL, start_new_session=True, umask=0o077)

    def close(self):
        if self.process.poll() is None:
            self.process.terminate()
            try:
                self.process.wait(timeout=3)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait(timeout=3)


def process_fact(path):
    raw = (path / 'stat').read_text()
    name = raw[raw.index('(') + 1:raw.rindex(')')]
    fields = raw[raw.rindex(')') + 2:].split()
    status = dict(line.split(':', 1) for line in (path / 'status').read_text().splitlines() if ':' in line)
    kib = lambda key: int(status.get(key, '0 kB').split()[0])
    return {'pid': int(path.name), 'name': name, 'ppid': int(fields[1]),
            'pgid': int(fields[2]), 'sid': int(fields[3]), 'start': fields[19],
            'cpu_ticks': int(fields[11]) + int(fields[12]),
            'rss_kib': kib('VmRSS'), 'peak_rss_kib': kib('VmHWM'),
            'swap_kib': kib('VmSwap'), 'threads': int(status.get('Threads', '0'))}


def sample(proc=Path('/proc'), uid=None):
    uid = os.getuid() if uid is None else uid
    rows, unavailable = [], 0
    for path in proc.iterdir():
        if not path.name.isdecimal():
            continue
        try:
            if path.stat().st_uid == uid:
                rows.append(process_fact(path))
        except (OSError, ValueError, KeyError, IndexError):
            unavailable += 1
    memory = {}
    try:
        for line in (proc / 'meminfo').read_text().splitlines():
            key, value = line.split(':', 1)
            if key in ['MemAvailable', 'MemFree', 'SwapFree', 'SwapTotal']:
                memory[key + '_kib'] = int(value.split()[0])
    except (OSError, ValueError):
        pass
    apps = [row for row in rows if row['name'] in ['com.termux', 'com.termux.api']]
    return {'memory': memory, 'owned_process_count': len(rows),
            'owned_rss_kib': sum(row['rss_kib'] for row in rows),
            'owned_threads': sum(row['threads'] for row in rows),
            'apps': apps, 'processes': rows, 'unavailable_process_reads': unavailable}


def classify(previous, current, closed=False):
    if closed:
        return 'recorder_closed'
    if previous.get('boot_id') != current.get('boot_id'):
        return 'device_boot_changed'
    identities = lambda row: {(p['name'], p['pid'], str(p['start'])) for p in row.get('apps', [])}
    old = identities(previous)
    if old and not old.issubset(identities(current)):
        return 'termux_app_identity_changed'
    return 'worker_recording_interrupted_cause_unknown'


class Recorder:
    def __init__(self, root, interval=5, max_sample_bytes=8 * 1024 * 1024):
        self.root = Path(root)
        self.root.mkdir(mode=0o700, parents=True, exist_ok=True)
        info = self.root.lstat()
        if self.root.resolve() != self.root or info.st_uid != os.getuid() or info.st_mode & 0o077:
            raise ValueError('private canonical evidence directory required')
        self.stage_file = (self.root / 'stages.jsonl').open('x')
        self.sample_file = (self.root / 'resources.jsonl').open('x')
        os.fchmod(self.stage_file.fileno(), 0o600)
        os.fchmod(self.sample_file.fileno(), 0o600)
        fcntl.flock(self.stage_file, fcntl.LOCK_EX | fcntl.LOCK_NB)
        self.lock = threading.Lock()
        self.stop = threading.Event()
        self.interval, self.max_sample_bytes = interval, max_sample_bytes
        self.phase = 'recorder_started'
        self.sequence = 0
        try:
            self.boot = Path('/proc/sys/kernel/random/boot_id').read_text().strip()
        except OSError:
            self.boot = 'unavailable'
        self.event('recorder_started', pid=os.getpid(), ppid=os.getppid(), boot_id=self.boot)
        self.capture()
        self.thread = threading.Thread(target=self._sample_loop, daemon=True)
        self.thread.start()

    def _append(self, stream, row):
        stream.write(json.dumps(row, separators=(',', ':')) + '\n')
        stream.flush()
        os.fsync(stream.fileno())

    def event(self, phase, **metadata):
        # Callers supply lifecycle metadata only; prompts/results remain in their
        # original private native/app records, never these diagnostic events.
        with self.lock:
            self.phase = phase
            self.sequence += 1
            row = dict(metadata, phase=phase, sequence=self.sequence,
                       UTC=datetime.datetime.now(datetime.timezone.utc).isoformat(),
                       monotonic_ns=time.monotonic_ns())
            self._append(self.stage_file, row)
            temporary = self.root / 'last-stage.tmp'
            with temporary.open('w') as stream:
                json.dump(row, stream)
                stream.flush()
                os.fsync(stream.fileno())
            temporary.replace(self.root / 'last-stage.json')
            descriptor = os.open(self.root, os.O_RDONLY | os.O_DIRECTORY)
            try:
                os.fsync(descriptor)
            finally:
                os.close(descriptor)
            return row

    def capture(self):
        row = sample()
        row.update(boot_id=self.boot, monotonic_ns=time.monotonic_ns(),
                   UTC=datetime.datetime.now(datetime.timezone.utc).isoformat())
        with self.lock:
            row['phase'] = self.phase
            encoded = json.dumps(row, separators=(',', ':')) + '\n'
            if self.sample_file.tell() + len(encoded.encode()) > self.max_sample_bytes:
                return False
            self._append(self.sample_file, row)
        return True

    def _sample_loop(self):
        while not self.stop.wait(self.interval):
            try:
                if not self.capture():
                    self.event(self.phase, diagnostic='resource_log_cap_reached')
                    return
            except Exception as error:
                self.event('resource_sampling_error', error_type=type(error).__name__)
                return

    def close(self, outcome):
        self.stop.set()
        self.thread.join(timeout=10)
        self.event('recorder_closed', outcome=outcome)
        self.stage_file.close()
        self.sample_file.close()
