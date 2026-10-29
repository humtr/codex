import copy
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

from scripts import build_runtime as runtime

ROOT = Path(__file__).resolve().parents[1]
# These are independently required by the actual producer/loader/normalizer flow.
PRODUCER_FILES = ('release/runtime-build-inputs-v1', 'scripts/build_runtime.py',
                  'crates/release-builder/src/source.rs', 'crates/release-builder/Cargo.toml',
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

    def artifact(self, root):
        recipe = json.loads((ROOT / 'release/upstream.json').read_text())
        data = b'owned raw runtime bytes'
        (root / 'codex').write_bytes(data)
        digest = hashlib.sha256(data).hexdigest()
        (root / 'sha256.txt').write_text(digest + '  codex\n')
        (root / 'version.txt').write_text('codex-cli ' + recipe['upstream']['version'] + '\n')
        (root / 'preparation.json').write_text(json.dumps({**recipe['upstream']}))
        (root / 'lock-audit.json').write_text(json.dumps(recipe['qualification']['lock_audit']))
        (root / 'source.patch').write_text('owned source adaptation')
        return recipe, digest

    def test_raw_artifact_preserves_exact_existing_contract_and_rejects_wrong_provenance(self):
        for fault in ('none', 'binary', 'source', 'version', 'audit', 'extra', 'alias'):
            with self.subTest(fault=fault), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                recipe, digest = self.artifact(root)
                if fault == 'binary':
                    (root / 'codex').write_bytes(b'changed')
                elif fault == 'source':
                    (root / 'preparation.json').write_text(json.dumps({'source': 'b'*40,
                        'version': recipe['upstream']['version']}))
                elif fault == 'version':
                    (root / 'version.txt').write_text('codex-cli other\n')
                elif fault == 'audit':
                    (root / 'lock-audit.json').write_text('{}')
                elif fault == 'extra':
                    (root / 'undeclared').write_text('extra')
                elif fault == 'alias':
                    (root / 'source.patch').unlink()
                    (root / 'source.patch').symlink_to(root / 'version.txt')
                if fault == 'none':
                    self.assertEqual(runtime.verify_artifact(root, recipe), digest)
                else:
                    with self.assertRaises(ValueError):
                        runtime.verify_artifact(root, recipe)

    def test_changed_checkout_is_rejected_before_tool_execution_or_source_edits(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = self.source(root / 'upstream')
            recipe = json.loads((ROOT / 'release/upstream.json').read_text())
            recipe['upstream']['source'] = subprocess.check_output(
                ['git', '-C', str(source), 'rev-parse', 'HEAD'], text=True).strip()
            recipe_path = root / 'recipe.json'
            recipe_path.write_text(json.dumps(recipe))
            protected = source / 'crates/release-builder/src/source.rs'
            original = protected.read_bytes() + b'\n# existing user change\n'
            protected.write_bytes(original)
            output = root / 'output'
            with self.assertRaisesRegex(ValueError, 'not pristine'):
                runtime.build(source, output, recipe_path)
            self.assertFalse(output.exists())
            self.assertEqual(protected.read_bytes(), original)


if __name__ == '__main__':
    unittest.main()
