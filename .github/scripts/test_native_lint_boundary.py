"""The real hosted lint allowance cannot become an upstream source modification."""
import os
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import textwrap
import unittest

WORKFLOW = Path(__file__).resolve().parents[1] / 'workflows/manager-tui-native.yml'


class NativeLintBoundaryTests(unittest.TestCase):
    def test_actual_lint_step_restores_qualified_sources_on_every_outcome(self):
        format_block = WORKFLOW.read_text().split('      - name: Format and inspect native profile scope', 1)[1].split('      - name:', 1)[0]
        format_script = textwrap.dedent(format_block.split('        run: |\n', 1)[1])
        schema_block = WORKFLOW.read_text().split('      - name: Verify generated Native protocol contracts', 1)[1].split('      - name:', 1)[0]
        schema_script = textwrap.dedent(schema_block.split('        run: |\n', 1)[1])
        block = WORKFLOW.read_text().split('      - name: Inspect TUI lint fixes', 1)[1].split('      - name:', 1)[0]
        script = textwrap.dedent(block.split('        run: |\n', 1)[1])
        self.assertEqual(subprocess.run(['bash', '-n'], input=script, text=True, capture_output=True).returncode, 0)
        git = shutil.which('git')
        for fault in (None, 'failure', 'strict-failure', 'root-mutation', 'other-core-mutation', 'config-mutation', 'schema-mutation', 'outside-mutation', 'foreign-source',
                      'daemon-mutation', 'foreign-daemon', 'strict-root-mutation', 'strict-daemon-mutation'):
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
                outside = upstream / 'codex-rs/outside.rs'
                outside.write_bytes(b'original\n')
                (upstream / 'codex-rs/tui').mkdir()
                (upstream / 'codex-rs/tui/original').write_bytes(b'original\n')
                daemon = upstream / 'codex-rs/tui/src/daemon_startup.rs'
                daemon.parent.mkdir()
                daemon.write_bytes(b'pub fn uses_wsl_drvfs() {}\n')
                for args in (['init', '-q'], ['add', '.'], ['-c', 'user.name=Owned', '-c', 'user.email=owned@example.invalid', 'commit', '-qm', 'owned']):
                    subprocess.run([git, *args], cwd=upstream, check=True, capture_output=True)
                # Native preparation may change Core's module root and add a
                # source file. Both must enter the proof used for binary reuse;
                # lint must retain the qualified bytes, not pristine HEAD.
                prepared_core = original + b'\nmod native_execution;\n'
                core.write_bytes(prepared_core)
                native = core.parent / 'native_execution.rs'
                native.write_bytes(b'pub fn native_execution() {}\n')
                config_before = b'// Complete model catalog source retained\n'
                config.write_bytes(config_before)
                (root / 'profile-tui-native-preparation.json').write_text(json.dumps({
                    'source': subprocess.check_output([git, 'rev-parse', 'HEAD'], cwd=upstream, text=True).strip(),
                    'changed_files': ['codex-rs/core/src/config/mod.rs', 'codex-rs/core/src/lib.rs',
                                      'codex-rs/core/src/native_execution.rs'],
                }))
                tools = root / 'bin'
                tools.mkdir()
                assembler = root / 'experiment/frontend/tui/apply.py'
                assembler.parent.mkdir(parents=True)
                shutil.copy2(Path(__file__).resolve().parents[2] / 'frontend/tui/apply.py', assembler)
                builder = root / 'source-tools/release/codex-release-builder'
                builder.parent.mkdir(parents=True)
                builder.write_text(f'#!{sys.executable}\n' + textwrap.dedent('''\
                    from pathlib import Path
                    import sys
                    assert sys.argv[1] == 'generate-schema'
                    schema = Path('codex-rs/app-server-protocol/schema/generated.json')
                    schema.parent.mkdir(parents=True, exist_ok=True)
                    schema.write_text('{"generated":"after format"}\\n')
                '''))
                builder.chmod(0o755)
                (tools / 'just').write_text(f'#!{sys.executable}\n' + textwrap.dedent('''\
                    import json, os, sys
                    from pathlib import Path
                    if sys.argv[1:] == ['fmt']:
                        sys.exit(0)
                    core = Path('codex-rs/core/src/lib.rs')
                    assert core.read_bytes().startswith(b'#![recursion_limit = "256"]\\n')
                    daemon = Path('codex-rs/tui/src/daemon_startup.rs')
                    assert daemon.read_bytes().startswith(b'#[allow(clippy::unnecessary_cast)]\\n')
                    mode = sys.argv[1]
                    assert mode in ('fix', 'clippy')
                    assert sys.argv[2:] == ['--locked', '--target', 'aarch64-unknown-linux-musl', '-p', 'codex-tui']
                    with Path('../lint-invocations.jsonl').open('a') as log:
                        log.write(json.dumps(sys.argv[1:]) + '\\n')
                    Path('../lint-invoked').write_text('yes')
                    fault = os.environ.get('LINT_FAULT')
                    if fault == 'root-mutation': core.write_bytes(b'changed')
                    if fault == 'daemon-mutation': daemon.write_bytes(b'changed')
                    if fault == 'other-core-mutation': Path('codex-rs/core/src/other.rs').write_bytes(b'changed')
                    if fault == 'config-mutation': Path('codex-rs/core/src/config/mod.rs').write_bytes(b'changed')
                    if fault == 'schema-mutation': Path('codex-rs/app-server-protocol/schema/generated.json').write_bytes(b'changed')
                    if fault == 'outside-mutation': Path('codex-rs/outside.rs').write_bytes(b'changed')
                    if mode == 'clippy':
                        if fault == 'strict-root-mutation': core.write_bytes(b'changed')
                        if fault == 'strict-daemon-mutation': daemon.write_bytes(b'changed')
                        if fault == 'strict-failure': sys.exit(11)
                    sys.exit(7 if fault == 'failure' else 0)
                '''))
                (tools / 'just').chmod(0o755)
                env = {**os.environ, 'PATH': str(tools)+os.pathsep+os.environ['PATH'],
                       'RUNNER_TEMP': str(root), 'LINT_FAULT': fault or '', 'PYTHONDONTWRITEBYTECODE': '1',
                       'GITHUB_WORKSPACE': str(root), 'CARGO_TARGET_DIR': str(root / 'target')}
                formatted = subprocess.run(['bash', '-c', format_script], cwd=root, env=env,
                                           text=True, capture_output=True, timeout=15)
                self.assertEqual(formatted.returncode, 0, formatted.stderr)
                self.assertEqual((root / 'profile-tui-core-lib.rs').read_bytes(), prepared_core)
                generated = subprocess.run(['bash', '-c', schema_script], cwd=root, env=env,
                                           text=True, capture_output=True, timeout=15)
                self.assertEqual(generated.returncode, 0, generated.stderr)
                proof = (root / 'profile-tui-formatted.patch').read_bytes()
                self.assertIn(b'diff --git a/codex-rs/core/src/native_execution.rs b/codex-rs/core/src/native_execution.rs', proof)
                self.assertIn(native.read_bytes(), proof.replace(b'+', b''))
                self.assertIn(b'diff --git a/codex-rs/app-server-protocol/schema/generated.json b/codex-rs/app-server-protocol/schema/generated.json', proof)
                self.assertIn(b'+{"generated":"after format"}\n', proof)
                if fault == 'foreign-source':
                    core.write_bytes(b'foreign\n')
                if fault == 'foreign-daemon':
                    daemon.write_bytes(b'foreign\n')
                before = core.read_bytes()
                daemon_before = daemon.read_bytes()
                result = subprocess.run(['bash', '-c', script], cwd=root, env=env,
                                        text=True, capture_output=True, timeout=15)
                self.assertEqual(result.returncode == 0, fault is None, result.stderr[-1000:])
                self.assertEqual(core.read_bytes(), before)
                self.assertEqual(daemon.read_bytes(), daemon_before)
                self.assertEqual((root / 'lint-invoked').exists(), fault not in ('foreign-source', 'foreign-daemon'))
                invocation_log = root / 'lint-invocations.jsonl'
                modes = [json.loads(line)[0] for line in invocation_log.read_text().splitlines()] if invocation_log.exists() else []
                if fault in ('foreign-source', 'foreign-daemon'):
                    self.assertEqual(modes, [])
                elif fault in ('failure', 'root-mutation', 'daemon-mutation'):
                    self.assertEqual(modes, ['fix'])
                else:
                    self.assertEqual(modes, ['fix', 'clippy'])
                if fault == 'strict-failure':
                    self.assertIn('exit status 11', result.stderr)
                    self.assertFalse((root / 'profile-tui-linted.patch').exists())
                if fault != 'other-core-mutation':
                    self.assertEqual(other.read_bytes(), b'original\n')
                if fault != 'config-mutation':
                    self.assertEqual(config.read_bytes(), config_before)


if __name__ == '__main__':
    unittest.main()
