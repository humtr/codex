#!/usr/bin/env python3
"""Regression tests for declared document dependencies and negative link cases."""
import shutil
import tempfile
import unittest
from pathlib import Path

from scripts.check_documentation import (
    ROOT, MANIFEST, CHECKS, DependencyError, dependencies, errors, check_link,
    git_slug, main, test_inputs, valid_anchors,
)


class DocumentationRules(unittest.TestCase):
    def setUp(self):
        errors.clear()

    def tearDown(self):
        errors.clear()

    def test_every_declared_validator_runs_and_required_inputs_exist(self):
        data = dependencies()
        self.assertEqual(set(data["tests"]), set(CHECKS))
        for test_id in data["tests"]:
            with self.subTest(test_id=test_id):
                paths = test_inputs(test_id)
                self.assertTrue(paths)

    def test_missing_required_document_is_a_real_failure(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / MANIFEST).parent.mkdir(parents=True)
            shutil.copyfile(ROOT / MANIFEST, root / MANIFEST)
            spec = Path(dependencies()["documents"]["spec"])
            (root / spec).parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / spec, root / spec)
            self.assertIn("spec", test_inputs("product_contract", root))
            (root / spec).unlink()
            with self.assertRaisesRegex(DependencyError, "required document missing"):
                test_inputs("product_contract", root)

    def test_obsolete_legacy_directory_is_not_a_mandatory_input(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / MANIFEST).parent.mkdir(parents=True)
            shutil.copyfile(ROOT / MANIFEST, root / MANIFEST)
            spec = Path(dependencies()["documents"]["spec"])
            (root / spec).parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / spec, root / spec)
            self.assertFalse((root / "docs/legacy").exists())
            self.assertEqual(set(test_inputs("product_contract", root)), {"spec"})

    def test_manifest_alias_relocation_needs_no_test_code_change(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            path = root / MANIFEST
            path.parent.mkdir(parents=True)
            data = dependencies()
            data["documents"]["spec"] = "contract/relocated.md"
            # The same contract test resolves a relocated input without any
            # file name in validator or test code.
            (root / "contract").mkdir()
            shutil.copyfile(ROOT / dependencies()["documents"]["spec"],
                            root / data["documents"]["spec"])
            self.assertEqual(test_inputs("product_contract", root, data)["spec"],
                             (root / "contract/relocated.md").resolve())

    def test_invalid_validator_mapping_fails_closed(self):
        data = dependencies()
        data["tests"]["product_contract"]["validation"] = "unimplemented"
        with self.assertRaisesRegex(DependencyError, "unbound validator"):
            test_inputs("product_contract", data=data)

    def test_product_contract_has_stable_deep_link_anchors(self):
        anchors = valid_anchors(test_inputs("product_contract")["spec"].read_text())
        for item in ("1-product-definition",
                     "4-architecture-and-ownership",
                     "46-component-responsibilities-and-user-flow-contracts",
                     "detailed-feature-and-release-acceptance"):
            self.assertIn(item, anchors)

    def test_bad_relative_target_fails(self):
        check_link(test_inputs("relative_links")["index"], "NOT_A_REAL_CONTRACT.md")
        self.assertEqual(len(errors), 1)
        self.assertIn("missing link", errors[0])

    def test_wrong_anchor_fails(self):
        check_link(test_inputs("current_status")["current"], "#this-never-existed")
        self.assertEqual(len(errors), 1)
        self.assertIn("unresolved anchor", errors[0])

    def test_unicode_and_numbered_headings(self):
        self.assertEqual(git_slug("4.6 Component Responsibilities and User-Flow Contracts"),
                         "46-component-responsibilities-and-user-flow-contracts")
        self.assertIn("현행", git_slug("현행 기준"))

    def test_full_validator_passes_on_real_tree(self):
        self.assertEqual(main(), 0, errors)


if __name__ == "__main__":
    unittest.main()
