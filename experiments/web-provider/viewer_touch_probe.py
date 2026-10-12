"""Account-free native X11/RFB touch and input proof; no live browser interaction."""
import json
import os
from pathlib import Path
import secrets
import struct
import subprocess
import sys
import tempfile
import time


def main():
    bun, candidate, assets = (str(Path(arg).resolve(strict=True)) for arg in sys.argv[1:])
    prefix = Path(os.environ['PREFIX'])
    root = Path(tempfile.mkdtemp(prefix='vt-', dir=os.environ['TMPDIR']))
    root.chmod(0o700)
    for name in ('h', 't', 'c'):
        (root / name).mkdir(mode=0o700)
    display = next(number for number in range(128, 256)
                   if not (Path(os.environ['TMPDIR']) / f'.X{number}-lock').exists()
                   and not (Path(os.environ['TMPDIR']) / '.X11-unix' / f'X{number}').exists())
    def field(value):
        return struct.pack('!H', len(value)) + value
    cookie = struct.pack('!H', 65535) + field(b'localhost') + field(str(display).encode())
    cookie += field(b'MIT-MAGIC-COOKIE-1') + field(secrets.token_bytes(16))
    (root / 'xauthority').write_bytes(cookie)
    (root / 'xauthority').chmod(0o600)
    (root / 'account-free-fixture').write_text('owned touch fixture\n')
    env = {'HOME': str(root / 'h'), 'TMPDIR': str(root / 't'), 'CODEX_HOME': str(root / 'c'),
           'PREFIX': str(prefix), 'PATH': str(prefix / 'bin'), 'DISPLAY': f':{display}',
           'XAUTHORITY': str(root / 'xauthority'), 'LANG': 'C.UTF-8',
           'XDG_CONFIG_HOME': str(root / 'h/.config'), 'XDG_CACHE_HOME': str(root / 'h/.cache'),
           'XDG_DATA_HOME': str(root / 'h/.local/share')}
    (root / 'environment.json').write_text(json.dumps(env))
    (root / 'environment.json').chmod(0o600)
    source = Path(__file__).resolve().parent
    processes = []
    try:
        def start(args, log_name, ready):
            with (root / log_name).open('w') as log:
                process = subprocess.Popen(args, env=env, stdout=log, stderr=log, start_new_session=True)
            processes.append(process)
            for _ in range(50):
                if ready.exists():
                    return
                if process.poll() is not None:
                    raise RuntimeError(f'Owned {log_name} startup failed')
                time.sleep(.1)
            raise RuntimeError(f'Owned {log_name} readiness timed out')
        start([str(prefix / 'bin/Xvnc'), f':{display}', '-geometry', '520x740', '-depth', '24',
               '-rfbunixpath', str(root / 'rfb.sock'), '-rfbunixmode', '0600', '-rfbport', '-1',
               '-SecurityTypes', 'None', '-auth', str(root / 'xauthority'), '-nolisten', 'tcp',
               '-AlwaysShared'], 'xvnc.log', root / 'rfb.sock')
        # Match the actual login desktop's window manager; do not run autostart.
        with (root / 'openbox.log').open('w') as log:
            manager = subprocess.Popen([str(prefix / 'bin/openbox')], env=env, stdout=log, stderr=log, start_new_session=True)
        processes.append(manager)
        time.sleep(.2)
        if manager.poll() is not None:
            raise RuntimeError('Owned Openbox startup failed')
        start([bun, str(source / 'termux_viewer.ts'), assets, str(root / 'rfb.sock'),
               str(root / 'viewer.json')], 'viewer.log', root / 'viewer.json')
        result = subprocess.run([bun, str(source / 'viewer_touch_probe.mjs'), candidate, str(root)],
                                env=env, capture_output=True, text=True, timeout=90)
        (root / 'proof.log').write_text(result.stdout + result.stderr)
        if result.returncode:
            print(json.dumps({'passed': False, 'proofRoot': str(root), 'exit': result.returncode}))
        else:
            print(result.stdout.strip())
        return result.returncode
    finally:
        for process in reversed(processes):
            if process.poll() is None:
                process.terminate()
        for process in processes:
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
        # Only private metadata/logs remain for diagnosis; no active test service.


if __name__ == '__main__':
    raise SystemExit(main())
