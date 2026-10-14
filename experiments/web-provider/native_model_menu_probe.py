"""Real Android Core/frontend /model round trip in disposable, unauthenticated state.

No inference is submitted, including Pro. A private fixture signer admits only
this disposable generation; no production launcher, key or activation is changed.
"""
import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import pty
import re
import select
import shutil
import signal
import struct
import subprocess
import sys
import tempfile
import termios
import time

SOURCE = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(SOURCE / 'scripts'))
from qualify_tasks import fixture_activation_state, materialize_signed_generation
from rald3_preflight import parse_descriptor
from rald5_publication import parse_manifest


def sha(path):
    with path.open('rb') as f:
        return hashlib.file_digest(f, 'sha256').hexdigest()


def fixture(root, core, frontend, named_permission=False):
    real_home = Path.home()
    activation = dict(line.split('=', 1) for line in
        (real_home / '.local/share/codex/core/activation-state').read_text().splitlines() if '=' in line)
    installed = real_home / '.local/lib/codex/core/generations' / activation['current']
    fields = parse_descriptor(installed / 'generation.meta')
    _, inventory = parse_manifest(installed / 'release.manifest')
    assert fields['upstream_package_version'] == '0.161.0'
    assert all(sha(installed / name) == digest for name, digest, _ in inventory)
    generation = root / 'local-chat-model-fixture'
    materialize_signed_generation(installed, generation)
    shutil.copy2(core, generation / 'core')
    raw = frontend.read_bytes()
    changed = 0
    for old, new, count in [
        (b'/etc/resolv.conf', b'/proc/self/fd/33', 2),
        (b'/etc/codex/config.toml', b'/dev/fd/34/config.toml', 1),
        (b'/etc/codex/requirements.toml', b'/dev/fd/34/requirements.toml', 1),
        (b'/etc/codex/managed_config.toml', b'/dev/fd/34/managed_config.toml', 1),
    ]:
        assert len(old) == len(new) and raw.count(old) == count and not raw.count(new)
        changed += sum(a != b for a, b in zip(old, new)) * count
        raw = raw.replace(old, new)
    assert changed == 54
    (generation / 'helpers/2').write_bytes(raw)
    (generation / 'helpers/2').chmod(0o755)
    text = (generation / 'generation.meta').read_text()
    text = text.replace(fields['generation_id'], generation.name)
    lines = []
    for line in text.splitlines():
        if line.startswith('core_artifact_digest\t'):
            line = 'core_artifact_digest\t' + sha(generation / 'core')
        elif line.startswith('helper\ttermux-manager-tui-v1:'):
            line = 'helper\ttermux-manager-tui-v1:0.161.0\t' + sha(generation / 'helpers/2')
        lines.append(line)
    (generation / 'generation.meta').write_text('\n'.join(lines) + '\n')
    key, public = root / 'fixture-key.pem', root / 'fixture-public.pem'
    subprocess.run(['openssl', 'genpkey', '-algorithm', 'Ed25519', '-out', str(key)], check=True, capture_output=True)
    key.chmod(0o600)
    subprocess.run(['openssl', 'pkey', '-in', str(key), '-pubout', '-out', str(public)], check=True, capture_output=True)
    der = subprocess.check_output(['openssl', 'pkey', '-pubin', '-in', str(public), '-outform', 'DER'])
    manifest = (generation / 'release.manifest').read_text().replace(fields['generation_id'], generation.name)
    manifest = '\n'.join(
        'release_public_key\t' + der[-32:].hex() if line.startswith('release_public_key\t') else
        '\t'.join(['file', line.split('\t')[1], sha(generation / line.split('\t')[1]), line.split('\t')[3]])
        if line.startswith('file\t') else line for line in manifest.splitlines()) + '\n'
    (generation / 'release.manifest').write_text(manifest)
    subprocess.run(['openssl', 'pkeyutl', '-sign', '-inkey', str(key), '-rawin', '-in', str(generation / 'release.manifest'), '-out', str(generation / 'release.sig')], check=True, capture_output=True)
    home, prefix = root / 'h', root / 'p'
    for path in [home / '.codex', home / '.local/share/codex/core/config', prefix / 'bin', prefix / 'etc/tls', root / 't', root / 'w']:
        path.mkdir(parents=True, mode=0o700)
    materialize_signed_generation(generation, home / '.local/lib/codex/core/generations' / generation.name)
    shutil.copy2(core, prefix / 'bin/codex')
    (home / '.local/share/codex/core/activation-state').write_text(fixture_activation_state(generation.name, public))
    host_prefix = Path(os.environ['PREFIX'])
    shutil.copy2(host_prefix / 'etc/resolv.conf', prefix / 'etc/resolv.conf')
    shutil.copy2(host_prefix / 'etc/tls/cert.pem', prefix / 'etc/tls/cert.pem')
    (prefix / 'bin/openssl').symlink_to(shutil.which('openssl'))
    trial = real_home / '.local/share/codex-web-experiment/native-provider'
    native = json.loads((trial / 'models.json').read_text())['models'][1].copy()
    native.update(slug='gpt-6.1-sol', display_name='Owned Codex', visibility='list', description='Account-free return choice')
    catalog = {'models': [native]}
    for name in ['native-models.json', 'models_cache.json']:
        (home / '.codex' / name).write_text(json.dumps(catalog))
        (home / '.codex' / name).chmod(0o600)
    (home / '.codex/config.toml').write_text(
        'model = "gpt-6.1-sol"\nmodel_reasoning_effort = "none"\n'
        'model_catalog_json = ' + json.dumps(str(home / '.codex/native-models.json')) + '\n'
        'approval_policy = "never"\napprovals_reviewer = "user"\nsandbox_mode = "danger-full-access"\n'
        '[features]\nmemories = false\n[model_providers.openai]\n'
        'base_url = "http://127.0.0.1:9/v1"\nrequires_openai_auth = false\n')
    (home / '.codex/config.toml').chmod(0o600)
    if named_permission:
        (home / '.codex/owned-config.config.toml').write_text(
            'default_permissions = "owned"\n[permissions.owned.filesystem]\n":root" = "write"\n'
            '[permissions.owned.network]\nenabled = true\n')
        (home / '.codex/owned-config.config.toml').chmod(0o600)
    selected_home = Path(os.environ.get('CODEX_HOME', real_home / '.codex'))
    profile = (selected_home / 'chatgpt.config.toml').read_text()
    assert profile.startswith('# codex-web-experiment ChatGPT profile begin\n')
    (home / '.codex/chatgpt.config.toml').write_text(profile)
    (home / '.codex/chatgpt.config.toml').chmod(0o600)
    env = dict(os.environ, HOME=str(home), CODEX_HOME=str(home / '.codex'), PREFIX=str(prefix),
        TMPDIR=str(root / 't'), PATH=str(prefix / 'bin') + ':' + str(host_prefix / 'bin'), TERM='xterm-256color')
    for name in ['OPENAI_API_KEY', 'CODEX_ACCESS_TOKEN', 'ACCESS_TOKEN', 'CODEX_SQLITE_HOME',
        'CODEX_PROFILE_CORE', 'CODEX_CHAT_RETURN_PROFILE', 'CODEX_EXEC_SERVER_URL',
        'TERMUX_SESSION_ID', 'CODEX_TERMUX_WINDOW_TOKEN', 'CODEX_TERMUX_WINDOW_READY', 'CODEX_TERMUX_SESSION_ORIGIN']:
        env.pop(name, None)
    return home, prefix, env


