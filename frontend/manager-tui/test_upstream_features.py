from pathlib import Path
import tempfile
import unittest

from check_upstream_features import SUITES, verify


class UpstreamFeatureProofTests(unittest.TestCase):
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
