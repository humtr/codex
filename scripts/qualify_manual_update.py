"""Exercise the real manual updater in an owned Android installation.

Only transport/CI responses are local witnesses. Core still verifies the real
fixture signature and installed inventory. No public request or device change.
"""
import argparse
import fcntl
import json
import os
from pathlib import Path
import pty
import re
import select
import signal
import struct
import subprocess
import sys
import tempfile
import termios
import time

from qualify_permission_lifecycle import fixture, server_paths


def visible(text):
    text = re.sub(r'\x1b\].*?(?:\x07|\x1b\\)', '', text, flags=re.S)
    return re.sub(r'\x1b\[[0-?]*[ -/]*[@-~]', '', text)


def probe(args):
    root = Path(tempfile.mkdtemp(prefix='mu.', dir=os.environ['TMPDIR']))
    result = {'accepted': False, 'root': str(root), 'mode': args.mode,
              'publicRequests': 0, 'productionChanged': False}
    home, prefix, env = fixture(root, args.core, args.frontend, generation=args.generation)
    state = home / '.local/share/codex/core/activation-state'
    initial = state.read_bytes()
    fields = dict(line.split('\t', 1) for line in
                  (root / 'local-permission-fixture/generation.meta').read_text().splitlines()[1:])
    installed = fields['upstream_package_version']
    major, minor, patch = map(int, installed.split('.'))
    latest = installed if args.mode == 'current' else f'{major}.{minor + 1}.{patch}'
    (root / 'official.json').write_text(json.dumps({'tag_name': 'rust-v' + latest,
        'draft': False, 'prerelease': False, 'assets': [
            {'name': 'codex-package-aarch64-unknown-linux-musl.tar.gz', 'digest': 'sha256:' + 'a' * 64}]}))
    index = root / 'index'
    index.write_text('codex-update-index-v1\nchannel\tstable\ngeneration_id\tlocal-permission-fixture\n'
                     'release_base\thttps://example.invalid/local-permission-fixture/\n')
    subprocess.run(['openssl', 'pkeyutl', '-sign', '-inkey', str(root / 'fixture-key.pem'),
                    '-rawin', '-in', str(index), '-out', str(root / 'index.sig')], check=True, capture_output=True)
    if args.mode == 'bad-signature':
        index.write_bytes(index.read_bytes() + b'unsigned change\n')
    (root / 'fixture-key.pem').unlink()
    # The production HTTPS URLs remain selected: custom-index overrides would
    # intentionally disable manual public freshness and would miss this path.
    curl = prefix / 'bin/curl'
    curl.write_text('#!' + sys.executable + '\n' +
        'from pathlib import Path\nimport json,sys\nroot=Path(' + repr(str(root)) + ')\n' +
        'url=sys.argv[sys.argv.index("--url")+1]\n' +
        'with (root/"transports.jsonl").open("a") as f:f.write(json.dumps(url)+"\\n")\n' +
        'if url=="https://api.github.com/repos/openai/codex/releases/latest":name="official.json"\n' +
        'elif url.endswith("/update-index-base-v1") or url.endswith("/update-index-v1"):name="index"\n' +
        'elif url.endswith("/update-index-base-v1.sig") or url.endswith("/update-index-v1.sig"):name="index.sig"\n' +
        'else:sys.exit(22)\n' +
        'sys.stdout.buffer.write((root/name).read_bytes())\n')
    curl.chmod(0o700)
    gh = prefix / 'bin/gh'
    gh.write_text('#!' + sys.executable + '\nfrom pathlib import Path\nimport json,sys\n' +
        'root=Path(' + repr(str(root)) + ')\n' +
        'with (root/"github.jsonl").open("a") as f:f.write(json.dumps(sys.argv[1:])+"\\n")\n' +
        'if "POST" in sys.argv:\n' +
        (' sys.exit(1)\n' if args.mode == 'uncertain' else ' print("12345")\n') +
        'elif "/actions/runs/12345" in sys.argv[2]:print("in_progress\\tpending")\n' +
        'else:print("0")\n')
    gh.chmod(0o700)
    process = master = None
    transcript = ''
    owners = []
    try:
        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 40, 160, 0, 0))
        saved = termios.tcgetattr(slave)
        command = [] if args.mode == 'ordinary' else ['update']
        if args.mode == 'no-wait':
            command += ['--no-wait']
        process = subprocess.Popen([str(prefix / 'bin/codex'), *command], env=env,
            cwd=root / 'w', stdin=slave, stdout=slave, stderr=slave, start_new_session=True)
        os.close(slave)
        deadline = time.monotonic() + 60
        key_sent = False
        expected_tui = args.mode in ['continue', 'no-wait', 'ordinary']
        while time.monotonic() < deadline:
            if select.select([master], [], [], .1)[0]:
                try:
                    transcript += os.read(master, 65536).decode(errors='replace')
                except OSError as error:
                    if error.errno != 5:
                        raise
            if ('Waiting for signed publication.' in transcript and not key_sent
                    and not termios.tcgetattr(master)[3] & termios.ICANON):
                # Waiting must not retain the shared generation lease.
                lease = os.open(home / '.local/lib/codex/core', os.O_RDONLY)
                try:
                    fcntl.flock(lease, fcntl.LOCK_EX | fcntl.LOCK_NB)
                    fcntl.flock(lease, fcntl.LOCK_UN)
                finally:
                    os.close(lease)
                result['waitLeaseReleased'] = True
                if args.mode == 'interrupt':
                    os.kill(process.pid, signal.SIGTERM)
                else:
                    os.write(master, b'\x03' if args.mode == 'ctrl-c' else b'c' if args.mode == 'continue' else b'q')
                key_sent = True
            if expected_tui and 'OpenAI Codex (v' + installed + ')' in visible(transcript):
                break
            if process.poll() is not None:
                assert not expected_tui, transcript[-2000:]
                break
        else:
            raise AssertionError('manual updater did not settle within its fixture bound')
        calls = [json.loads(line) for line in (root / 'github.jsonl').read_text().splitlines()] if (root / 'github.jsonl').exists() else []
        dispatches = sum('POST' in call for call in calls)
        expected_dispatches = 0 if args.mode in ['ordinary', 'current', 'bad-signature'] else 1
        assert dispatches == expected_dispatches, calls
        if args.mode in ['bad-signature', 'uncertain']:
            assert process.returncode == 1, transcript[-1200:]
        elif args.mode in ['interrupt', 'ctrl-c']:
            assert process.returncode in ([-15, 143] if args.mode == 'interrupt' else [-2, 130]), transcript[-1200:]
        elif not expected_tui:
            assert process.returncode == 0, transcript[-1200:]
        assert state.read_bytes() == initial, 'request/wait must preserve current selection'
        if args.mode in ['quit', 'interrupt', 'ctrl-c']:
            assert termios.tcgetattr(master) == saved, 'terminal mode was not restored'
        result.update(accepted=True, dispatches=dispatches, enteredInstalledTui=expected_tui,
                      activationPreserved=True, terminalExit=process.returncode)
        return result
    except BaseException as error:
        result['error'] = type(error).__name__ + ': ' + str(error)
        raise
    finally:
        for path in server_paths(home):
            pid = int(path.with_name('pid').read_text())
            owners.append((pid, Path(f'/proc/{pid}/exe').resolve()))
        if process is not None and process.poll() is None:
            os.killpg(process.pid, signal.SIGTERM)
            process.wait(timeout=8)
        for pid, executable in owners:
            if Path(f'/proc/{pid}/exe').resolve() == executable:
                os.kill(pid, signal.SIGTERM)
                until = time.monotonic() + 5
                while Path(f'/proc/{pid}/exe').resolve() == executable and time.monotonic() < until:
                    time.sleep(.05)
                if Path(f'/proc/{pid}/exe').resolve() == executable:
                    os.kill(pid, signal.SIGKILL)
        if master is not None:
            os.close(master)
        (root / 'terminal.log').write_text(transcript)
        (root / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
        args.report.write_text(json.dumps(result, indent=2) + '\n')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--core', type=Path, required=True)
    parser.add_argument('--generation', type=Path, required=True)
    parser.add_argument('--frontend', type=Path)
    parser.add_argument('--mode', choices=['quit', 'continue', 'no-wait', 'current', 'ordinary', 'bad-signature', 'uncertain', 'interrupt', 'ctrl-c'], required=True)
    parser.add_argument('--report', type=Path, required=True)
    print(json.dumps(probe(parser.parse_args())))
