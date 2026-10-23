"""The real hosted lint allowance cannot become an upstream source modification."""
import os
import importlib.util
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
        for fault in (None, 'failure', 'root-mutation', 'other-core-mutation', 'foreign-source',
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
                (upstream / 'codex-rs/tui').mkdir()
                (upstream / 'codex-rs/tui/original').write_bytes(b'original\n')
                daemon = upstream / 'codex-rs/tui/src/daemon_startup.rs'
                daemon.parent.mkdir()
                daemon.write_bytes(b'pub fn uses_wsl_drvfs() {}\n')
                checker = WORKFLOW.parents[2] / 'frontend/manager-tui/check_backend_boundary.py'
                spec = importlib.util.spec_from_file_location('owned_boundary', checker)
                boundary = importlib.util.module_from_spec(spec)
                spec.loader.exec_module(boundary)
                for name, (variable, indent) in boundary.GUARDS.items():
                    path = upstream / name
                    path.parent.mkdir(parents=True, exist_ok=True)
                    path.write_text(' ' * indent + f'if {variable}.approval_policy == Some(AskForApproval::UnlessTrusted) {{\n  reject();\n}}\n')
                for name, replacements in boundary.TRANSFORMS.items():
                    path = upstream / name
                    path.parent.mkdir(parents=True, exist_ok=True)
                    path.write_text('\n'.join(before for before, _ in replacements))
                for args in (['init', '-q'], ['add', '.'], ['-c', 'user.name=Owned', '-c', 'user.email=owned@example.invalid', 'commit', '-qm', 'owned']):
                    subprocess.run([git, *args], cwd=upstream, check=True, capture_output=True)
                for name, (variable, indent) in boundary.GUARDS.items():
                    path = upstream / name
                    path.write_text(boundary.compatible(path.read_text(), variable, indent))
                for name, replacements in boundary.TRANSFORMS.items():
                    path = upstream / name
                    value = path.read_text()
                    for before, after in replacements:
                        value = value.replace(before, after)
                    path.write_text(value)
                check_target = root / 'experiment/frontend/manager-tui/check_backend_boundary.py'
                check_target.parent.mkdir(parents=True)
                shutil.copy2(checker, check_target)
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
                    sys.exit(7 if fault == 'failure' else 0)
                '''))
                (tools / 'just').chmod(0o755)
                (root / 'profile-tui-formatted.patch').write_bytes(subprocess.check_output([git, 'diff', '--', 'codex-rs/tui', *boundary.GUARDS, *boundary.TRANSFORMS], cwd=upstream))
                env = {**os.environ, 'PATH': str(tools)+os.pathsep+os.environ['PATH'],
                       'RUNNER_TEMP': str(root), 'GITHUB_WORKSPACE': str(root), 'LINT_FAULT': fault or '', 'PYTHONDONTWRITEBYTECODE': '1'}
                result = subprocess.run(['bash', '-c', script], cwd=root, env=env,
                                        text=True, capture_output=True, timeout=15)
                self.assertEqual(result.returncode == 0, fault is None, result.stderr[-1000:])
                self.assertEqual(core.read_bytes(), before)
                self.assertEqual(daemon.read_bytes(), daemon_before)
                self.assertEqual((root / 'lint-invoked').exists(), fault not in ('foreign-source', 'foreign-daemon'))
                if fault != 'other-core-mutation':
                    self.assertEqual(other.read_bytes(), b'original\n')


if __name__ == '__main__':
    unittest.main()
