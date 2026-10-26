#!/usr/bin/env python3
"""Real Android signed optional-package lifecycle, without inference/activation."""
import argparse
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

                def complete_download(base, through_index=False):
                    prefix = stable_paths if through_index else []
                    expected = prefix + [base + name for name in flat_assets]
                    # Authentication precedes payload. Remaining assets may be
                    # fetched in any order, but every signed asset is read once.
                    assert remote_requests[:len(prefix) + 2] == expected[:len(prefix) + 2]
                    assert len(remote_requests) == len(expected) and set(remote_requests) == set(expected), remote_requests
                cert = case_root / 'remote-cert.pem'
                remote_key = case_root / 'remote-key.pem'
                run('create owned localhost TLS certificate', ['openssl', 'req', '-x509', '-newkey', 'rsa:2048', '-nodes', '-days', '1',
                    '-keyout', remote_key, '-out', cert, '-subj', '/CN=127.0.0.1', '-addext', 'subjectAltName=IP:127.0.0.1,DNS:raw.githubusercontent.com,DNS:github.com'])
                remote_key.chmod(0o600)
                shutil.copy2(cert, prefix / 'etc/tls/cert.pem')
                shutil.copy2(shutil.which('curl'), prefix / 'bin/curl')
                stable = dict(index=None, signature=None)
                class Handler(http.server.BaseHTTPRequestHandler):
                    def log_message(self, *_):
                        pass
                    def do_GET(self):
                        remote_requests.append(self.path)
                        if self.path.startswith('/humtr/codex/main/provider-index-v1'):
                            data = stable['signature' if self.path.endswith('.sig') else 'index']
                            self.send_response(200)
                            self.send_header('Content-Length', str(len(data)))
                            self.end_headers()
                            self.wfile.write(data)
                            return
                        if self.path.startswith('/humtr/codex/releases/download/chatgpt-g-test-1/'):
                            route, name = 'one', self.path.rsplit('/', 1)[1]
                        else:
                            route, name = self.path.strip('/').split('/', 1)
                        if route == 'redirect':
                            self.send_response(302)
                            self.send_header('Location', 'http://127.0.0.1:9/forbidden')
                            self.end_headers()
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
                    + repr('--connect-to') + ',' + repr(f'github.com:443:127.0.0.1:{server.server_port}') + ']+sys.argv[1:])\n')
                wrapper.chmod(0o755)
                index = case_root / 'provider-index-v1'
                index_sig = case_root / 'provider-index-v1.sig'
                index.write_text('codex-provider-index-v1\nprovider\tchatgpt\nchannel\tstable\ngeneration_id\tchatgpt-g-test-1\nrelease_sequence\t1\nrelease_base\thttps://github.com/humtr/codex/releases/download/chatgpt-g-test-1/\nmanifest_sha256\t' + sha(one / 'payload.manifest') + '\n')
                run('sign owned stable index', ['openssl', 'pkeyutl', '-sign', '-rawin', '-inkey', key, '-in', index, '-out', index_sig])
                stable.update(index=index.read_bytes(), signature=index_sig.read_bytes())
                try:
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
                    remote_requests.clear()
                    public('stable index authenticated update over real TLS', 'update', 'chatgpt')
                    complete_download('/humtr/codex/releases/download/chatgpt-g-test-1/', through_index=True)
                    remote_requests.clear()
                    public('stable index default install over real TLS', 'install', 'chatgpt')
                    complete_download('/humtr/codex/releases/download/chatgpt-g-test-1/', through_index=True)
                    saved_signature = stable['signature']
                    stable['signature'] = b'x' * 64
                    before = (home / '.local/share/codex/core/providers/chatgpt/selection').read_bytes()
                    remote_requests.clear()
                    public('reject unverified stable index before payload', 'update', 'chatgpt', expected=1)
                    assert remote_requests == stable_paths
                    assert (home / '.local/share/codex/core/providers/chatgpt/selection').read_bytes() == before
                    stable['signature'] = saved_signature
                    case['remote'] = dict(real_TLS=True, requests_per_complete_install=len(flat_assets), signature_before_payload=True,
                                          no_implicit_retry=True, local_certificate_only=True)
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
            try:
                public('live lease blocks update', 'install', 'chatgpt', '--local', two, expected=1)
                assert selection.read_bytes() == original_selection
                public('read status during live service', 'status', 'chatgpt')
                public('confirmed owned service stop', 'chatgpt', 'stop', '--state', service)
            finally:
                # Stop only the isolated service through its original immutable owner.
                status = subprocess.run([str(profile_binary), 'status', '--state', str(service)], env=env, capture_output=True, timeout=15)
                if status.returncode == 0 and json.loads(status.stdout).get('process'):
                    subprocess.run([str(profile_binary), 'stop', '--state', str(service)], env=env, capture_output=True, timeout=15, check=True)
            # Pinned profile executables bypass Core dispatch; /proc must still fence them.
            run('direct pinned executable prepare', [profile_binary, 'prepare', '--home', home / '.codex', '--state', service])
            try:
                rejected = public('direct immutable process blocks update', 'install', 'chatgpt', '--local', two, expected=1)
                assert b'process is running' in rejected.stderr
                assert selection.read_bytes() == original_selection
            finally:
                run('stop directly bound owner', [profile_binary, 'stop', '--state', service])
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
