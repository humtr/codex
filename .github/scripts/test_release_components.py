import copy
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

import release_components as component
import rald5_publication as publication
import test_rald5_publication as fixtures

ROOT=Path(__file__).resolve().parents[2]
BASE='local-base-relay-hosted-owned-s60'
FULL='local-relay-hosted-owned-s61'


class ComponentsTests(unittest.TestCase):
    def repository(self, root):
        root.mkdir()
        for name in ['Cargo.toml','Cargo.lock','release/upstream.json']:
            target=root/name;target.parent.mkdir(exist_ok=True,parents=True);shutil.copyfile(ROOT/name,target)
        for name in ['crates/core','crates/release-builder','crates/runtime-binding','crates/manager']:
            target=root/name;target.mkdir(parents=True);shutil.copyfile(ROOT/name/'Cargo.toml',target/'Cargo.toml')
            (target/'lib.rs').write_text('owned fixture')
        target=root/'.github/scripts';target.mkdir(parents=True)
        (target/'release_components.py').write_text('owned recipe')
        self.git(root,'init');self.git(root,'config','user.name','Fixture');self.git(root,'config','user.email','fixture@example.invalid')
        self.commit(root)
        return root

    def git(self, root, *args):
        return subprocess.check_output(['git','-C',str(root),*args],stderr=subprocess.DEVNULL).decode().strip()

    def commit(self, root):
        self.git(root,'add','.');self.git(root,'commit','-m','owned inputs')

    def test_base_fingerprint_ignores_manager_ui_and_output_refs_but_binds_real_inputs(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=self.repository(Path(tmp)/'source')
            before=component.base_fingerprint(root)
            (root/'crates/manager/lib.rs').write_text('quota implementation')
            (root/'frontend').mkdir();(root/'frontend/tui.rs').write_text('quota UI')
            recipe=root/'release/upstream.json';value=json.loads(recipe.read_text());value['frontend']=None
            recipe.write_text(json.dumps(value))
            # Manager-only new lock dependencies must not invalidate Core's closure.
            lock=root/'Cargo.lock';lock.write_text(lock.read_text()+'\n[[package]]\nname="owned-manager-only"\nversion="1.0.0"\n')
            self.commit(root)
            self.assertEqual(component.base_fingerprint(root),before)
            value['toolchains']['frontend_rust']='1.96.0';recipe.write_text(json.dumps(value));self.commit(root)
            after=component.base_fingerprint(root);self.assertNotEqual(after,before);before=after
            for name in ['crates/core/lib.rs','crates/release-builder/lib.rs','crates/runtime-binding/lib.rs']:
                (root/name).write_text('changed '+name);self.commit(root)
                after=component.base_fingerprint(root);self.assertNotEqual(after,before);before=after
            value['upstream']['archive_sha256']='f'*64;recipe.write_text(json.dumps(value));self.commit(root)
            self.assertNotEqual(component.base_fingerprint(root),before)

    def signed_assets(self, fixture, repo, generation, base=False, changes=None):
        files=[row for row in fixtures.FILES if not base or row[0]!='manager']
        with patch.object(fixtures,'GENERATION',generation),patch.object(fixtures,'BASE',publication.canonical_release_base(generation)),patch.object(fixtures,'FILES',files):
            fixture.build_signed(frontend=not base)
            if changes:
                release=fixture.signed/'releases'/generation
                manifest=release/'release.manifest'
                old_manifest=publication.sha256(manifest)
                text=manifest.read_text()
                for name,data in changes.items():
                    path=release/name
                    self.assertEqual(len(data),path.stat().st_size)
                    old=publication.sha256(path);path.write_bytes(data)
                    text=text.replace('file\t'+name+'\t'+old+'\t','file\t'+name+'\t'+publication.sha256(path)+'\t')
                manifest.write_text(text)
                fixtures.openssl('pkeyutl','-sign','-rawin','-inkey',str(fixture.private_key),
                    '-in',str(manifest),'-out',str(release/'release.sig'))
                sizes=fixture.signed/'download-size-v1'
                sizes.write_text(sizes.read_text().replace(old_manifest,publication.sha256(manifest)))
                fixtures.openssl('pkeyutl','-sign','-rawin','-inkey',str(fixture.private_key),
                    '-in',str(sizes),'-out',str(fixture.signed/'download-size-v1.sig'))
            component.sign_component(repo,fixture.signed,generation,fixture.private_key,Path(shutil.which('openssl')))
            controls=repo.parent/('controls-'+generation);controls.mkdir()
            for name in ['release.manifest','release.sig']:
                shutil.copyfile(fixture.signed/'releases'/generation/name,controls/name)
            for name in [component.COMPONENT,component.COMPONENT+'.sig']:
                shutil.copyfile(fixture.signed/name,controls/name)
            assets=repo.parent/('assets-'+generation)
            result=fixture.prepare(assets);self.assertEqual(result.returncode,0,result.stderr.decode())
            if base:
                component.compose(assets,None,fixture.public_key)
                shutil.copyfile(assets/component.MAP,controls/component.MAP)
        return assets,controls

    def test_signed_base_and_addons_reconstruct_complete_full_and_reject_wrong_base(self):
        fixture=fixtures.Rald5PublicationTests();fixture.setUp();self.addCleanup(fixture.tearDown)
        repo=self.repository(fixture.root/'source')
        base,controls=self.signed_assets(fixture,repo,BASE,True)
        full,_=self.signed_assets(fixture,repo,FULL)
        original={path.name:path.read_bytes() for path in full.iterdir()}
        with self.assertRaisesRegex(ValueError,'requires an authenticated'):
            component.compose(full,None,fixture.public_key)
        (full/'core').write_bytes(b'tampered')
        with self.assertRaisesRegex(ValueError,'digest differs'):
            component.compose(full,controls,fixture.public_key)
        (full/'core').write_bytes(original['core'])
        mapping=component.compose(full,controls,fixture.public_key)
        self.assertFalse((full/'runtime').exists());self.assertFalse((full/'core').exists())
        self.assertTrue((full/'manager').is_file());self.assertTrue((full/'helper-2').is_file())
        self.assertTrue(all(mapping['files'][name]['generation']==BASE for name in component.COMMON))
        # Reconstruct through the real map, then run the same full public validator.
        public=fixture.root/'public';target=public/FULL;target.mkdir(parents=True)
        for relative,route in mapping['files'].items():
            if relative.startswith('component-') or relative=='update-public-key.pem':continue
            source=base if route['generation']==BASE else full
            destination=target/relative;destination.parent.mkdir(exist_ok=True,parents=True)
            shutil.copyfile(source/route['asset'],destination)
        _,records=publication.parse_manifest(target/'release.manifest')
        for name,_,mode in records:(target/name).chmod(int(mode,8))
        result=fixtures.run('verify-public','--root',str(public),'--public-key',str(fixture.public_key),
            '--generation',FULL,'--release-sequence',fixtures.SEQUENCE,
            '--release-base',publication.canonical_release_base(FULL),'--require-download-size')
        self.assertEqual(result.returncode,0,result.stderr.decode())
        for fault in ['foreign','inheritance','omitted','extra','cycle','identity']:
            changed=copy.deepcopy(mapping)
            if fault=='foreign':changed['files']['runtime']['generation']='other/repo'
            elif fault=='inheritance':changed['files']['generation.meta']['generation']=BASE
            elif fault=='omitted':del changed['files']['core']
            elif fault=='extra':changed['files']['helpers/9']=changed['files']['helpers/0']
            elif fault=='cycle':changed['base_generation']=FULL
            else:changed['generation_id']=BASE
            with self.assertRaises(ValueError):component.validate_map(changed,FULL)

    def test_relay_origin_does_not_rewrite_historical_urls(self):
        self.assertEqual(publication.canonical_release_base(BASE),'https://codex-releases.humtr.workers.dev/'+BASE+'/')
        self.assertEqual(publication.canonical_release_base('local-hosted-old'),'https://humtr.github.io/codex/local-hosted-old/')
        for invalid in ['local-../escape','other','local-a\n']:
            with self.assertRaises(publication.ValidationError):publication.canonical_release_base(invalid)

    def test_changed_payload_publication_flattens_base_and_reuses_unchanged_addons(self):
        fixture=fixtures.Rald5PublicationTests();fixture.setUp();self.addCleanup(fixture.tearDown)
        repo=self.repository(fixture.root/'source')
        old_base_id='local-base-relay-hosted-old-s10'
        old_full_id='local-relay-hosted-old-s11'
        new_base_id='local-base-relay-hosted-new-s12'
        new_full_id='local-relay-hosted-new-s13'
        with patch.object(fixtures,'SEQUENCE','10'):
            old_base,old_base_controls=self.signed_assets(fixture,repo,old_base_id,True)
        with patch.object(fixtures,'SEQUENCE','11'):
            old_full,old_full_controls=self.signed_assets(fixture,repo,old_full_id)
        component.compose(old_full,old_base_controls,fixture.public_key)
        shutil.copyfile(old_full/component.MAP,old_full_controls/component.MAP)
        (repo/'crates/core/lib.rs').write_text('changed Core input');self.commit(repo)
        changed={'core':b'changed:core\n'}
        with patch.object(fixtures,'SEQUENCE','12'):
            new_base,new_base_controls=self.signed_assets(fixture,repo,new_base_id,True,changed)
        old_transport=old_base_controls/component.MAP;old_mapping=old_transport.read_bytes()
        old_transport.unlink()
        missing=[]
        def absent(url,target,limit,*,optional):
            self.assertTrue(optional);missing.append(url);return False
        retained=component.compose(new_base,None,fixture.public_key,old_base_controls,absent)
        self.assertEqual(missing,[publication.canonical_release_base(old_base_id)+component.MAP])
        self.assertEqual(retained['files']['runtime']['generation'],new_base_id)
        self.assertTrue((new_base/'runtime').is_file())
        old_transport.write_bytes(old_mapping)
        protected=fixture.root/'unrelated-user-data';protected.write_bytes(b'preserve this content')
        (new_base/component.MAP).unlink();(new_base/component.MAP).symlink_to(protected)
        result=subprocess.run([sys.executable,str(ROOT/'.github/scripts/release_components.py'),'compose',
            '--assets',str(new_base),'--authority',str(fixture.public_key),'--previous',str(old_base_controls)],capture_output=True)
        self.assertEqual(result.returncode,0,result.stderr.decode())
        self.assertEqual(protected.read_bytes(),b'preserve this content')
        self.assertFalse((new_base/component.MAP).is_symlink())
        base_map=json.loads((new_base/component.MAP).read_bytes())
        self.assertEqual(base_map['files']['core']['generation'],new_base_id)
        self.assertEqual(base_map['files']['runtime']['generation'],old_base_id)
        self.assertFalse((new_base/'runtime').exists())
        shutil.copyfile(new_base/component.MAP,new_base_controls/component.MAP)
        with patch.object(fixtures,'SEQUENCE','13'):
            new_full,_=self.signed_assets(fixture,repo,new_full_id,changes=changed)
        before={path.name:path.read_bytes() for path in new_full.iterdir()}
        signature=old_full_controls/'release.sig';original=signature.read_bytes()
        signature.write_bytes(bytes(64))
        with self.assertRaises(publication.ValidationError):
            component.compose(new_full,new_base_controls,fixture.public_key,old_full_controls)
        signature.write_bytes(original)
        transport=old_full_controls/component.MAP;original=transport.read_bytes()
        bad=json.loads(original);bad['manifest_sha256']='f'*64;transport.write_text(json.dumps(bad))
        with self.assertRaisesRegex(ValueError,'manifest differs'):
            component.compose(new_full,new_base_controls,fixture.public_key,old_full_controls)
        self.assertEqual({path.name:path.read_bytes() for path in new_full.iterdir()},before)
        transport.write_bytes(original)
        base_transport=new_base_controls/component.MAP;original=base_transport.read_bytes()
        bad=json.loads(original);bad['files']['runtime']['generation']=new_full_id
        base_transport.write_text(json.dumps(bad))
        with self.assertRaisesRegex(ValueError,'route points to candidate'):
            component.compose(new_full,new_base_controls,fixture.public_key,old_full_controls)
        self.assertEqual({path.name:path.read_bytes() for path in new_full.iterdir()},before)
        base_transport.write_bytes(original)
        result=subprocess.run([sys.executable,str(ROOT/'.github/scripts/release_components.py'),'compose',
            '--assets',str(new_full),'--authority',str(fixture.public_key),'--base',str(new_base_controls),
            '--previous',str(old_full_controls)],capture_output=True)
        self.assertEqual(result.returncode,0,result.stderr.decode())
        mapping=json.loads((new_full/component.MAP).read_bytes())
        self.assertEqual(mapping['files']['runtime']['generation'],old_base_id)
        self.assertEqual(mapping['files']['core']['generation'],new_base_id)
        for path in ['manager','helpers/2']:
            self.assertEqual(mapping['files'][path]['generation'],old_full_id)
            self.assertFalse((new_full/publication.FLAT_ASSETS[path]).exists())
        self.assertEqual(mapping['files']['generation.meta']['generation'],new_full_id)
        origins={old_base_id:old_base,old_full_id:old_full,new_base_id:new_base,new_full_id:new_full}
        public=fixture.root/'public';target=public/new_full_id;target.mkdir(parents=True)
        for path,route in mapping['files'].items():
            if path.startswith('component-') or path=='update-public-key.pem':continue
            destination=target/path;destination.parent.mkdir(parents=True,exist_ok=True)
            shutil.copyfile(origins[route['generation']]/route['asset'],destination)
        _,records=publication.parse_manifest(target/'release.manifest')
        for path,digest,mode in records:
            self.assertEqual(publication.sha256(target/path),digest)
            (target/path).chmod(int(mode,8))
        result=fixtures.run('verify-public','--root',str(public),'--public-key',str(fixture.public_key),
            '--generation',new_full_id,'--release-sequence','13',
            '--release-base',publication.canonical_release_base(new_full_id),'--require-download-size')
        self.assertEqual(result.returncode,0,result.stderr.decode())

    def test_display_metadata_is_signed_manifest_bound_and_matches_actual_base(self):
        fixture=fixtures.Rald5PublicationTests();fixture.setUp();self.addCleanup(fixture.tearDown)
        repo=self.repository(fixture.root/'source')
        with patch.object(fixtures,'SEQUENCE','10'):
            _,base=self.signed_assets(fixture,repo,BASE,True)
        full,_=self.signed_assets(fixture,repo,FULL)
        component.sign_component(repo,fixture.signed,FULL,fixture.private_key,Path(shutil.which('openssl')),base,fixture.public_key)
        for name in [component.COMPONENT,component.COMPONENT+'.sig']:
            shutil.copyfile(fixture.signed/name,full/name)
        value=component.component(full,FULL,fixture.public_key)
        self.assertEqual(value['display']['base_generation'],BASE)
        self.assertEqual(value['display']['base_sequence'],10)
        self.assertRegex(value['display']['release_date'],r'^\d{4}-\d{2}-\d{2}$')
        # Even independently valid display provenance cannot name another base
        # when assembling the actual immutable composition.
        with patch.object(fixtures,'SEQUENCE','9'):
            _,other=self.signed_assets(fixture,repo,'local-base-relay-hosted-other-s9',True)
        with self.assertRaisesRegex(ValueError,'display base differs'):
            component.compose(full,other,fixture.public_key)
        component.compose(full,base,fixture.public_key)

    def test_failed_full_repair_acquires_latest_base_not_the_serving_full(self):
        fixture=fixtures.Rald5PublicationTests();fixture.setUp();self.addCleanup(fixture.tearDown)
        repo=self.repository(fixture.root/'source')
        assets,_=self.signed_assets(fixture,repo,BASE,True)
        heads=fixture.root/'heads';heads.mkdir()
        (heads/'heads.json').write_text(json.dumps({'heads':{
            'base':{'generation':BASE},'full':{'generation':'local-hosted-previous-full'}}}))
        requested=[]
        def fetch(url,target,limit):
            self.assertTrue(url.startswith(publication.canonical_release_base(BASE)))
            requested.append(url)
            name=url.removeprefix(publication.canonical_release_base(BASE))
            source=assets/component.CONTROL_ASSETS.get(name,publication.FLAT_ASSETS.get(name,name))
            self.assertLessEqual(source.stat().st_size,limit)
            target.parent.mkdir(parents=True,exist_ok=True)
            shutil.copyfile(source,target)
        acquired=fixture.root/'latest-base'
        self.assertEqual(component.acquire_base(repo,heads,fixture.public_key,acquired,fetch),BASE)
        self.assertTrue((acquired/'runtime').is_file())
        self.assertEqual(len(requested),4+len(publication.BASE_FILES))
        # Changing Core invalidates this base. Never silently use the serving
        # full's older base or rebuild an unqualified replacement instead.
        (repo/'crates/core/lib.rs').write_text('new incompatible Core');self.commit(repo)
        self.assertIsNone(component.acquire_base(repo,heads,fixture.public_key,fixture.root/'incompatible',fetch))

    def test_core_only_change_reuses_signed_runtime_without_fetching_old_core(self):
        from test_base_qualification import BaseQualificationTests
        from rald3_preflight import parse_descriptor, verify_base_candidate
        fixture=fixtures.Rald5PublicationTests();fixture.setUp();self.addCleanup(fixture.tearDown)
        repo=self.repository(fixture.root/'runtime-source')
        for name in ['release/runtime-build-inputs-v1','.github/scripts/runtime_inputs.py',
                     'crates/release-builder/src/source.rs', 'crates/release-builder/src/lib.rs','crates/release-builder/src/runtime.rs','crates/release-builder/Cargo.toml','.github/scripts/upstream_release.py',
                     '.github/workflows/termux-runtime-source.yml']:
            target=repo/name;target.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(ROOT/name,target)
        published=fixture.root/'runtime-publication';base=published/'releases'/BASE;base.mkdir(parents=True)
        BaseQualificationTests().fixture(base)
        descriptor=base/'generation.meta'
        descriptor.write_text(descriptor.read_text().replace('base-qualification-owned',BASE))
        fields=parse_descriptor(descriptor)
        recipe_path=repo/'release/upstream.json';recipe=json.loads(recipe_path.read_text())
        recipe['upstream'].update(version=fields['upstream_package_version'],
                                  archive_sha256=fields['source_artifact_digest'])
        recipe_path.write_text(json.dumps(recipe));self.commit(repo)
        original=fixture.signed/'releases'/fixtures.GENERATION/'release.manifest'
        header=original.read_text().split('file_count\t',1)[0].replace(fixtures.GENERATION,BASE)
        (base/'release.manifest').write_text(header+'file_count\t'+str(len(publication.BASE_FILES))+'\n'+''.join(
            'file\t'+name+'\t'+component.sha256(base/name)+'\t'+mode+'\n'
            for name,mode in sorted(publication.BASE_FILES.items())))
        fixtures.openssl('pkeyutl','-sign','-rawin','-inkey',str(fixture.private_key),
                         '-in',str(base/'release.manifest'),'-out',str(base/'release.sig'))
        component.sign_component(repo,published,BASE,fixture.private_key,Path(shutil.which('openssl')))
        heads=fixture.root/'runtime-heads';heads.mkdir()
        (heads/'heads.json').write_text(json.dumps({'heads':{'base':{'generation':BASE}}}))
        requested=[]
        def fetch(url,target,limit):
            name=url.removeprefix(publication.canonical_release_base(BASE))
            self.assertNotEqual(name,url)
            requested.append(name)
            source=published/name if name.startswith('component-input-') else base/name
            self.assertLessEqual(source.stat().st_size,limit)
            target.parent.mkdir(exist_ok=True,parents=True);shutil.copyfile(source,target)
        (repo/'crates/core/lib.rs').write_text('changed Core, same runtime');self.commit(repo)
        reused=fixture.root/'runtime-reused'
        self.assertEqual(component.acquire_runtime_base(repo,heads,fixture.public_key,reused,fetch),BASE)
        self.assertNotIn('core',requested)
        self.assertFalse((reused/'core').exists())
        replacement=fixture.root/'new-core';replacement.write_bytes(b'new executable Core');replacement.chmod(0o755)
        output=fixture.root/'runtime-composed'
        component.candidate_from_base(reused,None,None,None,'base-qualification-reused',
            fields['upstream_package_version'],output,core=replacement)
        actual=verify_base_candidate(output,fields['upstream_package_version'])
        self.assertEqual(actual['core_artifact_digest'],component.sha256(replacement))
        for name in set(publication.BASE_FILES)-{'core','generation.meta'}:
            self.assertEqual((output/name).read_bytes(),(base/name).read_bytes(),name)
        # The real executor CLI reauthenticates the transferred subset with its
        # own authority/source context, rather than trusting the prior job's flag.
        cli_output=fixture.root/'runtime-cli-composed'
        command=['python3',str(ROOT/'.github/scripts/release_components.py'),'candidate-base',
            '--base',str(reused),'--core',str(replacement),'--source',str(repo),
            '--authority',str(fixture.public_key),'--base-generation',BASE,
            '--generation','base-qualification-cli','--version',fields['upstream_package_version'],
            '--output',str(cli_output)]
        result=subprocess.run(command,capture_output=True)
        self.assertEqual(result.returncode,0,result.stderr.decode())
        self.assertEqual(verify_base_candidate(cli_output,fields['upstream_package_version'])['runtime_digest'],fields['runtime_digest'])
        signature=(reused/'release.sig').read_bytes();(reused/'release.sig').write_bytes(bytes(64))
        rejected=fixture.root/'runtime-rejected-after-transfer'
        command[-1]=str(rejected)
        result=subprocess.run(command,capture_output=True)
        self.assertNotEqual(result.returncode,0)
        self.assertFalse(rejected.exists())
        (reused/'release.sig').write_bytes(signature)
        # A semantic preparation change is a real runtime cache miss: fetch only
        # authenticated controls and never download/execute a mismatching runtime.
        path=repo/'crates/release-builder/src/source.rs';path.write_text(path.read_text()+'\n// new policy\n');self.commit(repo)
        requested.clear()
        self.assertIsNone(component.acquire_runtime_base(repo,heads,fixture.public_key,fixture.root/'runtime-miss',fetch))
        self.assertNotIn('runtime',requested)
        # Failed authentication is an error, not a cache miss or permission to build.
        (base/'release.sig').write_bytes(bytes(64));requested.clear()
        with self.assertRaises(publication.ValidationError):
            component.acquire_runtime_base(repo,heads,fixture.public_key,fixture.root/'runtime-invalid',fetch)
        self.assertNotIn('runtime',requested)

    def test_worker_executes_positive_and_negative_redirects_without_proxying_payloads(self):
        node=shutil.which('node')
        self.assertIsNotNone(node,'Node is required for actual redirect module qualification')
        result=subprocess.run([node,str(ROOT/'release/test_asset_redirect.mjs')],capture_output=True,text=True)
        self.assertEqual(result.returncode,0,result.stderr)
        self.assertIn('redirect tests passed',result.stdout)


if __name__=='__main__':unittest.main()
