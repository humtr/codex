import hashlib
import json
import os
import tempfile
import unittest
from pathlib import Path

from playwright_android import BEFORE, AFTER, OLD, NEW, apply, corrected_bundle


class AndroidDependencyTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.original = Path(os.environ["PLAYWRIGHT_ORIGINAL_BUNDLE"]).read_bytes()
        if hashlib.sha256(cls.original).hexdigest() != BEFORE:
            raise AssertionError("Exact original bundle fixture required")

    def test_exact_transform_changes_only_three_cache_selectors(self):
        corrected = corrected_bundle(self.original)
        self.assertEqual(hashlib.sha256(corrected).hexdigest(), AFTER)
        self.assertEqual(len(OLD), 3)
        reversed_data = corrected
        for old, new in zip(OLD, NEW):
            self.assertEqual(corrected.count(new), 1)
            reversed_data = reversed_data.replace(new, old)
        self.assertEqual(reversed_data, self.original)

    def test_same_corrected_identity_is_idempotent(self):
        corrected = corrected_bundle(self.original)
        self.assertEqual(corrected_bundle(corrected), corrected)

    def test_drift_refuses(self):
        for data in [b"", self.original + b" ", self.original.replace(OLD[0], b"invalid", 1)]:
            with self.assertRaises(ValueError):
                corrected_bundle(data)

    def test_owned_apply_and_failed_version_leave_bytes_intact(self):
        with tempfile.TemporaryDirectory() as root:
            candidate = Path(root) / "candidate"
            package = candidate / "node_modules/playwright-core"
            (package / "lib").mkdir(parents=True)
            metadata = package / "package.json"
            metadata.write_text(json.dumps({"version": "1.62.0"}))
            bundle = package / "lib/coreBundle.js"
            bundle.write_bytes(self.original)
            result = apply(candidate)
            self.assertEqual(result["after"], AFTER)
            self.assertEqual(apply(candidate), result)
            kept = bundle.read_bytes()
            metadata.write_text(json.dumps({"version": "1.62.1"}))
            with self.assertRaises(ValueError):
                apply(candidate)
            self.assertEqual(bundle.read_bytes(), kept)


if __name__ == "__main__":
    unittest.main()
