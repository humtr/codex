import copy
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

import runtime_inputs as runtime

ROOT = Path(__file__).resolve().parents[2]
# These are independently required by the actual producer/loader/normalizer flow.
PRODUCER_FILES = ('release/runtime-build-inputs-v1', '.github/scripts/runtime_inputs.py',
                  'crates/release-builder/src/source.rs', 'crates/release-builder/src/lib.rs', 'crates/release-builder/src/runtime.rs', 'crates/release-builder/Cargo.toml',
                  '.github/scripts/upstream_release.py',
                  '.github/workflows/termux-runtime-source.yml')


class RuntimeProducer(unittest.TestCase):
    def test_dependency_closure_preserves_explicit_source_for_equal_name_and_version(self):
        inputs = {
            'tool.toml': b'[package]\nname="tool"\nversion="1.0.0"\n',
            'Cargo.lock': b'''version=4
[[package]]
name="tool"
version="1.0.0"
dependencies=["shared 1.0.0 (registry+https://example.invalid/selected)"]
[[package]]
name="shared"
version="1.0.0"
source="registry+https://example.invalid/selected"
checksum="selected bytes"
[[package]]
name="shared"
version="1.0.0"
source="git+https://example.invalid/other#012345"
'''}
        closure = runtime.locked_packages(runtime.cargo_dependencies(inputs.__getitem__, 'tool.toml'))
        self.assertEqual([(p['name'], p.get('source')) for p in closure],
                         [('shared', 'registry+https://example.invalid/selected'), ('tool', None)])
        inputs['Cargo.lock'] = inputs['Cargo.lock'].replace(
            b' (registry+https://example.invalid/selected)', b'')
        with self.assertRaisesRegex(ValueError, 'ambiguous Cargo build dependency'):
            runtime.cargo_dependencies(inputs.__getitem__, 'tool.toml')

    def source(self, root):
        for name in (*PRODUCER_FILES, 'release/upstream.json', 'Cargo.lock', 'crates/runtime-binding/Cargo.toml'):
            target = root / name
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(ROOT / name, target)
        subprocess.run(['git', 'init', '-q', str(root)], check=True)
        subprocess.run(['git', '-C', str(root), 'add', '.'], check=True)
        subprocess.run(['git', '-C', str(root), '-c', 'user.name=Runtime input test',
            '-c', 'user.email=test@localhost', 'commit', '-qm', 'Original runtime inputs'], check=True)
        return root

    def test_runtime_reuse_tracks_producer_inputs_but_not_core_or_output_references(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = self.source(Path(temporary))
            baseline = runtime.fingerprint(root)
            self.assertEqual(baseline, runtime.fingerprint(root, 'HEAD'))
            core = root / 'crates/core/src/main.rs'
            core.parent.mkdir(parents=True)
            core.write_text('new Core source')
            recipe = json.loads((root / 'release/upstream.json').read_text())
            changed = copy.deepcopy(recipe)
            changed['frontend'] = None
            (root / 'release/upstream.json').write_text(json.dumps(changed))
            self.assertEqual(runtime.fingerprint(root), baseline)
            self.assertNotEqual(runtime.fingerprint(root, recipe={**recipe,
                'upstream': {**recipe['upstream'], 'source': 'a'*40}}), baseline)
            lock = root / 'Cargo.lock'
            original_lock = lock.read_text()
            lock.write_text(original_lock + '\n[[package]]\nname="unrelated-manager-package"\nversion="1.0.0"\n')
            self.assertEqual(runtime.fingerprint(root), baseline)
            start = original_lock.index('name = "toml"')
            checksum = original_lock.index('checksum = "', start) + len('checksum = "')
            lock.write_text(original_lock[:checksum] + '0'*64 + original_lock[checksum+64:])
            self.assertNotEqual(runtime.fingerprint(root), baseline)
            lock.write_text(original_lock)
            for name in PRODUCER_FILES:
                path = root / name
                original = path.read_bytes()
                path.write_bytes(original + (b'input\trelease/additional-input.json\n'
                    if name == 'release/runtime-build-inputs-v1' else b'\n# changed producer semantics\n'))
                self.assertNotEqual(runtime.fingerprint(root), baseline, name)
                path.write_bytes(original)
            self.assertEqual(runtime.fingerprint(root), baseline)

    def test_adapted_runtime_reuse_also_binds_the_published_byte_transform(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = self.source(Path(temporary))
            adapter = root / 'crates/release-builder/src/runtime_patch.rs'
            adapter.parent.mkdir(parents=True, exist_ok=True)
            adapter.write_text('original runtime adaptation')
            subprocess.run(['git','-C',str(root),'add','.'],check=True)
            subprocess.run(['git','-C',str(root),'-c','user.name=Input test','-c',
                'user.email=test@localhost','commit','-qm','Original adapter'],check=True)
            raw = runtime.fingerprint(root, 'HEAD')
            adapted = runtime.fingerprint(root, 'HEAD', adapted=True)
            adapter.write_text('changed runtime adaptation')
            subprocess.run(['git','-C',str(root),'add','.'],check=True)
            subprocess.run(['git','-C',str(root),'-c','user.name=Input test','-c',
                'user.email=test@localhost','commit','-qm','New adapter'],check=True)
            self.assertEqual(runtime.fingerprint(root,'HEAD'), raw)
            self.assertNotEqual(runtime.fingerprint(root,'HEAD',adapted=True), adapted)



if __name__ == '__main__':
    unittest.main()
