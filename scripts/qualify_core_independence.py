#!/usr/bin/env python3
"""Actual Android Core-only lifecycle in disposable roots; no account or inference.

Uses the production builder and installer phases with a fresh fixture signing key.
This is not a base-only public release mode or a replacement publication validator.
"""
import argparse
import contextlib
import fcntl
import pty
import select
import struct
import termios
import hashlib
import json
import os
from pathlib import Path
import shutil
import shlex
import signal
import time
import subprocess
import tempfile


def sha(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def qualify(args, report):
    tools = {name: shutil.which(name) for name in ('openssl', 'gzip')}
    assert all(tools.values()), 'OpenSSL and gzip are required'
    report['inputs'] = {name: sha(getattr(args, name))
                        for name in ('core', 'builder', 'archive', 'manager', 'frontend', 'source_runtime')
                        if getattr(args, name) is not None}
    if args.source_runtime is not None:
        report['sourceRevision'] = args.source_revision
    report['driverSha256'] = sha(Path(__file__).resolve())
    assert report['inputs']['archive'] == args.archive_sha256
    report['version'] = args.version
    with tempfile.TemporaryDirectory(prefix='ci-', dir=os.environ['TMPDIR']) as temporary, contextlib.ExitStack() as cleanup:
        root = Path(temporary)
        report['fixtureRoot'] = str(root)
        home, prefix = root / 'h', root / 'p'
        for path in (home / '.codex', prefix / 'bin', prefix / 'etc/tls', root / 'work'):
            path.mkdir(parents=True, mode=0o700)
        host_prefix = Path(os.environ['PREFIX'])
        for relative in ('etc/resolv.conf', 'etc/tls/cert.pem'):
            shutil.copy2(host_prefix / relative, prefix / relative)
        (prefix / 'bin/openssl').symlink_to(tools['openssl'])
        env = dict(HOME=str(home), PREFIX=str(prefix), TMPDIR=str(root.parent),
                   PATH=f'{prefix / "bin"}:{host_prefix / "bin"}',
                   SSL_CERT_FILE=str(prefix / 'etc/tls/cert.pem'), TERM='dumb')

        def run(label, command, *, selected=env, expected=0, timeout=180):
            with subprocess.Popen([str(value) for value in command], env=selected,
                    cwd=root / 'work', stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                    start_new_session=True) as child:
                try:
                    stdout, stderr = child.communicate(timeout=timeout)
                except subprocess.TimeoutExpired:
                    try:
                        os.killpg(child.pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass
                    stdout, stderr = child.communicate()
                    report['steps'].append({'name': label, 'exitCode': child.returncode,
                        'expectedExit': expected, 'qualificationTimeoutSeconds': timeout})
                    raise subprocess.TimeoutExpired(child.args, timeout, output=stdout, stderr=stderr)
                result = subprocess.CompletedProcess(child.args, child.returncode, stdout, stderr)
            report['steps'].append({'name': label, 'exitCode': result.returncode,
                                    'expectedExit': expected})
            assert result.returncode == expected, (
                label, result.returncode, result.stdout[-2048:], result.stderr[-2048:])
            return result

        key, public = root / 'key.pem', root / 'public.pem'
        run('fixture signing key', [tools['openssl'], 'genpkey', '-algorithm', 'Ed25519', '-out', key])
        key.chmod(0o600)
        run('fixture public key', [tools['openssl'], 'pkey', '-in', key, '-pubout', '-out', public])

        def generation(identity, sequence, manager=None, frontend=None):
            unsigned, published = root / f'{identity}-unsigned', root / f'{identity}-published'
            command = [args.builder, 'build', '--version', args.version,
                       '--archive', args.archive, '--archive-sha256', args.archive_sha256,
                       '--generation-id', identity, '--core', args.core,
                       '--creation-metadata', 'core-independence-fixture',
                       '--gzip', tools['gzip'], '--openssl', tools['openssl'], '--output', unsigned]
            if manager:
                command += ['--manager', manager]
            if frontend:
                command += ['--manager-tui', frontend, '--manager-tui-version', args.version]
            if args.source_runtime is not None:
                command += ['--source-runtime', args.source_runtime,
                            '--source-revision', args.source_revision]
            run(f'build {identity}', command)
            run(f'sign {identity}', [args.builder, 'publish', '--generation', unsigned,
                '--release-sequence', str(sequence), '--release-base', f'https://humtr.github.io/codex/{identity}/',
                '--private-key', key, '--openssl', tools['openssl'], '--output', published])
            shutil.rmtree(unsigned)
            result = published / 'releases' / identity
            assert (result / 'helpers/2').exists() == bool(frontend)
            assert (result / 'manager').exists() == bool(manager)
            return result

        first = generation('local-core-independent-a', 1)
        second = generation('local-core-independent-b', 2)
        managed = None
        if args.manager is not None:
            managed = generation('local-core-independent-manager', 3, manager=args.manager)
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
            # Exercise the actual direct flag with an admitted, working Manager.
            # Version alone is not upstream config validation: features list also
            # loads the real runtime configuration, without inference or login.
            run('create owned account-selection fixture',
                [launcher, 'termux', 'profile', 'create', 'core-probe'])
            run('select owned account-selection fixture',
                [launcher, 'termux', 'profile', 'default', 'core-probe'])
            preference = home / '.local/share/codex/manager/default-profile-v1'
            preference_bytes = preference.read_bytes()
            core_config = home / '.local/share/codex/core/config/config.toml'
            direct = dict(env, CODEX_TERMUX_CORE_ONLY='1')
            for label, selected in [('managed before', env), ('direct', direct),
                                    ('managed after', env)]:
                run(f'{label} real configuration load',
                    [launcher, 'features', 'list'], selected=selected)
                config_bytes = core_config.read_bytes()
                assert b'approvals_reviewer' not in config_bytes
                snapshots = core_config.parent / 'snapshots'
                assert any((b'approvals_reviewer = "auto_review"' in (item / 'config.toml').read_bytes())
                           == (selected is env) for item in snapshots.iterdir())
                assert preference.read_bytes() == preference_bytes
            # A broken explicit choice must remain an error in the normal path.
            # Direct operation uses the caller's native home and does not repair
            # or silently replace the Manager's preference.
            preference.write_bytes(b'owned invalid profile preference\n')
            preference.chmod(0o600)
            run('invalid explicit account blocks normal execution',
                [launcher, 'features', 'list'], expected=1)
            run('direct execution ignores invalid optional account preference',
                [launcher, 'features', 'list'], selected=direct)
            assert preference.read_bytes() == b'owned invalid profile preference\n'
            preference.write_bytes(preference_bytes)
            run('restored account selection resumes normal execution',
                [launcher, 'features', 'list'])
            run('reset owned account-selection fixture',
                [launcher, 'termux', 'profile', 'default', 'default'])
            # Same home/runtime, different optional settings: query actual native
            # RPC and the live process's FD34, not just files the launcher wrote.
            from qualify_tasks import Ws
            observed = []
            started = set()

            def live(label, selected, optional):
                result = run(label, [launcher, '__app-server-default-v1'], selected=selected)
                socket = Path(result.stdout.decode().strip())
                record = socket.parent
                pid = int((record / 'pid').read_text())
                proc = Path('/proc') / str(pid)
                executable = (proc / 'exe').resolve(strict=True)
                identity = (proc / 'stat').read_text().rsplit(')', 1)[1].split()[19]
                assert executable == generations / state()['current'] / 'runtime'
                if pid not in started:
                    started.add(pid)
                    def stop_owned():
                        try:
                            assert (proc / 'exe').resolve(strict=True) == executable
                            assert (proc / 'stat').read_text().rsplit(')', 1)[1].split()[19] == identity
                        except (FileNotFoundError, AssertionError):
                            return
                        os.kill(pid, signal.SIGTERM)
                        until = time.monotonic() + 5
                        while time.monotonic() < until and (proc / 'exe').exists():
                            time.sleep(.02)
                        if (proc / 'exe').exists():
                            assert (proc / 'stat').read_text().rsplit(')', 1)[1].split()[19] == identity
                            os.kill(pid, signal.SIGKILL)
                    cleanup.callback(stop_owned)
                config = (proc / 'fd/34/config.toml').read_bytes()
                assert config == (record / 'config/config.toml').read_bytes()
                assert (b'approvals_reviewer = "auto_review"' in config) == optional
                client = Ws(socket)
                try:
                    settings = client.call('config/read', {'includeLayers': False})['config']
                    assert settings['sandbox_mode'] == 'danger-full-access'
                    assert settings.get('approvals_reviewer') in (['auto_review'] if optional else [None, 'user'])
                finally:
                    client.close()
                observed.append((pid, config))
                return pid

            first_pid = live('normal live settings', env, True)
            direct_pid = live('direct live sibling settings', direct, False)
            assert first_pid != direct_pid
            assert live('normal reuses unchanged live sibling', env, True) == first_pid
            assert observed[0] == observed[2]
            assert (Path('/proc') / str(first_pid) / 'fd/34/config.toml').read_bytes() == observed[0][1]
            assert b'approvals_reviewer' not in core_config.read_bytes()
            report['liveSiblingSettings'] = {
                'sameHome': True, 'separateSettingsServers': True,
                'originalServerReused': True, 'nativeConfigRpc': True,
                'liveFd34Unchanged': True,
            }
            report['directExecution'] = {
                'admittedManager': True, 'realUpstreamConfigLoad': True,
                'normalDirectNormal': True, 'invalidPreferencePreserved': True,
                'liveSiblingIsolationProven': True, 'tuiExercised': False,
            }
            installed_manager.unlink()  # Only this owned fixture's optional asset.
            current(managed.name)
            run('rollback with installed Manager unavailable', [launcher, 'update', '--rollback'])
            current(second.name)
            run('retained installed optional absence remains Core-usable', [launcher, 'update', '--rollback'])
            current(managed.name)
            # Independently compiled admitted optional artifact: malformed policy
            # reaches the real upstream loader without a production fault hook.
            rustc = shutil.which('rustc')
            assert rustc, 'Rust is required to compile the owned optional-artifact fixture'
            source, fixture_manager = root / 'optional-fixture.rs', root / 'optional-fixture'
            source.write_text(r'''fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--artifact-probe") => print!("codex-manager-artifact-v1\ncore_api=codex-manager-core-v1\n"),
        Some("__prepare-launch-v1") => print!("codex-manager-launch-v1\ncurrent\n"),
        Some("__prepare-runtime-v1") => {
            match std::env::var("OWNED_CORE_OPTIONAL_FIXTURE").as_deref() {
                Ok("syntax") => print!("codex-manager-runtime-config-v1\n[unfinished\n"),
                Ok("type") => print!("codex-manager-runtime-config-v1\napproval_policy = 42\n"),
                Ok("envelope") => print!("unrecognized-config-v1\n"),
                Ok("hung") => std::thread::sleep(std::time::Duration::from_secs(30)),
                _ => std::process::exit(3),
            }
        }
        _ => std::process::exit(2),
    }
}
''')
            run('compile owned optional-artifact fixture',
                [rustc, '--edition=2021', source, '-o', fixture_manager])
            faulty = generation('local-core-independent-optional-faults', 4, manager=fixture_manager)
            run('admit owned optional-artifact fixture', [launcher, 'update', '--local', faulty])
            for mode in ('syntax', 'type', 'envelope', 'hung'):
                selected = dict(env, OWNED_CORE_OPTIONAL_FIXTURE=mode)
                result = run(f'{mode} optional settings preserve real Core execution',
                             [launcher, 'features', 'list'], selected=selected)
                assert b'optional Manager' in result.stderr
                assert b'approvals_reviewer' not in core_config.read_bytes()
            report['optionalFaultIsolation'] = {
                'realUpstreamTomlSyntax': True, 'realUpstreamFieldTypes': True,
                'unknownEnvelope': True, 'boundedHungArtifact': True,
                'noWorkloadReplay': True,
            }
            run('rollback optional-artifact fixture', [launcher, 'update', '--rollback'])
            current(managed.name)
            if args.frontend is not None:
                independent = generation('local-core-independent-tui', 5, frontend=args.frontend)
                run('install TUI without Manager', [launcher, 'update', '--local', independent])
                current(independent.name)
                assert not (generations / independent.name / 'manager').exists()
                config = home / '.codex/config.toml'
                config.write_text('model="fixture"\nmodel_provider="fixture"\n'
                    '[model_providers.fixture]\nname="Owned fixture"\n'
                    'base_url="http://127.0.0.1:9/v1"\nwire_api="responses"\n'
                    'requires_openai_auth=false\n'
                    f'[projects.{json.dumps(str(root / "work"))}]\ntrust_level="trusted"\n')
                master, slave = pty.openpty()
                fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 40, 130, 0, 0))
                process = subprocess.Popen([launcher, '--no-alt-screen'],
                    env=dict(env, TERM='xterm-256color'), cwd=root / 'work',
                    stdin=slave, stdout=slave, stderr=slave, start_new_session=True)
                os.close(slave)
                os.set_blocking(master, False)
                screen = ''
                def wait_screen(text):
                    nonlocal screen
                    until = time.monotonic() + 45
                    while time.monotonic() < until:
                        if select.select([master], [], [], .05)[0]:
                            try:
                                screen += os.read(master, 262144).decode(errors='replace')
                            except (OSError, BlockingIOError):
                                pass
                        if text in screen:
                            return
                        assert process.poll() is None, screen[-4000:]
                    raise AssertionError('owned TUI timeout: ' + screen[-4000:])
                try:
                    wait_screen('Ask Codex to do anything')
                    for byte in b'/switch':
                        os.write(master, bytes([byte])); time.sleep(.08)
                    os.write(master, b'\r')
                    wait_screen('Profiles are unavailable.')
                    assert process.poll() is None
                    report['tuiWithoutManager'] = {'nativeUiLoaded': True,
                        'optionalProfilesUnavailable': True, 'nativeUiRemainsLive': True,
                        'noInferenceSubmitted': True}
                finally:
                    process.terminate()
                    try:
                        process.wait(timeout=8)
                    except subprocess.TimeoutExpired:
                        process.kill(); process.wait()
                    os.close(master)
                live('TUI-only Core server settings', env, False)
        if args.qualify_local_signing:
            # The earlier rollback trials may leave a legitimate hold. Admit a
            # newer signed baseline through Core instead of editing that state.
            sequence = 1 + max(
                int(line.split('\t')[1])
                for manifest in generations.glob('*/release.manifest')
                for line in manifest.read_text().splitlines()
                if line.startswith('release_sequence\t'))
            baseline = generation('local-core-independent-signing', sequence)
            run('admit newer signed baseline for local signing',
                [launcher, 'update', '--local', baseline])
            current(baseline.name)
            # Exercise the release executable and real official HTTPS acquisition;
            # the wrapper only records argv before execing the native curl.
            curl = shutil.which('curl')
            assert curl, 'Native curl is required for local-build qualification'
            requests = root / 'local-build-curl-requests'
            archive_url = ('https://releases.openai.com/codex/releases/'
                           f'{args.version}/codex-package-aarch64-unknown-linux-musl.tar.gz')
            replacement = root / 'replacement-core'
            replaced_image = root / 'replaced-updater-image'
            # Inject an atomic pathname replacement during real acquisition.
            # Preserve the admitted bytes; only the kernel process-image lifetime
            # changes. An input selected through current_exe now names a deleted
            # inode, while the immutable signed generation stays available.
            (prefix / 'bin/curl').write_text(
                f'#!{host_prefix}/bin/sh\n'
                f'printf \'%s\\n\' "$@" >> {shlex.quote(str(requests))}\n'
                f'for arg do if [ "$arg" = {shlex.quote(archive_url)} ]; then\n'
                f'  {shlex.quote(shutil.which("cp"))} -p {shlex.quote(str(launcher))} {shlex.quote(str(replacement))} &&\n'
                f'  {shlex.quote(shutil.which("mv"))} {shlex.quote(str(replacement))} {shlex.quote(str(launcher))} || exit 96\n'
                f'  {shlex.quote(shutil.which("readlink"))} /proc/$PPID/exe > {shlex.quote(str(replaced_image))} || exit 97\n'
                'fi; done\n'
                f'exec {shlex.quote(curl)} "$@"\n')
            (prefix / 'bin/curl').chmod(0o755)
            # Core accepts this earlier fixture symlink, while the builder's
            # external-tool admission requires the regular executable itself.
            (prefix / 'bin/openssl').unlink()
            shutil.copy2(tools['openssl'], prefix / 'bin/openssl')
            shutil.copy2(tools['gzip'], prefix / 'bin/gzip')
            reference = home / '.local/share/codex/core/config/build-signing-key'
            reference.parent.mkdir(parents=True, exist_ok=True)
            unapproved = root / 'unapproved-local-key.pem'
            run('create unapproved owned signer', [tools['openssl'], 'genpkey',
                '-algorithm', 'Ed25519', '-out', unapproved])
            unapproved.chmod(0o600)
            before = state()
            selected = dict(env, CODEX_TERMUX_UPDATE_VERSION=args.version)
            reference.write_text(f'file:{unapproved}\n')
            rejected = run('unapproved configured signer refuses before archive',
                [launcher, 'update', '--build-local'], selected=selected, expected=1)
            assert b'not authorized by the installed update key' in rejected.stderr, (
                rejected.stdout[-2048:], rejected.stderr[-2048:])
            assert state() == before
            assert archive_url not in requests.read_text().splitlines()
            assert not replaced_image.exists()
            key_digest = sha(key)
            reference_text = f'file:{key}\n'
            reference.write_text(reference_text)
            result = run('real official acquisition and configured local signing',
                [launcher, 'update', '--build-local'], selected=selected,
                # Product archive acquisition has a300s bound; retain the usual
                #180s construction/probe allowance after observing that result.
                timeout=300 + 180)
            assert b'Local build signing: configured release authority' in result.stdout, result.stdout[-2048:]
            assert replaced_image.read_text().strip() == str(launcher) + ' (deleted)'
            after = state()
            assert after['current'].startswith('local-')
            assert after['previous'] == before['current']
            assert after['current_key'] == after['update_key'] == before['update_key']
            assert sha(key) == key_digest and reference.read_text() == reference_text
            assert requests.read_text().splitlines().count(archive_url) == 1
            current(after['current'])
            run('configured local signing rollback', [launcher, 'update', '--rollback'])
            current(before['current'])
            run('configured local signing retained forward rollback',
                [launcher, 'update', '--rollback'])
            current(after['current'])
            assert sha(key) == key_digest and reference.read_text() == reference_text
            report['configuredLocalSigning'] = {
                'actualReleaseExecutable': True, 'realOfficialHttpsAcquisition': True,
                'archiveUrl': archive_url, 'archiveDigest': args.archive_sha256,
                'unapprovedSignerRefusedBeforeArchive': True,
                'configuredAuthorityMatchesAdmittedUpdateKey': True,
                'externalKeyAndReferencePreserved': True,
                'rollbackAndRetainedForwardPassed': True, 'publicUpload': False,
                'atomicEntrypointReplacementDuringAcquisition': True,
                'runningImageObservedDeleted': True,
                'admittedImmutableCoreReused': True,
            }
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
    parser.add_argument('--frontend', type=lambda value: Path(value).resolve(),
                        help='Qualified raw frontend for actual TUI-without-Manager execution')
    parser.add_argument('--source-runtime', type=lambda value: Path(value).resolve(),
                        help='Compiled runtime passed through the production source-runtime admission path')
    parser.add_argument('--source-revision', help='Exact official revision of --source-runtime')
    parser.add_argument('--qualify-local-signing', action='store_true',
                        help='Also exercise configured signing through --build-local and real official HTTPS')
    parser.add_argument('--archive-sha256', required=True)
    parser.add_argument('--version', required=True)
    args = parser.parse_args()
    if (args.source_runtime is None) != (args.source_revision is None):
        parser.error('--source-runtime and --source-revision are required together')
    report = {'accepted': False, 'steps': [], 'realChatSubmissions': 0,
              'publicPublication': False, 'deviceActivation': False}
    try:
        qualify(args, report)
    finally:
        if 'fixtureRoot' in report:
            report['fixtureRemoved'] = not Path(report['fixtureRoot']).exists()
        args.report.write_text(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
