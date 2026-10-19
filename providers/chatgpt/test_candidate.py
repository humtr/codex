"""Admission failures must preserve the previous owned candidate and user files."""
import json
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


if __name__ == '__main__':
    unittest.main()
