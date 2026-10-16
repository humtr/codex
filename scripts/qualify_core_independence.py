#!/usr/bin/env python3
"""Actual Android Core-only lifecycle in disposable roots; no account or inference.

Uses the production builder and installer phases with a fresh fixture signing key.
This is not a base-only public release mode or a replacement publication validator.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile


def sha(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def qualify(args, report):
    tools = {name: shutil.which(name) for name in ('openssl', 'gzip')}
    assert all(tools.values()), 'OpenSSL and gzip are required'
    report['inputs'] = {name: sha(getattr(args, name))
                        for name in ('core', 'builder', 'archive', 'manager')
                        if getattr(args, name) is not None}
    assert report['inputs']['archive'] == args.archive_sha256
    report['version'] = args.version
    with tempfile.TemporaryDirectory(prefix='ci-', dir=os.environ['TMPDIR']) as temporary:
        root = Path(temporary)
        report['fixtureRoot'] = str(root)
        home, prefix = root / 'h', root / 'p'
        for path in (home / '.codex', prefix / 'bin', prefix / 'etc/tls', root / 'work'):
            path.mkdir(parents=True, mode=0o700)
        host_prefix = Path(os.environ['PREFIX'])
        for relative in ('etc/resolv.conf', 'etc/tls/cert.pem'):
            shutil.copy2(host_prefix / relative, prefix / relative)
        (prefix / 'bin/openssl').symlink_to(tools['openssl'])
        env = dict(HOME=str(home), PREFIX=str(prefix), TMPDIR=str(root),
                   PATH=f'{prefix / "bin"}:{host_prefix / "bin"}',
                   SSL_CERT_FILE=str(prefix / 'etc/tls/cert.pem'), TERM='dumb')

        def run(label, command, *, selected=env, expected=0):
            result = subprocess.run([str(value) for value in command], env=selected,
                                    cwd=root / 'work', capture_output=True, timeout=180)
            report['steps'].append({'name': label, 'exitCode': result.returncode})
            assert result.returncode == expected, (
                label, result.returncode, result.stdout[-2048:], result.stderr[-2048:])
            return result

        key, public = root / 'key.pem', root / 'public.pem'
        run('fixture signing key', [tools['openssl'], 'genpkey', '-algorithm', 'Ed25519', '-out', key])
        key.chmod(0o600)
        run('fixture public key', [tools['openssl'], 'pkey', '-in', key, '-pubout', '-out', public])

        def generation(identity, sequence, manager=False):
            unsigned, published = root / f'{identity}-unsigned', root / f'{identity}-published'
            command = [args.builder, 'build', '--version', args.version,
                       '--archive', args.archive, '--archive-sha256', args.archive_sha256,
                       '--generation-id', identity, '--core', args.core,
                       '--creation-metadata', 'core-independence-fixture',
                       '--gzip', tools['gzip'], '--openssl', tools['openssl'], '--output', unsigned]
            if manager:
                command += ['--manager', args.manager]
            run(f'build {identity}', command)
            run(f'sign {identity}', [args.builder, 'publish', '--generation', unsigned,
                '--release-sequence', str(sequence), '--release-base', f'https://humtr.github.io/codex/{identity}/',
                '--private-key', key, '--openssl', tools['openssl'], '--output', published])
            shutil.rmtree(unsigned)
            result = published / 'releases' / identity
            assert not (result / 'helpers/2').exists()
            assert (result / 'manager').exists() == manager
            return result

        first = generation('local-core-independent-a', 1)
        second = generation('local-core-independent-b', 2)
        managed = None
        if args.manager is not None:
            managed = generation('local-core-independent-manager', 3, manager=True)
            assert sha(managed / 'manager') == report['inputs']['manager']
            report['managerReusedWithoutCompilation'] = True

        for mode in ('activate', 'publish-bootstrap-pin', 'publish-fresh-entrypoint'):
            selected = dict(env, CODEX_TERMUX_INTERNAL_BOOTSTRAP=mode,
                CODEX_TERMUX_INTERNAL_BOOTSTRAP_KEY=str(public),
                CODEX_TERMUX_INTERNAL_BOOTSTRAP_CORE=str(first / 'core'))
            if mode == 'activate':
                selected['CODEX_TERMUX_INTERNAL_BOOTSTRAP_SOURCE'] = str(first)
            run(f'installer {mode}', [args.core], selected=selected)

        launcher = prefix / 'bin/codex'
        state_path = home / '.local/share/codex/core/activation-state'
        generations = home / '.local/lib/codex/core/generations'

        def state():
            return dict(line.split('=', 1) for line in state_path.read_text().splitlines() if '=' in line)

        def current(identity):
            assert state()['current'] == identity
            assert sha(launcher) == report['inputs']['core']
            result = run(f'{identity} upstream version', [launcher, '--version'])
            assert result.stdout.strip() == f'codex-cli {args.version}'.encode()

        sentinel = home / '.codex/owned-preservation-sentinel'
        sentinel.write_bytes(b'owned fixture conversation/config data\n')
        sentinel_hash = sha(sentinel)
        current(first.name)
        run('Core-only exec command surface', [launcher, 'exec', '--help'])
        run('Core-only resume command surface', [launcher, 'resume', '--help'])
        absent = run('Manager absence is explicit', [launcher, 'termux', '--help'], expected=1)
        assert b'Manager is unavailable' in absent.stderr
        run('unsupported sandbox refuses', [launcher, '-s', 'workspace-write', '--version'], expected=2)
        run('Core-only signed update', [launcher, 'update', '--local', second])
        current(second.name)
        run('Core-only rollback', [launcher, 'update', '--rollback'])
        current(first.name)
        run('Core-only forward retained rollback', [launcher, 'update', '--rollback'])
        current(second.name)

        if managed is not None:
            manager_bytes = (managed / 'manager').read_bytes()
            (managed / 'manager').unlink()
            before = state_path.read_bytes()
            run('incomplete declared optional candidate rejects',
                [launcher, 'update', '--local', managed], expected=1)
            assert state_path.read_bytes() == before
            assert not (generations / managed.name).exists()
            (managed / 'manager').write_bytes(manager_bytes)
            (managed / 'manager').chmod(0o755)
            run('update reuses admitted Manager', [launcher, 'update', '--local', managed])
            current(managed.name)
            installed_manager = generations / managed.name / 'manager'
            assert sha(installed_manager) == report['inputs']['manager']
            installed_manager.unlink()  # Only this owned fixture's optional asset.
            current(managed.name)
            run('rollback with installed Manager unavailable', [launcher, 'update', '--rollback'])
            current(second.name)
            run('retained installed optional absence remains Core-usable', [launcher, 'update', '--rollback'])
            current(managed.name)
        assert sha(sentinel) == sentinel_hash
        report['userStatePreserved'] = True
        report['accepted'] = True
    report['fixtureRemoved'] = not Path(report['fixtureRoot']).exists()
    report['realChatSubmissions'] = 0
    report['publicPublication'] = False
    report['deviceActivation'] = False


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('core', 'builder', 'archive', 'report'):
        parser.add_argument('--' + name, type=lambda value: Path(value).resolve(), required=True)
    parser.add_argument('--manager', type=lambda value: Path(value).resolve(),
                        help='Also prove unchanged Manager reuse and optional-asset failure boundaries')
    parser.add_argument('--archive-sha256', required=True)
    parser.add_argument('--version', required=True)
    args = parser.parse_args()
    report = {'accepted': False, 'steps': []}
    try:
        qualify(args, report)
    finally:
        if 'fixtureRoot' in report:
            report['fixtureRemoved'] = not Path(report['fixtureRoot']).exists()
        args.report.write_text(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
