"""Run the real producer acquisition and builder dispatch with owned transports."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile
import textwrap
import unittest

import test_manager_tui_artifact as fixtures

ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = ROOT / '.github/workflows/auto-release-termux.yml'


def step(name):
    block = WORKFLOW.read_text().split('      - name: ' + name + '\n', 1)[1].split('\n      - name:', 1)[0]
    return textwrap.dedent(block.split('        run: |\n', 1)[1])


class NativeProducerTests(unittest.TestCase):
    def test_actual_acquisition_refuses_bad_archive_failed_run_and_changed_source(self):
        for fault in (None, 'archive', 'failed-run', 'source'):
            with self.subTest(fault=fault), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                source, artifact, proof, sha, digest, run, run_file = fixtures.NativeArtifactTests().fixture(root)
                (source / '.github/scripts').symlink_to(ROOT / '.github/scripts', target_is_directory=True)
                archive = root / 'input.tar.gz'
                with tarfile.open(archive, 'w:gz') as bundle:
                    for p in artifact.iterdir(): bundle.add(p, arcname='artifact/' + p.name)
                    for p in proof.iterdir(): bundle.add(p, arcname='proof/_temp/' + p.name)
                archive_digest = hashlib.sha256(archive.read_bytes()).hexdigest()
                if fault == 'archive': archive.write_bytes(b'not qualified')
                if fault == 'failed-run':
                    run['conclusion'] = 'failure'; run_file.write_text(json.dumps(run))
                if fault == 'source':
                    p = source / 'frontend/manager-tui/native.patch'; p.write_text('changed\n')
                    subprocess.run(['git', '-C', str(source), 'add', '.'], check=True)
                    subprocess.run(['git', '-C', str(source), '-c', 'user.name=Owned', '-c', 'user.email=x@example.invalid', 'commit', '-m', 'changed'], check=True, stdout=subprocess.DEVNULL)
                tools = root / 'tools'; tools.mkdir()
                real_git = shutil.which('git')
                commands = {
                    'curl': 'import shutil,sys; shutil.copyfile(' + repr(str(archive)) + ',sys.argv[sys.argv.index("--output")+1])',
                    'gh': 'import sys; sys.stdout.write(' + repr(run_file.read_text()) + ')',
                    'git': 'import os,sys;\nif "fetch" not in sys.argv: os.execv(' + repr(real_git) + ',[' + repr(real_git) + ']+sys.argv[1:])',
                }
                for name, code in commands.items():
                    p = tools / name; p.write_text('#!' + sys.executable + '\n' + code + '\n'); p.chmod(0o755)
                env = {**os.environ, 'PATH': str(tools) + ':' + os.environ['PATH'], 'RUNNER_TEMP': str(root),
                       'MANAGER_TUI_INPUT_URL': 'https://example.invalid/qualified', 'MANAGER_TUI_INPUT_SHA256': archive_digest,
                       'MANAGER_TUI_RUN_ID': '123', 'MANAGER_TUI_SOURCE_SHA': sha, 'MANAGER_TUI_VERSION': '0.161.0',
                       'MANAGER_TUI_SHA256': digest, 'PYTHONDONTWRITEBYTECODE': '1'}
                result = subprocess.run(['bash', '-c', step('Acquire successful qualified Manager TUI input')], cwd=root, env=env, text=True, capture_output=True, timeout=20)
                self.assertEqual(result.returncode == 0, fault is None, result.stderr)
                if fault == 'archive': self.assertFalse((root / 'manager-tui-input/artifact').exists())
                if fault is None: self.assertIn(digest, result.stdout)

    def test_actual_candidate_builder_requires_the_qualified_pair(self):
        for version in ('0.161.0','0.154.0'):
            with self.subTest(version=version), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp); binary = root / 'source/target/release/codex-release-builder'; binary.parent.mkdir(parents=True)
                binary.write_text('#!' + sys.executable + '\n' + textwrap.dedent('''\
                    import json,os,sys
                    from pathlib import Path
                    if sys.argv[1] == 'fetch': print('archive_sha256\\t' + 'a'*64)
                    else: Path(os.environ['CAPTURE']).write_text(json.dumps(sys.argv[1:]))
                ''')); binary.chmod(0o755)
                script = step('Fetch, adapt, and qualify unsigned candidate').split('\npython3 source/.github/scripts/rald3_preflight.py candidate', 1)[0]
                for field, value in [('upstream_version', version), ('archive_sha256', 'a'*64), ('generation_id', 'owned-candidate')]:
                    script = script.replace('${{ steps.decision.outputs.' + field + ' }}', value)
                env = {**os.environ, 'RUNNER_TEMP': str(root), 'GITHUB_WORKSPACE': str(root), 'MANAGER_TUI_VERSION': '0.161.0', 'CAPTURE': str(root/'args.json')}
                result = subprocess.run(['bash', '-c', script], cwd=root, env=env, capture_output=True, text=True, timeout=15)
                if version != '0.161.0':
                    self.assertNotEqual(result.returncode,0)
                    self.assertFalse((root/'args.json').exists())
                    continue
                self.assertEqual(result.returncode, 0, result.stderr)
                args = json.loads((root/'args.json').read_text())
                self.assertIn('--manager', args)
                self.assertEqual(args[args.index('--manager-tui')+1], str(root/'manager-tui-input/artifact/codex'))
                self.assertEqual(args[args.index('--manager-tui-version')+1], version)

    def test_actual_native_smoke_requires_same_version_success_without_stderr(self):
        script = step('Run exact Manager, Core, and runtime smoke natively').split('timeout 20',1)[1].split('\nrm "$candidate/.manager-probe-deferred"',1)[0]
        script='timeout 20'+script
        for output, stderr, rc in [('codex-cli 0.161.0', '', 0), ('codex-cli 0.160.0', '', 0), ('codex-cli 0.161.0', 'failure', 0), ('codex-cli 0.161.0', '', 1)]:
            with self.subTest(output=output, stderr=stderr, rc=rc), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp); (root/'helpers').mkdir(); native = root/'helpers/2'
                native.write_text('#!/bin/sh\nprintf "%s\\n" "' + output + '"\nprintf "%s" "' + stderr + '" >&2\nexit ' + str(rc) + '\n'); native.chmod(0o755)
                result = subprocess.run(['bash','-ec',script], env={**os.environ, 'candidate':str(root), 'RUNNER_TEMP':str(root), 'MANAGER_TUI_VERSION':'0.161.0'}, text=True, capture_output=True)
                self.assertEqual(result.returncode == 0, output == 'codex-cli 0.161.0' and not stderr and rc == 0, result.stderr)



if __name__ == '__main__': unittest.main()