def probe(core, frontend, named_permission=False):
    with tempfile.TemporaryDirectory(prefix='chat-model-menu-', dir=os.environ['TMPDIR']) as tmp:
        root = Path(tmp)
        home, prefix, env = fixture(root, core, frontend, named_permission)
        preserved = {p: sha(p) for p in (home / '.codex').glob('*.toml')}
        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 40, 130, 0, 0))
        def session():
            os.setsid()
            fcntl.ioctl(slave, termios.TIOCSCTTY, 0)
        launch = ['--profile', 'owned-config'] if named_permission else ['--no-daemon']
        process = subprocess.Popen([str(prefix / 'bin/codex'), *launch, '--no-alt-screen'],
            env=env, cwd=root / 'w', stdin=slave, stdout=slave, stderr=slave, preexec_fn=session)
        os.close(slave)
        def collect(seconds):
            output = b''
            deadline = time.monotonic() + seconds
            while time.monotonic() < deadline and process.poll() is None:
                if select.select([master], [], [], .1)[0]:
                    try:
                        chunk = os.read(master, 65536)
                    except OSError:
                        break
                    for query, response in [(b'\x1b[6n', b'\x1b[1;1R'), (b'\x1b[c', b'\x1b[?1;2c'),
                        (b'\x1b]10;?', b'\x1b]10;rgb:ffff/ffff/ffff\x1b\\'), (b'\x1b]11;?', b'\x1b]11;rgb:0000/0000/0000\x1b\\')]:
                        if query in chunk:
                            os.write(master, response)
                    output = (output + chunk)[-131072:]
            return re.sub(r'\x1b\[[0-?]*[ -/]*[@-~]', '', output.decode(errors='replace'))
        def model_menu():
            for byte in b'/model':
                os.write(master, bytes([byte]));time.sleep(.07)
            os.write(master, b'\r')
            return collect(3)
        try:
            initial = collect(8)
            if 'trust' in initial.lower():
                os.write(master, b'\r');initial += collect(4)
            before = model_menu()
            assert 'ChatGPT' in before and 'GPT-6' in before, 'chat_choices_absent'
            os.write(master, b'\x1b[H\x1b[B\r');reasoning = collect(2)
            assert 'Instant' in reasoning, 'chat_reasoning_absent'
            os.write(master, b'\x1b[H\r');entered = collect(15)
            assert process.poll() is None, 'chat_reentry_failed'
            argv = Path(f'/proc/{process.pid}/cmdline').read_bytes().split(b'\0')
            assert b'chatgpt-desktop/gpt-6' in argv and b'model_provider="chatgpt_desktop_trial"' in argv, 'chat_reentry_failed'
            permission = b'default_permissions="owned"' if named_permission else b'sandbox_mode="danger-full-access"'
            assert permission in argv and b'approvals_reviewer="user"' in argv and b'approval_policy="never"' in argv, 'permission_selection_changed'
            assert Path(f'/proc/{process.pid}/exe').resolve().name == '2', 'qualified_frontend_lost'
            files = list((home / '.codex/sessions').rglob('*.jsonl'))
            assert len(files) == 1, 'same_thread_not_preserved'
            thread_file = files[0]
            returning = model_menu()
            assert 'Owned Codex' in returning, 'native_return_choice_absent'
            os.write(master, b'\x1b[H\r');native_reasoning = collect(2)
            assert 'Instant' in native_reasoning, 'native_reasoning_absent'
            os.write(master, b'\x1b[H\r');returned = collect(15)
            assert process.poll() is None, 'native_reentry_failed'
            assert b'gpt-6.1-sol' in Path(f'/proc/{process.pid}/cmdline').read_bytes().split(b'\0'), 'native_reentry_failed'
            assert list((home / '.codex/sessions').rglob('*.jsonl')) == [thread_file], 'return_thread_changed'
            state = dict(part.split(b'=', 1) for part in Path(f'/proc/{process.pid}/environ').read_bytes().split(b'\0') if b'=' in part)
            assert state[b'CODEX_HOME'] == os.fsencode(home / '.codex')
            assert b'CODEX_CHAT_RETURN_PROFILE' not in state
            assert all(sha(p) == digest for p, digest in preserved.items()), 'configuration_files_changed'
            return {'samePidAndPty': True, 'chatEntered': True, 'nativeReturned': True,
                'sameHomeAndThread': True, 'qualifiedFrontendRetained': True,
                'originalConfigurationPreserved': True, 'namedPermissionRequested': named_permission,
                'noAuthCopied': True, 'modelInferenceSubmitted': False, 'proInference': False, 'productionChanged': False}
        finally:
            if process.poll() is None:
                os.killpg(process.pid, signal.SIGINT)
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    os.killpg(process.pid, signal.SIGTERM);process.wait(timeout=5)
            os.close(master)


if __name__ == '__main__':
    os.umask(0o077)
    parser = argparse.ArgumentParser()
    parser.add_argument('--core', type=Path, required=True)
    parser.add_argument('--frontend', type=Path, required=True)
    parser.add_argument('--report', type=Path, required=True)
    parser.add_argument('--named-permission', action='store_true')
    args = parser.parse_args()
    try:
        report = probe(args.core, args.frontend, args.named_permission)
    except Exception as error:
        report = {'accepted': False, 'failureClass': type(error).__name__,
            'failureCode': str(error) if re.fullmatch('[a-z_]+', str(error)) else 'unclassified'}
    else:
        report['accepted'] = True
    from linux_desktop_trial import write
    write(args.report, report)
    print(json.dumps(report))
    raise SystemExit(0 if report['accepted'] else 1)
