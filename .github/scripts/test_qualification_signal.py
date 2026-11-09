from pathlib import Path
import unittest
from unittest.mock import patch

from qualification_signal import native_recipe, signal_job
import qualification_signal as qualification

ROOT=Path(__file__).resolve().parents[2]


class QualificationSignalTests(unittest.TestCase):
    def test_runtime_transfer_selects_only_an_owned_successful_exact_source_and_live_artifact(self):
        source='a'*40
        good={'id':123,'path':'.github/workflows/core-runtime-qualification.yml','head_sha':source,
            'head_branch':'rewrite/rust-core','event':'push','repository':{'full_name':'humtr/codex'},
            'status':'completed','conclusion':'success'}
        live={'name':'termux-runtime-source','expired':False}
        cases=[(good,[live],123),({**good,'head_sha':'b'*40},[live],None),
               ({**good,'head_branch':'main'},[live],None),({**good,'event':'pull_request'},[live],None),
               ({**good,'head_branch':'integration/owned-source','event':'workflow_dispatch'},[live],None),
               ({**good,'conclusion':'failure'},[live],None),
               (good,[{**live,'expired':True}],None),(good,[],None)]
        for run,artifacts,expected in cases:
            def api(path,**kwargs):
                if path.startswith('actions/workflows/core-runtime-qualification.yml/runs?head_sha='+source):
                    return {'workflow_runs':[run]}
                self.assertEqual(path,'actions/runs/123/artifacts?per_page=100')
                return {'total_count':len(artifacts),'artifacts':artifacts}
            with self.subTest(run=run,artifacts=artifacts),patch.object(qualification,'api',side_effect=api):
                self.assertEqual(qualification.qualified_runtime_run(source),expected)
        for page in [{'total_count':2,'artifacts':[live]},
                     {'total_count':2,'artifacts':[live,live]}]:
            with patch.object(qualification,'api',side_effect=[{'workflow_runs':[good]},page]):
                with self.assertRaises(ValueError):qualification.qualified_runtime_run(source)

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
        for name,dependency in [('source-acceptance.yml','accepted'),('core-runtime-qualification.yml','qualify'),('manager-tui-native.yml','upstream-baseline')]:
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
