#!/usr/bin/env python3
"""Negative/positive tests for the documentation source-of-truth link gate."""
import tempfile
import unittest
from pathlib import Path

import check_doc_links


class MarkdownLinkChecks(unittest.TestCase):
    def test_github_style_duplicate_and_code_fences(self):
        content = "# Section\n# Section\n```md\n# Not a heading\n```\n"
        self.assertEqual(check_doc_links.anchors_for(content), {"section", "section-1"})

    def test_links_validate_with_correct_relative_anchors(self):
        with tempfile.TemporaryDirectory(prefix="wrapper-doc-check-") as temp:
            root = Path(temp).resolve()
            sub = root / "docs"
            sub.mkdir()
            source = sub / "index.md"
            destination = root / "README.md"
            destination.write_text("# Accepted Stage 1\n")
            source.write_text(
                "[parent](../README.md#accepted-stage-1)\n"
                "[external](https://example.invalid/unresolved#does-not-apply)\n"
            )
            docs = {source: source.read_text(), destination: destination.read_text()}
            self.assertEqual(check_doc_links.validate(docs, root), [])

    def test_missing_heading_and_missing_file_are_rejected(self):
        with tempfile.TemporaryDirectory(prefix="wrapper-doc-check-") as temp:
            root = Path(temp).resolve()
            source = root / "source.md"
            destination = root / "ok.md"
            destination.write_text("# Heading\n")
            source.write_text("[bad](ok.md#wrong)\n[absent](missing.md)\n")
            issues = check_doc_links.validate(
                {source: source.read_text(), destination: destination.read_text()}, root
            )
            self.assertEqual(len(issues), 2)
            self.assertTrue(any("missing heading" in item for item in issues))
            self.assertTrue(any("missing file" in item for item in issues))

    def test_tracked_files_and_directory_prefixes_pass(self):
        with tempfile.TemporaryDirectory(prefix="wrapper-doc-check-") as temp:
            root = Path(temp).resolve()
            destination = root / "tools" / "bin" / "script"
            destination.parent.mkdir(parents=True)
            destination.touch()
            source = root / "source.md"
            contents = "[script](tools/bin/script)\n[bin](tools/bin/)\n[tools](tools/)\n"
            source.write_text(contents)
            self.assertEqual(check_doc_links.validate(
                {source: contents}, root, {source, destination}
            ), [])

    def test_untracked_file_and_directory_are_rejected_even_when_present(self):
        with tempfile.TemporaryDirectory(prefix="wrapper-doc-check-") as temp:
            root = Path(temp).resolve()
            (root / "tools").mkdir()
            (root / "tools-extra").mkdir()
            tracked = root / "tools" / "tracked"
            tracked.touch()
            (root / "tools" / "generated").touch()
            (root / "alias").symlink_to(tracked)
            source = root / "source.md"
            contents = "[generated](tools/generated)\n[similar name](tools-extra/)\n[alias](alias)\n"
            source.write_text(contents)
            self.assertEqual(check_doc_links.validate(
                {source: contents}, root, {source, tracked}
            ), [
                "source.md:1: untracked link: tools/generated",
                "source.md:2: untracked link: tools-extra/",
                "source.md:3: untracked link: alias",
            ])

    def test_out_of_tree_reference_is_rejected(self):
        with tempfile.TemporaryDirectory(prefix="wrapper-doc-check-") as temp:
            root = (Path(temp) / "repo").resolve()
            root.mkdir()
            source = root / "source.md"
            source.write_text("[escape](../foreign.md)\n")
            issues = check_doc_links.validate({source: source.read_text()}, root)
            self.assertEqual(len(issues), 1)
            self.assertIn("out-of-tree", issues[0])


if __name__ == "__main__":
    unittest.main()
