"""Exact-run, archive, composition and execution-proof refusal oracles."""
import copy
import hashlib
import io
import json
import textwrap
import sys
import os
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

    def test_core_transfer_roundtrips_every_record_alignment_and_rejects_bad_tails(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);base=self.base(root/'base');ends=set()
            for index in range(20):
                with self.subTest(alignment=index):
                    payload=b'owned Core bytes'+b'x'*(index*512)
                    (base/'core').write_bytes(payload)
                    target=root/('alignment-'+str(index)+'.tar');self.pack(base,target)
                    destination=root/('out-'+str(index));full.unpack_candidate(target,destination)
                    self.assertEqual((destination/'core').read_bytes(),payload)
                    raw=target.read_bytes()
                    with tarfile.open(target,'r:') as archive:
                        last=list(archive)[-1];end=last.offset_data+(last.size+511)//512*512
                    ends.add(len(raw)-end)
            self.assertEqual(ends,set(range(1024,10753,512)))
            for fault,data in {'truncated':raw[:-512], 'second-archive':raw+raw,
                               'extra-zero-record':raw+b'\0'*10240,
                               'nonzero-ending':raw[:-1]+b'x'}.items():
                with self.subTest(fault=fault):
                    target.write_bytes(data);destination=root/('bad-tail-'+fault)
                    with self.assertRaises((ValueError,tarfile.TarError)):
                        full.unpack_candidate(target,destination)
                    self.assertFalse(destination.exists())

    def version_probe(self,path,receipt):
        body=textwrap.dedent(r"""
            import json,os,stat,sys
            from pathlib import Path
            if not os.environ.get('HOME'):
                sys.stderr.write('WARNING: proceeding, even though we could not create PATH aliases: Could not find home directory\n')
            elif Path(os.environ['HOME']).joinpath('.codex').is_relative_to(Path(os.environ['TMPDIR'])):
                sys.stderr.write('WARNING: proceeding, even though we could not create PATH aliases: '
                                 'Refusing to create helper binaries under temporary dir '+repr(os.environ['TMPDIR'])+'\n')
            else:
                home=Path(os.environ['HOME']);temporary=Path(os.environ['TMPDIR'])
                assert set(os.environ)<={'HOME','TMPDIR','LC_CTYPE'}
                for root in (home,temporary):
                    assert root==root.resolve(strict=True)
                    info=root.stat()
                    assert info.st_uid==os.getuid() and stat.S_IMODE(info.st_mode)==0o700
                assert len(os.fsencode(temporary/str(os.getuid())/('0'*64)))<=107
                (home/'owned-alias').write_text('private version probe')
                Path(@@RECEIPT@@).write_text(json.dumps({'home':str(home),'tmpdir':str(temporary),
                                                       'environment_keys':sorted(os.environ)}))
            print('codex-cli '+@@VERSION@@)
            """).replace('@@RECEIPT@@',repr(str(receipt))).replace('@@VERSION@@',repr(VERSION))
        path.write_text('#!'+sys.executable+'\n'+body);path.chmod(0o755)

    def composition(self,root,*,private_probes=False):
        base=self.base(root/'base')
        shell=shutil.which('sh')
        script={'core':"printf 'usage: codex update [--wait | --no-wait]\\n'",
                'runtime':"printf 'codex-cli "+VERSION+"\\n'"}
        metadata=base/'generation.meta'
        for name,command in script.items():
            old=sha256(base/name);(base/name).write_text('#!'+shell+'\n'+command+'\n');(base/name).chmod(0o755)
            metadata.write_text(metadata.read_text().replace(old,sha256(base/name)))
        if private_probes:
            old=sha256(base/'runtime');self.version_probe(base/'runtime',root/'runtime-env.json')
            metadata.write_text(metadata.read_text().replace(old,sha256(base/'runtime')))
        manager=root/'manager';manager.write_text('#!'+shell+"\nprintf 'codex-manager-artifact-v1\\ncore_api=codex-manager-core-v1\\n'\n");manager.chmod(0o755)
        frontend=root/'frontend';frontend.write_text('#!'+shell+"\nprintf 'codex-cli "+VERSION+"\\n'\n");frontend.chmod(0o755)
        if private_probes:self.version_probe(frontend,root/'tui-env.json')
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
        with tempfile.TemporaryDirectory() as temp, tempfile.TemporaryDirectory(prefix='.full-probe-homes-',dir=ROOT) as homes:
            root=Path(temp);_,candidate,_,_,_=self.composition(root,private_probes=True)
            inputs=root/'inputs';inputs.mkdir();(inputs/'inputs.json').write_bytes(canonical(self.resolve()))
            logs=root/'logs'
            selected={name:{'inputs_sha256':'b'*64,'artifact_sha256':sha256(candidate/path),'interface':'test-interface-v1'}
                      for name,path in {'core':'core','runtime':'runtime','manager':'manager','tui':'helpers/2'}.items()}
            with patch.object(full,'git',side_effect=self.source_git),patch.object(full,'selected',return_value=(VERSION,selected)),patch.dict(os.environ,{'PROBE_SECRET_SENTINEL':'not-a-credential'}):
                receipt=full.probe(ROOT,inputs,candidate,logs,'99',home_parent=Path(homes))
                self.assertEqual(set(receipt['exit_codes'].values()),{0})
                environments=[json.loads((root/(name+'-env.json')).read_text()) for name in ('runtime','tui')]
                self.assertEqual(environments[0],environments[1])
                probe_home=Path(environments[0]['home']);probe_tmp=Path(environments[0]['tmpdir'])
                self.assertEqual(probe_home.parent,Path(homes).resolve())
                self.assertFalse(probe_home.is_relative_to(probe_tmp))
                self.assertEqual(list(Path(homes).iterdir()),[])
                self.assertFalse(probe_home.exists())
                if probe_tmp!=Path(tempfile.gettempdir()).resolve():self.assertFalse(probe_tmp.exists())
                self.assertNotIn('PROBE_SECRET_SENTINEL',environments[0]['environment_keys'])
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

    def test_probe_environment_is_private_canonical_and_cleans_only_owned_directories(self):
        with tempfile.TemporaryDirectory() as temp, tempfile.TemporaryDirectory(prefix='.full-probe-homes-',dir=ROOT) as homes:
            root=Path(temp);logs=root/'logs';logs.mkdir(mode=0o700)
            parent=Path(tempfile.gettempdir()).resolve(strict=True)
            parent_identity=parent.stat().st_dev,parent.stat().st_ino,parent.stat().st_mode
            home_parent=Path(homes);home_parent.chmod(0o755)
            home_parent_identity=home_parent.stat().st_dev,home_parent.stat().st_ino,home_parent.stat().st_mode
            with self.assertRaisesRegex(RuntimeError,'owned failure'):
                with full.probe_environment(logs,Path(homes)) as environment:
                    probe_home=Path(environment['HOME']);probe_tmp=Path(environment['TMPDIR'])
                    self.assertEqual(set(environment),{'HOME','TMPDIR'})
                    self.assertTrue(full.owned_private_directory(probe_home))
                    self.assertTrue(full.owned_private_directory(probe_tmp))
                    self.assertLessEqual(len(os.fsencode(probe_tmp/str(os.getuid())/('0'*64))),107)
                    (probe_home/'owned').write_text('remove only this new home')
                    raise RuntimeError('owned failure')
            self.assertFalse(probe_home.exists())
            if probe_tmp!=parent:self.assertFalse(probe_tmp.exists())
            self.assertEqual((parent.stat().st_dev,parent.stat().st_ino,parent.stat().st_mode),parent_identity)
            self.assertEqual((home_parent.stat().st_dev,home_parent.stat().st_ino,home_parent.stat().st_mode),home_parent_identity)
            self.assertEqual(list(home_parent.iterdir()),[])
            self.assertEqual(list(logs.iterdir()),[])
            long_root=root/('long-'+80*'x');long_root.mkdir(mode=0o700)
            alias=root/'short';alias.symlink_to(long_root)
            for temporary in (long_root,alias):
                with self.subTest(root=temporary.name),patch.object(full.tempfile,'gettempdir',return_value=str(temporary)):
                    with self.assertRaisesRegex(ValueError,'socket root is too long'):
                        with full.probe_environment(logs,Path(homes)):self.fail('unsafe socket root accepted')
                    self.assertEqual(list(logs.iterdir()),[])
            unsafe=root/'unsafe';unsafe.mkdir(mode=0o777);unsafe.chmod(0o777)
            with patch.object(full.tempfile,'gettempdir',return_value=str(unsafe)):
                with self.assertRaisesRegex(ValueError,'temporary parent is not protected'):
                    with full.probe_environment(logs,Path(homes)):self.fail('unprotected temporary parent accepted')
            self.assertTrue(alias.is_symlink());self.assertTrue(long_root.is_dir());self.assertTrue(unsafe.is_dir())
            logs.chmod(0o755)
            with self.assertRaisesRegex(ValueError,'log root must be owned and private'):
                with full.probe_environment(logs,Path(homes)):self.fail('nonprivate log root accepted')


    def test_version_probe_preserves_both_release_arg0_home_failures(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);binary=root/'version';receipt=root/'receipt'
            self.version_probe(binary,receipt)
            home=root/'home';home.mkdir(mode=0o700)
            for environment,diagnostic in [({},b'Could not find home directory'),
                    ({'HOME':str(home),'TMPDIR':str(root)},b'Refusing to create helper binaries under temporary dir')]:
                with self.subTest(environment_keys=list(environment)):
                    result=subprocess.run([str(binary),'--version'],env=environment,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
                    self.assertEqual(result.returncode,0)
                    self.assertEqual(result.stdout,('codex-cli '+VERSION+'\n').encode())
                    self.assertIn(diagnostic,result.stderr)
                    self.assertFalse(receipt.exists());self.assertEqual(list(home.iterdir()),[])

    def test_probe_home_parent_rejects_temporary_aliases_and_unprotected_roots(self):
        with tempfile.TemporaryDirectory() as temp, tempfile.TemporaryDirectory(prefix='.full-probe-homes-',dir=ROOT) as homes:
            logs=Path(temp)/'logs';logs.mkdir(mode=0o700);home_parent=Path(homes)
            with full.probe_environment(logs,home_parent) as environment:
                temporary=Path(environment['TMPDIR'])
                with tempfile.TemporaryDirectory(prefix='home-',dir=temporary) as nested:
                    unsafe=Path(nested);alias=home_parent/'alias';alias.symlink_to(unsafe)
                    before=sorted(temporary.iterdir())
                    for selected in (temporary,unsafe,alias):
                        with self.subTest(parent=str(selected)),patch.object(full.tempfile,'gettempdir',return_value=str(temporary)):
                            with self.assertRaisesRegex(ValueError,'outside canonical temporary root'):
                                with full.probe_environment(logs,selected):self.fail('HOME beneath TMPDIR accepted')
                            self.assertEqual(list(unsafe.iterdir()),[])
                            self.assertEqual(sorted(temporary.iterdir()),before)
                    alias.unlink()
                unsafe=home_parent/'unprotected';unsafe.mkdir(mode=0o700);unsafe.chmod(0o777)
                with self.assertRaisesRegex(ValueError,'home parent must be owned and protected'):
                    with full.probe_environment(logs,unsafe):self.fail('unprotected home parent accepted')
                self.assertEqual(list(unsafe.iterdir()),[])
            self.assertTrue(home_parent.is_dir());self.assertTrue(logs.is_dir())

    def test_probe_cli_requires_explicit_home_parent_before_candidate_access(self):
        command=[sys.executable,'-B',str(ROOT/'.github/scripts/full_qualification.py'),'probe',
                 '--source',str(ROOT),'--inputs','/nonexistent-owned-inputs','--candidate','/nonexistent-owned-candidate',
                 '--logs','/nonexistent-owned-logs','--run-id','99']
        result=subprocess.run(command,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
        self.assertEqual(result.returncode,2)
        self.assertIn(b'probe requires candidate, logs, run-id and home-parent',result.stderr)

    def test_failed_component_execution_retains_unqualified_marker_and_no_receipt(self):
        with tempfile.TemporaryDirectory() as temp, tempfile.TemporaryDirectory(prefix='.full-probe-homes-',dir=ROOT) as homes:
            root=Path(temp);_,candidate,_,_,_=self.composition(root)
            manager=candidate/'manager';old=sha256(manager);manager.write_text('#!'+shutil.which('sh')+'\nexit 9\n')
            meta=candidate/'generation.meta';meta.write_text(meta.read_text().replace(old,sha256(manager)))
            inputs=root/'inputs';inputs.mkdir();(inputs/'inputs.json').write_bytes(canonical(self.resolve()))
            with patch.object(full,'git',side_effect=self.source_git),self.assertRaisesRegex(ValueError,'exited 9'):
                full.probe(ROOT,inputs,candidate,root/'logs','99',home_parent=Path(homes))
            self.assertTrue((candidate/'.manager-probe-deferred').is_file())
            self.assertFalse((root/'logs/execution.json').exists())
            self.assertEqual(list(Path(homes).iterdir()),[])

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
