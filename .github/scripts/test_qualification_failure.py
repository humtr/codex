import hashlib
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import textwrap
import unittest

from qualification_failure import TAIL_BYTES, annotation, log_fields, report

ROOT = Path(__file__).resolve().parents[2]


def retain_step():
    text = (ROOT / '.github/workflows/manager-tui-native.yml').read_text()
    block = text.split('      - name: Qualify retained upstream 0.162 features\n', 1)[1]
    return textwrap.dedent(block.split('        run: |\n', 1)[1].split('      - name:', 1)[0])


class QualificationFailureTests(unittest.TestCase):
    def test_bounded_tail_emits_only_structured_failure_fields(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'log'
            path.write_text('private original text\n' * TAIL_BYTES +
                '\x1b[31mFAIL\x1b[0m [ 0.013s] codex-core::all suite::retained_test\n'
                'left: private assertion value\nAuthorization: Bearer private-token\n'
                '::error::injected workflow command\n'
                'error[E0308]: private compiler details\n'
                'Summary [ 1.20s] 2 tests run: 1 passed, 1 failed, 90 skipped\n')
            value = log_fields(path)
            self.assertLessEqual(value['tail_bytes'], TAIL_BYTES)
            self.assertEqual(value['failed_tests'], [{'status': 'FAIL', 'package': 'codex-core::all',
                                                     'test': 'suite::retained_test'}])
            self.assertEqual(value['summary'], {'run': 2, 'passed': 1, 'failed': 1, 'skipped': 90})
            self.assertEqual(value['compiler_error_codes'], ['E0308'])
            self.assertNotIn('private', json.dumps(value))
            self.assertNotIn('injected', json.dumps(value))
            path.unlink(); path.symlink_to('/does-not-exist')
            self.assertEqual(log_fields(path), {'state': 'unavailable'})
            path.unlink(); path.mkdir()
            self.assertEqual(log_fields(path), {'state': 'not-regular'})

    def test_annotation_escapes_control_data_and_rejects_false_status(self):
        text = annotation({'value': '%\r\n::error::injected'})
        self.assertEqual(text.count('::error'), 2)  # The second one remains JSON data on one line.
        self.assertNotIn('\n', text)
        self.assertNotIn('\r', text)
        self.assertIn('%25', text)
        for stage, status in [('unknown', 1), ('retained-features', 0), ('retained-features', 256)]:
            with self.assertRaises(ValueError):
                report(stage, status, Path('/not-read'))

    def test_actual_step_preserves_checker_command_environment_exit_and_full_logs(self):
        for status, reporter_failed, download_status in [(0, False, 0), (17, False, 0),
                                                          (17, True, 0), (0, False, 23)]:
            with self.subTest(status=status, reporter_failed=reporter_failed, download=download_status), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                logs = root / 'logs'; logs.mkdir()
                checker = root / 'experiment/frontend/tui/check_upstream_features.py'
                checker.parent.mkdir(parents=True)
                checker.write_text('import json,os,sys\nfrom pathlib import Path\n'
                    'Path(os.environ["CAPTURE"]).write_text(json.dumps({"argv":sys.argv[1:], '
                    '"dontwrite":os.environ.get("PYTHONDONTWRITEBYTECODE"), '
                    '"cli":os.environ.get("CARGO_BIN_EXE_codex"), '
                    '"host":os.environ.get("CARGO_BIN_EXE_codex_code_mode_host")}))\n'
                    'Path(os.environ["RUNNER_TEMP"],"profile-tui-upstream-tui.log").write_text('
                    '"FAIL [ 0.001s] codex-tui module::specific_test\\nfull private fixture output\\n")\n'
                    'print("original stdout",flush=True)\nprint("original stderr",file=sys.stderr,flush=True)\n'
                    'sys.exit(' + str(status) + ')\n')
                reporter = root / 'experiment/.github/scripts/qualification_failure.py'
                reporter.parent.mkdir(parents=True)
                if reporter_failed:
                    reporter.write_text('raise SystemExit(92)\n')
                else:
                    reporter.symlink_to(ROOT / '.github/scripts/qualification_failure.py')
                archive = root / 'package.tar.gz'
                with tarfile.open(archive, 'w:gz') as bundle:
                    # Use a real portable interpreter on Android as well as Linux.
                    host = ('#!' + sys.executable + '\nraise SystemExit(0)\n').encode()
                    member = tarfile.TarInfo('bin/codex-code-mode-host'); member.size = len(host)
                    bundle.addfile(member, io.BytesIO(host))
                tools = root / 'tools'; tools.mkdir()
                curl = tools / 'curl'
                curl.write_text('#!' + sys.executable + '\nimport shutil,sys\n'
                    'sys.exit(' + str(download_status) + ') if ' + str(bool(download_status)) + ' else None\n'
                    'shutil.copyfile(' + repr(str(archive)) + ',sys.argv[sys.argv.index("--output")+1])\n')
                curl.chmod(0o755)
                env = {**os.environ, 'GITHUB_WORKSPACE': str(root), 'RUNNER_TEMP': str(logs),
                       'PATH': str(tools) + ':' + os.environ['PATH'], 'UPSTREAM_VERSION': '0.167.1',
                       'UPSTREAM_ARCHIVE_SHA256': hashlib.sha256(archive.read_bytes()).hexdigest(),
                       'CAPTURE': str(root / 'capture.json'), 'CODEX_QUALIFICATION_CLI': str(root / 'exact-cli')}
                result = subprocess.run(['bash', '-c', retain_step()], cwd=root, env=env,
                                        capture_output=True, text=True, timeout=10)
                expected_status = download_status or status
                self.assertEqual(result.returncode, expected_status, result.stderr)
                if not download_status:
                    capture = json.loads((root / 'capture.json').read_text())
                    self.assertEqual(capture, {'argv': ['--run', str(root / 'upstream'), '--logs', str(logs)],
                                              'dontwrite': '1', 'cli': str(root / 'exact-cli'),
                                              'host': str(logs / 'profile-code-mode-host/codex-code-mode-host')})
                    self.assertIn('original stdout', result.stdout)
                    self.assertIn('original stderr', result.stderr)
                    self.assertIn('full private fixture output', (logs / 'profile-tui-upstream-tui.log').read_text())
                receipt = logs / 'profile-tui-failure-summary.json'
                if expected_status and not reporter_failed:
                    value = json.loads(receipt.read_text())
                    self.assertEqual(value['command_exit'], expected_status)
                    self.assertEqual(value['stage'], 'host-download' if download_status else 'retained-features')
                    self.assertIn('::error title=Retained feature qualification failed::', result.stdout)
                    self.assertNotIn('full private fixture output', result.stdout)
                else:
                    self.assertFalse(receipt.exists())


if __name__ == '__main__':
    unittest.main()
