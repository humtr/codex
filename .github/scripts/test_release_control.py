import argparse
import base64
import hashlib
import io
import json
from pathlib import Path
import tarfile
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import release_control as control


class ReleaseControlTests(unittest.TestCase):
    def test_documentation_head_requests_the_existing_accepted_product_without_a_rebuild_or_pointer_write(self):
        root=Path(__file__).resolve().parents[2];head='a'*40;product='c'*40
        version=control.load(root/'release/upstream.json')['upstream']['version']
        def api(path,**kwargs):
            if path=='git/ref/heads/rewrite/rust-core':return {'object':{'sha':head}}
            if path=='git/ref/heads/main':return {'object':{'sha':'b'*40}}
            if path.startswith('contents/'):return {'content':base64.b64encode((product+'\n').encode()).decode()}
            if path.startswith('actions/workflows/auto-release-termux.yml/runs'):return {'workflow_runs':[]}
            self.fail('unexpected authority access '+path)
        for proven in [True,False]:
            with self.subTest(proven=proven),patch.object(control.subprocess,'check_output',return_value=head+'\n'),\
                 patch.object(control,'api',side_effect=api),patch.object(control,'same_product_inputs',return_value=True),\
                 patch.object(control,'frontend_registration_needed',return_value=False),\
                 patch.object(control,'successful_run',return_value=({'id':1},[]) if proven else (None,[])) as proof,\
                 patch.object(control,'ensure_frontend') as frontend,patch.object(control,'caller_compatible',return_value=True),\
                 patch.object(control,'release_lane',return_value='both'),patch.object(control,'commit_files') as commit,\
                 patch.object(control,'dispatch') as dispatch:
                if proven:
                    result=control.run(root,version)
                    self.assertEqual(result['state'],'requested-build-started');self.assertEqual(result['source'],product)
                    dispatch.assert_called_once_with('auto-release-termux.yml','main',{'publish':'true','rebuild_current':'true','variant':'both','upstream_version':version})
                else:
                    with self.assertRaisesRegex(ValueError,'accepted product proof is missing'):control.run(root,version)
                    dispatch.assert_not_called()
                self.assertTrue(all(call.args[1]==product for call in proof.call_args_list))
                frontend.assert_not_called();commit.assert_not_called()

    def test_explicit_request_starts_build_for_already_pinned_source_and_reuses_active_publication(self):
        sha='a'*40;root=Path(__file__).resolve().parents[2]
        version=control.load(root/'release/upstream.json')['upstream']['version']
        for requested,active in [(None,False),(version,False),(version,True)]:
            def api(path,**kwargs):
                if path=='git/ref/heads/rewrite/rust-core':return {'object':{'sha':sha}}
                if path=='git/ref/heads/main':return {'object':{'sha':'b'*40}}
                if path.startswith('contents/'):return {'content':base64.b64encode((sha+'\n').encode()).decode()}
                if path.startswith('actions/workflows/auto-release-termux.yml/runs'):
                    return {'workflow_runs':[{'status':'in_progress'}] if active else []}
                self.fail('unexpected authority access '+path)
            with self.subTest(requested=requested,active=active),\
                 patch.object(control.subprocess,'check_output',return_value=sha+'\n'),\
                 patch.object(control,'api',side_effect=api),patch.object(control,'successful_run',return_value=({'id':1},[])),\
                 patch.object(control,'ensure_frontend',return_value=None),patch.object(control,'caller_compatible',return_value=True),\
                 patch.object(control,'release_lane',return_value='both'),\
                 patch.object(control,'dispatch') as dispatch,patch.object(control,'commit_files') as commit:
                result=control.run(root,requested)
                expected='already-pinned' if requested is None else 'awaiting-publication' if active else 'requested-build-started'
                self.assertEqual(result['state'],expected)
                commit.assert_not_called()
                if expected=='requested-build-started':
                    dispatch.assert_called_once_with('auto-release-termux.yml','main',{'publish':'true','rebuild_current':'true','variant':'both','upstream_version':version})
                else:dispatch.assert_not_called()
        with patch.object(control.subprocess,'check_output',return_value=sha+'\n'),\
             patch.object(control,'api',return_value={'object':{'sha':sha}}),\
             patch.object(control,'commit_files') as commit,patch.object(control,'dispatch') as dispatch:
            with self.assertRaisesRegex(ValueError,'requested upstream differs'):
                control.run(root,'0.999.0')
            commit.assert_not_called();dispatch.assert_not_called()

    def test_nested_runtime_workflow_change_requires_a_new_accepted_caller(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp)
            def git(*args):
                return subprocess.check_output(['git','-C',str(root),*args],stderr=subprocess.DEVNULL).decode().strip()
            git('init');git('config','user.name','Owned');git('config','user.email','owned@example.invalid')
            for name in ['auto-release-termux.yml','qualify-termux-runtime.yml','termux-runtime-source.yml']:
                path=root/'.github/workflows'/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_text('accepted '+name+'\n')
            (root/'scripts').mkdir();(root/'scripts/pin_release.py').write_text('accepted caller\n')
            git('add','.');git('commit','-qm','accepted implementation');accepted=git('rev-parse','HEAD')
            caller=('uses: humtr/codex/.github/workflows/auto-release-termux.yml@'+accepted+'\n')*2
            run=subprocess.run
            def without_network(command,**kwargs):
                if 'fetch' in command:return subprocess.CompletedProcess(command,0,b'',b'')
                return run(command,**kwargs)
            with patch.object(control,'api',return_value={'content':base64.b64encode(caller.encode()).decode()}),\
                 patch.object(control.subprocess,'run',side_effect=without_network):
                self.assertTrue(control.caller_compatible(root,'a'*40))
                path=root/'.github/workflows/termux-runtime-source.yml';path.write_text('different runtime producer\n')
                git('add','.');git('commit','-qm','new runtime implementation')
                self.assertFalse(control.caller_compatible(root,'a'*40))

    def test_exact_failed_proof_is_reported_without_waiting_or_repeating_dispatch(self):
        sha='a'*40;root=Path(__file__).resolve().parents[2]
        def api(path,**kwargs):
            if path=='git/ref/heads/rewrite/rust-core':return {'object':{'sha':sha}}
            if path=='git/ref/heads/main':return {'object':{'sha':'b'*40}}
            if path.startswith('contents/'):return {'content':base64.b64encode(('c'*40+'\n').encode()).decode()}
            self.fail('unexpected authority access '+path)
        failed={'id':42,'path':'.github/workflows/core-runtime-qualification.yml','head_sha':sha,
                'head_branch':'rewrite/rust-core','event':'push','status':'completed',
                'conclusion':'failure','repository':{'full_name':'humtr/codex'}}
        for status in ['completed','in_progress']:
            with self.subTest(status=status), patch.object(control.subprocess,'check_output',return_value=sha+'\n'),\
                 patch.object(control,'api',side_effect=api),patch.object(control,'same_product_inputs',return_value=False),\
                 patch.object(control,'successful_run',side_effect=[({'id':1},[]),(None,[{**failed,'status':status}])]),\
                 patch.object(control,'dispatch') as dispatch,patch.object(control,'commit_files') as commit:
                result=control.run(root)
                self.assertEqual(result['state'],'qualification-failed' if status=='completed' else 'awaiting-exact-proof')
                if status=='completed':
                    self.assertEqual(result['run_id'],42)
                    self.assertEqual(result['url'],'https://github.com/humtr/codex/actions/runs/42')
                dispatch.assert_not_called();commit.assert_not_called()

    def test_only_exact_owned_successful_runs_can_advance(self):
        sha='a'*40
        run={'path':'.github/workflows/rald7-full-acceptance.yml','head_sha':sha,
             'head_branch':'rewrite/rust-core','event':'push','status':'completed',
             'conclusion':'success','repository':{'full_name':'humtr/codex'}}
        self.assertTrue(control.accepted_run(run,'rald7-full-acceptance.yml',sha))
        for field,bad in [('head_sha','b'*40),('head_branch','main'),('event','pull_request'),
                          ('status','in_progress'),('conclusion','failure'),
                          ('path','.github/workflows/other.yml'),('repository',{'full_name':'other/codex'})]:
            with self.subTest(field=field):
                self.assertFalse(control.accepted_run({**run,field:bad},'rald7-full-acceptance.yml',sha))

    def test_github_failure_reports_status_without_private_response_or_tokens(self):
        error=subprocess.CalledProcessError(1,['gh','release','create'],stderr=b'HTTP 403 opaque-private-value')
        with patch.object(control.subprocess,'run',side_effect=error):
            with self.assertRaisesRegex(control.GitHubFailure,'GitHub release operation failed: HTTP 403') as caught:
                control.gh('release','create','owned')
            self.assertNotIn('opaque-private-value',str(caught.exception))
        text=(Path(__file__).with_name('release_control.py')).read_text()
        self.assertIn("'--target','main'",text)
        self.assertNotIn("'--target',run['head_sha']",text)

    def test_branch_move_refuses_before_writing_and_commit_is_nonforced(self):
        calls=[]
        def api(path,**kwargs):
            calls.append((path,kwargs))
            if path.startswith('git/ref/'):return {'object':{'sha':'a'*40}}
            if path.startswith('git/commits/') :return {'tree':{'sha':'tree'}}
            if path=='git/blobs':return {'sha':'blob'}
            if path=='git/trees':return {'sha':'newtree'}
            if path=='git/commits':return {'sha':'b'*40}
        with patch.object(control,'api',side_effect=api):
            with self.assertRaisesRegex(ValueError,'branch moved'):
                control.commit_files('main','c'*40,{'release-source-v1':b'owned'},'owned')
            self.assertEqual(len(calls),1);calls.clear()
            control.commit_files('main','a'*40,{'release-source-v1':b'owned'},'owned')
        self.assertEqual(calls[-1][1]['payload'],{'sha':'b'*40,'force':False})
        self.assertEqual(calls[-1][1]['method'],'PATCH')
        self.assertEqual(calls[-2][1]['payload']['parents'],['a'*40])
        self.assertFalse(any('update-index' in str(call) for call in calls))

    def test_frontend_input_packaging_is_reproducible_without_running_artifact(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);artifact=root/'artifact';proof=root/'proof'
            artifact.mkdir();proof.mkdir()
            (artifact/'codex').write_bytes(b'owned nonexecutable fixture')
            (proof/'_temp').mkdir();(proof/'_temp/log').write_text('proof')
            first=control.bundle_inputs(artifact,proof)
            self.assertEqual(first,control.bundle_inputs(artifact,proof))
            with tarfile.open(fileobj=io.BytesIO(first),mode='r:gz') as archive:
                self.assertEqual(archive.getnames(),['artifact/codex','proof/_temp/log'])
                self.assertTrue(all(entry.mtime==entry.uid==entry.gid==0 for entry in archive))
                self.assertEqual(archive.getmember('artifact/codex').mode,0o755)
                self.assertEqual(archive.getmember('proof/_temp/log').mode,0o644)

    def test_base_can_advance_without_frontend_and_missing_core_proof_dispatches_only_proof(self):
        root=Path(__file__).resolve().parents[2];sha='a'*40;calls=[]
        def api(path,**kwargs):
            calls.append((path,kwargs))
            if path=='git/ref/heads/rewrite/rust-core':return {'object':{'sha':sha}}
            if path=='git/ref/heads/main':return {'object':{'sha':'b'*40}}
            if path.startswith('actions/workflows/auto-release-termux.yml/runs'):return {'workflow_runs':[]}
            if path.startswith('contents/'):return {'content':base64.b64encode(('c'*40+'\n').encode()).decode()}
        with patch.object(control.subprocess,'check_output',return_value=sha+'\n'),patch.object(control,'api',side_effect=api),\
             patch.object(control,'same_product_inputs',return_value=False),\
             patch.object(control,'successful_run',return_value=({'id':1},[])),\
             patch.object(control,'ensure_frontend',return_value=None),patch.object(control,'caller_compatible',return_value=True),\
             patch.object(control,'commit_files',return_value='d'*40) as commit,patch.object(control,'dispatch') as dispatch:
            result=control.run(root)
            self.assertEqual(result['state'],'accepted-product-pinned-public-proof-started')
            self.assertEqual(set(commit.call_args.args[2]),{'release-source-v1'})
            self.assertEqual(dispatch.call_args.args[2]['variant'],'both')
        with patch.object(control.subprocess,'check_output',return_value=sha+'\n'),patch.object(control,'api',side_effect=api),\
             patch.object(control,'same_product_inputs',return_value=False),\
             patch.object(control,'successful_run',side_effect=[({'id':1},[]),(None,[])]),\
             patch.object(control,'dispatch') as dispatch,patch.object(control,'commit_files') as commit:
            self.assertEqual(control.run(root)['state'],'awaiting-exact-proof')
            commit.assert_not_called()
            self.assertEqual(dispatch.call_args.args[0],'core-runtime-qualification.yml')

    def test_matching_inflight_native_build_is_reused_without_duplicate_dispatch(self):
        run={'path':'.github/workflows/manager-tui-native.yml','head_sha':'a'*40,
             'head_branch':'rewrite/rust-core','event':'push','status':'in_progress',
             'conclusion':None,'repository':{'full_name':'humtr/codex'}}
        with patch.object(control,'frontend_recipe_matches',return_value=True):
            self.assertTrue(control.native_build_inflight(Path('.'),[run]))
            for field,bad in [('status','completed'),('event','pull_request'),
                              ('repository',{'full_name':'foreign/repo'}),('head_branch','main')]:
                self.assertFalse(control.native_build_inflight(Path('.'),[{**run,field:bad}]))
        with patch.object(control,'frontend_recipe_matches',return_value=False):
            self.assertFalse(control.native_build_inflight(Path('.'),[run]))

    def test_frontend_registration_only_releases_full_after_matching_base_is_public(self):
        root_source=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);(root/'release').mkdir()
            def git(*args):
                return subprocess.check_output(['git','-C',str(root),*args],stderr=subprocess.DEVNULL).decode().strip()
            git('init');git('config','user.name','Owned');git('config','user.email','owned@example.invalid')
            value=control.load(root_source/'release/upstream.json')
            frontend=value['frontend'] or {'source':'a'*40,'run_id':'123','sha256':'b'*64,
                'input_url':'https://github.com/humtr/codex/releases/download/owned/inputs.tar.gz','input_sha256':'c'*64}
            value['frontend']=None
            path=root/'release/upstream.json';path.write_text(json.dumps(value))
            git('add','.');git('commit','-m','base');base=git('rev-parse','HEAD')
            value['frontend']=frontend;path.write_text(json.dumps(value))
            git('add','.');git('commit','-m','frontend');current=git('rev-parse','HEAD')
            def content(source):
                index='codex-update-index-v1\nchannel\tstable\ngeneration_id\tlocal-base-hosted-0-163-0-'+source[:12]+'-s55\nrelease_base\thttps://example.invalid/\n'
                return {'content':base64.b64encode(index.encode()).decode()}
            with patch.object(control,'api',return_value=content(base)):
                self.assertEqual(control.release_lane(root,base,current,'a'*40),'full')
            with patch.object(control,'api',return_value=content('b'*40)):
                self.assertEqual(control.release_lane(root,base,current,'a'*40),'both')
            value['upstream']['archive_sha256']='c'*64;path.write_text(json.dumps(value))
            git('add','.');git('commit','-m','new recipe');current=git('rev-parse','HEAD')
            with patch.object(control,'api',return_value=content(base)):
                self.assertEqual(control.release_lane(root,base,current,'a'*40),'both')

    def test_manager_source_changes_select_only_full_when_exact_base_inputs_are_public(self):
        source=Path('.')
        with patch.object(control,'base_fingerprint',return_value='a'*64), \
             patch.object(control,'verified_published_base',return_value='local-base-relay-hosted-owned-s60'), \
             patch.object(control.subprocess,'check_output',return_value='crates/manager/src/profile.rs\n'):
            self.assertEqual(control.release_lane(source,'b'*40,'c'*40,'d'*40),'full')
        with patch.object(control,'base_fingerprint',side_effect=['a'*64,'e'*64]), \
             patch.object(control,'verified_published_base') as verify, \
             patch.object(control.subprocess,'check_output',return_value='crates/core/src/main.rs\n'):
            self.assertEqual(control.release_lane(source,'b'*40,'c'*40,'d'*40),'both')
            verify.assert_not_called()
        with patch.object(control,'base_fingerprint',return_value='a'*64), \
             patch.object(control,'verified_published_base',return_value=None), \
             patch.object(control.subprocess,'check_output',return_value='crates/manager/src/profile.rs\n'):
            self.assertEqual(control.release_lane(source,'b'*40,'c'*40,'d'*40),'both')

    def test_inflight_publication_delays_pointer_write_until_its_completion_event(self):
        root=Path(__file__).resolve().parents[2];sha='a'*40
        def api(path,**kwargs):
            if path=='git/ref/heads/rewrite/rust-core':return {'object':{'sha':sha}}
            if path=='git/ref/heads/main':return {'object':{'sha':'b'*40}}
            if path.startswith('contents/'):return {'content':base64.b64encode(('c'*40+'\n').encode()).decode()}
            if path.startswith('actions/workflows/auto-release-termux.yml/runs'):
                return {'workflow_runs':[{'status':'in_progress'}]}
        with patch.object(control.subprocess,'check_output',return_value=sha+'\n'),patch.object(control,'api',side_effect=api),\
             patch.object(control,'same_product_inputs',return_value=False),\
             patch.object(control,'successful_run',return_value=({'id':1},[])),\
             patch.object(control,'ensure_frontend',return_value=None),patch.object(control,'caller_compatible',return_value=True),\
             patch.object(control,'commit_files') as commit,patch.object(control,'dispatch') as dispatch:
            self.assertEqual(control.run(root)['state'],'awaiting-publication')
            commit.assert_not_called();dispatch.assert_not_called()
        text=(root/'.github/workflows/accepted-release-inputs.yml').read_text()
        self.assertIn('completed_run:',text)
        self.assertNotIn('workflow_run:',text)
        caller=(root/'scripts/pin_release.py').read_text()
        self.assertIn('Continue after publication reaches a terminal state',caller)

    def test_git_comparison_preserves_unknown_product_changes(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp)
            def git(*args):
                return subprocess.check_output(['git','-C',str(root),*args],stderr=subprocess.DEVNULL).decode().strip()
            git('init');git('config','user.name','Owned');git('config','user.email','owned@example.invalid')
            (root/'runtime').write_text('owned source')
            git('add','.');git('commit','-m','baseline');baseline=git('rev-parse','HEAD')
            (root/'CURRENT.md').write_text('updated evidence')
            git('add','.');git('commit','-m','docs');docs=git('rev-parse','HEAD')
            self.assertTrue(control.same_product_inputs(root,baseline,docs))
            (root/'previously-unknown-source').write_text('new product input')
            git('add','.');git('commit','-m','product');product=git('rev-parse','HEAD')
            self.assertFalse(control.same_product_inputs(root,baseline,product))
            with self.assertRaisesRegex(ValueError,'invalid accepted'):
                control.same_product_inputs(root,'main',product)

    def test_documentation_only_updates_do_not_rebuild_or_publish(self):
        root=Path(__file__).resolve().parents[2];sha='a'*40
        def api(path,**kwargs):
            if path=='git/ref/heads/rewrite/rust-core':return {'object':{'sha':sha}}
            if path=='git/ref/heads/main':return {'object':{'sha':'b'*40}}
            if path.startswith('actions/workflows/auto-release-termux.yml/runs'):return {'workflow_runs':[]}
            if path.startswith('contents/'):return {'content':base64.b64encode(('c'*40+'\n').encode()).decode()}
        with patch.object(control.subprocess,'check_output',return_value=sha+'\n'),patch.object(control,'api',side_effect=api),\
             patch.object(control,'same_product_inputs',return_value=True),\
             patch.object(control,'frontend_registration_needed',return_value=False),\
             patch.object(control,'successful_run') as proof,patch.object(control,'dispatch') as dispatch,\
             patch.object(control,'commit_files') as commit:
            self.assertEqual(control.run(root)['state'],'no-product-change')
            proof.assert_not_called();dispatch.assert_not_called();commit.assert_not_called()

    def test_controller_executes_owned_code_only_after_exact_source_acceptance(self):
        root=Path(__file__).resolve().parents[2]
        workflow=(root/'.github/workflows/accepted-release-inputs.yml').read_text()
        self.assertLess(workflow.index('source-runs.json'),workflow.index('python3 source/.github/scripts/release_control.py'))
        self.assertIn("if: steps.source.outputs.accepted == 'true'",workflow)
        self.assertNotIn('secrets.',workflow)
        text=(root/'.github/scripts/release_control.py').read_text()
        self.assertNotIn('force\':True',text)
        self.assertNotIn('CODEX_RELEASE_SIGNING_KEY',text)


if __name__=='__main__':unittest.main()
