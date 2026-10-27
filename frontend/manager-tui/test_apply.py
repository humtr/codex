"""Exercise the actual frontend assembler with disposable upstream trees."""
from pathlib import Path
import subprocess
import tempfile
import unittest

from apply import apply, check_scope


class AssemblyTests(unittest.TestCase):
    def test_scope_accepts_catalog_config_and_tui_but_rejects_other_source_changes(self):
        with tempfile.TemporaryDirectory() as temporary:
            upstream, package, target = self.fixture(Path(temporary))
            core = upstream / 'codex-rs/core/src/lib.rs'
            core.parent.mkdir(parents=True)
            core.write_text('original\n')
            outside = upstream / 'codex-rs/outside.rs'
            outside.write_text('original\n')
            subprocess.run(['git', '-C', str(upstream), 'add', '.'], check=True)
            subprocess.run(['git', '-C', str(upstream), '-c', 'user.name=Owned',
                            '-c', 'user.email=owned@example.invalid', 'commit', '-qm', 'owned'], check=True)
            apply(upstream, package)
            config = core.parent / 'config/mod.rs'
            config.parent.mkdir()
            config.write_text('complete catalog source\n')
            (upstream / 'codex-rs/Cargo.lock').write_text('normalized release workspace\n')
            check_scope(upstream)
            for path in [core, outside, core.parent / 'new.rs', upstream / 'new.rs']:
                with self.subTest(path=path.relative_to(upstream)):
                    before = path.read_bytes() if path.exists() else None
                    path.write_text('unrelated change\n')
                    with self.assertRaises(ValueError):
                        check_scope(upstream)
                    if before is None:
                        path.unlink()
                    else:
                        path.write_bytes(before)
            check_scope(upstream)
            # Moving an unrelated source into TUI must still expose its deletion.
            outside.rename(target / 'moved.rs')
            subprocess.run(['git', '-C', str(upstream), 'add', '-A'], check=True)
            with self.assertRaises(ValueError):
                check_scope(upstream)

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
