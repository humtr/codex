#!/usr/bin/env python3
"""Negative checks for documentation structure/anchor validation."""
import unittest
from pathlib import Path

from scripts.check_documentation import ROOT, errors, check_link, git_slug, valid_anchors


class DocumentationRules(unittest.TestCase):
    def setUp(self):
        errors.clear()

    def tearDown(self):
        errors.clear()

    def test_handoff_uses_one_current_name_and_preserves_historical_copy(self):
        self.assertTrue((ROOT / "HANDOFF.md").is_file())
        self.assertFalse((ROOT / "PRO_HANDOFF.md").exists())
        self.assertTrue((ROOT / "docs/legacy/2026-11-10/PRO_HANDOFF.md").is_file())

    def test_original_heading_links_appear_in_split_indexes(self):
        for relative, anchor in (
            ("SPEC.md", "46-component-responsibilities-and-user-flow-contracts"),
            ("docs/spec/BRIDGE.md", "work-recording-scope-and-entry-selection"),
            ("docs/spec/DELIVERY.md", "app-server-daemon-and-remote-control-authority"),
            ("docs/spec/USER_INTERFACES.md", "profile-selection-and-isolated-launch"),
        ):
            with self.subTest(path=relative):
                self.assertIn(anchor, valid_anchors((ROOT / relative).read_text()))

    def test_bad_relative_target_is_not_silently_accepted(self):
        check_link(ROOT / "README.md", "docs/spec/NOT_A_REAL_CONTRACT.md")
        self.assertEqual(len(errors), 1)
        self.assertIn("missing link", errors[0])

    def test_wrong_anchor_is_not_silently_accepted(self):
        check_link(ROOT / "README.md", "CURRENT.md#this-never-existed")
        self.assertEqual(len(errors), 1)
        self.assertIn("unresolved anchor", errors[0])

    def test_real_unicode_and_numeric_headers_slug_consistently(self):
        self.assertEqual(git_slug("4.6 Component Responsibilities and User-Flow Contracts"),
                         "46-component-responsibilities-and-user-flow-contracts")
        self.assertIn("현행", git_slug("현행 기준"))


if __name__ == "__main__":
    unittest.main()
