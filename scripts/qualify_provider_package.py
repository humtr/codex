#!/usr/bin/env python3
"""Real Android signed optional-package lifecycle, without inference/activation."""
import argparse
import fcntl
import datetime
import hashlib
import http.server
import ssl
import threading
import urllib.parse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time
import traceback

from qualify_permission_lifecycle import fixture
from clean_development import fixture_cleanup
from provider_qualification import source_files


def sha(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def utc():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()


def qualify(args):
    source = Path(__file__).resolve().parents[1]
    root = Path(tempfile.mkdtemp(prefix='g-package-', dir=os.environ['TMPDIR']))
    report = dict(schema='termux-codex-provider-package-proof-v1', accepted=False,
                  root=str(root), started_UTC=utc(), actual_Chat_requests=0,
                  operating_runtime_changed=False, public_delivery_qualified=False,
                  same_binary_test_generations=True, cases=[])
    start = time.monotonic()
    inputs = {name: sha(getattr(args, name)) for name in ('core', 'frontend', 'provider', 'builder')}
    inputs['generation_manifest'] = sha(args.generation / 'release.manifest')
    helpers = ('scripts/qualify_provider_package.py', 'scripts/qualify_permission_lifecycle.py',
               'scripts/qualify_tasks.py', 'scripts/fixtures/permission-model.json',
               'scripts/clean_development.py', 'scripts/provider_qualification.py', '.github/scripts/rald3_preflight.py',
               '.github/scripts/rald5_publication.py')
    inputs['helpers'] = {}
    # Freeze dirty source explicitly; do not label the binary as a clean HEAD build.
    paths = sorted({*source.glob('crates/core/src/**/*.rs'),
                    *source.glob('crates/release-builder/src/**/*.rs'),
                    *source.glob('crates/runtime-binding/src/**/*.rs'),
                    *(source / name for name in source_files(source)),
                    *(source / name for name in ('Cargo.toml', 'Cargo.lock',
                      'crates/core/Cargo.toml', 'crates/release-builder/Cargo.toml',
                      'crates/runtime-binding/Cargo.toml', 'providers/chatgpt/Cargo.toml',
                      'providers/chatgpt/Cargo.lock', 'providers/chatgpt/NOTICE.md')),
                    *source.glob('providers/chatgpt/licenses/*')})
    inventory = {str(p.relative_to(source)): sha(p) for p in paths if p.is_file()}
    for name, digest in inventory.items():
        copied = root / 'source' / name
        copied.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source / name, copied)
        assert sha(copied) == digest
    for name in helpers:
        copied = root / 'source' / name
        copied.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source / name, copied)
        inputs['helpers'][name] = sha(copied)
    source_record = dict(base_commit=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=source).decode().strip(),
                         dirty=True, files=inventory, binaries=inputs.copy(),
                         build=dict(debug_information=0, incremental=False))
    source_inputs = root / 'source-inputs.json'
    source_inputs.write_text(json.dumps(source_record, indent=2) + '\n')
    inputs['source_inputs_sha256'] = sha(source_inputs)
    report['inputs'] = inputs
    (root / 'inputs.json').write_text(json.dumps(inputs, indent=2) + '\n')
    try:
        for composition in ('base', 'full'):
            case_root = root / composition
            case_root.mkdir(mode=0o700)
            case = dict(composition=composition, accepted=False, steps=[])
            report['cases'].append(case)
            home, prefix, env = fixture(case_root, args.core, args.frontend, generation=args.generation)
            env.update(XDG_CONFIG_HOME=str(case_root / 'config'), XDG_STATE_HOME=str(case_root / 'state'))
            launcher = prefix / 'bin/codex'
            key = case_root / 'fixture-key.pem'
            installed = home / '.local/lib/codex/core/generations/local-permission-fixture'

            def run(label, command, expected=0):
                entry = dict(name=label, started_UTC=utc())
                case['steps'].append(entry)
                begin = time.monotonic()
                result = subprocess.run([str(v) for v in command], env=env, cwd=case_root / 'w',
                                        capture_output=True, timeout=65)
                entry.update(exit=result.returncode, expected_exit=expected, seconds=time.monotonic() - begin,
                             ended_UTC=utc())
                log = case_root / f'{len(case["steps"]):03d}.stdout'
                log.write_bytes(result.stdout)
                log.with_suffix('.stderr').write_bytes(result.stderr)
                entry['stdout_sha256'] = sha(log)
                entry['stderr_sha256'] = sha(log.with_suffix('.stderr'))
                assert result.returncode == expected, (label, result.returncode, result.stderr[-1500:].decode(errors='replace'))
                return result

            def public(label, *command, expected=0):
                return run(label, [launcher, 'provider', *command], expected)

            if composition == 'base':
                removed = {'helpers/2', 'manager'}
                for name in removed:
                    (installed / name).unlink()
                descriptor = (installed / 'generation.meta').read_text().splitlines()
                descriptor = [line for line in descriptor if not line.startswith('helper\ttermux-manager-tui-v1:')]
                descriptor = ['helper_count\t2' if line.startswith('helper_count\t') else
                              'manager_artifact_digest\t-' if line.startswith('manager_artifact_digest\t') else line
                              for line in descriptor]
                (installed / 'generation.meta').write_text('\n'.join(descriptor) + '\n')
                manifest = (installed / 'release.manifest').read_text().splitlines()
                manifest = [line for line in manifest if not (line.startswith('file\t') and line.split('\t')[1] in removed)]
                manifest = ['file_count\t6' if line.startswith('file_count\t') else
                            '\t'.join(['file', 'generation.meta', sha(installed / 'generation.meta'), '0644'])
                            if line.startswith('file\tgeneration.meta\t') else line for line in manifest]
                (installed / 'release.manifest').write_text('\n'.join(manifest) + '\n')
                run('sign isolated Base composition', ['openssl', 'pkeyutl', '-sign', '-inkey', key,
                    '-rawin', '-in', installed / 'release.manifest', '-out', installed / 'release.sig'])
            sentinels = {p: sha(p) for p in (home / '.codex/config.toml', home / '.codex/auth.json')}
            core_state = home / '.local/share/codex/core/activation-state'
            core_before = sha(core_state)
            packages = case_root / 'packages'
            packages.mkdir(mode=0o700)

            def package(label, name, sequence, signing=key, assets=None, binary=None, expected=0, output_name=None):
                destination = packages / (output_name or name)
                run(label, [args.builder, 'provider-package', '--binary', binary or args.provider,
                    '--assets', assets or source / 'providers/chatgpt', '--source-inputs', source_inputs,
                    '--source-sha', source_record['base_commit'], '--generation-id', name,
                    '--release-sequence', sequence, '--private-key', signing, '--openssl', shutil.which('openssl'),
                    '--output', destination], expected)
                return destination

            one = package('signed package 1', 'chatgpt-g-test-1', 1)
            # Exercise an independently declared retained eight-file envelope
            # through the actual installer, update and rollback consumers.
            for name in ('bridge-LICENSE', 'upstream-codex-LICENSE'):
                (one / 'licenses' / name).write_text('Owned historical-layout fixture.\n')
                (one / 'licenses' / name).chmod(0o644)
            legacy_names = ('Cargo.lock', 'NOTICE.md', 'codex-chatgpt-provider',
                            'licenses/bridge-LICENSE', 'licenses/novnc-LICENSE.txt',
                            'licenses/proot-LICENSE', 'licenses/upstream-codex-LICENSE',
                            'source-inputs.json')
            header = (one / 'payload.manifest').read_text().split('file_count\t', 1)[0]
            (one / 'payload.manifest').write_text(header + 'file_count\t8\n' + ''.join(
                'file\t' + name + '\t' + sha(one / name) + '\t'
                + str((one / name).stat().st_size) + '\t'
                + ('0755' if name == 'codex-chatgpt-provider' else '0644') + '\n'
                for name in legacy_names))
            run('sign retained eight-file fixture', ['openssl', 'pkeyutl', '-sign',
                '-inkey', key, '-rawin', '-in', one / 'payload.manifest',
                '-out', one / 'payload.sig'])
            two = package('signed package 2', 'chatgpt-g-test-2', 2)
            package('refuse package output replacement', 'chatgpt-g-test-1', 1, expected=1)
            public('absent Provider inventory', 'status', 'chatgpt')
            public('absent Provider dispatch', 'chatgpt', 'models', expected=1)
            if args.remote:
                remote_requests = []
                declared = [line.split('\t')[1] for line in (one / 'payload.manifest').read_text().splitlines()
                            if line.startswith('file\t')]
                flat_assets = ['payload.manifest', 'payload.sig', *[name.replace('/', '--') for name in declared]]
                stable_paths = ['/humtr/codex/main/provider-index-v1', '/humtr/codex/main/provider-index-v1.sig']

                def complete_download(base):
                    expected = [base + name for name in flat_assets]
                    # Authentication precedes payload. Remaining assets may be
                    # fetched in any order, but every signed asset is read once.
                    assert remote_requests[:2] == expected[:2]
                    assert len(remote_requests) == len(expected) and set(remote_requests) == set(expected), remote_requests
                cert = case_root / 'remote-cert.pem'
                remote_key = case_root / 'remote-key.pem'
                run('create owned localhost TLS certificate', ['openssl', 'req', '-x509', '-newkey', 'rsa:2048', '-nodes', '-days', '1',
                    '-keyout', remote_key, '-out', cert, '-subj', '/CN=127.0.0.1', '-addext', 'subjectAltName=IP:127.0.0.1,DNS:raw.githubusercontent.com,DNS:github.com,DNS:api.github.com'])
                remote_key.chmod(0o600)
                shutil.copy2(cert, prefix / 'etc/tls/cert.pem')
                shutil.copy2(shutil.which('curl'), prefix / 'bin/curl')
                stable = dict(index=None, signature=None)
                core_index = case_root / 'core-index'
                core_index_sig = case_root / 'core-index.sig'
                core_index.write_text('codex-update-index-v1\nchannel\tstable\ngeneration_id\t'
                    + installed.name + '\nrelease_base\thttps://github.com/humtr/codex/releases/download/'
                    + installed.name + '/\n')
                run('sign owned Core current index', ['openssl', 'pkeyutl', '-sign', '-rawin',
                    '-inkey', key, '-in', core_index, '-out', core_index_sig])
                installed_version = next(line.split('\t')[1] for line in
                    (installed / 'generation.meta').read_text().splitlines()
                    if line.startswith('upstream_package_version\t'))
                stable['official_version'] = installed_version
                class Handler(http.server.BaseHTTPRequestHandler):
                    def log_message(self, *_):
                        pass
                    def do_GET(self):
                        remote_requests.append(self.path)
                        if self.path in ('/humtr/codex/main/update-index-v1',
                                         '/humtr/codex/main/update-index-base-v1',
                                         '/humtr/codex/main/update-index-v1.sig',
                                         '/humtr/codex/main/update-index-base-v1.sig'):
                            data = (core_index_sig if self.path.endswith('.sig') else core_index).read_bytes()
                        elif self.path == '/repos/openai/codex/releases/latest':
                            data = json.dumps(dict(tag_name='rust-v' + stable['official_version'],
                                published_at='2026-10-28T17:04:05Z', assets=[dict(
                                    name='codex-package-aarch64-unknown-linux-musl.tar.gz',
                                    digest='sha256:' + 'a' * 64)])).encode()
                        elif self.path.startswith('/repos/humtr/codex/releases/tags/'):
                            data = json.dumps(dict(tag_name=self.path.rsplit('/', 1)[1], draft=False,
                                published_at='2026-10-28T17:04:05Z',
                                body='## Core\n- Owned Core fixture.\n## Provider\n- Selected Provider fixture.\n')).encode()
                        else:
                            data = None
                        if data is not None:
                            self.send_response(200)
                            self.send_header('Content-Length', str(len(data)))
                            self.end_headers()
                            self.wfile.write(data)
                            return
                        held = stable.get('held_core')
                        held_prefix = '/humtr/codex/releases/download/' + held.name + '/' if held else None
                        if held_prefix and self.path.startswith(held_prefix):
                            name = urllib.parse.unquote(self.path[len(held_prefix):])
                            declared = {'release.manifest', 'release.sig', *[
                                line.split('\t')[1] for line in (held / 'release.manifest').read_text().splitlines()
                                if line.startswith('file\t')]}
                            if name not in declared:
                                self.send_error(404)
                                return
                            data = (held / name).read_bytes()
                            self.send_response(200)
                            self.send_header('Content-Length', str(len(data)))
                            self.end_headers()
                            self.wfile.write(data)
                            return
                        if self.path.startswith('/humtr/codex/releases/download/' + installed.name + '/'):
                            self.send_error(404, 'No optional component display sidecar in fixture')
                            return
                        if self.path.startswith('/humtr/codex/main/provider-index-v1'):
                            data = stable['signature' if self.path.endswith('.sig') else 'index']
                            self.send_response(200)
                            self.send_header('Content-Length', str(len(data)))
                            self.end_headers()
                            self.wfile.write(data)
                            return
                        if self.path.startswith('/humtr/codex/releases/download/chatgpt-g-test-1/'):
                            route, name = 'one', self.path.rsplit('/', 1)[1]
                        elif self.path.startswith('/humtr/codex/releases/download/chatgpt-g-test-2/'):
                            route, name = 'two', self.path.rsplit('/', 1)[1]
                        else:
                            route, name = self.path.strip('/').split('/', 1)
                        if route == 'redirect':
                            self.send_response(302)
                            self.send_header('Location', 'http://127.0.0.1:9/forbidden')
                            self.end_headers()
                            return
                        if name not in flat_assets:
                            self.send_error(404)
                            return
                        file = (one if route == 'one' else two) / name.replace('licenses--', 'licenses/')
                        if route == 'missing' and name == 'NOTICE.md':
                            self.send_error(404)
                            return
                        data = file.read_bytes()
                        if route == 'bad-signature' and name == 'payload.sig':
                            data = b'x' * 64
                        if route == 'tampered' and name == 'NOTICE.md':
                            data = b'x' * len(data)
                        self.send_response(200)
                        self.send_header('Content-Length', str(len(data)))
                        self.end_headers()
                        self.wfile.write(data)
                server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Handler)
                tls = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
                tls.load_cert_chain(cert, remote_key)
                server.socket = tls.wrap_socket(server.socket, server_side=True)
                thread = threading.Thread(target=server.serve_forever, daemon=True)
                thread.start()
                url = f'https://127.0.0.1:{server.server_port}/'
                # Only this private PREFIX changes the network destination. The
                # actual curl/TLS implementation still verifies the requested SNI.
                (prefix / 'bin/curl').rename(prefix / 'bin/owned-curl')
                wrapper = prefix / 'bin/curl'
                wrapper.write_text('#!' + str(Path(shutil.which('python')).resolve()) + '\n'
                    'import os,sys\n'
                    'os.execv(' + repr(str(prefix / 'bin/owned-curl')) + ',[' + repr(str(prefix / 'bin/owned-curl')) + ',\n'
                    + repr('--connect-to') + ',' + repr(f'raw.githubusercontent.com:443:127.0.0.1:{server.server_port}') + ',\n'
                    + repr('--connect-to') + ',' + repr(f'github.com:443:127.0.0.1:{server.server_port}') + ',\n'
                    + repr('--connect-to') + ',' + repr(f'api.github.com:443:127.0.0.1:{server.server_port}') + ']+sys.argv[1:])\n')
                wrapper.chmod(0o755)
                index = case_root / 'provider-index-v1'
                index_sig = case_root / 'provider-index-v1.sig'
                index.write_text('codex-provider-index-v1\nprovider\tchatgpt\nchannel\tstable\ngeneration_id\tchatgpt-g-test-1\nrelease_sequence\t1\nrelease_base\thttps://github.com/humtr/codex/releases/download/chatgpt-g-test-1/\nmanifest_sha256\t' + sha(one / 'payload.manifest') + '\n')
                run('sign owned stable index', ['openssl', 'pkeyutl', '-sign', '-rawin', '-inkey', key, '-in', index, '-out', index_sig])
                stable.update(index=index.read_bytes(), signature=index_sig.read_bytes())
                try:
                    run('default manual update does not enroll absent Provider', [launcher, 'update'])
                    assert not (home / '.local/share/codex/core/providers/chatgpt').exists()
                    assert not any(path in remote_requests for path in stable_paths)
                    remote_requests.clear()
                    for bad in ('http://127.0.0.1/not-https/', url + 'one', url + 'one/?query=x', url + '../one/'):
                        public('reject remote URL syntax', 'install', 'chatgpt', '--remote', bad, expected=1)
                    assert not remote_requests
                    for route in ('redirect', 'bad-signature', 'tampered', 'missing'):
                        remote_requests.clear()
                        public('reject remote ' + route, 'install', 'chatgpt', '--remote', url + route + '/', expected=1)
                        assert not (home / '.local/share/codex/core/providers/chatgpt/selection').exists()
                        if route == 'bad-signature':
                            assert remote_requests == ['/bad-signature/payload.manifest', '/bad-signature/payload.sig']
                    remote_requests.clear()
                    public('real TLS remote installation', 'install', 'chatgpt', '--remote', url + 'one/')
                    complete_download('/one/')
                    remote_requests.clear()
                    public('real TLS remote exact reinstall', 'install', 'chatgpt', '--remote', url + 'one/')
                    complete_download('/one/')
                    selection_path = home / '.local/share/codex/core/providers/chatgpt/selection'
                    unchanged_selection = selection_path.read_bytes()
                    for label, action in [('stable index authenticated update over real TLS', 'update'),
                                          ('stable index default install over real TLS', 'install')]:
                        remote_requests.clear()
                        public(label, action, 'chatgpt')
                        assert remote_requests == stable_paths, remote_requests
                        assert selection_path.read_bytes() == unchanged_selection
                    remote_requests.clear()
                    no_login = case_root / 'saved-inert-auth.json'
                    (home / '.codex/auth.json').rename(no_login)
                    try:
                        unchanged = run('default manual update reuses selected Provider without model login',
                                        [launcher, 'update'])
                    finally:
                        no_login.rename(home / '.codex/auth.json')
                    assert b'Provider chatgpt unchanged:' in unchanged.stdout
                    assert b'Selected Provider fixture.' in unchanged.stdout
                    assert all(remote_requests.count(path) == 1 for path in stable_paths)
                    assert not any('/releases/download/chatgpt-g-test-' in path for path in remote_requests)
                    assert selection_path.read_bytes() == unchanged_selection
                    saved_signature = stable['signature']
                    stable['signature'] = b'x' * 64
                    before = (home / '.local/share/codex/core/providers/chatgpt/selection').read_bytes()
                    remote_requests.clear()
                    public('reject unverified stable index before payload', 'update', 'chatgpt', expected=1)
                    assert remote_requests == stable_paths
                    assert (home / '.local/share/codex/core/providers/chatgpt/selection').read_bytes() == before
                    remote_requests.clear()
                    stable['official_version'] = '9.999.0'
                    env['CODEX_TERMUX_UPDATE_BUILD_POLICY'] = 'keep'
                    rejected = run('default manual update rejects unauthenticated Provider index',
                                   [launcher, 'update'], expected=1)
                    assert all(remote_requests.count(path) == 1 for path in stable_paths)
                    assert not any('/releases/download/chatgpt-g-test-' in path for path in remote_requests)
                    assert selection_path.read_bytes() == before and sha(core_state) == core_before
                    assert b'Keeping Codex' in rejected.stdout and b'9.999.0' in rejected.stdout
                    stable['official_version'] = installed_version
                    del env['CODEX_TERMUX_UPDATE_BUILD_POLICY']
                    stable['signature'] = saved_signature
                    index.write_text(index.read_text().replace('chatgpt-g-test-1', 'chatgpt-g-test-2')
                        .replace('release_sequence\t1\n', 'release_sequence\t2\n')
                        .replace(sha(one / 'payload.manifest'), sha(two / 'payload.manifest')))
                    run('sign next owned Provider index', ['openssl', 'pkeyutl', '-sign', '-rawin',
                        '-inkey', key, '-in', index, '-out', index_sig])
                    stable.update(index=index.read_bytes(), signature=index_sig.read_bytes(),
                                  official_version='9.999.0')
                    env['CODEX_TERMUX_UPDATE_BUILD_POLICY'] = 'keep'
                    remote_requests.clear()
                    advanced = run('manual update advances Provider while missing Core build is kept',
                                   [launcher, 'update'])
                    assert b'Keeping Codex' in advanced.stdout and b'9.999.0' in advanced.stdout
                    assert b'current\tchatgpt-g-test-2\n' in selection_path.read_bytes()
                    assert all(remote_requests.count(path) == 1 for path in stable_paths)
                    expected_payload = ['/humtr/codex/releases/download/chatgpt-g-test-2/'
                        + name.replace('/', '--') for name in ['payload.manifest', 'payload.sig',
                            *[line.split('\t')[1] for line in (two / 'payload.manifest').read_text().splitlines()
                              if line.startswith('file\t')]]]
                    assert [path for path in remote_requests if '/releases/download/chatgpt-g-test-' in path] == expected_payload
                    assert sha(core_state) == core_before
                    del env['CODEX_TERMUX_UPDATE_BUILD_POLICY']
                    stable['official_version'] = installed_version
                    public('restore previous Provider with actual rollback', 'rollback', 'chatgpt')
                    status = public('manual update rollback retains high water', 'status', 'chatgpt').stdout
                    assert b'release_sequence: 1' in status and b'high_water: 2' in status
                    core_signature = core_index_sig.read_bytes()
                    core_index_sig.write_bytes(b'x' * 64)
                    failed_core = run('failed Core candidate still permits independent Provider forward update',
                                      [launcher, 'update'], expected=1)
                    assert b'Provider chatgpt selected:' in failed_core.stdout
                    assert b'current\tchatgpt-g-test-2\n' in selection_path.read_bytes()
                    assert sha(core_state) == core_before
                    core_index_sig.write_bytes(core_signature)
                    public('restore Provider after independently failed Core check', 'rollback', 'chatgpt')

                    installed_signature = installed / 'release.sig'
                    current_signature = installed_signature.read_bytes()
                    installed_signature.write_bytes(b'x' * 64)
                    remote_requests.clear()
                    selected_before_bad_core = selection_path.read_bytes()
                    run('invalid installed Core cannot authorize Provider updates',
                        [launcher, 'update'], expected=1)
                    assert not any(path in remote_requests for path in stable_paths)
                    assert selection_path.read_bytes() == selected_before_bad_core
                    assert sha(core_state) == core_before
                    installed_signature.write_bytes(current_signature)

                    # Establish two already-admitted same-binary Core fixtures,
                    # then use real Core rollback to create its policy hold.
                    held = installed.with_name(installed.name + '-held')
                    shutil.copytree(installed, held)
                    descriptor = held / 'generation.meta'
                    descriptor.write_text(descriptor.read_text().replace(installed.name, held.name))
                    held_manifest = held / 'release.manifest'
                    sequence = int(next(line.split('\t')[1] for line in
                        held_manifest.read_text().splitlines() if line.startswith('release_sequence\t')))
                    held_manifest.write_text('\n'.join(
                        'generation_id\t' + held.name if line.startswith('generation_id\t') else
                        'release_sequence\t' + str(sequence + 1) if line.startswith('release_sequence\t') else
                        '\t'.join(['file', 'generation.meta', sha(descriptor), '0644'])
                        if line.startswith('file\tgeneration.meta\t') else line
                        for line in held_manifest.read_text().splitlines()) + '\n')
                    run('sign already-admitted Core fixture', ['openssl', 'pkeyutl', '-sign', '-rawin',
                        '-inkey', key, '-in', held_manifest, '-out', held / 'release.sig'])
                    values = dict(line.split('=', 1) for line in core_state.read_text().splitlines())
                    values.update(current=held.name, previous_present='1', previous=installed.name,
                                  previous_key=values['current_key'])
                    core_state.write_text('\n'.join(k + '=' + v for k, v in values.items()) + '\n')
                    run('actual Core rollback establishes update hold', [launcher, 'update', '--rollback'])
                    held_state = core_state.read_bytes()
                    hold_path = home / '.local/share/codex/core/update-hold'
                    assert hold_path.exists()
                    stable['held_core'] = held
                    core_index.write_text(core_index.read_text().replace(installed.name, held.name))
                    run('sign held Core candidate index', ['openssl', 'pkeyutl', '-sign', '-rawin',
                        '-inkey', key, '-in', core_index, '-out', core_index_sig])
                    remote_requests.clear()
                    policy = run('Core rollback hold still permits independent Provider forward update',
                                 [launcher, 'update'], expected=1)
                    assert b'locally held after rollback' in policy.stderr
                    assert b'Provider chatgpt selected:' in policy.stdout
                    assert b'current\tchatgpt-g-test-2\n' in selection_path.read_bytes()
                    assert core_state.read_bytes() == held_state and hold_path.exists()
                    assert all(remote_requests.count(path) == 1 for path in stable_paths)
                    public('restore Provider while preserving Core hold', 'rollback', 'chatgpt')
                    core_before = sha(core_state)
                    case['remote'] = dict(real_TLS=True, requests_per_complete_install=len(flat_assets), signature_before_payload=True,
                                          no_implicit_retry=True, local_certificate_only=True,
                                          unchanged_stable_package_requests=0, unchanged_selection_preserved=True,
                                          manual_selected_update=True, unselected_not_enrolled=True,
                                          one_provider_index_per_manual_invocation=True,
                                          missing_core_keep_allows_independent_provider_update=True,
                                          manual_signature_failure_preserves_both_groups=True,
                                          independent_provider_update_after_core_failure=True,
                                          actual_core_rollback_hold_preserved=True,
                                          invalid_installed_core_cannot_authorize_provider=True,
                                          provider_failure_does_not_block_core_keep=True,
                                          no_model_login_required=True)
                finally:
                    case['remote_requests'] = list(remote_requests)
                    server.shutdown()
                    server.server_close()
                    thread.join(timeout=5)
                assert not list((home / '.local/share/codex/core/providers/chatgpt').glob('.download-*'))
            public('install generation 1', 'install', 'chatgpt', '--local', one)
            selection = home / '.local/share/codex/core/providers/chatgpt/selection'
            original_selection = selection.read_bytes()
            public('exact reinstall', 'install', 'chatgpt', '--local', one)
            assert selection.read_bytes() == original_selection
            public('selected inventory', 'status', 'chatgpt')
            public('opaque model dispatch', 'chatgpt', 'models')
            public('explicit isolated native binding', 'chatgpt', 'config', 'set', 'execution.core', launcher,
                   'execution.home', home / '.codex', 'execution.workspace', case_root / 'w')
            service = case_root / 'provider-state'
            prepared = json.loads(public('explicit prepare generation 1', 'chatgpt', 'prepare', '--home', home / '.codex', '--state', service).stdout)
            profile = Path(prepared['profile'])
            profile_before = sha(profile)
            profile_binary = home / '.local/lib/codex/providers/chatgpt/generations/chatgpt-g-test-1/codex-chatgpt-provider'
            assert str(profile_binary) in profile.read_text()
            def running_owner(label):
                observed = json.loads(run(label, [profile_binary, 'status', '--state', service]).stdout)
                assert observed['live'] is True
                process = observed['process']
                assert Path('/proc/' + str(process['pid']) + '/exe').resolve(strict=True) == profile_binary
                return process

            def selection_during_execution(label):
                before_owner = running_owner(label + ' owner before')
                public(label + ' install generation 2 while generation 1 runs',
                       'install', 'chatgpt', '--local', two)
                assert b'current\tchatgpt-g-test-2\n' in selection.read_bytes()
                assert running_owner(label + ' owner after install') == before_owner
                assert sha(profile) == profile_before, 'selection rewrote the pinned profile'
                public(label + ' rollback selection while generation 1 runs', 'rollback', 'chatgpt')
                status = public(label + ' high water after rollback', 'status', 'chatgpt').stdout
                assert b'release_sequence: 1' in status and b'high_water: 2' in status
                assert running_owner(label + ' owner after rollback') == before_owner
                assert sha(profile) == profile_before
                assert profile_binary.exists()
                case.setdefault('live_selection', []).append(dict(route=label,
                    kernel_pid_preserved=True, immutable_executable_preserved=True,
                    original_profile_preserved=True, rollback_high_water_preserved=True))

            try:
                selection_during_execution('Core-prepared detached service')
            finally:
                # Stop only the isolated service through its original immutable owner.
                status = subprocess.run([str(profile_binary), 'status', '--state', str(service)], env=env, capture_output=True, timeout=15)
                if status.returncode == 0 and json.loads(status.stdout).get('process'):
                    subprocess.run([str(profile_binary), 'stop', '--state', str(service)], env=env, capture_output=True, timeout=15, check=True)
            # A pinned profile bypasses Core dispatch. Its immutable bytes and
            # kernel identity must survive the same independent selection changes.
            run('direct pinned executable prepare', [profile_binary, 'prepare', '--home', home / '.codex', '--state', service])
            try:
                selection_during_execution('direct pinned service')
            finally:
                run('stop directly bound owner', [profile_binary, 'stop', '--state', service])
            # Foreground Core exec retains the actual store lease. Detached
            # prepare intentionally closes incidental handles, so it alone
            # cannot exercise the former shared-reader/exclusive-writer defect.
            with (case_root / 'foreground-service.log').open('wb') as log:
                foreground = subprocess.Popen([str(launcher), 'provider', 'chatgpt',
                    'serve', '--state', str(service)], env=env, cwd=case_root / 'w',
                    stdin=subprocess.DEVNULL, stdout=log, stderr=subprocess.STDOUT,
                    start_new_session=True)
                try:
                    deadline = time.monotonic() + 20
                    while True:
                        observed = subprocess.run([str(profile_binary), 'status', '--state', str(service)],
                            env=env, capture_output=True, timeout=5)
                        if observed.returncode == 0 and json.loads(observed.stdout).get('live'):
                            break
                        assert foreground.poll() is None, 'foreground owner exited before readiness'
                        assert time.monotonic() < deadline, 'foreground owner never became ready'
                        time.sleep(.05)
                    assert json.loads(observed.stdout)['process']['pid'] == foreground.pid
                    store_root = home / '.local/share/codex/core/providers/chatgpt'
                    guard = os.open(store_root, os.O_RDONLY | os.O_DIRECTORY)
                    try:
                        try:
                            fcntl.flock(guard, fcntl.LOCK_EX | fcntl.LOCK_NB)
                        except BlockingIOError:
                            pass
                        else:
                            raise AssertionError('foreground Core did not retain the actual shared store lease')
                    finally:
                        os.close(guard)
                    selection_during_execution('Core foreground store lease')
                finally:
                    stopped = subprocess.run([str(profile_binary), 'stop', '--state', str(service)],
                        env=env, capture_output=True, timeout=15)
                    if foreground.poll() is None:
                        foreground.terminate()
                    try:
                        foreground.wait(timeout=8)
                    except subprocess.TimeoutExpired:
                        foreground.kill()
                        foreground.wait(timeout=5)
                    assert stopped.returncode == 0, stopped.stderr
            public('update generation 2', 'install', 'chatgpt', '--local', two)
            assert sha(profile) == profile_before, 'selection silently rewrote pinned profile'
            assert profile_binary.exists()
            public('rollback to admitted generation 1', 'rollback', 'chatgpt')
            status = public('rollback keeps high water', 'status', 'chatgpt').stdout
            assert b'release_sequence: 1' in status and b'high_water: 2' in status
            public('reselect admitted highest sequence', 'install', 'chatgpt', '--local', two)
            bad_root = packages / 'bad'
            def bad_copy():
                if bad_root.exists():
                    shutil.rmtree(bad_root)  # Only this invocation's private negative fixture.
                shutil.copytree(two, bad_root)
                return bad_root
            negatives = (
                ('changed asset', lambda p: (p / 'NOTICE.md').write_bytes(b'tampered')),
                ('extended inventory', lambda p: (p / 'extra').write_bytes(b'not admitted')),
                ('wrong executable mode', lambda p: (p / 'codex-chatgpt-provider').chmod(0o777)),
                ('truncated signature', lambda p: (p / 'payload.sig').write_bytes(b'x')),
                ('license symlink', lambda p: ((p / 'licenses/proot-LICENSE').unlink(), (p / 'licenses/proot-LICENSE').symlink_to(p / 'NOTICE.md'))),
                ('changed manifest', lambda p: (p / 'payload.manifest').write_text((p / 'payload.manifest').read_text().replace('release_sequence\t2', 'release_sequence\t3'))),
            )
            for label, mutate in negatives:
                before = selection.read_bytes()
                bad = bad_copy()
                mutate(bad)
                public('reject ' + label, 'install', 'chatgpt', '--local', bad, expected=1)
                assert selection.read_bytes() == before
            alternate_assets = case_root / 'collision-assets'
            shutil.copytree(source / 'providers/chatgpt/licenses', alternate_assets / 'licenses')
            for name in ('Cargo.lock', 'NOTICE.md'):
                shutil.copy2(source / 'providers/chatgpt' / name, alternate_assets / name)
            with (alternate_assets / 'NOTICE.md').open('a') as stream:
                stream.write('\nOwned negative fixture: identity collision.\n')
            collision = package('signed same-identity changed payload', 'chatgpt-g-test-2', 4,
                                assets=alternate_assets, output_name='collision')
            before = selection.read_bytes()
            rejected = public('reject signed identity collision', 'install', 'chatgpt', '--local', collision, expected=1)
            assert b'identity collision' in rejected.stderr
            assert selection.read_bytes() == before
            public('reject downgrade through install', 'install', 'chatgpt', '--local', one, expected=1)
            assert selection.read_bytes() == before
            other_key = case_root / 'other-key.pem'
            run('new private fixture authority', ['openssl', 'genpkey', '-algorithm', 'Ed25519', '-out', other_key])
            other_key.chmod(0o600)
            three = package('package with future fixture authority', 'chatgpt-g-test-3', 3, signing=other_key)
            public('reject unadmitted signer', 'install', 'chatgpt', '--local', three, expected=1)
            assert selection.read_bytes() == before
            # Simulate an already-admitted Core authority rotation in this fixture.
            # This qualifies Provider key retention, not Core authority rotation itself.
            der = run('read fixture future public key', ['openssl', 'pkey', '-in', other_key, '-pubout', '-outform', 'DER']).stdout
            core_text = core_state.read_text()
            core_state.write_text('\n'.join('update_key=' + der[-32:].hex() if line.startswith('update_key=') else line for line in core_text.splitlines()) + '\n')
            public('install newly admitted signer', 'install', 'chatgpt', '--local', three)
            public('rollback retains previous signer across authority change', 'rollback', 'chatgpt')
            core_state.write_text(core_text)
            assert sha(core_state) == core_before
            for label, content in [('malformed selection', b'bad\n'), ('zero sequence state', selection.read_bytes().replace(b'high_water\t3', b'high_water\t0'))]:
                before = selection.read_bytes()
                selection.write_bytes(content)
                public('reject ' + label, 'status', 'chatgpt', expected=1)
                run('ordinary Core ignores broken optional state', [launcher, '--version'])
                assert selection.read_bytes() == content
                selection.write_bytes(before)
            assert all(sha(p) == digest for p, digest in sentinels.items())
            assert sha(profile) == profile_before
            assert not (service / 'desktop').exists()
            run('ordinary native configuration retained', [launcher, 'features', 'list'])
            case.update(accepted=True, profile_preserved=True, native_defaults_preserved=True,
                        core_authority_restored=True, actual_Chat_requests=0,
                        selection_sha256=sha(selection), profile_sha256=sha(profile))
        assert {name: sha(getattr(args, name)) for name in ('core', 'frontend', 'provider', 'builder')} == {name: inputs[name] for name in ('core', 'frontend', 'provider', 'builder')}
        assert {name: sha(source / name) for name in inventory} == inventory
        assert {name: sha(source / name) for name in helpers} == inputs['helpers']
        report['accepted'] = True
    except Exception as error:
        report['failure'] = dict(type=type(error).__name__, detail=str(error), traceback=traceback.format_exc())
    if report['accepted'] and not args.keep_payloads:
        record = root / 'payload-cleanup.jsonl'
        try:
            report['payload_retention'] = dict(record=str(record), **fixture_cleanup(record, True, root))
        except Exception as error:
            report['payload_retention'] = dict(record=str(record), error=type(error).__name__)
    report.update(ended_UTC=utc(), seconds=time.monotonic() - start)
    (root / 'result.json').write_text(json.dumps(report, indent=2) + '\n')
    args.report.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({key: report.get(key) for key in ('accepted', 'root', 'seconds', 'failure')}))
    return 0 if report['accepted'] else 1


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('core', 'frontend', 'provider', 'builder', 'generation', 'report'):
        parser.add_argument('--' + name, type=Path, required=True)
    parser.add_argument('--keep-payloads', action='store_true')
    parser.add_argument('--remote', action='store_true', help='exercise real owned localhost TLS and remote rejection paths')
    args = parser.parse_args()
    for name in ('core', 'frontend', 'provider', 'builder', 'generation'):
        setattr(args, name, getattr(args, name).resolve(strict=True))
    if args.report.exists():
        parser.error('report already exists; inspect completed state instead of replaying it')
    raise SystemExit(qualify(args))
