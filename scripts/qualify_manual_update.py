"""Exercise the real manual updater in an owned Android installation.

Only transport/CI responses are local witnesses. Core still verifies the real
fixture signature and installed inventory. No public request or device change.
"""
import argparse
import codecs
import fcntl
import hashlib
import json
import os
from pathlib import Path
import pty
import select
import signal
import shutil
import struct
import subprocess
import sys
import tempfile
import termios
import time

from qualify_permission_lifecycle import fixture, server_paths
from qualification_diagnostics import terminal_contains
SOURCE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SOURCE / '.github/scripts'))


def provider_display_fixture(args, root, home, prefix, env):
    """Real package admission and index verification; only HTTP response bytes vary."""
    import hashlib
    from release_notes import render
    source = SOURCE
    authority = root / 'fixture-key.pem'
    installed = home / '.local/lib/codex/core/generations/local-permission-fixture'
    sequence = int(next(line.split('\t')[1] for line in (installed / 'release.manifest').read_text().splitlines() if line.startswith('release_sequence\t')))
    full = any(line.startswith('file\tmanager\t') for line in (installed / 'release.manifest').read_text().splitlines())
    component = root / 'component-input-v1'
    component.write_text(json.dumps({'schema': 'codex-component-input-v1',
        'generation_id': 'local-permission-fixture',
        'manifest_sha256': hashlib.sha256((installed / 'release.manifest').read_bytes()).hexdigest(),
        'display': {'release_date': '2026-10-27',
            'base_generation': 'local-base-display-owned' if full else None,
            'base_sequence': sequence - 1 if full else None}}))
    subprocess.run(['openssl', 'pkeyutl', '-sign', '-inkey', str(authority), '-rawin',
        '-in', str(component), '-out', str(root / 'component-input-v1.sig')], check=True, capture_output=True)
    if full:
        (root / 'base-publication.json').write_text(json.dumps({'tag_name': 'local-base-display-owned',
            'draft': False, 'published_at': '2026-10-26T02:00:00Z', 'body': render(source, 'base')}))
    packages = []
    for number in [7, 8]:
        package = root / ('provider-' + str(number))
        inputs = root / ('provider-inputs-' + str(number) + '.json')
        inputs.write_text('{"fixture":"owned update presentation"}\n')
        subprocess.run([str(args.builder), 'provider-package', '--binary', str(args.provider),
            '--assets', str(source / 'providers/chatgpt'), '--source-inputs', str(inputs),
            '--source-sha', 'a' * 40, '--generation-id', 'chatgpt-display-owned-' + str(number),
            '--release-sequence', str(number), '--private-key', str(authority),
            '--openssl', str(Path(os.environ['PREFIX']) / 'bin/openssl'), '--output', str(package)],
            check=True, capture_output=True, timeout=120)
        packages.append(package)
    admitted = subprocess.run([str(prefix / 'bin/codex'), 'provider', 'install', 'chatgpt',
        '--local', str(packages[0])], env=env, cwd=root / 'w', capture_output=True, timeout=45)
    (root / 'provider-install.stdout').write_bytes(admitted.stdout)
    (root / 'provider-install.stderr').write_bytes(admitted.stderr)
    assert admitted.returncode == 0, admitted.stderr.decode(errors='replace')
    index = root / 'provider-index'
    digest = hashlib.sha256((packages[1] / 'payload.manifest').read_bytes()).hexdigest()
    index.write_text('codex-provider-index-v1\nprovider\tchatgpt\nchannel\tstable\n'
        'generation_id\tchatgpt-display-owned-8\nrelease_sequence\t8\n'
        'release_base\thttps://github.com/humtr/codex/releases/download/chatgpt-display-owned-8/\n'
        'manifest_sha256\t' + digest + '\n')
    subprocess.run(['openssl', 'pkeyutl', '-sign', '-inkey', str(authority), '-rawin',
        '-in', str(index), '-out', str(root / 'provider-index.sig')], check=True, capture_output=True)
    for name, tag, timestamp, component in [
        ('core-publication', 'local-permission-fixture', '2026-10-27T12:30:00Z', 'full' if full else 'base'),
        ('provider-publication', 'chatgpt-display-owned-8', '2026-10-28T17:04:05Z', 'provider'),
    ]:
        (root / (name + '.json')).write_text(json.dumps({'tag_name': tag, 'draft': False,
            'published_at': timestamp, 'body': render(source, component)}))
    return home / '.local/share/codex/core/providers/chatgpt/selection'


