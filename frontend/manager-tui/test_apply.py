"""Exercise the actual frontend assembler with disposable upstream trees."""
from pathlib import Path
import subprocess
import tempfile
import unittest

from apply import apply


class AssemblyTests(unittest.TestCase):
    def fixture(self, root):
        upstream, package = root / 'upstream', root / 'package'
        target = upstream / 'codex-rs/tui/src'
        target.mkdir(parents=True)
        (target / 'lib.rs').write_text('upstream\n')
        (package / 'src/app').mkdir(parents=True)
        (package / 'src/app/owned.rs').write_text('owned implementation\n')
        (package / 'native.patch').write_text(
            'diff --git a/codex-rs/tui/src/lib.rs b/codex-rs/tui/src/lib.rs\n'
            '--- a/codex-rs/tui/src/lib.rs\n+++ b/codex-rs/tui/src/lib.rs\n'
            '@@ -1 +1,2 @@\n upstream\n+integration\n')
        subprocess.run(['git', 'init', '-q', str(upstream)], check=True)
        return upstream, package, target

    def test_real_assembly_preserves_sources_and_refuses_second_application(self):
        with tempfile.TemporaryDirectory() as temporary:
            upstream, package, target = self.fixture(Path(temporary))
            self.assertEqual(apply(upstream, package), 1)
            self.assertEqual((target / 'lib.rs').read_text(), 'upstream\nintegration\n')
            self.assertEqual((target / 'app/owned.rs').read_bytes(),
                             (package / 'src/app/owned.rs').read_bytes())
            with self.assertRaises(ValueError):
                apply(upstream, package)

    def test_collisions_substitutions_and_bad_patch_fail_before_mutation(self):
        for fault in ('collision', 'source-link', 'parent-link', 'bad-patch'):
            with self.subTest(fault=fault), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                upstream, package, target = self.fixture(root)
                sentinel = root / 'outside'; sentinel.mkdir()
                if fault == 'collision':
                    (target / 'app').mkdir()
                    (target / 'app/owned.rs').write_text('real upstream file\n')
                elif fault == 'source-link':
                    (package / 'src/linked.rs').symlink_to(target / 'lib.rs')
                elif fault == 'parent-link':
                    (target / 'app').symlink_to(sentinel, target_is_directory=True)
                else:
                    (package / 'native.patch').write_text('invalid patch\n')
                with self.assertRaises((ValueError, subprocess.CalledProcessError)):
                    apply(upstream, package)
                self.assertEqual((target / 'lib.rs').read_text(), 'upstream\n')
                self.assertEqual(list(sentinel.iterdir()), [])
                if fault == 'collision':
                    self.assertEqual((target / 'app/owned.rs').read_text(), 'real upstream file\n')


if __name__ == '__main__':
    unittest.main()
