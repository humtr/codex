"""Protect release privilege boundaries and parse every executable shell step."""
import json
from pathlib import Path
import re
import subprocess
import sys
import unittest

ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'scripts'))
from pin_release import caller, source_workflow_registrations


class WorkflowTests(unittest.TestCase):
    def test_action_pins_and_shell_syntax(self):
        for p in (ROOT/'.github/workflows').glob('*.yml'):
            text=p.read_text()
            for use in re.findall(r'(?m)^\s*uses:\s*(\S+)',text):
                if not use.startswith('./'):self.assertRegex(use,r'@[0-9a-f]{40}$')
            # All workflows use literal run blocks. Extract by next indentation.
            for m in re.finditer(r'(?m)^( +)run: \|\n',text):
                indent=len(m[1]);lines=[]
                for line in text[m.end():].splitlines():
                    if line.strip() and len(line)-len(line.lstrip())<=indent:break
                    lines.append(line[indent+2:])
                script=re.sub(r'\$\{\{.*?\}\}','placeholder','\n'.join(lines))
                result=subprocess.run(['bash','-n'],input=script,text=True,capture_output=True)
                self.assertEqual(result.returncode,0,str(p)+': '+result.stderr)

    def test_runner_paths_are_resolved_after_job_allocation(self):
        # Actions rejects runner context in workflow/job env while validating a
        # reusable workflow, before any runner or shell can exist.
        for path in (ROOT/'.github/workflows').glob('*.yml'):
            text=path.read_text()
            for match in re.finditer(r'(?m)^( {0,4})env: *\n',text):
                indent=len(match[1]);lines=[]
                for line in text[match.end():].splitlines():
                    if line.strip() and len(line)-len(line.lstrip())<=indent:break
                    lines.append(line)
                self.assertNotRegex('\n'.join(lines),r'\$\{\{\s*runner\.',str(path))
        producer=(ROOT/'.github/workflows/android-component.yml').read_text()
        build=producer.split('      - name: Cross-build selected Android component once',1)[1].split('      - name:',1)[0]
        self.assertIn('export CARGO_TARGET_DIR="$RUNNER_TEMP/component-target"',build)
        self.assertLess(build.index('export CARGO_TARGET_DIR='),build.index('cargo "+$RUST_VERSION" build'))

    def test_every_final_development_head_has_readonly_component_receipts(self):
        for name in ('core-runtime-qualification.yml','manager-qualification.yml','full-qualification.yml','provider-qualification.yml'):
            text=(ROOT/'.github/workflows'/name).read_text()
            header=text.split('permissions:',1)[0]
            self.assertIn('branches: [rewrite/rust-core]',header)
            self.assertNotIn('paths:',header)
        text=(ROOT/'.github/workflows/manager-tui-native.yml').read_text()
        step=text.split('      - name: Verify actual hook matching used by Native resource proof',1)[1].split('      - name:',1)[0]
        self.assertIn("if: inputs.scope == 'full'",step)
        self.assertIn('-p codex-hooks --lib native_resource_proof_',step)
        for name in ('retains_unrelated_input_hook_and_its_dispatch',
                     'rejects_each_matching_tool_event_and_permission_alias',
                     'rejects_unknown_async_and_non_tool_hook_effects',
                     'rejects_async_work_retained_after_reconfiguration'):
            self.assertIn('native_resource_proof_'+name,step)
        self.assertIn('0 failed; 0 ignored;',step)
        self.assertNotIn('continue-on-error',step)

    def test_required_provider_and_switch_regressions_are_executed_and_positive(self):
        from tui_regression_results import REQUIRED
        text=(ROOT/'.github/workflows/manager-tui-native.yml').read_text()
        step=text.split('      - name: Focused actual native TUI regressions',1)[1].split('      - name:',1)[0]
        self.assertIn("if: inputs.scope == 'full'",step)
        self.assertIn('--retries 0',step)
        selectors=re.findall(r'test\(([^)]+)\)',step)
        for name in REQUIRED:
            # The widget replay test lives under this selected module.
            selected=any(selector in name for selector in selectors)
            self.assertTrue(selected or (name.startswith('replayed_rate_limit_error_') and
                                        'rate_limit_recovery_tests' in selectors),name)
        self.assertIn('tui_regression_results.py" "$RUNNER_TEMP/profile-tui-focused.log"',step)
        self.assertNotIn('continue-on-error',step)

    def test_android_window_proof_uses_event_source_and_observes_the_real_activity(self):
        workflow=(ROOT/'.github/workflows/stock-window-proof.yml').read_text()
        self.assertIn('PRODUCT_SOURCE: ${{ github.sha }}',workflow)
        self.assertIn('branches: [rewrite/rust-core]',workflow)
        self.assertIn('test "$PRODUCT_SOURCE" = "$GITHUB_SHA"',workflow)
        self.assertNotIn('contents: write',workflow)
        self.assertNotIn('secrets.',workflow)
        self.assertNotIn('EXTERNAL_STORAGE',workflow)
        fixture=(ROOT/'.github/stock-window/StockWindowTest.java').read_text()
        for evidence in ('getCurrentSession().getPid()', 'exact native foreground UUID',
                         'hasWindowFocus()', 'concurrent-single-surface','foreign-home-refusal'):
            self.assertIn(evidence,fixture)
        self.assertIn("'native_owner_inference':False",workflow)
        self.assertIn("'synthetic_core_transport':True",workflow)
        self.assertIn('owner-free-synthetic-thread',fixture)
        self.assertIn('elapsedRealtime()+15000',fixture)
        self.assertIn('TestWatcher failureEvidence',fixture)
        self.assertIn('getTranscriptText()',fixture)
        failure=workflow.split('      - name: Collect failed disposable-emulator diagnostics',1)[1].split('      - name:',1)[0]
        self.assertIn('if: failure()',failure)
        self.assertIn('stock-stages.txt',failure)
        self.assertIn('stock-activity.txt',failure)
        self.assertNotIn('continue-on-error',workflow)

    def test_single_source_and_no_unsigned_public_authority(self):
        text=(ROOT/'.github/workflows/auto-release-termux.yml').read_text()
        header,body=text.split('\njobs:\n',1)
        producer=body.split('\n  reserve:',1)[0]
        qualification=(ROOT/'.github/workflows/qualify-termux-runtime.yml').read_text()
        sign=body.split('\n  sign:',1)[1].split('\n  stage_release:',1)[0]
        self.assertIn('workflow_call:',header)
        self.assertNotIn('workflow_dispatch:',header)
        self.assertIn('CODEX_SOURCE_SHA: ${{ inputs.source_sha }}',header)
        self.assertNotIn('RALD5_SOURCE_SHA',text)
        self.assertIn('test "$(git -C publication-source rev-parse HEAD)" = "$CODEX_SOURCE_SHA"',text)
        self.assertNotIn('publish-termux-update-pages.yml',text)
        for job in (producer,qualification,sign):
            for forbidden in ('contents: write','pages: write','gh release','git push','deploy-pages'):
                self.assertNotIn(forbidden,job)
        self.assertNotIn('secrets.',producer+qualification)
        self.assertEqual(text.count('${{ secrets.CODEX_RELEASE_SIGNING_KEY }}'),1)
        self.assertIn("needs.producer.result == 'success'",sign)
        self.assertIn("needs.reserve.outputs.created == 'true' && inputs.publish",sign)
        reservation=body.split('\n  reserve:',1)[1].split('\n  sign:',1)[0]
        self.assertIn("needs.producer.result == 'success' && inputs.publish",reservation)
        self.assertNotIn('secrets.',reservation)
        self.assertIn('release_reservations.py reserve',reservation)
        self.assertIn('uses: ./.github/workflows/qualify-termux-runtime.yml',producer)
        self.assertIn('base_only: false',producer)
        self.assertNotIn('base_only:',header)
        self.assertIn('      actions: read',producer)
        self.assertNotIn('secrets:',qualification)
        base=(ROOT/'.github/workflows/core-runtime-qualification.yml').read_text()
        self.assertIn('base_only: true',base)
        self.assertIn('publish: false',base)
        self.assertNotIn('write',base.split('\n  notify_acceptance:',1)[0].replace('rewrite/rust-core','source-branch'))
        self.assertIn('actions: write',base.split('\n  notify_acceptance:',1)[1])
        for key in ('stage_release','verify_public','promote'):
            job=re.split(r'\n  [a-z_]+:',body.split('\n  '+key+':',1)[1],maxsplit=1)[0]
            self.assertIn('&& inputs.publish',job)
        self.assertIn('force:false',text)
        self.assertNotIn('force:true',text)
        self.assertIn('Bootstrap protected source state and update through immutable composition',text)
        self.assertIn('Prepare manifest-owned current generation bootstrap fixture',text)
        self.assertIn('after-noop-update',text)
        self.assertNotIn('same_version_acceptance',text)
        self.assertNotIn('legacy_lag_jump_remediation',text)

    def test_incremental_scope_requires_a_successful_predecessor(self):
        text=(ROOT/'.github/workflows/source-acceptance.yml').read_text()
        scope=text.split('      - name: Select verification scope',1)[1].split('      - name:',1)[0]
        self.assertIn('head_sha=$BEFORE',scope)
        self.assertIn('.conclusion=="success"',scope)
        self.assertIn("scope=full",scope)
        self.assertNotIn("RUST_VERSION: '1.",text)
        self.assertIn('uses: ./.github/workflows/android-component.yml',text)
        self.assertIn('source/release/upstream.json --component core',text)

    def test_manual_core_validation_binds_event_source_and_only_rewrite_relays_acceptance(self):
        text=(ROOT/'.github/workflows/core-runtime-qualification.yml').read_text()
        before, relay=text.split('\n  notify_acceptance:',1)
        self.assertIn('branches: [rewrite/rust-core]',before)
        self.assertIn('workflow_dispatch:',before)
        self.assertIn('source_sha: ${{ github.sha }}',before)
        self.assertNotIn('inputs.source',text)
        self.assertIn('base_only: true',before)
        self.assertIn('publish: false',before)
        expression=re.search(r'(?m)^    if: (.+)$',relay)[1]
        for ref in ('refs/heads/rewrite/rust-core', 'refs/heads/integration/owned-source',
                    'refs/heads/main', 'refs/tags/v0.161.0', 'refs/pull/1/merge', ''):
            for result in ('success', 'failure', 'cancelled', 'skipped'):
                with self.subTest(ref=ref, result=result):
                    evaluated=(expression.replace('always()','True')
                               .replace('github.ref',repr(ref))
                               .replace('needs.qualify.result',repr(result))
                               .replace('github.event_name',repr('workflow_dispatch'))
                               .replace('inputs.advance_release','True')
                               .replace('&&',' and '))
                    self.assertEqual(eval(evaluated,{'__builtins__':{}},{}),
                                     ref=='refs/heads/rewrite/rust-core' and result=='success')
        qualification=(ROOT/'.github/workflows/qualify-termux-runtime.yml').read_text()
        self.assertTrue('--event-sha "$GITHUB_SHA"' in qualification,
                        'the decision must receive the actual immutable event SHA')
        self.assertIn('needs: [producer, smoke]',qualification)
        self.assertIn('steps.decision.outputs.candidate == \'true\'',qualification)

    def test_source_and_provider_gates_reject_mismatched_run_metadata(self):
        sha='a'*40
        checks=[
            ('.github/workflows/source-acceptance.yml', 'source-acceptance.yml'),
            ('.github/workflows/provider-release.yml', 'provider-qualification.yml'),
            ('.github/workflows/provider-release.yml', 'source-acceptance.yml'),
        ]
        for file, workflow in checks:
            text=(ROOT/file).read_text()
            expressions=re.findall(r"'(\.workflow_runs \| any\([^']+\))'",text)
            self.assertEqual(len(expressions),1,file)
            path='.github/workflows/'+workflow
            good={'head_sha':sha,'path':path,'head_branch':'rewrite/rust-core',
                  'repository':{'full_name':'humtr/codex'},'event':'push',
                  'status':'completed','conclusion':'success'}
            def evaluate(run):
                args=['jq','-e','--arg','sha',sha,'--arg','path',path,expressions[0]]
                return subprocess.run(args,input=json.dumps({'workflow_runs':[run]}),
                                      text=True,capture_output=True).returncode
            with self.subTest(file=file,workflow=workflow):
                self.assertEqual(evaluate(good),0)
                for field, bad in [
                    ('head_sha','b'*40),('path','.github/workflows/rald7-full-acceptance.yml'),
                    ('head_branch','main'),('repository',{'full_name':'other/codex'}),
                    ('event','pull_request'),('status','in_progress'),('conclusion','failure')]:
                    with self.subTest(field=field):
                        self.assertNotEqual(evaluate({**good,field:bad}),0)

    def test_default_branch_registrations_have_no_second_proof_or_publication_path(self):
        entries=source_workflow_registrations()
        self.assertEqual(len(entries),3)
        for name,text in entries.items():
            self.assertIn('workflow_dispatch:',text)
            self.assertIn('exit 1',text)
            self.assertNotIn('secrets.',text)
            self.assertNotIn('write',text.replace('rewrite/rust-core','source'))
            self.assertIn(text.splitlines()[0],(ROOT/'.github/workflows'/name).read_text())

    def test_main_caller_pins_both_inputs_and_default_dry_run(self):
        sha='a'*40; text=caller(sha)
        self.assertIn('/auto-release-termux.yml@'+sha,text)
        self.assertIn('source_sha: ${{ needs.source.outputs.sha }}',text)
        self.assertIn('contents/release-source-v1?ref=',text)
        self.assertIn('core-runtime-qualification.yml',text)
        self.assertEqual(text.count('default: false'),2)
        self.assertEqual(text.count('secrets.CODEX_RELEASE_SIGNING_KEY'),2)
        self.assertNotIn('secrets: inherit',text)
        self.assertIn('release_variant: base',text)
        self.assertIn('release_variant: full',text)
        self.assertIn('needs: [source, base]',text)
        self.assertIn('!cancelled()',text)
        self.assertIn('group: termux-hosted-release-callers',text)
        self.assertIn('cancel-in-progress: false',text)
        self.assertNotIn("github.event_name == 'schedule'",text)
        for bad in ('main','a'*39,'A'*40,'a'*40+'\ninjected'):
            with self.assertRaises(ValueError):caller(bad)

    def test_full_lane_runs_only_after_success_or_an_explicit_full_only_selection(self):
        text=caller('a'*40)
        full=text.split('\n  release:',1)[1].split('\n  notify_acceptance:',1)[0]
        expression=re.search(r'(?m)^    if: \$\{\{ (.+) \}\}$',full)[1]
        for variant in ['both','base','full']:
            for base in ['success','failure','cancelled','skipped']:
                for source in ['success','failure','skipped']:
                    for cancelled in [False,True]:
                        terms={'inputs.variant':repr(variant),'needs.base.result':repr(base),
                               'needs.source.result':repr(source),'github.event_name':repr('workflow_dispatch'),
                               'cancelled()':repr(cancelled)}
                        evaluated=expression
                        for key,value in terms.items():evaluated=evaluated.replace(key,value)
                        evaluated=evaluated.replace('&&',' and ').replace('||',' or ')
                        evaluated=re.sub(r'!(?!=)',' not ',evaluated)
                        allowed=not cancelled and source=='success' and (
                            variant=='both' and base=='success' or
                            variant=='full' and base in ['success','skipped'])
                        with self.subTest(variant=variant,base=base,source=source,cancelled=cancelled):
                            self.assertEqual(eval(evaluated,{'__builtins__':{}},{}),allowed)


if __name__=='__main__':unittest.main()