def probe(args):
    root = Path(tempfile.mkdtemp(prefix='mu.', dir=os.environ['TMPDIR']))
    result = {'accepted': False, 'root': str(root), 'mode': args.mode,
              'publicRequests': 0, 'productionChanged': False}
    process = master = None
    transcript = ''
    owners = []
    home = fixture_key = None
    decoder = codecs.getincrementaldecoder('utf-8')(errors='replace')

    def receive_terminal():
        nonlocal transcript
        try:
            chunk = os.read(master, 65536)
        except OSError as error:
            if error.errno != 5:
                raise
            chunk = b''
        transcript += decoder.decode(chunk, final=not chunk)
        return bool(chunk)

    try:
        home, prefix, env = fixture(root, args.core, args.frontend, generation=args.generation)
        state = home / '.local/share/codex/core/activation-state'
        initial = state.read_bytes()
        fields = dict(line.split('\t', 1) for line in
                      (root / 'local-permission-fixture/generation.meta').read_text().splitlines()[1:])
        installed = fields['upstream_package_version']
        major, minor, patch = map(int, installed.split('.'))
        latest = installed if args.mode in ['current', 'display'] else f'{major}.{minor + 1}.{patch}'
        next_publication = next_identity = None
        payload_urls = {}
        for path in (root/'local-permission-fixture').rglob('*'):
            if path.is_file():
                relative=path.relative_to(root/'local-permission-fixture').as_posix()
                payload_urls['https://example.invalid/local-permission-fixture/'+relative]=str(path.relative_to(root))
        if args.refresh_current_archive:
            archive=args.refresh_current_archive.resolve(strict=True)
            with archive.open('rb') as stream:
                digest=hashlib.file_digest(stream,'sha256').hexdigest()
            (root/'current-upstream.json').write_text(json.dumps({'tag_name':'rust-v'+installed,
                'draft':False,'prerelease':False,'assets':[{'name':'codex-package-aarch64-unknown-linux-musl.tar.gz','digest':'sha256:'+digest}]}))
            base='https://releases.openai.com/codex/releases/'+installed+'/'
            payload_urls[base+'release.json']='current-upstream.json'
            payload_urls[base+'codex-package-aarch64-unknown-linux-musl.tar.gz']=str(archive)
        if args.mode == 'publication-arrives':
            from rald3_preflight import parse_descriptor
            next_fields = parse_descriptor(args.next_generation / 'generation.meta')
            assert next_fields['upstream_package_version'] == latest
            next_identity = next_fields['generation_id']
            current_sequence = int(next(line.split('\t')[1] for line in
                (root/'local-permission-fixture/release.manifest').read_text().splitlines()
                if line.startswith('release_sequence\t')))
            next_publication = root / 'next-publication'
            next_base = 'https://example.invalid/' + next_identity + '/'
            completed = subprocess.run([str(args.builder), 'publish', '--generation', str(args.next_generation),
                '--release-sequence', str(current_sequence + 1), '--release-base', next_base,
                '--private-key', str(root/'fixture-key.pem'), '--openssl', shutil.which('openssl'),
                '--output', str(next_publication)], capture_output=True)
            (root/'next-publication.stdout').write_bytes(completed.stdout)
            (root/'next-publication.stderr').write_bytes(completed.stderr)
            assert completed.returncode == 0, completed.stderr.decode(errors='replace')
            for path in (next_publication/'releases'/next_identity).rglob('*'):
                if path.is_file():
                    relative = path.relative_to(next_publication/'releases'/next_identity).as_posix()
                    payload_urls[next_base + relative] = str(path.relative_to(root))
        (root/'payload-urls.json').write_text(json.dumps(payload_urls))
        config_before = (home/'.codex/config.toml').read_bytes()
        (root / 'official.json').write_text(json.dumps({'tag_name': 'rust-v' + latest,
            'draft': False, 'prerelease': False, 'published_at': '2026-10-28T17:04:05Z',
            'body': '## New Features\n- Official fixture update.\n', 'assets': [
                {'name': 'codex-package-aarch64-unknown-linux-musl.tar.gz', 'digest': 'sha256:' + 'a' * 64}]}))
        if args.official_metadata:
            assert args.official_metadata.stat().st_size <= 1024 * 1024
            captured = args.official_metadata.read_bytes()
            assert json.loads(captured)['tag_name'] == 'rust-v' + latest, 'captured upstream version differs from this case'
            (root / 'official.json').write_bytes(captured)
        index = root / 'index'
        release_base = 'https://humtr.github.io/codex/local-permission-fixture/' if args.mode == 'display' else 'https://example.invalid/local-permission-fixture/'
        index.write_text('codex-update-index-v1\nchannel\tstable\ngeneration_id\tlocal-permission-fixture\n'
                         'release_base\t' + release_base + '\n')
        subprocess.run(['openssl', 'pkeyutl', '-sign', '-inkey', str(root / 'fixture-key.pem'),
                        '-rawin', '-in', str(index), '-out', str(root / 'index.sig')], check=True, capture_output=True)
        if args.mode == 'bad-signature':
            index.write_bytes(index.read_bytes() + b'unsigned change\n')
        selection = provider_display_fixture(args, root, home, prefix, env) if args.mode == 'display' else None
        selected_before = selection.read_bytes() if selection else None
        fixture_key = (root / 'fixture-key.pem').read_bytes()
        (root / 'fixture-key.pem').unlink()
        # The production HTTPS URLs remain selected: custom-index overrides would
        # intentionally disable manual public freshness and would miss this path.
        curl = prefix / 'bin/curl'
        curl.write_text('#!' + sys.executable + '\n' +
            'from pathlib import Path\nimport json,sys\nroot=Path(' + repr(str(root)) + ')\n' +
            'url=sys.argv[sys.argv.index("--url")+1]\n' +
            'with (root/"transports.jsonl").open("a") as f:f.write(json.dumps(url)+"\\n")\n' +
            ('if url.endswith(("/update-index-v1","/update-index-base-v1")):\n'
             ' reads=sum(json.loads(line).endswith(("/update-index-v1","/update-index-base-v1")) for line in (root/"transports.jsonl").read_text().splitlines())\n'
             ' if reads>=2:(root/"publication-visible").touch()\n' if args.mode=='publication-arrives' else '') +
            'if url=="https://api.github.com/repos/openai/codex/releases/latest":name="official.json"\n' +
            'elif url=="https://api.github.com/repos/humtr/codex/releases/tags/local-permission-fixture":name="core-publication.json"\n' +
            'elif url=="https://api.github.com/repos/humtr/codex/releases/tags/chatgpt-display-owned-8":name="provider-publication.json"\n' +
            'elif url=="https://api.github.com/repos/humtr/codex/releases/tags/local-base-display-owned":name="base-publication.json"\n' +
            'elif url.endswith("/component-input-v1"):name="component-input-v1"\n' +
            'elif url.endswith("/component-input-v1.sig"):name="component-input-v1.sig"\n' +
            'elif url.endswith("/provider-index-v1"):name="provider-index"\n' +
            'elif url.endswith("/provider-index-v1.sig"):name="provider-index.sig"\n' +
            'elif url.endswith("/update-index-base-v1") or url.endswith("/update-index-v1"):name="next-publication/update-index-v1" if (root/"publication-visible").exists() else "index"\n' +
            'elif url.endswith("/update-index-base-v1.sig") or url.endswith("/update-index-v1.sig"):name="next-publication/update-index-v1.sig" if (root/"publication-visible").exists() else "index.sig"\n' +
            'elif url in json.loads((root/"payload-urls.json").read_text()):name=json.loads((root/"payload-urls.json").read_text())[url]\n' +
            'else:sys.exit(22)\n' +
            'if not (root/name).is_file():sys.exit(22)\n' +
            'sys.stdout.buffer.write((root/name).read_bytes())\n')
        curl.chmod(0o700)
        gh = prefix / 'bin/gh'
        status_run={'head_sha':'a'*40,'head_branch':'rewrite/rust-core','event':'push',
                    'repository':{'full_name':'humtr/codex'},'status':'completed','conclusion':'failure'}
        core_run={**status_run,'id':99,'path':'.github/workflows/core-runtime-qualification.yml',
                  'status':'completed' if args.mode in ['qualification-failed','publication-arrives'] else 'in_progress',
                  'conclusion':'failure' if args.mode in ['qualification-failed','publication-arrives'] else None}
        (root/'qualification-runs.json').write_text(json.dumps({'workflow_runs':[
            {**core_run,'id':77,'head_sha':'b'*40},
            {**core_run,'id':78,'repository':{'full_name':'other/repository'}},
            {**core_run,'id':79,'head_branch':'other-branch'},
            {**status_run,'id':98,'path':'.github/workflows/rald7-full-acceptance.yml','conclusion':'success'},
            core_run,
        ]}))
        jq=shutil.which('jq')
        assert jq is not None,'qualification witness requires jq for the actual Core-supplied query'
        gh.write_text('#!' + sys.executable + '\nfrom pathlib import Path\nimport json,sys\n' +
            'root=Path(' + repr(str(root)) + ')\n' +
            'with (root/"github.jsonl").open("a") as f:f.write(json.dumps(sys.argv[1:])+"\\n")\n' +
            'if "POST" in sys.argv:\n' +
            (' sys.exit(1)\n' if args.mode == 'uncertain' else ' print("12345")\n') +
            'elif "/actions/runs/12345" in sys.argv[2]:print("in_progress\\tpending")\n' +
            'elif "/git/ref/heads/" in sys.argv[2]:print("a"*40)\n' +
            'elif "/contents/release/upstream.json" in sys.argv[2]:print(' + repr(latest) + ')\n' +
            'elif any(name in sys.argv[2] for name in ["rald7-full-acceptance.yml", "core-runtime-qualification.yml"]):\n' +
            ' import subprocess\n' +
            ' subprocess.run([' + repr(jq) + ', "-r", sys.argv[-1]],input=(root/"qualification-runs.json").read_bytes(),check=True)\n' +
            'else:print("0")\n')
        gh.chmod(0o700)
        if args.refresh_current_archive:
            # The source builder admits regular explicit tool files; ordinary
            # runtime-only fixtures use an OpenSSL symlink instead.
            openssl=prefix/'bin/openssl'
            assert openssl.is_symlink()
            openssl.unlink()
            shutil.copy2(shutil.which('openssl'),openssl)
            shutil.copy2(shutil.which('gzip'),prefix/'bin/gzip')
            refreshed=subprocess.run([str(prefix/'bin/codex'),'update','--build-local'],
                env={**env,'CODEX_TERMUX_UPDATE_VERSION':installed},cwd=root/'w',capture_output=True,timeout=300)
            (root/'local-refresh.stdout').write_bytes(refreshed.stdout)
            (root/'local-refresh.stderr').write_bytes(refreshed.stderr)
            assert refreshed.returncode==0,refreshed.stderr.decode(errors='replace')
            prior=dict(line.split('=',1) for line in initial.decode().splitlines() if '=' in line)
            initial=state.read_bytes()
            selected=dict(line.split('=',1) for line in initial.decode().splitlines() if '=' in line)
            assert selected['update_key']==prior['update_key']
            assert selected['current_key']!=selected['update_key']
            current=home/'.local/lib/codex/core/generations'/selected['current']
            from rald3_preflight import parse_descriptor
            assert parse_descriptor(current/'generation.meta')['upstream_package_version']==installed
            result['localRefreshPreservedUpstreamAndPublicAuthority']=True
        initial_current=dict(line.split('=',1) for line in initial.decode().splitlines() if '=' in line)['current']
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
                receive_terminal()
            if ('Waiting for signed publication.' in transcript and not key_sent
                    and args.mode != 'qualification-failed'
                    and not termios.tcgetattr(master)[3] & termios.ICANON):
                if args.mode == 'quit':
                    transports = [json.loads(line) for line in (root / 'transports.jsonl').read_text().splitlines()]
                    if sum(url.endswith(('/update-index-v1', '/update-index-base-v1')) for url in transports) < 2:
                        continue
                # Waiting must not retain the shared generation lease.
                lease = os.open(home / '.local/lib/codex/core', os.O_RDONLY)
                try:
                    fcntl.flock(lease, fcntl.LOCK_EX | fcntl.LOCK_NB)
                    fcntl.flock(lease, fcntl.LOCK_UN)
                finally:
                    os.close(lease)
                result['waitLeaseReleased'] = True
                if args.mode == 'publication-arrives':
                    result['signedPublicationIntroducedDuringWait']=True
                elif args.mode == 'interrupt':
                    os.kill(process.pid, signal.SIGTERM)
                else:
                    os.write(master, b'\x03' if args.mode == 'ctrl-c' else b'c' if args.mode == 'continue' else b'q')
                key_sent = True
            if (expected_tui and terminal_contains(transcript, 'OpenAI Codex (v' + installed + ')')
                    and terminal_contains(transcript, 'Ask Codex to do anything')
                    and server_paths(home)):
                break
            if process.poll() is not None:
                # Exit closes the producer; consume all already-queued PTY bytes.
                while select.select([master], [], [], 0)[0] and receive_terminal():
                    pass
                assert not expected_tui, transcript[-2000:]
                break
        else:
            raise AssertionError('manual updater did not settle within its fixture bound')
        calls = [json.loads(line) for line in (root / 'github.jsonl').read_text().splitlines()] if (root / 'github.jsonl').exists() else []
        dispatches = sum('POST' in call for call in calls)
        expected_dispatches = 0 if args.mode in ['ordinary', 'current', 'display', 'bad-signature'] else 1
        assert dispatches == expected_dispatches, calls
        if args.mode in ['bad-signature', 'uncertain', 'qualification-failed']:
            assert process.returncode == 1, transcript[-1200:]
        elif args.mode in ['interrupt', 'ctrl-c']:
            assert process.returncode in ([-15, 143] if args.mode == 'interrupt' else [-2, 130]), transcript[-1200:]
        elif not expected_tui:
            assert process.returncode == 0, transcript[-1200:]
        if args.mode == 'publication-arrives':
            selected = dict(line.split('=',1) for line in state.read_text().splitlines() if '=' in line)
            assert selected['current'] == next_identity, selected
            assert selected['previous'] == initial_current, selected
            generation = home/'.local/lib/codex/core/generations'/next_identity
            from rald3_preflight import parse_descriptor
            assert parse_descriptor(generation/'generation.meta')['upstream_package_version'] == latest
            assert (home/'.codex/config.toml').read_bytes() == config_before
            result.update(installedVersion=latest,originalConfigurationPreserved=True,
                          signedWaitDownloadedAndActivated=True,
                          signedReleasePrecededFailedNewRecipe=True)
        else:
            assert state.read_bytes() == initial, 'request/wait must preserve current selection'
        transports = [json.loads(line) for line in (root / 'transports.jsonl').read_text().splitlines()] if (root / 'transports.jsonl').exists() else []
        if args.mode == 'ordinary':
            assert transports == [
                'https://raw.githubusercontent.com/humtr/codex/main/update-index-v1',
                'https://raw.githubusercontent.com/humtr/codex/main/update-index-v1.sig',
            ], 'ordinary launch must perform only its existing signed-index discovery'
            result['ordinaryLaunchNoPublicationLookup'] = True
        elif args.mode == 'publication-arrives':
            assert next_base+'runtime' in transports, 'changed runtime was not acquired through the updater transport'
            if (args.next_generation/'helpers/2').is_file():
                assert next_base+'helpers/2' in transports, 'changed frontend was not acquired through the updater transport'
            for line in (generation/'release.manifest').read_text().splitlines():
                if line.startswith('file\t'):
                    _,name,digest,mode=line.split('\t')
                    path=generation/name
                    with path.open('rb') as stream:
                        assert hashlib.file_digest(stream,'sha256').hexdigest()==digest,name
                    assert path.stat().st_mode & 0o777 == int(mode,8),name
            result['candidatePayloadRequests']=sum(url in payload_urls for url in transports)
        elif args.mode != 'bad-signature':
            assert sum(url.endswith('/component-input-v1') for url in transports) == 1, 'optional release display was refetched for an unchanged candidate'
        if args.mode == 'quit':
            assert sum(url.endswith(('/update-index-v1', '/update-index-base-v1')) for url in transports) >= 2
            result['sameCandidateMetadataReadOnceAcrossBuildWait'] = True

        if args.mode in ['quit', 'interrupt', 'ctrl-c', 'qualification-failed']:
            assert termios.tcgetattr(master) == saved, 'terminal mode was not restored'
        if args.mode == 'qualification-failed':
            assert 'Termux qualification failed: core-runtime-qualification.yml (failure)' in transcript, transcript[-2000:]
            assert 'https://github.com/humtr/codex/actions/runs/99' in transcript, transcript[-2000:]
            assert not any('/actions/runs/'+str(run) in transcript for run in [77,78,79])
            result['requestAckDidNotHideQualificationFailure']=True
            result['foreignSourceRepositoryAndBranchRejected']=True
        if args.mode == 'display':
            assert 'Latest upstream Codex: ' + installed in transcript
            assert '2026-10-27 21:30:00 KST (UTC+09:00)' in transcript
            if fields['manager_artifact_digest'] != '-':
                component = json.loads((root / 'component-input-v1').read_text())['display']
                assert f"Core (Base #{component['base_sequence']}) · Published: 2026-10-26 11:00:00 KST (UTC+09:00)" in transcript
            assert 'Provider chatgpt: #7 -> latest #8 (separate Provider update)' in transcript
            assert '2026-10-29 02:04:05 KST (UTC+09:00)' in transcript
            assert 'Provider changes:' in transcript and 'Core changes:' in transcript
            assert 'Publish component-scoped update notes' in transcript
            assert selection.read_bytes() == selected_before
            installed_provider = home / '.local/lib/codex/providers/chatgpt/generations/chatgpt-display-owned-8'
            assert not installed_provider.exists(), 'information query installed a Provider'
            # A second real CLI invocation exercises the release pipeline's
            # Core-only oracle and proves an unselected Provider is not queried.
            backup = root / 'provider-selection-before-core-display'
            selection.rename(backup)
            try:
                before_queries = (root / 'transports.jsonl').read_text().count('/provider-index-v1')
                core_env = dict(env, CODEX_TERMUX_UPDATE_INDEX_URL='https://raw.githubusercontent.com/humtr/codex/main/update-index-v1')
                core_only = subprocess.run([str(prefix / 'bin/codex'), 'update'], env=core_env,
                    cwd=root / 'w', capture_output=True, timeout=45)
                actual = root / 'core-display.stdout'
                actual.write_bytes(core_only.stdout)
                (root / 'core-display.stderr').write_bytes(core_only.stderr)
                assert core_only.returncode == 0
                assert (root / 'transports.jsonl').read_text().count('/provider-index-v1') == before_queries
                plain = root / 'core-display.plain'
                plain.write_text('Codex ' + installed + ' is already up to date.\n')
                check = [sys.executable, str(SOURCE / '.github/scripts/check_update_display.py'),
                    '--plain', str(plain), '--actual', str(actual), '--component', str(root / 'component-input-v1'),
                    '--current', str(home / '.local/lib/codex/core/generations/local-permission-fixture/release.manifest'),
                    '--target', str(home / '.local/lib/codex/core/generations/local-permission-fixture/release.manifest'),
                    '--publication', str(root / 'core-publication.json')]
                if (root / 'base-publication.json').exists():
                    check += ['--base-publication', str(root / 'base-publication.json')]
                checked = subprocess.run(check, capture_output=True)
                (root / 'release-output-oracle.stdout').write_bytes(checked.stdout)
                (root / 'release-output-oracle.stderr').write_bytes(checked.stderr)
                assert checked.returncode == 0, checked.stderr.decode(errors='replace')
            finally:
                backup.rename(selection)
            (root / 'provider-index.sig').write_bytes(bytes(64))
            unavailable = subprocess.run([str(prefix / 'bin/codex'), 'update'], env=env,
                cwd=root / 'w', capture_output=True, timeout=45)
            (root / 'provider-unavailable.stdout').write_bytes(unavailable.stdout)
            (root / 'provider-unavailable.stderr').write_bytes(unavailable.stderr)
            assert unavailable.returncode == 0
            assert b'Provider chatgpt: latest information unavailable' in unavailable.stdout
            assert selection.read_bytes() == selected_before and state.read_bytes() == initial
            result.update(componentPublicationTimes=True, componentChangelog=True,
                unselectedProviderNotQueried=True, realReleaseOutputOracle=True,
                selectedAndLatestProviderDistinct=True, providerNotInstalledByCoreUpdate=True,
                invalidProviderIndexNonfatalAndSelectionPreserved=True)
        result.update(accepted=True, dispatches=dispatches, enteredInstalledTui=expected_tui,
                      activationPreserved=args.mode != 'publication-arrives', terminalExit=process.returncode)
        return result
    except BaseException as error:
        result['error'] = type(error).__name__ + ': ' + str(error)
        raise
    finally:
        for path in server_paths(home) if home else []:
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
        # Restore only the retired owned witness key so manifest-based fixture
        # cleanup can archive generated payloads after all test processes exit.
        if fixture_key is not None:
            (root / 'fixture-key.pem').write_bytes(fixture_key)
            (root / 'fixture-key.pem').chmod(0o600)
        (root / 'terminal.log').write_text(transcript)
        (root / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
        args.report.write_text(json.dumps(result, indent=2) + '\n')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--core', type=Path, required=True)
    parser.add_argument('--generation', type=Path, required=True)
    parser.add_argument('--frontend', type=Path)
    parser.add_argument('--mode', choices=['quit', 'continue', 'no-wait', 'current', 'display', 'ordinary', 'bad-signature', 'uncertain', 'interrupt', 'ctrl-c', 'qualification-failed', 'publication-arrives'], required=True)
    parser.add_argument('--report', type=Path, required=True)
    parser.add_argument('--provider', type=Path)
    parser.add_argument('--builder', type=Path)
    parser.add_argument('--next-generation', type=Path, help='unsigned qualified next-version generation for the publication-arrives trial')
    parser.add_argument('--refresh-current-archive', type=Path, help='official installed-version archive for an isolated local updater refresh before the manual trial')
    parser.add_argument('--official-metadata', type=Path, help='captured official response for the exact version expected by this case')
    args = parser.parse_args()
    if args.mode == 'display' and not all([args.provider, args.builder]):
        parser.error('display qualification requires --provider and --builder')
    if args.mode == 'publication-arrives' and not all([args.next_generation,args.builder]):
        parser.error('publication-arrives requires --next-generation and --builder')
    print(json.dumps(probe(args)))
