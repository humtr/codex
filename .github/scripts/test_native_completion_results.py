import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import textwrap
import unittest

from native_completion_results import SUITES, verify

ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = ROOT / '.github/workflows/manager-tui-native.yml'


class NativeCompletionResultsTests(unittest.TestCase):
    def output(self, suite, names=None):
        return '\n'.join('  PASS [ 0.01s] package module::' + name
                         for name in (SUITES[suite] if names is None else names))

    def test_each_named_positive_is_required_once(self):
        for suite, names in SUITES.items():
            self.assertEqual(verify(self.output(suite), suite), len(names))
            for name in names:
                rest = self.output(suite, [item for item in names if item != name])
                for replacement in ('', 'compiled ' + name, 'SKIP package module::' + name,
                                    'FAIL package module::' + name, self.output(suite, [name, name]),
                                    self.output(suite, [name + '_different'])):
                    with self.subTest(suite=suite, name=name, replacement=replacement):
                        with self.assertRaises(ValueError):
                            verify(rest + '\n' + replacement, suite)
                with self.assertRaises(ValueError):
                    verify(self.output(suite) + '\nFAIL package module::' + name, suite)

    def test_actual_workflow_gate_preserves_argv_results_and_failure_status(self):
        self.verify_workflow_gate(WORKFLOW, full_scope=True)

    def test_independent_runtime_gate_preserves_each_named_result_and_failure_status(self):
        self.verify_workflow_gate(ROOT / '.github/workflows/native-runtime-qualification.yml', full_scope=False)

    def verify_workflow_gate(self, workflow, full_scope):
        block = workflow.read_text().split('      - name: Verify Native completion uses original OS wait evidence\n', 1)[1].split('      - name:', 1)[0]
        if full_scope:
            self.assertIn("if: inputs.scope == 'full'", block)
        else:
            self.assertNotIn('if:', block)
        command = textwrap.dedent(block.split('        run: |\n', 1)[1])
        packages = [('pty', 'codex-utils-pty'), ('core', 'codex-core'),
                    ('protocol', 'codex-protocol'), ('rollout', 'codex-rollout'),
                    ('app-server-protocol', 'codex-app-server-protocol'), ('hooks', 'codex-hooks')]
        self.assertEqual(set(SUITES), {suite for suite, _ in packages})
        names = [name for suite in SUITES.values() for name in suite]
        self.assertEqual(len(names), 50)
        self.assertEqual(len(set(names)), 50)
        expected_args = []
        for suite, package in packages:
            args = ['test', '--retries', '0', '--status-level', 'pass', '--locked', '--target',
                    'aarch64-unknown-linux-musl', '-p', package, '--lib']
            if suite not in ('pty', 'rollout'):
                args += ['-E', 'test(native_completion_)']
            expected_args.append(args)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'upstream').mkdir()
            scripts = root / 'experiment/.github/scripts'
            scripts.mkdir(parents=True)
            for name in ('native_completion_results.py', 'tui_regression_results.py'):
                shutil.copyfile(ROOT / '.github/scripts' / name, scripts / name)
            bin_dir = root / 'bin'
            bin_dir.mkdir()
            just = bin_dir / 'just'
            just.write_text('#!' + sys.executable + '\nimport json,os,sys\nfrom pathlib import Path\n'
                            'args=sys.argv[1:]; package=args[args.index("-p")+1]\n'
                            'with Path(os.environ["ARGV_RECORD"]).open("a") as f: f.write(json.dumps(args)+"\\n")\n'
                            'print(json.loads(os.environ["TEST_OUTPUTS"])[package],flush=True)\n'
                            'print("stderr:"+package,file=sys.stderr,flush=True)\n'
                            'sys.exit(int(json.loads(os.environ["TEST_STATUSES"]).get(package,0)))\n')
            just.chmod(0o700)
            cases = [(None, 0, 0)]
            for index in range(len(packages)):
                cases += [(index, 0, 1), (index, 7, 7)]
            for failed, status, expected in cases:
                with self.subTest(failed=failed, status=status):
                    record = root / 'argv.jsonl'
                    record.unlink(missing_ok=True)
                    outputs = {package: self.output(suite) for suite, package in packages}
                    statuses = {}
                    if failed is not None:
                        suite, package = packages[failed]
                        if status:
                            statuses[package] = status
                        else:
                            outputs[package] = self.output(suite, SUITES[suite][:-1])
                    env = os.environ.copy()
                    env.update(PATH=str(bin_dir) + os.pathsep + env['PATH'], GITHUB_WORKSPACE=str(root),
                               RUNNER_TEMP=str(root), ARGV_RECORD=str(record),
                               TEST_OUTPUTS=json.dumps(outputs), TEST_STATUSES=json.dumps(statuses))
                    result = subprocess.run(['bash', '-c', command], cwd=root, env=env,
                                            capture_output=True, text=True)
                    self.assertEqual(result.returncode, expected, result.stdout + result.stderr)
                    executed = len(packages) if failed is None else failed + 1
                    self.assertEqual([json.loads(line) for line in record.read_text().splitlines()],
                                     expected_args[:executed])
                    for suite, package in packages[:executed]:
                        self.assertEqual((root / f'native-provider-completion-{suite}.log').read_text(),
                                         outputs[package] + '\n' + 'stderr:' + package + '\n')


if __name__ == '__main__':
    unittest.main()
