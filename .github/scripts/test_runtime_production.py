"""Immutable raw-origin admission and complete compiler-input normalization."""
import copy
import hashlib
import io
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch
import zipfile
import runtime_inputs as inputs
import runtime_production as production
import test_runtime_inputs as fixtures
ROOT=fixtures.ROOT

class RawProductionTests(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory(prefix='raw-production-')
        self.addCleanup(self.temp.cleanup)
        self.folder=Path(self.temp.name)
        self.source=fixtures.RuntimeProducer().source(self.folder/'source')
        self.previous=self.sha()
        self.original=inputs.production_inputs(self.source)
        self.run={'id':11,'path':'.github/workflows/core-runtime-qualification.yml',
                  'head_sha':self.previous,'head_branch':'rewrite/rust-core','event':'push',
                  'repository':{'full_name':'humtr/codex'},'status':'completed','conclusion':'failure'}
        # Another failed job does not erase completed raw production; it does
        # prevent claiming that the complete Core qualification succeeded.
        self.job={'id':22,'run_id':11,'name':'qualify / runtime-source / runtime',
                  'status':'completed','conclusion':'success',
                  'steps':[{'name':production.BUILD,'conclusion':'success'},
                           {'name':production.UPLOAD,'conclusion':'success'}]}
        buffer=io.BytesIO()
        with zipfile.ZipFile(buffer,'w') as zipped:zipped.writestr('runtime.tar',b'owned raw transfer')
        self.archive=buffer.getvalue()
        self.artifact={'id':33,'name':'termux-runtime-source','expired':False,
                       'digest':'sha256:'+hashlib.sha256(self.archive).hexdigest(),
                       'size_in_bytes':len(self.archive),
                       'workflow_run':{'id':11,'head_sha':self.previous}}
        self.raw=self.folder/'raw';self.raw.mkdir()
        for name in production.RAW_FILES:(self.raw/name).write_bytes(('owned '+name).encode())
        (self.raw/'codex').chmod(0o700)
        transport=self.source/'.github/scripts/runtime_artifact.py'
        transport.write_text(transport.read_text()+'\n# stricter consumer-only transfer validation\n')
        self.commit()
    def sha(self):
        return production.git(self.source,'rev-parse','HEAD').decode().strip()
    def commit(self):
        subprocess.run(['git','-C',str(self.source),'add','.'],check=True)
        subprocess.run(['git','-C',str(self.source),'-c','user.name=Input test','-c',
                        'user.email=test@example.invalid','commit','-qm','fixture change'],check=True)
    def request(self,path):
        values={
          'actions/workflows/core-runtime-qualification.yml/runs?per_page=100':{'workflow_runs':[self.run]},
          'actions/runs/11/jobs?per_page=100':{'total_count':1,'jobs':[self.job]},
          'actions/runs/11/artifacts?per_page=100':{'total_count':1,'artifacts':[self.artifact]},
          'actions/runs/11':self.run,'actions/jobs/22':self.job,'actions/artifacts/33':self.artifact}
        self.assertIn(path,values)
        return copy.deepcopy(values[path])
    def test_transport_revision_and_identity_script_are_not_compiler_or_module_revision(self):
        self.assertEqual(inputs.production_inputs(self.source),self.original)
        self.assertNotEqual(inputs.fingerprint(self.source,'HEAD'),inputs.fingerprint(self.source,self.previous))
        self.assertEqual(inputs.adapted_production_fingerprint(self.source),
                         inputs.adapted_production_fingerprint(self.source,self.previous))
        path=self.source/'.github/scripts/runtime_inputs.py';path.write_text(path.read_text()+'\n# new metadata checker\n')
        self.commit();self.assertEqual(inputs.production_inputs(self.source),self.original)
    def test_selected_runtime_rn_uses_production_not_the_legacy_consumer_recipe(self):
        from component_revisions import selected,assign
        descriptor={'upstream_package_version':'0.167.1','core_api_identity':'core-v1','manager_artifact_digest':'-'}
        with patch('component_revisions.parse_descriptor',return_value=descriptor),\
             patch('component_revisions.component_fingerprint',return_value='a'*64),\
             patch('component_revisions.sha256_file',return_value='b'*64):
            version,current=selected(self.source,self.raw)
        expected=inputs.adapted_production_fingerprint(self.source,self.previous)
        self.assertEqual(current['runtime']['inputs_sha256'],expected)
        original=assign(version,current,[])
        self.assertEqual(assign(version,current,[original])['components']['runtime']['revision'],1)

    def test_compiler_patch_upstream_dependency_and_unknown_cargo_config_are_bound(self):
        before=inputs.production_inputs(self.source)
        path=self.source/'crates/release-builder/src/native_route.patch'
        path.write_bytes(path.read_bytes()+b'\n# changed compiler adaptation\n');self.commit()
        self.assertNotEqual(inputs.production_inputs(self.source),before)
        before=inputs.production_inputs(self.source)
        config=self.source/'.cargo/config.toml';config.parent.mkdir()
        config.write_text('[env]\nRUST_TEST_THREADS="1"\n');self.commit()
        self.assertEqual(inputs.production_inputs(self.source),before)
        config.write_text('[env]\nRUST_TEST_THREADS="1"\nCFLAGS="-O0"\n');self.commit()
        self.assertNotEqual(inputs.production_inputs(self.source),before)
        before=inputs.production_inputs(self.source)
        workspace=self.source/'Cargo.toml';workspace.write_text(workspace.read_text()+'\n[profile.release]\nopt-level=1\n');self.commit()
        self.assertNotEqual(inputs.production_inputs(self.source),before)
        before=inputs.production_inputs(self.source)
        recipe=self.source/'release/upstream.json';value=json.loads(recipe.read_text())
        value['upstream']['source']='a'*40;recipe.write_text(json.dumps(value));self.commit()
        self.assertNotEqual(inputs.production_inputs(self.source),before)
        before=inputs.production_inputs(self.source)
        lock=self.source/'Cargo.lock';text=lock.read_text()
        begin=text.index('name = "toml"');checksum=text.index('checksum = "',begin)+len('checksum = "')
        lock.write_text(text[:checksum]+'0'*64+text[checksum+64:]);self.commit()
        self.assertNotEqual(inputs.production_inputs(self.source),before)
    def test_known_consumer_name_cannot_hide_environment_or_unknown_producer_steps(self):
        text=(self.source/'.github/workflows/termux-runtime-source.yml').read_text()
        before=inputs.compiler_workflow(text)
        for replacement in [
          '          echo "CFLAGS=-O0" >> "$GITHUB_ENV"\n',
          '        env:\n          RUSTFLAGS: -C opt-level=0\n',
          '        working-directory: unbound\n']:
            changed=text.replace('      - name: Build shared Rust source preparation',replacement+'      - name: Build shared Rust source preparation')
            self.assertNotEqual(inputs.compiler_workflow(changed),before)
        changed=text+'      - name: Unexpected compiler input\n        run: rustc changed.rs\n'
        self.assertNotEqual(inputs.compiler_workflow(changed),before)
        with self.assertRaises(ValueError):inputs.compiler_workflow(text+'      - run: rustc hidden.rs\n')
    def test_actual_producer_keeps_original_provenance_and_never_grants_qualification(self):
        selected=production.resolve(self.source,self.request)
        self.assertEqual(selected['producer_source'],self.previous)
        self.assertNotEqual(selected['consumer_source'],self.previous)
        receipt=production.record(self.source,self.raw,selected,request=self.request)
        self.assertEqual(receipt['production_run_id'],11);self.assertEqual(receipt['production_job_id'],22)
        self.assertEqual(receipt['production_artifact_id'],33)
        self.assertEqual(receipt['production_artifact_digest'],self.artifact['digest'])
        self.assertEqual(receipt['producer_source'],self.previous)
        self.assertFalse(receipt['qualification'])
        target=self.folder/'consumer.json';production.consumer_record(self.source,receipt,target)
        self.assertFalse(json.loads(target.read_text())['qualification'])
        self.assertFalse(hasattr(production,'admit')) # no self-authenticating cache receipt path
    def test_incomplete_foreign_reused_failed_or_unavailable_origins_cannot_be_selected(self):
        for fault in ('foreign-ref','foreign-repo','different-source','build-skipped','build-failed',
                      'upload-skipped','pending-job','failed-job','expired-artifact'):
            with self.subTest(fault=fault):
                run,job,artifact=copy.deepcopy((self.run,self.job,self.artifact))
                if fault=='foreign-ref':self.run['head_branch']='main'
                if fault=='foreign-repo':self.run['repository']['full_name']='another/codex'
                if fault=='different-source':
                    path=self.source/'crates/release-builder/src/native_route.patch';old=path.read_bytes()
                    path.write_bytes(old+b'\nchanged\n');self.commit()
                if fault=='build-skipped':self.job['steps'][0]['conclusion']='skipped'
                if fault=='build-failed':self.job['steps'][0]['conclusion']='failure'
                if fault=='upload-skipped':self.job['steps'][1]['conclusion']='skipped'
                if fault=='pending-job':self.job['status']='in_progress';self.job['conclusion']=None
                if fault=='failed-job':self.job['conclusion']='failure'
                if fault=='expired-artifact':self.artifact['expired']=True
                self.assertIsNone(production.resolve(self.source,self.request))
                if fault=='different-source':path.write_bytes(old);self.commit()
                self.run,self.job,self.artifact=run,job,artifact
    def test_changed_selection_digest_or_archive_owner_refuses(self):
        selected=production.resolve(self.source,self.request)
        for field,value in [('consumer_source','f'*40),('production_artifact_digest','sha256:'+'0'*64),
                            ('production_inputs',{})]:
            with self.subTest(field=field),self.assertRaises(ValueError):
                production.record(self.source,self.raw,{**selected,field:value},request=self.request)
        self.artifact['workflow_run']['head_sha']='f'*40
        with self.assertRaises(ValueError):production.record(self.source,self.raw,selected,request=self.request)
    def test_incomplete_and_ambiguous_api_inventories_refuse(self):
        for key in ('jobs','artifacts'):
            for duplicate in (False,True):
                def request(path):
                    value=self.request(path)
                    if path.endswith('/'+key+'?per_page=100'):
                        value['total_count']=2
                        if duplicate:value[key]*=2
                    return value
                with self.subTest(key=key,duplicate=duplicate),self.assertRaises(ValueError):
                    production.resolve(self.source,request)
    def test_download_binds_api_archive_digest_size_inventory_and_original_id(self):
        selected=production.resolve(self.source,self.request)
        def run(args,**kwargs):
            self.assertIn('/artifacts/33/zip',args[-1]);kwargs['stdout'].write(self.archive)
            return SimpleNamespace(returncode=0)
        target=self.folder/'transfer'
        with patch.object(production.subprocess,'run',side_effect=run):production.download(selected,target,self.request)
        self.assertEqual({p.name for p in target.iterdir()},{'runtime.tar'})
        self.assertEqual((target/'runtime.tar').read_bytes(),b'owned raw transfer')
        for fault in ('digest','size','member'):
            with self.subTest(fault=fault):
                artifact=copy.deepcopy(self.artifact);archive=self.archive
                if fault=='digest':self.archive=self.archive[:-1]+b'x'
                if fault=='size':self.archive+=b'x'
                if fault=='member':
                    buffer=io.BytesIO()
                    with zipfile.ZipFile(buffer,'w') as zipped:zipped.writestr('../outside',b'x')
                    self.archive=buffer.getvalue();self.artifact['size_in_bytes']=len(self.archive)
                    self.artifact['digest']='sha256:'+hashlib.sha256(self.archive).hexdigest()
                    selected={**selected,'production_artifact_digest':self.artifact['digest']}
                destination=self.folder/('bad-'+fault)
                with patch.object(production.subprocess,'run',side_effect=run),self.assertRaises(ValueError):
                    production.download(selected,destination,self.request)
                self.assertFalse(destination.exists());self.assertFalse((self.folder/'outside').exists())
                self.artifact,self.archive=artifact,archive
    def test_only_canonical_fresh_source_run_receives_reusable_origin_label(self):
        self.run['head_sha']=self.sha()
        self.assertTrue(production.recordable_origin(self.source,11,self.request))
        for field,value in [('head_branch','main'),('head_branch','integration/explicit-proof'),
                            ('path','.github/workflows/approved-publication-parent.yml'),
                            ('head_sha',self.previous)]:
            with self.subTest(field=field,value=value):
                original=self.run[field];self.run[field]=value
                self.assertFalse(production.recordable_origin(self.source,11,self.request))
                self.run[field]=original
        with self.assertRaises(ValueError):production.recordable_origin(self.source,False,self.request)
        text=(ROOT/'.github/workflows/termux-runtime-source.yml').read_text()
        self.assertIn('elif test "$eligible" = true; then',text)
        self.assertIn("steps.production.outputs.recorded == 'true'",text)
        self.assertIn('runtime_production.py eligible-origin',text)

    def test_current_build_records_only_its_own_uploaded_artifact(self):
        self.run['head_sha']=self.sha();self.run['status']='in_progress';self.run['conclusion']=None
        self.job['status']='in_progress';self.job['conclusion']=None
        self.artifact['workflow_run']['head_sha']=self.sha()
        receipt=production.record(self.source,self.raw,run_id=11,artifact_id=33,request=self.request)
        self.assertEqual(receipt['producer_source'],self.sha());self.assertEqual(receipt['production_artifact_id'],33)
        self.job['steps'][0]['conclusion']='skipped'
        with self.assertRaises(ValueError):production.record(self.source,self.raw,run_id=11,artifact_id=33,request=self.request)
    def test_workflow_only_reuses_authentic_archive_and_current_consumers_remain_exact(self):
        text=(ROOT/'.github/workflows/termux-runtime-source.yml').read_text()
        self.assertNotIn('termux-raw-runtime-v1-',text);self.assertNotIn('termux-raw-runtime-v2-',text)
        self.assertIn('runtime_production.py download',text);self.assertIn('runtime_artifact.py unpack',text)
        self.assertIn('runtime_inputs.py --production',text)
        self.assertLess(text.index('Verify reusable raw runtime before qualification'),
                        text.index('Record original raw runtime production identity'))
        full=(ROOT/'.github/scripts/full_qualification.py').read_text()
        self.assertIn('successful_run',full);self.assertIn('accepted_run',full)

if __name__=='__main__':unittest.main()
