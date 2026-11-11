import json
from pathlib import Path
import shutil
import struct
import subprocess
import tempfile
import unittest
import component_inputs as inputs
import component_artifact as artifact
from runtime_inputs import assets

ROOT=Path(__file__).resolve().parents[2]

class ComponentInputsTests(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root=Path(self.temp.name)/'source';self.root.mkdir()
        names={'Cargo.toml','Cargo.lock','release/upstream.json',*inputs.PRODUCER_INPUTS,
               *assets((ROOT/'release/runtime-build-inputs-v1').read_text())}
        for directory in ['crates/core','crates/manager','crates/release-builder','crates/runtime-binding']:
            shutil.copytree(ROOT/directory,self.root/directory)
        for name in names:
            target=self.root/name;target.parent.mkdir(exist_ok=True,parents=True)
            shutil.copyfile(ROOT/name,target)
        self.git('init','-q');self.commit()

    def git(self,*args):
        return subprocess.check_output(['git','-C',str(self.root),*args],stderr=subprocess.DEVNULL)

    def commit(self):
        self.git('add','.')
        self.git('-c','user.name=Component test','-c','user.email=test@example.invalid','commit','-qm','owned fixture')

    def test_all_callers_use_one_input_serialized_core_producer(self):
        source=(ROOT/'.github/workflows/source-acceptance.yml').read_text()
        runtime=(ROOT/'.github/workflows/qualify-termux-runtime.yml').read_text()
        producer=(ROOT/'.github/workflows/android-component.yml').read_text()
        manager=(ROOT/'.github/workflows/manager-qualification.yml').read_text()
        for caller in (source,runtime):
            self.assertIn('uses: ./.github/workflows/android-component.yml',caller)
            self.assertNotIn('--target aarch64-linux-android -p codex',caller)
        self.assertIn('group: android-${{ inputs.component }}-${{ needs.identity.outputs.fingerprint }}',producer)
        body=producer.split('  produce:',1)[1]
        gate=body.split('    concurrency:',1)[1].split('    outputs:',1)[0]
        self.assertIn('cancel-in-progress: false',gate)
        self.assertIn('queue: max',gate)
        self.assertNotIn('github.workflow',gate)
        # The shared job gate must enclose restore/build/verify/save, so a queued
        # first-miss caller observes the completed producer before deciding to build.
        positions=[body.index(name) for name in (
            '      - name: Restore identical production output',
            '      - name: Cross-build selected Android component once',
            '      - name: Reverify every reused or new artifact',
            '      - name: Retain verified exact-input production output',
            '      - name: Transfer unsigned component')]
        self.assertEqual(positions,sorted(positions))
        self.assertEqual(body.count("if: steps.component-cache.outputs.cache-hit != 'true'"),2)
        self.assertNotIn('restore-keys:',producer)
        self.assertIn('component_artifact.py verify',producer)
        self.assertIn('cargo "+$RUST_VERSION" test --locked -p codex-manager',manager)
        self.assertIn('CODEX_MANAGER_ARTIFACT_PROBE=1',manager)
        self.assertNotIn('component: core',manager)

    def test_actual_dependency_graph_and_embedded_lock_are_exact(self):
        core=inputs.fingerprint(self.root,'core');manager=inputs.fingerprint(self.root,'manager')
        (self.root/'crates/manager/src/lib.rs').write_text('changed manager source')
        self.commit()
        self.assertEqual(inputs.fingerprint(self.root,'core'),core)
        self.assertNotEqual(inputs.fingerprint(self.root,'manager'),manager)
        manager=inputs.fingerprint(self.root,'manager')
        (self.root/'crates/core/src/main.rs').write_text('changed core source')
        self.commit()
        self.assertNotEqual(inputs.fingerprint(self.root,'core'),core)
        self.assertEqual(inputs.fingerprint(self.root,'manager'),manager)
        core=inputs.fingerprint(self.root,'core')
        with (self.root/'Cargo.lock').open('a') as stream:
            stream.write('\n[[package]]\nname="unrelated-test-dependency"\nversion="1.0.0"\n')
        self.commit()
        self.assertNotEqual(inputs.fingerprint(self.root,'core'),core)
        self.assertEqual(inputs.fingerprint(self.root,'manager'),manager)
        (self.root/'crates/runtime-binding/src/lib.rs').write_text('changed shared interface')
        self.commit()
        self.assertNotEqual(inputs.fingerprint(self.root,'manager'),manager)

    def test_documentation_outputs_and_upstream_version_do_not_rebuild_wrappers(self):
        before={name:inputs.fingerprint(self.root,name) for name in inputs.COMPONENTS}
        (self.root/'CURRENT.md').write_text('new evidence')
        recipe=self.root/'release/upstream.json';value=json.loads(recipe.read_text())
        value['upstream']['version']='0.167.2';value['frontend']=None
        recipe.write_text(json.dumps(value));self.commit()
        self.assertEqual(before,{name:inputs.fingerprint(self.root,name) for name in inputs.COMPONENTS})
        value['toolchains']['core_rust']='1.99.0';recipe.write_text(json.dumps(value));self.commit()
        for name in inputs.COMPONENTS:self.assertNotEqual(inputs.fingerprint(self.root,name),before[name])

    def test_symlink_and_changed_compiler_settings_refuse_or_invalidate(self):
        before=inputs.fingerprint(self.root,'manager')
        config=self.root/inputs.BUILD_INPUTS;value=json.loads(config.read_text());value['android_api']=25
        config.write_text(json.dumps(value));self.commit()
        self.assertNotEqual(inputs.fingerprint(self.root,'manager'),before)
        config.unlink();config.symlink_to('upstream.json');self.commit()
        with self.assertRaises((ValueError,KeyError)):inputs.fingerprint(self.root,'manager')

    def binary(self):
        data=bytearray(512);data[:6]=b'\x7fELF\x02\x01';struct.pack_into('<H',data,18,183)
        struct.pack_into('<Q',data,32,64);struct.pack_into('<HH',data,54,56,1)
        struct.pack_into('<I',data,64,3);struct.pack_into('<Q',data,72,128)
        linker=b'/system/bin/linker64\0';struct.pack_into('<Q',data,96,len(linker));data[128:128+len(linker)]=linker
        return bytes(data)

    def test_transfer_preserves_bytes_and_rejects_wrong_inputs_digest_or_target(self):
        folder=Path(self.temp.name)/'artifact';folder.mkdir()
        binary=folder/'codex-manager';binary.write_bytes(self.binary())
        receipt=artifact.record(self.root,'manager',folder,'123')
        self.assertEqual(receipt['artifact_bytes'],512)
        transfer=Path(self.temp.name)/'transfer'
        artifact.pack(self.root,'manager',folder,transfer)
        restored=Path(self.temp.name)/'restored'
        self.assertEqual(artifact.unpack(self.root,'manager',restored,transfer),receipt)
        self.assertEqual((restored/'codex-manager').read_bytes(),binary.read_bytes())
        for field,value in [('inputs_sha256','a'*64),('artifact_sha256','b'*64),('target','x86_64-linux')]:
            (restored/'component.json').write_bytes(inputs.canonical({**receipt,field:value}))
            with self.assertRaises(ValueError):artifact.verify(self.root,'manager',restored)
        (restored/'component.json').write_bytes(inputs.canonical(receipt))
        (restored/'codex-manager').write_bytes(self.binary().replace(b'/system/bin/linker64',b'/system/bin/invalid!'))
        with self.assertRaises(ValueError):artifact.verify(self.root,'manager',restored)
        archive=transfer/'component.tar';archive.write_bytes(archive.read_bytes()+b'unbound trailing bytes')
        with self.assertRaises(ValueError):artifact.unpack(self.root,'manager',Path(self.temp.name)/'bad',transfer)
        self.assertFalse((Path(self.temp.name)/'bad').exists())

    def test_base_fingerprint_tracks_the_actual_embedded_full_lock(self):
        from release_components import base_fingerprint
        before=base_fingerprint(self.root)
        with (self.root/'Cargo.lock').open('a') as stream:stream.write('\n# changes compiled into runtime-inputs\n')
        self.commit()
        self.assertNotEqual(base_fingerprint(self.root),before)

if __name__=='__main__':unittest.main()
