"""Actual raw verifier and CI transport roundtrips; no Native compilation proof."""
import copy
import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import stat
import subprocess
import sys
import tarfile
import tempfile
import textwrap
import unittest
import zipfile

import runtime_artifact as transport
from test_local_fixture_generation import elf

SOURCE = Path(__file__).resolve().parents[2]


def workflow_step(name):
    text = (SOURCE / '.github/workflows/termux-runtime-source.yml').read_text()
    block = text.split('      - name: ' + name + '\n', 1)[1].split('\n      - name:', 1)[0]
    return textwrap.dedent(block.split('        run: |\n', 1)[1])


def actions_download(source, destination):
    """Actions v4 uploads lose modes: directories 0755, every file 0644."""
    archive = io.BytesIO()
    with zipfile.ZipFile(archive, 'w') as zipped:
        for path in source.iterdir():
            zipped.write(path, path.name)
    archive.seek(0)
    destination.mkdir()
    with zipfile.ZipFile(archive) as zipped:
        zipped.extractall(destination)
    for path in destination.iterdir():
        path.chmod(0o644)


class RuntimeTransferTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        target = Path(os.environ.get('CARGO_TARGET_DIR',
                      Path(os.environ.get('XDG_CACHE_HOME', Path.home() / '.cache')) / 'codex/check-target')).resolve()
        subprocess.run(['cargo', 'build', '--locked', '-p', 'codex-release-builder',
                        '--manifest-path', str(SOURCE / 'Cargo.toml')], check=True,
                       env={**os.environ, 'CARGO_TARGET_DIR': str(target),
                            'CARGO_PROFILE_DEV_DEBUG': '0', 'CARGO_INCREMENTAL': '0'},
                       stdout=subprocess.DEVNULL)
        cls.builder = target / 'debug/codex-release-builder'
        cls.openssl = Path(shutil.which('openssl'))
        cls.recipe = SOURCE / 'release/upstream.json'

    def raw(self, parent):
        artifact = parent / 'runtime-artifact'
        artifact.mkdir()
        recipe = json.loads(self.recipe.read_text())
        payloads = {
            'codex': elf(),
            'version.txt': ('codex-cli ' + recipe['upstream']['version'] + '\n').encode(),
            'preparation.json': json.dumps({'source': recipe['upstream']['source'],
                                           'version': recipe['upstream']['version']}).encode(),
            'lock-audit.json': json.dumps(recipe['qualification']['lock_audit']).encode(),
            'source.patch': b'owned source preparation evidence\n',
        }
        payloads['sha256.txt'] = (hashlib.sha256(payloads['codex']).hexdigest() + '  codex\n').encode()
        for name, content in payloads.items():
            path = artifact / name
            path.write_bytes(content)
            path.chmod(0o700 if name == 'codex' else 0o644)
        return artifact

    def run_transfer(self, action, artifact, transfer):
        return subprocess.run([sys.executable, str(SOURCE / '.github/scripts/runtime_artifact.py'), action,
            '--artifact', str(artifact), '--transfer', str(transfer), '--builder', str(self.builder),
            '--recipe', str(self.recipe), '--openssl', str(self.openssl)], capture_output=True, text=True)

    def verify(self, artifact):
        return subprocess.run([str(self.builder), 'verify-runtime', '--artifact', str(artifact),
            '--recipe', str(self.recipe), '--openssl', str(self.openssl)], capture_output=True, text=True)

    def test_actions_roundtrip_preserves_producer_modes_and_all_evidence(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            original = self.raw(root)
            self.assertEqual(self.verify(original).returncode, 0)
            before = {p.name: (p.read_bytes(), stat.S_IMODE(p.stat().st_mode)) for p in original.iterdir()}
            actions_download(original, root / 'loose')
            rejected = self.verify(root / 'loose')
            self.assertNotEqual(rejected.returncode, 0)
            self.assertIn('runtime artifact executable mode is unsafe', rejected.stderr)
            source = root / 'source'
            (source / '.github/scripts').mkdir(parents=True)
            (source / 'release').mkdir()
            (source / '.github/scripts/runtime_artifact.py').symlink_to(SOURCE / '.github/scripts/runtime_artifact.py')
            shutil.copy2(self.recipe, source / 'release/upstream.json')
            packed = subprocess.run(['bash', '-c', workflow_step('Package runtime and preparation evidence with original modes')],
                cwd=root, env={**os.environ, 'RUNNER_TEMP': str(root), 'GITHUB_WORKSPACE': str(root),
                               'CODEX_RELEASE_BUILDER': str(self.builder)}, capture_output=True, text=True)
            self.assertEqual(packed.returncode, 0, packed.stderr)
            self.assertEqual({p.name for p in (root / 'runtime-upload').iterdir()}, {transport.ARCHIVE})
            actions_download(root / 'runtime-upload', root / 'download')
            restored = root / 'restored'
            result = self.run_transfer('unpack', restored, root / 'download')
            self.assertEqual(result.returncode, 0, result.stderr)
            for directory in (original, restored):
                self.assertEqual({p.name: (p.read_bytes(), stat.S_IMODE(p.stat().st_mode))
                                  for p in directory.iterdir()}, before)

    def test_pack_does_not_repair_an_unsafe_runtime_or_replace_existing_output(self):
        for mode in (0o644, 0o666, 0o777, 0o4700):
            with self.subTest(mode=oct(mode)), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                original = self.raw(root)
                (original / 'codex').chmod(mode)
                result = self.run_transfer('pack', original, root / 'transfer')
                self.assertNotEqual(result.returncode, 0)
                self.assertIn('runtime artifact executable mode is unsafe', result.stderr)
                self.assertEqual(stat.S_IMODE((original / 'codex').stat().st_mode), mode)
                self.assertFalse((root / 'transfer').exists())
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            original = self.raw(root)
            transfer = root / 'transfer'
            transfer.mkdir()
            (transfer / 'retained').write_bytes(b'existing caller artifact')
            self.assertNotEqual(self.run_transfer('pack', original, transfer).returncode, 0)
            self.assertEqual((transfer / 'retained').read_bytes(), b'existing caller artifact')
            self.assertEqual(self.run_transfer('pack', original, root / 'valid-transfer').returncode, 0)
            self.assertNotEqual(self.run_transfer('unpack', transfer, root / 'valid-transfer').returncode, 0)
            self.assertEqual((transfer / 'retained').read_bytes(), b'existing caller artifact')

    def test_unpack_rejects_inventory_alias_mode_and_admission_faults(self):
        faults = ('extra', 'missing', 'duplicate', 'traversal', 'absolute', 'symlink', 'hardlink',
                  'directory', 'fifo', 'not-executable', 'writable', 'setuid', 'digest', 'source',
                  'audit', 'trailing-data', 'loose', 'transfer-extra', 'transfer-alias')
        for fault in faults:
            with self.subTest(fault=fault), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                original = self.raw(root)
                transfer = root / 'transfer'
                result = self.run_transfer('pack', original, transfer)
                self.assertEqual(result.returncode, 0, result.stderr)
                archive_path = transfer / transport.ARCHIVE
                with tarfile.open(archive_path, 'r:') as archive:
                    entries = [(copy.copy(entry), archive.extractfile(entry).read()) for entry in archive]
                if fault in ('extra', 'duplicate'):
                    entry, data = copy.deepcopy(entries[0])
                    if fault == 'extra': entry.name = 'undeclared'
                    entries.append((entry, data))
                elif fault == 'missing': entries.pop()
                elif fault in ('traversal', 'absolute'):
                    entries[0][0].name = '../escaped' if fault == 'traversal' else '/escaped'
                elif fault in ('symlink', 'hardlink', 'directory', 'fifo'):
                    entries[0][0].type = {'symlink': tarfile.SYMTYPE, 'hardlink': tarfile.LNKTYPE,
                        'directory': tarfile.DIRTYPE, 'fifo': tarfile.FIFOTYPE}[fault]
                    entries[0][0].linkname = 'outside' if fault in ('symlink', 'hardlink') else ''
                elif fault in ('not-executable', 'writable', 'setuid'):
                    next(entry for entry, _ in entries if entry.name == 'codex').mode = {
                        'not-executable': 0o644, 'writable': 0o777, 'setuid': 0o4700}[fault]
                elif fault in ('digest', 'source', 'audit'):
                    name = {'digest': 'sha256.txt', 'source': 'preparation.json', 'audit': 'lock-audit.json'}[fault]
                    position = next(i for i, (entry, _) in enumerate(entries) if entry.name == name)
                    entry, data = entries[position]
                    data = b'0' * 64 + b'  codex\n' if fault == 'digest' else b'{}'
                    entry.size = len(data)
                    entries[position] = (entry, data)
                with tarfile.open(archive_path, 'w', format=tarfile.USTAR_FORMAT) as archive:
                    for entry, data in entries: archive.addfile(entry, io.BytesIO(data))
                if fault == 'trailing-data':
                    with archive_path.open('ab') as output: output.write(b'undeclared trailer')
                elif fault == 'loose':
                    archive_path.unlink()
                    for path in original.iterdir(): shutil.copy2(path, transfer / path.name)
                elif fault == 'transfer-extra': (transfer / 'extra').write_bytes(b'undeclared')
                elif fault == 'transfer-alias':
                    moved = root / 'archive'
                    archive_path.rename(moved)
                    archive_path.symlink_to(moved)
                result = self.run_transfer('unpack', root / 'restored', transfer)
                self.assertNotEqual(result.returncode, 0, result.stdout)
                self.assertFalse((root / 'restored').exists(), fault)
                self.assertFalse((root / 'escaped').exists())

    def test_reusable_raw_workflow_distinguishes_cache_from_actions_transfer(self):
        for qualified, rejected in ((False, False), (False, True), (True, False), (True, True)):
            with self.subTest(qualified=qualified, rejected=rejected), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                source = root / 'source'
                (source / '.github/scripts').mkdir(parents=True)
                (source / 'release').mkdir()
                (source / '.github/scripts/runtime_artifact.py').symlink_to(SOURCE / '.github/scripts/runtime_artifact.py')
                shutil.copy2(self.recipe, source / 'release/upstream.json')
                original = self.raw(root)
                if qualified:
                    packed = self.run_transfer('pack', original, root / 'upload')
                    self.assertEqual(packed.returncode, 0, packed.stderr)
                    actions_download(root / 'upload', root / 'runtime-transfer')
                    shutil.rmtree(original)
                    if rejected:
                        # An advertised old loose artifact must fail, not trigger
                        # chmod, a new source build or another artifact lookup.
                        (root / 'runtime-transfer/runtime.tar').unlink()
                        (root / 'runtime-transfer/codex').write_bytes(b'old loose runtime')
                elif rejected:
                    (original / 'codex').chmod(0o644)
                env = {**os.environ, 'QUALIFIED_RUN': '123' if qualified else '',
                       'CACHE_HIT': '' if qualified else 'true', 'RUNNER_TEMP': str(root),
                       'GITHUB_WORKSPACE': str(root), 'GITHUB_OUTPUT': str(root / 'outputs'),
                       'CODEX_RELEASE_BUILDER': str(self.builder)}
                result = subprocess.run(['bash', '-c', workflow_step('Verify reusable raw runtime before qualification')],
                    cwd=root, env=env, capture_output=True, text=True)
                self.assertEqual(result.returncode == 0, not rejected, result.stderr)
                self.assertEqual((root / 'outputs').read_text() if (root / 'outputs').exists() else '',
                                 '' if rejected else 'reuse=true\n')
                if not rejected:
                    self.assertEqual(self.verify(root / 'runtime-artifact').returncode, 0)


if __name__ == '__main__':
    unittest.main()
