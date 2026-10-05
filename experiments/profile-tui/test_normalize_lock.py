import importlib.util
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('normalize_lock', Path(__file__).with_name('normalize_lock.py'))
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)

class LockTests(unittest.TestCase):
    def fixture(self, root):
        (root/'member').mkdir()
        (root/'Cargo.toml').write_text('[workspace]\nmembers=["member"]\n[workspace.package]\nversion="0.160.0"\n')
        (root/'member/Cargo.toml').write_text('[package]\nname="codex-sample"\nversion.workspace=true\n')
        (root/'Cargo.lock').write_text('version = 4\n\n[[package]]\nname = "codex-sample"\nversion = "0.0.0"\ndependencies = ["external"]\n\n[[package]]\nname = "external"\nversion = "1.2.3"\nsource = "registry+https://example.invalid/index"\nchecksum = "'+'a'*64+'"\n')
    def test_only_workspace_versions_change(self):
        with tempfile.TemporaryDirectory() as t:
            p=Path(t);self.fixture(p);before=(p/'Cargo.lock').read_text();audit=m.normalize(p)
            self.assertEqual((p/'Cargo.lock').read_text(),before.replace('version = "0.0.0"','version = "0.160.0"'))
            self.assertEqual(audit['workspace_versions_changed'],1);self.assertEqual(audit['external_packages_unchanged'],1)
    def test_path_dependency_omitted_from_members_is_normalized(self):
        with tempfile.TemporaryDirectory() as t:
            p=Path(t);self.fixture(p)
            (p/'path-dependency').mkdir()
            (p/'path-dependency/Cargo.toml').write_text('[package]\nname="codex-path"\nversion.workspace=true\n')
            lock=p/'Cargo.lock'
            before=lock.read_text().replace('dependencies = ["external"]','dependencies = ["external", "codex-path 0.0.0"]')
            before+='\n[[package]]\nname = "codex-path"\nversion = "0.0.0"\n'
            lock.write_text(before)
            audit=m.normalize(p)
            self.assertEqual(lock.read_text(),before.replace('version = "0.0.0"','version = "0.160.0"').replace('"codex-path 0.0.0"','"codex-path 0.160.0"'))
            self.assertEqual(audit['workspace_versions_changed'],2)
            self.assertEqual(audit['external_packages_unchanged'],1)
    def test_release_inventory_and_version_fail_without_mutation(self):
        for field in ['release','member','inventory','version']:
            with self.subTest(field=field), tempfile.TemporaryDirectory() as t:
                p=Path(t);self.fixture(p)
                if field=='release':(p/'Cargo.toml').write_text((p/'Cargo.toml').read_text().replace('0.160.0','0.161.0'))
                if field=='member':(p/'member/Cargo.toml').unlink()
                if field=='inventory':(p/'Cargo.lock').write_text((p/'Cargo.lock').read_text().replace('codex-sample','other'))
                if field=='version':(p/'Cargo.lock').write_text((p/'Cargo.lock').read_text().replace('0.0.0','0.160.0'))
                before=(p/'Cargo.lock').read_bytes()
                with self.assertRaises((ValueError,FileNotFoundError)):m.normalize(p)
                self.assertEqual((p/'Cargo.lock').read_bytes(),before)

if __name__=='__main__':unittest.main()
