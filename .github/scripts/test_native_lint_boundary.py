"""The real hosted lint allowance cannot become an upstream source modification."""
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import textwrap
import unittest

WORKFLOW = Path(__file__).resolve().parents[1] / 'workflows/manager-tui-native.yml'


class NativeLintBoundaryTests(unittest.TestCase):
    def test_actual_lint_step_restores_pristine_upstream_sources_on_every_outcome(self):
        block = WORKFLOW.read_text().split('      - name: Inspect TUI lint fixes', 1)[1].split('      - name:', 1)[0]
        script = textwrap.dedent(block.split('        run: |\n', 1)[1])
        self.assertEqual(subprocess.run(['bash', '-n'], input=script, text=True, capture_output=True).returncode, 0)
        git = shutil.which('git')
        for fault in (None, 'failure', 'root-mutation', 'other-core-mutation', 'config-mutation', 'foreign-source',
                      'daemon-mutation', 'foreign-daemon'):
            with self.subTest(fault=fault), tempfile.TemporaryDirectory(prefix='native-lint-') as directory:
                root = Path(directory)
                upstream = root / 'upstream'
                upstream.mkdir()
                core = upstream / 'codex-rs/core/src/lib.rs'
                core.parent.mkdir(parents=True)
                original = b'//! Original Core\n\npub fn original() {}\n'
                core.write_bytes(original)
                other = core.parent / 'other.rs'
                other.write_bytes(b'original\n')
                config = core.parent / 'config/mod.rs'
                config.parent.mkdir()
                config.write_bytes(b'// Original configuration\n')
                (upstream / 'codex-rs/tui').mkdir()
                (upstream / 'codex-rs/tui/original').write_bytes(b'original\n')
                daemon = upstream / 'codex-rs/tui/src/daemon_startup.rs'
                daemon.parent.mkdir()
                daemon.write_bytes(b'pub fn uses_wsl_drvfs() {}\n')
                for args in (['init', '-q'], ['add', '.'], ['-c', 'user.name=Owned', '-c', 'user.email=owned@example.invalid', 'commit', '-qm', 'owned']):
                    subprocess.run([git, *args], cwd=upstream, check=True, capture_output=True)
                # The assembled frontend owns a bounded Config change. Lint
                # must retain that exact input while rejecting new mutations.
                config_before = b'// Complete model catalog source retained\n'
                config.write_bytes(config_before)
                if fault == 'foreign-source':
                    core.write_bytes(b'foreign\n')
                if fault == 'foreign-daemon':
                    daemon.write_bytes(b'foreign\n')
                before = core.read_bytes()
                daemon_before = daemon.read_bytes()
                tools = root / 'bin'
                tools.mkdir()
                (tools / 'just').write_text(f'#!{sys.executable}\n' + textwrap.dedent('''\
                    import os, sys
                    from pathlib import Path
                    core = Path('codex-rs/core/src/lib.rs')
                    assert core.read_bytes().startswith(b'#![recursion_limit = "256"]\\n')
                    daemon = Path('codex-rs/tui/src/daemon_startup.rs')
                    assert daemon.read_bytes().startswith(b'#[allow(clippy::unnecessary_cast)]\\n')
                    assert sys.argv[1:] == ['fix', '--locked', '--target', 'aarch64-unknown-linux-musl', '-p', 'codex-tui']
                    Path('../lint-invoked').write_text('yes')
                    fault = os.environ.get('LINT_FAULT')
                    if fault == 'root-mutation': core.write_bytes(b'changed')
                    if fault == 'daemon-mutation': daemon.write_bytes(b'changed')
                    if fault == 'other-core-mutation': Path('codex-rs/core/src/other.rs').write_bytes(b'changed')
                    if fault == 'config-mutation': Path('codex-rs/core/src/config/mod.rs').write_bytes(b'changed')
                    sys.exit(7 if fault == 'failure' else 0)
                '''))
                (tools / 'just').chmod(0o755)
                (root / 'profile-tui-formatted.patch').write_bytes(subprocess.check_output(
                    [git, 'diff', '--', 'codex-rs/tui', 'codex-rs/core'], cwd=upstream))
                env = {**os.environ, 'PATH': str(tools)+os.pathsep+os.environ['PATH'],
                       'RUNNER_TEMP': str(root), 'LINT_FAULT': fault or '', 'PYTHONDONTWRITEBYTECODE': '1'}
                result = subprocess.run(['bash', '-c', script], cwd=root, env=env,
                                        text=True, capture_output=True, timeout=15)
                self.assertEqual(result.returncode == 0, fault is None, result.stderr[-1000:])
                self.assertEqual(core.read_bytes(), before)
                self.assertEqual(daemon.read_bytes(), daemon_before)
                self.assertEqual((root / 'lint-invoked').exists(), fault not in ('foreign-source', 'foreign-daemon'))
                if fault != 'other-core-mutation':
                    self.assertEqual(other.read_bytes(), b'original\n')
                if fault != 'config-mutation':
                    self.assertEqual(config.read_bytes(), config_before)


if __name__ == '__main__':
    unittest.main()
