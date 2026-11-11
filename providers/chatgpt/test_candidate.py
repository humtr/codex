"""Admission failures must preserve the previous owned candidate and user files."""
import hashlib
import json
import os
import sys
from pathlib import Path
import subprocess
import tempfile
import unittest

import candidate


class CandidateTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix='provider-candidate-')
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.source = self.root / 'source'
        self.source.mkdir()
        for name in candidate.SOURCE_FILES:
            path = self.source / 'providers/chatgpt' / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text('owned source fixture: ' + name)
        self.git('init', '--quiet')
        self.git('add', '.')
        self.git('-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid',
                 '-c', 'commit.gpgsign=false', '-c', 'core.hooksPath=/dev/null',
                 'commit', '--quiet', '-m', 'owned candidate fixture')
        self.sha = self.git('rev-parse', 'HEAD').decode().strip()
        self.binary = self.root / 'binary'
        elf = bytearray(64)
        elf[:6] = b'\x7fELF\x02\x01'
        elf[18:20] = b'\xb7\x00'
        self.binary.write_bytes(elf)
        self.artifact = self.root / 'candidate'
        candidate.package(self.binary, self.artifact, self.source, self.sha,
                          '123', '27.3.13750724', '1.98.0')

    def git(self, *args):
        return subprocess.check_output(['git', '-C', str(self.source), *args],
                                       stderr=subprocess.DEVNULL)

    def verify(self):
        return candidate.verify(self.artifact, self.source, self.sha)

    def test_complete_readback_and_create_new_only(self):
        record = self.verify()
        self.assertEqual(set(record['files']), candidate.FILES)
        for flag in candidate.FLAGS:
            self.assertIs(record[flag], False)
        before = {p: p.read_bytes() for p in self.artifact.rglob('*') if p.is_file()}
        with self.assertRaises(FileExistsError):
            candidate.package(self.binary, self.artifact, self.source, self.sha,
                              '124', '27.3.13750724', '1.98.0')
        for path, data in before.items():
            self.assertEqual(path.read_bytes(), data)

    def test_changed_missing_and_extra_content_refused(self):
        binary = self.artifact / candidate.BINARY
        old = binary.read_bytes()
        binary.write_bytes(old + b'changed')
        with self.assertRaisesRegex(ValueError, 'content mismatch'):
            self.verify()
        binary.write_bytes(old)
        license_path = self.artifact / candidate.SOURCE_FILES[-1]
        license_path.unlink()
        with self.assertRaisesRegex(ValueError, 'inventory'):
            self.verify()
        license_path.write_bytes(candidate.source_bytes(self.source, self.sha, candidate.SOURCE_FILES[-1]))
        (self.artifact / 'unexpected').write_text('preserve')
        with self.assertRaisesRegex(ValueError, 'inventory'):
            self.verify()

    def test_symlinks_and_fifo_never_admitted_or_followed(self):
        license_dir = self.artifact / 'licenses'
        license_dir.rename(self.root / 'saved-licenses')
        license_dir.symlink_to(self.root / 'saved-licenses', target_is_directory=True)
        with self.assertRaisesRegex(ValueError, 'nonregular'):
            self.verify()
        license_dir.unlink()
        (self.root / 'saved-licenses').rename(license_dir)
        binary = self.artifact / candidate.BINARY
        binary.unlink()
        import os
        os.mkfifo(binary)
        with self.assertRaisesRegex(ValueError, 'nonregular'):
            self.verify()

    def test_forged_metadata_and_source_assets_refused(self):
        path = self.artifact / 'candidate.json'
        original = path.read_text()
        record = json.loads(original)
        record['signed'] = True
        path.write_text(json.dumps(record))
        with self.assertRaisesRegex(ValueError, 'boundary'):
            self.verify()
        path.write_text(original[:-2] + ',"source":"' + self.sha + '"}\n')
        with self.assertRaisesRegex(ValueError, 'duplicate'):
            self.verify()
        path.write_text(original)
        with self.assertRaisesRegex(ValueError, 'boundary'):
            candidate.verify(self.artifact, self.source, 'a' * 40)
        lock = self.artifact / 'Cargo.lock'
        lock.write_text('substituted dependency lock')
        data = lock.read_bytes()
        record = json.loads(original)
        import hashlib
        record['files']['Cargo.lock'] = {'sha256': hashlib.sha256(data).hexdigest(), 'bytes': len(data)}
        path.write_text(json.dumps(record))
        with self.assertRaisesRegex(ValueError, 'exact source'):
            self.verify()

    def test_dirty_provider_source_cannot_be_labeled_as_exact_commit(self):
        (self.source / 'providers/chatgpt/new.rs').write_text('uncommitted source')
        with self.assertRaisesRegex(ValueError, 'clean exact'):
            candidate.package(self.binary, self.root / 'next', self.source, self.sha,
                              '124', '27.3.13750724', '1.98.0')
        self.assertFalse((self.root / 'next').exists())

    def test_dirty_shared_codec_cannot_be_omitted_from_provider_source_binding(self):
        path=self.source / 'crates/runtime-binding/src/lib.rs'
        path.parent.mkdir(parents=True)
        path.write_text('uncommitted shared execution codec')
        with self.assertRaisesRegex(ValueError,'clean exact'):
            candidate.package(self.binary,self.root / 'next',self.source,self.sha,
                              '124','27.3.13750724','1.98.0')
        self.assertFalse((self.root / 'next').exists())


    def worker(self, name='worker'):
        artifact = self.root / name
        candidate.package(self.binary, artifact, self.source, self.sha,
                          '456', '27.3.13750724', '1.98.0', component='termux-mcp-worker')
        return artifact

    def verify_worker(self, artifact):
        return candidate.verify(artifact, self.source, self.sha, component='termux-mcp-worker')

    def test_worker_is_a_distinct_unsigned_component_and_preserves_provider_defaults(self):
        before = {p.relative_to(self.artifact): p.read_bytes()
                  for p in self.artifact.rglob('*') if p.is_file()}
        artifact = self.worker()
        record = self.verify_worker(artifact)
        expected = {'codex-termux-mcp-worker', *candidate.SOURCE_FILES}
        self.assertEqual(record['schema'], 'termux-codex-mcp-worker-candidate-v1')
        self.assertEqual(set(record['files']), expected)
        self.assertEqual(record['source'], self.sha)
        self.assertEqual(record['run_id'], '456')
        self.assertEqual((record['target'], record['api'], record['ndk'], record['rust']),
                         ('aarch64-linux-android', 24, '27.3.13750724', '1.98.0'))
        self.assertEqual(record['files']['codex-termux-mcp-worker'],
                         {'sha256': hashlib.sha256(self.binary.read_bytes()).hexdigest(), 'bytes': 64})
        for flag in candidate.FLAGS:
            self.assertIs(record[flag], False)
        with self.assertRaisesRegex(ValueError, 'inventory'):
            candidate.verify(artifact, self.source, self.sha)
        with self.assertRaisesRegex(ValueError, 'inventory'):
            self.verify_worker(self.artifact)
        legacy = self.verify()
        self.assertEqual(legacy['schema'], 'termux-codex-provider-candidate-v1')
        self.assertEqual(set(legacy['files']), candidate.FILES)
        for name, data in before.items():
            self.assertEqual((self.artifact / name).read_bytes(), data)

    def test_worker_inventory_symlinks_hash_and_wrong_abi_are_rejected(self):
        for case, expected in [('changed', 'content mismatch'), ('missing', 'inventory'),
                               ('extra', 'inventory'), ('binary-alias', 'nonregular'),
                               ('directory-alias', 'nonregular'), ('fifo', 'nonregular'),
                               ('wrong-abi', 'AArch64')]:
            with self.subTest(case=case):
                artifact = self.worker(case)
                binary = artifact / 'codex-termux-mcp-worker'
                if case == 'changed':
                    binary.write_bytes(binary.read_bytes() + b'changed')
                elif case == 'missing':
                    (artifact / candidate.SOURCE_FILES[-1]).unlink()
                elif case == 'extra':
                    (artifact / candidate.BINARY).write_bytes(self.binary.read_bytes())
                elif case == 'binary-alias':
                    binary.unlink(); binary.symlink_to(self.binary)
                elif case == 'directory-alias':
                    licenses = artifact / 'licenses'
                    licenses.rename(self.root / 'original-worker-licenses')
                    licenses.symlink_to(self.artifact / 'licenses', target_is_directory=True)
                elif case == 'fifo':
                    binary.unlink(); os.mkfifo(binary)
                else:
                    data = bytearray(binary.read_bytes()); data[18:20] = b'\x3e\x00'
                    binary.write_bytes(data)
                    metadata = artifact / 'candidate.json'
                    record = json.loads(metadata.read_text())
                    record['files'][binary.name] = {'sha256': hashlib.sha256(data).hexdigest(), 'bytes': len(data)}
                    metadata.write_text(json.dumps(record))
                with self.assertRaisesRegex(ValueError, expected):
                    self.verify_worker(artifact)
                self.assertEqual(self.binary.read_bytes()[18:20], b'\xb7\x00')
                self.verify()  # No external alias target or previous candidate changed.

    def test_worker_metadata_and_exact_source_assets_are_not_substitutable(self):
        artifact = self.worker()
        path = artifact / 'candidate.json'
        original = path.read_text()
        changes = [('schema', 'termux-codex-provider-candidate-v1'), ('source', 'a' * 40),
                   ('target', 'x86_64-unknown-linux-gnu'), ('api', 23), ('run_id', {}),
                   ('ndk', '../toolchain'), ('rust', None), ('signed', True),
                   ('device_qualified', True), ('installation_authorized_by_this_record', True),
                   ('app_rootfs_or_credentials_bundled', True), ('extra', 'not admitted')]
        for key, value in changes:
            with self.subTest(field=key):
                record = json.loads(original); record[key] = value
                path.write_text(json.dumps(record))
                with self.assertRaisesRegex(ValueError, 'boundary'):
                    self.verify_worker(artifact)
        path.write_text(original[:-2] + ',"source":"' + self.sha + '"}\n')
        with self.assertRaisesRegex(ValueError, 'duplicate'):
            self.verify_worker(artifact)
        record = json.loads(original)
        lock = artifact / 'Cargo.lock'; lock.write_bytes(b'forged shared dependencies')
        record['files']['Cargo.lock'] = {'sha256': hashlib.sha256(lock.read_bytes()).hexdigest(), 'bytes': lock.stat().st_size}
        path.write_text(json.dumps(record))
        with self.assertRaisesRegex(ValueError, 'exact source'):
            self.verify_worker(artifact)

    def test_worker_packaging_is_create_new_and_requires_clean_source_and_regular_input(self):
        artifact = self.worker()
        before = {p.relative_to(artifact): p.read_bytes() for p in artifact.rglob('*') if p.is_file()}
        with self.assertRaises(FileExistsError):
            self.worker()
        for name, data in before.items():
            self.assertEqual((artifact / name).read_bytes(), data)
        alias = self.root / 'aliased-input'; alias.symlink_to(self.binary)
        output = self.root / 'unsafe-input-output'
        with self.assertRaises(OSError):
            candidate.package(alias, output, self.source, self.sha,
                              '456', '27.3.13750724', '1.98.0', component='termux-mcp-worker')
        self.assertFalse(output.exists())
        with self.assertRaisesRegex(ValueError, 'component'):
            candidate.package(self.binary, output, self.source, self.sha,
                              '456', '27.3.13750724', '1.98.0', component='unknown-worker')
        self.assertFalse(output.exists())
        path = self.source / 'providers/chatgpt/src/termux_shell.rs'
        path.parent.mkdir(parents=True); path.write_text('uncommitted worker change')
        with self.assertRaisesRegex(ValueError, 'clean exact'):
            self.worker('dirty-worker')
        self.assertFalse((self.root / 'dirty-worker').exists())

    def test_worker_cli_requires_explicit_component_and_keeps_provider_v1_readback(self):
        program = Path(candidate.__file__).resolve()
        artifact = self.root / 'cli-worker'
        common = ['--artifact', str(artifact), '--source', str(self.source), '--source-sha', self.sha]
        build = subprocess.run([sys.executable, str(program), 'package', *common,
                                '--component', 'termux-mcp-worker', '--binary', str(self.binary),
                                '--run-id', '456', '--ndk', '27.3.13750724', '--rust', '1.98.0'],
                               capture_output=True, text=True)
        self.assertEqual(build.returncode, 0, build.stderr)
        verified = subprocess.run([sys.executable, str(program), 'verify', *common,
                                   '--component', 'termux-mcp-worker'], capture_output=True, text=True)
        self.assertEqual(verified.returncode, 0, verified.stderr)
        self.assertEqual(json.loads(verified.stdout), json.loads(build.stdout))
        refused = subprocess.run([sys.executable, str(program), 'verify', *common], capture_output=True, text=True)
        self.assertEqual(refused.returncode, 1)
        legacy = subprocess.run([sys.executable, str(program), 'verify', '--artifact', str(self.artifact),
                                 '--source', str(self.source), '--source-sha', self.sha], capture_output=True, text=True)
        self.assertEqual(legacy.returncode, 0, legacy.stderr)
        self.assertEqual(set(json.loads(legacy.stdout)), {'source', 'files_verified', 'sha256', 'signed'})

if __name__ == '__main__':
    unittest.main()
