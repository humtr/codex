"""Owned actual preview Core bridge, rollback and public argv forwarding."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'scripts'))
from qualify_tasks import fixture_activation_state, materialize_signed_generation

STABLE = '8acc8219507780095a3a03e0cd0f8bf4b6346bbe7bd4b41b0a8f761851978cfa'

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def qualify(core, manager, generation, public_key, parent):
    results = []
    for case in ['rollback', 'forward-help', 'bad-backup', 'changed-launcher', 'bad-generation', 'bridge']:
        with tempfile.TemporaryDirectory(prefix='pr', dir=parent) as temporary:
            root = Path(temporary); home = root/'h'; prefix = root/'p'
            identity = dict(line.split('\t', 1) for line in (generation/'generation.meta').read_text().splitlines()[1:])['generation_id']
            installed = home/'.local/lib/codex/core/generations'/identity
            materialize_signed_generation(generation, installed)
            for directory in [home/'.codex', prefix/'bin', home/'.local/share/codex/core/config']:
                directory.mkdir(parents=True, exist_ok=True, mode=0o700)
            assets = home/'.local/lib/codex/profile-tui-preview/424e0950'
            assets.mkdir(parents=True, mode=0o700)
            shutil.copy2(generation/'core', assets/'stable-core')
            shutil.copy2(manager, assets/'manager'); os.chmod(assets/'manager', 0o755)
            launcher = prefix/'bin/codex'; shutil.copy2(core, launcher)
            alternate = root/'alternate-core'; shutil.copy2(core, alternate)
            os.symlink(shutil.which('openssl'), prefix/'bin/openssl')
            state = home/'.local/share/codex/core/activation-state'
            state.write_text(fixture_activation_state(identity, public_key))
            env = {'HOME': str(home), 'PREFIX': str(prefix), 'PATH': str(Path(shutil.which('sh')).parent), 'TMPDIR': str(parent)}
            if case == 'bad-backup': (assets/'stable-core').write_bytes(b'corrupt')
            if case == 'changed-launcher': launcher.write_bytes(b'changed-launcher')
            if case == 'bad-generation': (installed/'core').write_bytes(b'corrupt')
            before_state = state.read_bytes(); before_launcher = digest(launcher)
            before = {str(p.relative_to(installed)): digest(p) for p in installed.rglob('*') if p.is_file()}
            command = alternate if case == 'changed-launcher' else launcher
            args = ['update', '--help'] if case == 'forward-help' else ['update', '--rollback']
            if case == 'bridge': args = ['termux', '__profile-snapshot-v1']
            result = subprocess.run([str(command), *args], env=env, cwd=root, capture_output=True, timeout=30)
            assert state.read_bytes() == before_state, 'activation state changed'
            assert before == {str(p.relative_to(installed)): digest(p) for p in installed.rglob('*') if p.is_file()}, 'signed generation changed'
            assert not (home/'.codex/auth.json').exists()
            if case in ['rollback', 'forward-help']:
                assert result.returncode == 0, 'restore/forward failed'
                assert digest(launcher) == STABLE, 'wrong restored launcher'
                if case == 'rollback': assert result.stdout == b'Restored Codex release 40; activation state is unchanged.\n'
                else: assert b'codex update' in result.stdout
            elif case == 'bridge':
                assert result.returncode == 0, 'read-only bridge failed'
                assert json.loads(result.stdout) == {'schema':'codex-manager-profiles-v1','profiles':['default'],'current':'default','current_source':'default','saved_default':'default'}
                assert digest(launcher) == before_launcher
                extra = subprocess.run([str(command), *args, 'extra'], env=env, cwd=root, capture_output=True, timeout=15)
                assert extra.returncode == 2, 'snapshot accepted extra argv'
                restricted = subprocess.run([str(command), '--sandbox', 'read-only'], env=env, cwd=root, capture_output=True, timeout=15)
                assert restricted.returncode == 2, 'explicit unsupported sandbox was accepted'
                assert state.read_bytes() == before_state and digest(launcher) == before_launcher
            else:
                assert result.returncode == 1, 'unsafe restore accepted'
                assert digest(launcher) == before_launcher, 'failed restore replaced launcher'
            results.append(case)
    return {'preview_core_sha256':digest(core), 'manager_sha256':digest(manager), 'proof':results, 'activation_and_generations_unchanged':True}

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    for name in ['core', 'manager', 'generation', 'public-key', 'parent']: parser.add_argument('--'+name, type=Path, required=True)
    a = parser.parse_args()
    print(json.dumps(qualify(a.core.resolve(), a.manager.resolve(), a.generation.resolve(), a.public_key.resolve(), a.parent.resolve()), sort_keys=True))
