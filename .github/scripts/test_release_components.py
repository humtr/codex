import copy
import json
import os
from pathlib import Path
import shutil
import subprocess
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

    def signed_assets(self, fixture, repo, generation, base=False):
        files=[row for row in fixtures.FILES if not base or row[0]!='manager']
        with patch.object(fixtures,'GENERATION',generation),patch.object(fixtures,'BASE',publication.canonical_release_base(generation)),patch.object(fixtures,'FILES',files):
            fixture.build_signed(frontend=not base)
            component.sign_component(repo,fixture.signed,generation,fixture.private_key,Path(shutil.which('openssl')))
            controls=repo.parent/('controls-'+generation);controls.mkdir()
            for name in ['release.manifest','release.sig']:
                shutil.copyfile(fixture.signed/'releases'/generation/name,controls/name)
            for name in [component.COMPONENT,component.COMPONENT+'.sig']:
                shutil.copyfile(fixture.signed/name,controls/name)
            assets=repo.parent/('assets-'+generation)
            result=fixture.prepare(assets);self.assertEqual(result.returncode,0,result.stderr.decode())
        return assets,controls

    def test_signed_base_and_addons_reconstruct_complete_full_and_reject_wrong_base(self):
        fixture=fixtures.Rald5PublicationTests();fixture.setUp();self.addCleanup(fixture.tearDown)
        repo=self.repository(fixture.root/'source')
        base,controls=self.signed_assets(fixture,repo,BASE,True)
        component.compose(base,None,fixture.public_key)
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
            elif fault=='inheritance':changed['files']['manager']['generation']=BASE
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

    def test_worker_executes_positive_and_negative_redirects_without_proxying_payloads(self):
        node=shutil.which('node')
        self.assertIsNotNone(node,'Node is required for actual redirect module qualification')
        result=subprocess.run([node,str(ROOT/'release/test_asset_redirect.mjs')],capture_output=True,text=True)
        self.assertEqual(result.returncode,0,result.stderr)
        self.assertIn('redirect tests passed',result.stdout)


if __name__=='__main__':unittest.main()
