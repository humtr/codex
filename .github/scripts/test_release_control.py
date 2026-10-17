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

    def test_base_can_advance_without_frontend_and_missing_core_proof_dispatches_only_proof(self):
        root=Path(__file__).resolve().parents[2];sha='a'*40;calls=[]
        def api(path,**kwargs):
            calls.append((path,kwargs))
            if path=='git/ref/heads/rewrite/rust-core':return {'object':{'sha':sha}}
            if path=='git/ref/heads/main':return {'object':{'sha':'b'*40}}
            if path.startswith('contents/'):return {'content':base64.b64encode(('c'*40+'\n').encode()).decode()}
        with patch.object(control.subprocess,'check_output',return_value=sha+'\n'),patch.object(control,'api',side_effect=api),\
             patch.object(control,'same_product_inputs',return_value=False),\
             patch.object(control,'successful_run',return_value=({'id':1},[])),\
             patch.object(control,'ensure_frontend',return_value=None),\
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
            if path.startswith('contents/'):return {'content':base64.b64encode(('c'*40+'\n').encode()).decode()}
        with patch.object(control.subprocess,'check_output',return_value=sha+'\n'),patch.object(control,'api',side_effect=api),\
             patch.object(control,'same_product_inputs',return_value=True),\
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
