from pathlib import Path
import unittest

from qualification_signal import native_recipe, signal_job

ROOT=Path(__file__).resolve().parents[2]


class QualificationSignalTests(unittest.TestCase):
    def test_only_trigger_metadata_and_exact_isolated_relay_are_reusable(self):
        original='name: Native\non:\n  push:\n    branches: [rewrite/rust-core]\npermissions:\n  contents: read\njobs:\n  upstream-baseline:\n    runs-on: ubuntu-24.04-arm\n'
        changed=original.replace('  push:\n    branches: [rewrite/rust-core]','  workflow_dispatch:')+signal_job('upstream-baseline')
        self.assertEqual(native_recipe(original),native_recipe(changed))
        for bad in [changed.replace('ubuntu-24.04-arm','ubuntu-latest'),
                    changed.replace('contents: read','contents: write',1),
                    changed+'env:\n  RUSTFLAGS: dangerous\n',
                    changed.replace('completed_run=', 'unexpected='),
                    changed.replace('jobs:\n','env:\n  EVENT: ${{ inputs.value }}\njobs:\n')]:
            self.assertNotEqual(native_recipe(original),native_recipe(bad))

    def test_native_job_budget_reuses_commands_but_not_expressions_or_build_settings(self):
        original = ('jobs:\n  upstream-baseline:\n    runs-on: ubuntu-24.04-arm\n'
                    '    timeout-minutes: 90\n    steps:\n      - run: echo build\n')
        changed = original.replace('    timeout-minutes: 90',
                                   '    # Four mutation rebuilds.\n    timeout-minutes: 120')
        self.assertEqual(native_recipe(original), native_recipe(changed))
        for bad in [changed.replace('echo build', 'echo changed'),
                    changed.replace('120', '${{ inputs.budget }}'),
                    changed.replace('echo build', 'echo timeout-minutes'),
                    changed.replace('ubuntu-24.04-arm', 'ubuntu-latest')]:
            self.assertNotEqual(native_recipe(original), native_recipe(bad))

    def test_all_qualification_workflows_use_the_same_relay_and_only_controller_starts_native(self):
        for name,dependency in [('rald7-full-acceptance.yml','accepted'),('core-runtime-qualification.yml','qualify'),('manager-tui-native.yml','upstream-baseline')]:
            text=(ROOT/'.github/workflows'/name).read_text()
            self.assertTrue(text.endswith(signal_job(dependency)),name)
            self.assertIn("if: always()",signal_job(dependency))
        native=(ROOT/'.github/workflows/manager-tui-native.yml').read_text()
        self.assertNotIn('  push:',native)
        receiver=(ROOT/'.github/workflows/accepted-release-inputs.yml').read_text()
        self.assertIn('COMPLETED_RUN: ${{ inputs.completed_run }}',receiver)
        self.assertIn('[[ "$completed" =~ ^[1-9][0-9]{0,19}$ ]]',receiver)
        self.assertLess(receiver.index('test "$status" = completed'),receiver.index('source-runs.json'))
        for dependency in ['unknown','accepted\ninjected']:
            with self.assertRaises(ValueError):signal_job(dependency)


if __name__=='__main__':unittest.main()
