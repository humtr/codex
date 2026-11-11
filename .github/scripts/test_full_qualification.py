"""Exact-run, archive, composition and execution-proof refusal oracles."""
import copy
import hashlib
import io
import json
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import unittest
from unittest.mock import patch
import full_qualification as full
from component_revisions import candidate_digest
from release_components import candidate_from_base
from release_preflight import parse_descriptor,PreflightError
from release_publication import BASE_FILES,sha256
import test_base_qualification as base_fixtures
from runtime_inputs import canonical

ROOT=Path(__file__).resolve().parents[2]
SHA='a'*40
VERSION='0.161.0'

class FullQualificationTests(unittest.TestCase):
    def source_git(self,source,*args):
        return (SHA+'\n').encode() if args[0]=='rev-parse' else json.dumps({'upstream':{'version':VERSION}}).encode()

    def metadata(self,name):
        return {'id':{'core':11,'manager':12,'tui':13}[name],'path':'.github/workflows/'+full.WORKFLOWS[name],
            'head_sha':SHA,'head_branch':'rewrite/rust-core','event':'push','status':'completed',
            'conclusion':'success','display_title':'Completed qualification','repository':{'full_name':'humtr/codex'}}

    def resolve(self,mutation=None):
        def lookup(workflow,sha):
            self.assertEqual(sha,SHA)
            name=next(name for name,value in full.WORKFLOWS.items() if value==workflow)
            run=self.metadata(name)
            if mutation:mutation('run',name,run)
            return run,[run]
        def request(path):
            number=int(path.split('/')[2])
            names={11:['android-core','base-qualified-0-161-0-'+SHA[:12]+'-s166'],
                   12:['android-manager','manager-qualification'],13:['manager-tui-native','manager-tui-native-proof']}[number]
            value={'total_count':2,'artifacts':[{'id':index+1,'name':name,'expired':False} for index,name in enumerate(names)]}
            if mutation:mutation('artifact',number,value)
            return value
        with patch.object(full,'git',side_effect=self.source_git):return full.resolve(ROOT,request,lookup)

    def test_only_completed_owned_exact_source_components_are_admitted(self):
        plan=self.resolve()
        with patch.object(full,'git',side_effect=self.source_git):self.assertEqual(full.admit_plan(ROOT,plan),plan)
        for key,value in [('head_sha','b'*40),('head_branch','main'),('status','in_progress'),
                          ('conclusion','failure'),('event','pull_request'),('display_title','Focused Native smoke')]:
            def mutate(kind,name,data):
                if kind=='run' and name=='tui':data[key]=value
            with self.subTest(key=key),self.assertRaises(ValueError):self.resolve(mutate)
        for fault in ('expired','missing','duplicate','incomplete'):
            def mutate(kind,name,data):
                if kind!='artifact' or name!=12:return
                if fault=='expired':data['artifacts'][0]['expired']=True
                elif fault=='missing':data['artifacts'][0]['name']='another-component'
                elif fault=='duplicate':data['artifacts'].append(dict(data['artifacts'][0]));data['total_count']=3
                else:data['total_count']=3
            with self.subTest(fault=fault),self.assertRaises(ValueError):self.resolve(mutate)
        broken=copy.deepcopy(plan);broken['artifacts']['android-manager']['run_id']=11
        with patch.object(full,'git',side_effect=self.source_git),self.assertRaises(ValueError):full.admit_plan(ROOT,broken)

    def base(self,root):
        root.mkdir();base_fixtures.BaseQualificationTests().fixture(root)
        return root

    def pack(self,root,target,extra=None):
        with tarfile.open(target,'w',format=tarfile.USTAR_FORMAT) as archive:
            archive.add(root,arcname='candidate')
            if extra:archive.addfile(*extra)

    def test_complete_unsigned_core_transfer_preserves_modes_without_general_extraction(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);base=self.base(root/'base');archive=root/'candidate.tar'
            self.pack(base,archive);full.unpack_candidate(archive,root/'out')
            self.assertEqual(candidate_digest(base),candidate_digest(root/'out'))
            with self.assertRaises(ValueError):full.unpack_candidate(archive,root/'out')
            for fault in ('traversal','symlink','duplicate','mode','extra','trailing'):
                target=root/(fault+'.tar');member=tarfile.TarInfo('candidate/../../outside');member.mode=0o755
                stream=io.BytesIO(b'')
                if fault=='symlink':member.name='candidate/extra';member.type=tarfile.SYMTYPE;member.linkname='/outside'
                elif fault=='duplicate':member.name='candidate/core'
                elif fault=='extra':member.name='candidate/unselected'
                if fault=='mode':
                    (base/'core').chmod(0o644);self.pack(base,target);(base/'core').chmod(0o755)
                elif fault=='trailing':self.pack(base,target);target.write_bytes(target.read_bytes()+b'not padding')
                else:self.pack(base,target,(member,stream))
                with self.subTest(fault=fault),self.assertRaises((ValueError,tarfile.TarError)):
                    full.unpack_candidate(target,root/('out-'+fault))
                self.assertFalse((root/('out-'+fault)).exists())
            self.assertFalse((root/'outside').exists())

    def composition(self,root):
        base=self.base(root/'base')
        shell=shutil.which('sh')
        script={'core':"printf 'usage: codex update [--wait | --no-wait]\\n'",
                'runtime':"printf 'codex-cli "+VERSION+"\\n'"}
        metadata=base/'generation.meta'
        for name,command in script.items():
            old=sha256(base/name);(base/name).write_text('#!'+shell+'\n'+command+'\n');(base/name).chmod(0o755)
            metadata.write_text(metadata.read_text().replace(old,sha256(base/name)))
        manager=root/'manager';manager.write_text('#!'+shell+"\nprintf 'codex-manager-artifact-v1\\ncore_api=codex-manager-core-v1\\n'\n");manager.chmod(0o755)
        frontend=root/'frontend';frontend.write_text('#!'+shell+"\nprintf 'codex-cli "+VERSION+"\\n'\n");frontend.chmod(0o755)
        builder=root/'builder'
        builder.write_text('#!'+shutil.which('python3')+'\nimport pathlib,shutil,sys\na=dict(zip(sys.argv[2::2],sys.argv[3::2]))\nshutil.copyfile(a["--artifact"],a["--output"])\npathlib.Path(a["--output"]).chmod(0o755)\n');builder.chmod(0o755)
        output=root/'full'
        before=candidate_digest(base)
        candidate_from_base(base,manager,frontend,builder,'full-qualification-'+VERSION.replace('.','-')+'-'+SHA[:12],VERSION,output,qualified=True)
        self.assertEqual(candidate_digest(base),before)
        return base,output,manager,frontend,builder

    def test_unsigned_full_cannot_become_publishable_or_omit_full_integrity(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);base,full_root,manager,frontend,builder=self.composition(root)
            with self.assertRaises(PreflightError):full.verify_full(full_root,VERSION)
            full.verify_full(full_root,VERSION,deferred=True)
            for identity in ('full-v0.161.0-aaaaaaa-s166','core-v0.161.0-aaaaaaa-s166'):
                with self.assertRaises(ValueError):
                    candidate_from_base(base,manager,frontend,builder,identity,VERSION,root/'invalid',qualified=True)
            (full_root/'.manager-probe-deferred').unlink()
            self.assertEqual(full.verify_full(full_root,VERSION)['helper_count'],'3')
            for name in ('helpers/0','helpers/1','helpers/2','codex-code-mode-host','manager','core','runtime'):
                path=full_root/name;old=path.read_bytes();path.write_bytes(b'changed')
                with self.subTest(name=name),self.assertRaises((ValueError,PreflightError)):full.verify_full(full_root,VERSION)
                path.write_bytes(old)
            (full_root/'undeclared').write_text('extra')
            with self.assertRaises(ValueError):full.verify_full(full_root,VERSION)

    def test_actual_process_results_and_exact_candidate_are_bound_before_full_proof(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);_,candidate,_,_,_=self.composition(root)
            inputs=root/'inputs';inputs.mkdir();(inputs/'inputs.json').write_bytes(canonical(self.resolve()))
            logs=root/'logs'
            selected={name:{'inputs_sha256':'b'*64,'artifact_sha256':sha256(candidate/path),'interface':'test-interface-v1'}
                      for name,path in {'core':'core','runtime':'runtime','manager':'manager','tui':'helpers/2'}.items()}
            with patch.object(full,'git',side_effect=self.source_git),patch.object(full,'selected',return_value=(VERSION,selected)):
                receipt=full.probe(ROOT,inputs,candidate,logs,'99')
                self.assertEqual(set(receipt['exit_codes'].values()),{0})
                proof=full.prove(ROOT,inputs,candidate,logs,'99')
                self.assertEqual(proof['source_sha'],SHA);self.assertEqual(proof['component_runs'],{'core':11,'manager':12,'tui':13})
                self.assertEqual(set(proof['inventory']),set(full.EXTENDED_FILES))
                run={'id':99,'path':'.github/workflows/full-qualification.yml','head_sha':SHA,'head_branch':'rewrite/rust-core',
                     'event':'push','status':'completed','conclusion':'success','repository':{'full_name':'humtr/codex'}}
                full.admit_published_candidate(ROOT,candidate,proof,run)
                meta=candidate/'generation.meta';original_meta=meta.read_text()
                meta.write_text(original_meta.replace(proof['generation_id'],'full-v0.161.0-aaaaaaa-s166'))
                # Publication changes only the new generation identity; existing
                # bytes and the normalized descriptor must match the proof.
                full.admit_published_candidate(ROOT,candidate,proof,run)
                for fault in ('source','run','payload','descriptor','composition','pending'):
                    bad=copy.deepcopy(proof);bad_run=dict(run)
                    if fault=='source':bad['source_sha']='b'*40
                    elif fault=='run':bad['run_id']='100'
                    elif fault=='payload':bad['inventory']['runtime']['sha256']='c'*64
                    elif fault=='descriptor':bad['descriptor_sha256']='d'*64
                    elif fault=='composition':bad['composition_sha256']='e'*64
                    else:bad_run['status']='in_progress'
                    with self.subTest(fault=fault),self.assertRaises(ValueError):full.admit_published_candidate(ROOT,candidate,bad,bad_run)
                meta.write_text(original_meta)
                with self.assertRaises(ValueError):full.prove(ROOT,inputs,candidate,logs,'100')
                original=(logs/'runtime.out').read_bytes();(logs/'runtime.out').write_text('codex-cli wrong\n')
                with self.assertRaises(ValueError):full.prove(ROOT,inputs,candidate,logs,'99')
                (logs/'runtime.out').write_bytes(original);(logs/'manager.err').write_text('actual fault\n')
                with self.assertRaises(ValueError):full.prove(ROOT,inputs,candidate,logs,'99')

    def test_failed_component_execution_retains_unqualified_marker_and_no_receipt(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);_,candidate,_,_,_=self.composition(root)
            manager=candidate/'manager';old=sha256(manager);manager.write_text('#!'+shutil.which('sh')+'\nexit 9\n')
            meta=candidate/'generation.meta';meta.write_text(meta.read_text().replace(old,sha256(manager)))
            inputs=root/'inputs';inputs.mkdir();(inputs/'inputs.json').write_bytes(canonical(self.resolve()))
            with patch.object(full,'git',side_effect=self.source_git),self.assertRaisesRegex(ValueError,'exited 9'):
                full.probe(ROOT,inputs,candidate,root/'logs','99')
            self.assertTrue((candidate/'.manager-probe-deferred').is_file())
            self.assertFalse((root/'logs/execution.json').exists())

    def test_manager_proof_cannot_substitute_a_build_or_another_accountable_run(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);(root/'manager-proof').mkdir()
            plan=self.resolve();manager={'artifact_sha256':'a'*64}
            value={'schema':'codex-component-qualification-v1','component':manager,'qualification_source':SHA,
                'run_id':'12','checks':['android-interface-probe','manager-behavior-regressions'],
                'probe_sha256':hashlib.sha256(b'codex-manager-artifact-v1\ncore_api=codex-manager-core-v1\n').hexdigest()}
            path=root/'manager-proof/manager-qualification.json';path.write_bytes(canonical(value))
            full.manager_proof(ROOT,plan,root,manager)
            for change in ({'qualification_source':'b'*40},{'run_id':'13'},{'checks':['build-only']},{'component':{}}):
                path.write_bytes(canonical({**value,**change}))
                with self.assertRaises(ValueError):full.manager_proof(ROOT,plan,root,manager)

    def test_absent_full_proof_never_downloads_or_admits_a_release(self):
        with tempfile.TemporaryDirectory() as temp,patch.object(full,'git',side_effect=self.source_git):
            with self.assertRaisesRegex(ValueError,'successful exact-source'):
                full.require_qualified_full(ROOT,Path(temp),Path(temp)/'proof',lookup=lambda *args:(None,[]),
                    request=lambda *args:self.fail('unexpected API'),fetch=lambda *args:self.fail('unexpected download'))
            self.assertFalse((Path(temp)/'proof').exists())

    def test_full_workflow_has_only_readonly_qualification_authority(self):
        workflow=(ROOT/'.github/workflows/full-qualification.yml').read_text()
        self.assertIn('branches: [rewrite/rust-core]',workflow)
        self.assertNotIn('    paths:',workflow.split('permissions:',1)[0])
        self.assertIn('contents: read',workflow);self.assertIn('actions: read',workflow)
        for forbidden in ('contents: write','actions: write','secrets:','release_signing','release_reservations','git push','gh release','--publish'):
            self.assertNotIn(forbidden,workflow)
        for stage in ('acquire','assemble','probe','prove'):
            self.assertIn('full_qualification.py '+stage,workflow)
        self.assertNotIn('cargo-ndk',workflow)

if __name__=='__main__':unittest.main()
