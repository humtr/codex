"""Owned real Core/native/Manager foreground binding, with Android dispatch captured."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shlex
import shutil
import signal
import subprocess
import sys
import tempfile
import time
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'scripts'))
from qualify_tasks import fixture_activation_state, materialize_signed_generation
from qualify_native_profile_display import preview_assets

UUID = re.compile(r'[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}')


def qualify(binary, core, manager, generation, public_key, ai_source, parent, generation_mode=False):
    sys.path.insert(0, str(ai_source / 'lib'))
    import ai_tmux
    with tempfile.TemporaryDirectory(prefix='nt', dir=parent) as temporary:
        root = Path(temporary)
        home, prefix = root / 'h', root / 'p'
        identity = next(line.split('\t', 1)[1] for line in (generation / 'generation.meta').read_text().splitlines() if line.startswith('generation_id\t'))
        stable = home / '.local/lib/codex/core/generations' / identity
        if generation_mode:
            shutil.copytree(generation, stable)
        else:
            materialize_signed_generation(generation, stable)
        for path in [home / '.codex', prefix / 'bin', prefix / 'etc/tls', home / '.local/share/codex/core/config']:
            path.mkdir(parents=True, exist_ok=True, mode=0o700)
        if generation_mode:
            frontend = stable / 'helpers/2'
            assert frontend.read_bytes() == binary.read_bytes()
            assert (stable / 'manager').read_bytes() == manager.read_bytes()
        else:
            assets = preview_assets(home)
            assets.mkdir(parents=True, mode=0o700)
            for source, name in [(binary, 'native'), (manager, 'manager'), (generation / 'core', 'stable-core')]:
                shutil.copy2(source, assets / name)
                (assets / name).chmod(0o755)
            frontend = assets / 'native'
        installed = prefix / 'bin/codex'
        shutil.copy2(core, installed)
        os.symlink(shutil.which('openssl'), prefix / 'bin/openssl')
        (prefix / 'etc/resolv.conf').write_text('nameserver 127.0.0.1\n')
        (prefix / 'etc/tls/cert.pem').write_text('owned\n')
        state = home / '.local/share/codex/core/activation-state'
        state.write_text(fixture_activation_state(identity, public_key))
        config = home / '.codex/config.toml'
        config.write_text(f'''model = "fixture-model"
model_provider = "fixture"
check_for_update_on_startup = false
[tui]
screen_reader_detection_done = true
show_tooltips = false
terminal_title = ["project-name"]
[analytics]
enabled = false
[feedback]
enabled = false
[projects."{root}"]
trust_level = "trusted"
[features]
memories = false
[model_providers.fixture]
name = "Owned fixture"
base_url = "http://127.0.0.1:9/v1"
wire_api = "responses"
requires_openai_auth = false
''')
        config.chmod(0o600)
        protected = {path: path.read_bytes() for path in [state, config]}
        real_bin = Path(shutil.which('sh')).parent
        env = {'HOME': str(home), 'PREFIX': str(prefix), 'TMPDIR': str(parent),
               'PATH': str(prefix / 'bin') + os.pathsep + str(real_bin), 'TERM': 'xterm-256color',
               'SSL_CERT_FILE': str(real_bin.parent / 'etc/tls/cert.pem')}
        socket = str(root / 'tmux.sock')
        tmux_config = root / 'tmux.conf'
        tmux_config.write_text('set-option -g remain-on-exit on\nset-option -g default-shell '
                               + shlex.quote(shutil.which('sh')) + '\n')
        def tm(*args):
            result = subprocess.run(['tmux', '-S', socket, *args], env=env, text=True, capture_output=True, timeout=5)
            assert result.returncode == 0, result.stderr
            return result.stdout.rstrip('\n')
        pane = tm('-f', str(tmux_config), 'new-session', '-d', '-P', '-F', '#{pane_id}', '-s', 'owned', '-x', '180', '-y', '40', '-c', str(root), 'exec ' + shlex.join([str(installed), '--no-alt-screen']))
        def screen():
            return tm('capture-pane', '-p', '-t', pane)
        def await_value(reader, accept, label):
            deadline = time.monotonic() + 20
            last = None
            while time.monotonic() < deadline:
                last = reader()
                if accept(last): return last
                time.sleep(.05)
            raise AssertionError(label + ': ' + repr(last))
        def send(text):
            tm('send-keys', '-t', pane, '-l', text)
            # A fast synthetic burst is upstream paste input, not typed slash+Enter.
            time.sleep(.3)
            tm('send-keys', '-t', pane, 'Enter')
        requests = []
        try:
            await_value(screen, lambda text: 'fixture-model' in text, 'native ready')
            pid = tm('display-message', '-p', '-t', pane, '#{pane_pid}')
            assert Path(f'/proc/{pid}/exe').resolve() == frontend, 'public Core did not execute qualified native'
            tty = tm('display-message', '-p', '-t', pane, '#{pane_tty}')
            binding = await_value(lambda: tm('show-options', '-pqv', '-t', pane, '@codex_terminal_binding_v1'), bool, 'native-to-Core-to-Manager registration')
            with patch.dict(os.environ, env, clear=True):
                ai_tmux.register(socket)
                def current(): return ai_tmux._bound_thread(binding, pid, tty)
                first = await_value(current, lambda value: value is not None, 'first foreground full UUID')
                assert UUID.fullmatch(first)
                send('/status')
                await_value(screen, lambda text: first in text and 'Token usage:' in text, 'actual first /status')
                def dispatch(args, **kwargs):
                    requests.append(args)
                    return subprocess.CompletedProcess(args, 0, '', '')
                real_run = subprocess.run
                def run(args, **kwargs):
                    return dispatch(args, **kwargs) if args[0] == '/owned/am' else real_run(args, **kwargs)
                with patch.object(ai_tmux, '_activity_manager', return_value='/owned/am'), patch.object(subprocess, 'run', side_effect=run):
                    assert ai_tmux.focus(first) == 0
                    # The production frontend has no ID in its title; registration owns identity.
                    assert first[:29] not in tm('display-message', '-p', '-t', pane, '#{pane_title}')
                    send('/title')
                    await_value(screen, lambda text: 'terminal title' in text.lower(), 'actual /title picker')
                    tm('send-keys', '-t', pane, 'Escape')
                    assert current() == first and ai_tmux.focus(first) == 0
                    send('/new')
                    second = await_value(current, lambda value: value is not None and value != first, 'actual /new foreground switch')
                    assert tm('show-options', '-pqv', '-t', pane, '@codex_terminal_binding_v1') == binding, 'thread change must not require re-registration'
                    assert ai_tmux.focus(first) == 1 and ai_tmux.focus(second) == 0
                    send('/status')
                    await_value(screen, lambda text: second in text and 'Token usage:' in text, 'actual second /status')
                    before = tm('list-panes', '-a', '-F', '#{pane_id}:#{pane_pid}')
                    assert ai_tmux.focus(second) == 0
                    assert tm('list-panes', '-a', '-F', '#{pane_id}:#{pane_pid}') == before
                    assert len(requests) == 4 and requests[0] == requests[-1]
            assert all(path.read_bytes() == value for path, value in protected.items()), 'unrequested configuration/state change'
            assert not list(home.rglob('auth.json')), 'owned test wrote authentication'
            return {'native_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
                    'manager_sha256': hashlib.sha256(manager.read_bytes()).hexdigest(),
                    'core_sha256': hashlib.sha256(core.read_bytes()).hexdigest(),
                    'proof': ['real-Core/native/Manager-private-route', 'no-title-ID', 'actual-/title-picker',
                              'actual-/new/current-UUID/old-UUID-refusal', 'same-PID/pane/binding/repeated-return',
                              'Android-dispatch-captured', 'config/state/no-auth-preserved']}
        except Exception:
            (parent / 'native-terminal-owned-failure.txt').write_text(screen())
            raise
        finally:
            subprocess.run(['tmux', '-S', socket, 'kill-server'], env=env, capture_output=True)
            for record in (home / '.local/share/codex/core/servers').glob('*/pid'):
                owned_pid = int(record.read_text())
                try:
                    if Path(f'/proc/{owned_pid}/exe').resolve() != stable / 'runtime': continue
                    os.kill(owned_pid, signal.SIGTERM)
                    deadline = time.monotonic() + 5
                    while Path(f'/proc/{owned_pid}/exe').exists() and time.monotonic() < deadline: time.sleep(.05)
                    if Path(f'/proc/{owned_pid}/exe').resolve() == stable / 'runtime': os.kill(owned_pid, signal.SIGKILL)
                except (FileNotFoundError, ProcessLookupError): pass


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    for name in ['binary', 'core', 'manager', 'generation', 'public-key', 'ai-source', 'parent']:
        parser.add_argument('--' + name, required=True, type=Path)
    parser.add_argument('--generation-mode', action='store_true')
    args = parser.parse_args()
    print(json.dumps(qualify(args.binary.resolve(), args.core.resolve(), args.manager.resolve(), args.generation.resolve(), args.public_key.resolve(), args.ai_source.resolve(), args.parent.resolve(), args.generation_mode), sort_keys=True))
