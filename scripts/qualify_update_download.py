#!/usr/bin/env python3
"""Real native signed update over a faulted local HTTPS origin; no public writes."""
import argparse
import hashlib
import http.server
import json
import os
from pathlib import Path
import shutil
import signal
import socket
import ssl
import subprocess
import tempfile
import threading
import time

from qualify_permission_lifecycle import fixture


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def qualify(args):
    root = Path(tempfile.mkdtemp(prefix='update-download-', dir=os.environ['TMPDIR']))
    root.chmod(0o700)
    report = {'accepted': False, 'root': str(root), 'publicRequests': 0,
              'productionChanged': False, 'transport': 'native curl / real local TLS'}
    origin = thread = None
    try:
        home, prefix, env = fixture(root, args.core, None, generation=args.generation)
        shutil.copy2(shutil.which('curl'), prefix / 'bin/curl')
        (prefix / 'etc/tls/certs').mkdir()
        initial = home / '.local/lib/codex/core/generations/local-permission-fixture'
        candidate = root / 'download-next'
        shutil.copytree(initial, candidate)
        descriptor = candidate / 'generation.meta'
        descriptor.write_text(descriptor.read_text().replace('local-permission-fixture', candidate.name))
        manifest = candidate / 'release.manifest'
        original = manifest.read_text()
        sequence = int(next(s.split('\t')[1] for s in original.splitlines() if s.startswith('release_sequence\t')))
        original = original.replace('local-permission-fixture', candidate.name)
        original = '\n'.join('release_sequence\t' + str(sequence+1) if line.startswith('release_sequence\t') else
            '\t'.join(['file', 'generation.meta', digest(descriptor), line.split('\t')[3]])
            if line.startswith('file\tgeneration.meta\t') else line for line in original.splitlines()) + '\n'
        manifest.write_text(original)
        subprocess.run(['openssl', 'pkeyutl', '-sign', '-inkey', str(root/'fixture-key.pem'),
            '-rawin', '-in', str(manifest), '-out', str(candidate/'release.sig')], check=True, capture_output=True)
        cert, key = prefix/'etc/tls/cert.pem', root/'origin-key.pem'
        subprocess.run(['openssl','req','-x509','-newkey','rsa:2048','-nodes','-days','1',
            '-subj','/CN=127.0.0.1','-addext','subjectAltName=IP:127.0.0.1',
            '-keyout',str(key),'-out',str(cert)],check=True,capture_output=True)
        key.chmod(0o600)
        records = []
        broken = False
        served = {'release.manifest', 'release.sig'} | {
            line.split('\t')[1] for line in original.splitlines() if line.startswith('file\t')}

        class Handler(http.server.BaseHTTPRequestHandler):
            def log_message(self, *_):
                pass

            def do_GET(self):
                nonlocal broken
                relative = self.path.removeprefix('/download-next/')
                record = {'path': self.path, 'range': self.headers.get('Range')}
                records.append(record)
                if not self.path.startswith('/download-next/') or relative not in served:
                    self.send_error(404)
                    return
                path = candidate/relative
                start = 0
                if record['range']:
                    assert record['range'].startswith('bytes=') and record['range'].endswith('-')
                    start = int(record['range'][6:-1])
                size = path.stat().st_size
                self.send_response(206 if start else 200)
                self.send_header('Content-Length', str(size-start))
                if start:
                    self.send_header('Content-Range', f'bytes {start}-{size-1}/{size}')
                self.end_headers()
                with path.open('rb') as stream:
                    stream.seek(start)
                    if relative == 'runtime' and not broken:
                        broken = True
                        body = stream.read(1024*1024)
                        self.wfile.write(body)
                        self.wfile.flush()
                        self.connection.shutdown(socket.SHUT_RDWR)
                        record['interruptedAfter'] = len(body)
                    else:
                        shutil.copyfileobj(stream, self.wfile)

        origin = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Handler)
        context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        context.load_cert_chain(cert, key)
        origin.socket = context.wrap_socket(origin.socket, server_side=True)
        thread = threading.Thread(target=origin.serve_forever, daemon=True)
        thread.start()
        base = f'https://127.0.0.1:{origin.server_port}/download-next/'
        state = home/'.local/share/codex/core/activation-state'
        before = state.read_bytes()
        protected = {str(path): path.read_bytes() for path in [home/'.codex/config.toml', home/'.codex/auth.json', prefix/'etc/resolv.conf']}

        def run(name, arguments):
            started = time.monotonic()
            child = subprocess.Popen([str(prefix/'bin/codex'), *arguments], env=env,
                cwd=root/'w', stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True)
            try:
                stdout, stderr = child.communicate(timeout=180)
            except subprocess.TimeoutExpired:
                os.killpg(child.pid, signal.SIGTERM)
                try:
                    stdout, stderr = child.communicate(timeout=5)
                except subprocess.TimeoutExpired:
                    os.killpg(child.pid, signal.SIGKILL)
                    stdout, stderr = child.communicate()
                (root/(name+'.stdout')).write_bytes(stdout)
                (root/(name+'.stderr')).write_bytes(stderr)
                raise AssertionError('owned update exceeded the proof deadline; its process group was reaped')
            (root/(name+'.stdout')).write_bytes(stdout)
            (root/(name+'.stderr')).write_bytes(stderr)
            report[name] = {'exit': child.returncode, 'seconds': round(time.monotonic()-started, 3)}
            return subprocess.CompletedProcess(child.args, child.returncode, stdout, stderr)

        first = run('interrupted', ['update', '--remote', base])
        assert first.returncode == 1 and b'remote release transport failed' in first.stderr.lower(), first.stderr
        assert state.read_bytes() == before and not list(initial.parent.glob('.acquire-*'))
        runtime_line = next(line.split('\t') for line in original.splitlines() if line.startswith('file\truntime\t'))
        part = home/'.local/share/codex/core/downloads-v1'/ (runtime_line[2]+'.part')
        assert part.stat().st_size == 1024*1024
        prior_records = len(records)
        second = run('resumed', ['update', '--remote', base])
        assert second.returncode == 0, second.stderr
        assert any(r['path'] == '/download-next/runtime' and r['range'] == 'bytes=1048576-' for r in records[prior_records:])
        installed = initial.parent/'download-next'
        assert digest(installed/'runtime') == runtime_line[2]
        assert state.read_bytes() != before
        assert b'codex-cli ' in run('version', ['--version']).stdout
        for filename, content in protected.items():
            assert Path(filename).read_bytes() == content
        rollback = run('rollback', ['update', '--rollback'])
        assert rollback.returncode == 0, rollback.stderr
        fields = dict(line.split('=', 1) for line in state.read_text().splitlines() if '=' in line)
        assert fields['current'] == 'local-permission-fixture' and fields['previous'] == 'download-next'
        for filename, content in protected.items():
            assert Path(filename).read_bytes() == content
        report.update(accepted=True, authenticatedResume=True, partialNotActivated=True,
            actualVersionExecuted=True, rollback=True, protectedStatePreserved=True)
        return report
    except BaseException as error:
        report['error'] = type(error).__name__ + ': ' + str(error)
        raise
    finally:
        if origin is not None:
            origin.shutdown()
            origin.server_close()
        if thread is not None:
            thread.join(timeout=5)
        if 'records' in locals():
            (root/'origin-requests.json').write_text(json.dumps(records, indent=2)+'\n')
        args.report.write_text(json.dumps(report, indent=2)+'\n')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--core', type=Path, required=True)
    parser.add_argument('--generation', type=Path, required=True)
    parser.add_argument('--report', type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(qualify(args)))
