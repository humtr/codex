from pathlib import Path
import hashlib
import tempfile
import unittest

from check_upstream_features import SUITES, reusable_frontend, verify


class UpstreamFeatureProofTests(unittest.TestCase):
    def test_reuse_requires_original_build_identity_bytes_and_equal_production_inputs(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = 'a' * 40
            upstream = 'b' * 40
            metadata = {'repository': {'full_name': 'owner/repo'},
                        'head_repository': {'full_name': 'owner/repo'},
                        'path': '.github/workflows/manager-tui-native.yml',
                        'status': 'completed', 'head_sha': source}
            (root / 'source.txt').write_text(f'upstream_sha={upstream}\nsource_sha={source}\ntarget=aarch64-unknown-linux-musl\nprofile=release opt-level=z codegen-units=1 strip=symbols\n')
            (root / 'version.txt').write_text('codex-cli 0.166.0\n')
            (root / 'codex').write_bytes(b'identified build output')
            (root / 'sha256.txt').write_text(hashlib.sha256((root / 'codex').read_bytes()).hexdigest())
            production = 'diff --git a/codex-rs/tui/src/lib.rs b/codex-rs/tui/src/lib.rs\n+production\n'
            test = 'diff --git a/codex-rs/app-server/tests/suite/v2/thread_settings_update.rs b/codex-rs/app-server/tests/suite/v2/thread_settings_update.rs\n+original test\n'
            (root / 'native.patch').write_text(production + test)
            current = root / 'current.patch'
            current.write_text(production + test.replace('original test', 'fixed test'))
            args = (root, current, metadata, 'owner/repo', upstream, '0.166.0')
            self.assertEqual(reusable_frontend(*args), root / 'codex')
            for key, value in [('head_sha', 'c' * 40), ('status', 'in_progress'),
                               ('head_repository', {'full_name': 'other/repo'})]:
                with self.subTest(identity=key), self.assertRaises(ValueError):
                    reusable_frontend(root, current, {**metadata, key: value}, *args[3:])
            current.write_text((production + test).replace('+production', '+changed production'))
            with self.assertRaisesRegex(ValueError, 'production inputs changed'):
                reusable_frontend(*args)
            current.write_text(production + test)
            (root / 'codex').write_bytes(b'different output')
            with self.assertRaisesRegex(ValueError, 'executable differs'):
                reusable_frontend(*args)

    def test_every_topic_requires_executed_success_and_complete_suite(self):
        for fault in [None, "missing-topic", "zero", "failed", "missing", "symlink"]:
            with self.subTest(fault=fault), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                for suite, (package, _, topics) in SUITES.items():
                    path = root / f"profile-tui-upstream-{suite}.log"
                    passed = "".join(f"PASS [0.01s] {package} {name}\n" for name in topics.values())
                    summary = f"Summary [3s] {len(topics)} tests run: {len(topics)} passed\n"
                    path.write_text(passed + summary)
                target = root / "profile-tui-upstream-engine.log"
                if fault == "missing-topic":
                    target.write_text("PASS [0.01s] codex-core unrelated\nSummary [3s] 1 tests run: 1 passed\n")
                elif fault == "zero": target.write_text("Summary [3s] 0 tests run: 0 passed\n")
                elif fault == "failed": target.write_text("Summary [3s] 2 tests run: 1 passed, 1 failed\n")
                elif fault == "missing": target.unlink()
                elif fault == "symlink":
                    other = root / "other"
                    target.rename(other)
                    target.symlink_to(other)
                if fault:
                    with self.assertRaises(ValueError): verify(root)
                else:
                    result = verify(root)
                    self.assertEqual(set(result), set(SUITES))
                    self.assertEqual(result["engine"]["topics"], list(SUITES["engine"][2]))


if __name__ == "__main__":
    unittest.main()
