#!/usr/bin/env python3
from __future__ import annotations

from pathlib import Path
import re
import subprocess
import unittest

ROOT = Path(__file__).resolve().parents[2]
BASE = "9dedc27b73ba1b045b3c1724036323e049e1dcf9"
ACCEPTED_SOURCE = "566034e1aff42bde2f3221ebc2da2b16def77d44"
EXPECTED_MAIN = "9aa8be4c63fe8d60dea130b979c0c12d1a5164bc"


def run(*args: str) -> str:
    return subprocess.check_output(args, cwd=ROOT, text=True)


class Rald7AcceptanceContract(unittest.TestCase):
    def test_all_remote_actions_are_sha_pinned(self) -> None:
        action = re.compile(r"^[ \t-]*uses:\s*([^\s]+)\s*$")
        sha = re.compile(r"^[^@\s]+@[0-9a-f]{40}$")
        for path in sorted((ROOT / ".github" / "workflows").glob("*.yml")):
            for number, line in enumerate(path.read_text().splitlines(), 1):
                match = action.match(line)
                if not match:
                    continue
                value = match.group(1)
                if value.startswith("./"):
                    continue
                self.assertRegex(value, sha, f"{path}:{number} action is not SHA-pinned")

    def test_scheduled_producer_is_pinned_to_current_accepted_source(self) -> None:
        text = (ROOT / ".github/workflows/auto-release-termux.yml").read_text()
        self.assertEqual(
            text.count(f"CODEX_SOURCE_SHA: '{ACCEPTED_SOURCE}'"),
            1,
        )
        self.assertIn("cron: '0 */6 * * *'", text)
        self.assertIn(
            "RALD5_PUBLICATION_AUTHORIZED: ${{ github.event_name == 'schedule' || (github.event_name == 'workflow_dispatch' && inputs.rald5_publication_authorized) }}",
            text,
        )
        self.assertIn('case "$GITHUB_EVENT_NAME" in', text)
        self.assertIn("schedule)", text)
        for gate in [
            "RALD4_POSITIVE_GATE",
            "RALD5_SAME_VERSION_ACCEPTANCE",
            "RALD45_TRANSITION_STAGE",
            "RALD45_TRANSITION_PROMOTE",
            "RALD5_NEGATIVE_GATE",
        ]:
            self.assertIn(f'test "$' + gate + '" != true', text)
        self.assertIn(
            "github.event_name == 'workflow_dispatch' && inputs.rald5_publication_authorized",
            text,
        )
        self.assertIn("uses: ./.github/workflows/publish-termux-update-pages.yml", text)

    def test_added_lines_contain_no_credentials_or_private_keys(self) -> None:
        diff = run("git", "diff", "--unified=0", f"{BASE}..HEAD", "--", ".")
        added = "\n".join(
            line[1:] for line in diff.splitlines()
            if line.startswith("+") and not line.startswith("+++")
        )
        private_marker = "BEGIN " + "PRIVATE KEY"
        private_patterns = [
            private_marker,
            "BEGIN RSA " + "PRIVATE KEY",
            "BEGIN OPENSSH " + "PRIVATE KEY",
            "BEGIN EC " + "PRIVATE KEY",
        ]
        for marker in private_patterns:
            self.assertNotIn(marker, added)

        token_patterns = [
            re.compile("gh" + r"[pousr]_[A-Za-z0-9_]{20,}"),
            re.compile("github_" + r"pat_[A-Za-z0-9_]{20,}"),
            re.compile("sk-" + r"(?:proj-)?[A-Za-z0-9_-]{20,}"),
            re.compile("AKIA" + r"[0-9A-Z]{16}"),
        ]
        for pattern in token_patterns:
            self.assertIsNone(pattern.search(added), pattern.pattern)

        secret_name = "CODEX_RELEASE_" + "SIGNING_KEY"
        for line in added.splitlines():
            if secret_name in line:
                self.assertIn("secrets." + secret_name, line)

    def test_tracked_tree_contains_no_private_key_material(self) -> None:
        marker = ("BEGIN " + "PRIVATE KEY").encode()
        variants = [
            marker,
            ("BEGIN RSA " + "PRIVATE KEY").encode(),
            ("BEGIN OPENSSH " + "PRIVATE KEY").encode(),
            ("BEGIN EC " + "PRIVATE KEY").encode(),
        ]
        for raw in run("git", "ls-files", "-z").split("\0"):
            if not raw:
                continue
            data = (ROOT / raw).read_bytes()
            for value in variants:
                self.assertNotIn(value, data, raw)

    def test_android_core_override_is_test_only(self) -> None:
        source = (ROOT / "crates/core/src/main.rs").read_text()
        helper = source.split("fn running_core_artifact_for_local_build()", 1)[1].split(
            "#[cfg(unix)]\nfn activate_local_built_update", 1
        )[0]
        self.assertIn("#[cfg(test)]", helper)
        self.assertIn('std::env::var_os("CODEX_B10_RELEASE_CORE")', helper)
        production_tail = helper.split("#[cfg(test)]", 1)[1]
        self.assertIn("std::env::current_exe()", production_tail)

        platform = source.split("fn release_platform_matches_this_build", 1)[1].split(
            "#[cfg(unix)]\nfn validate_local_release_policy", 1
        )[0]
        self.assertIn("#[cfg(test)]", platform)
        self.assertIn('std::env::var_os("CODEX_B10_RELEASE_CORE")', platform)
        self.assertIn('platform == "android"', platform)
        self.assertIn('architecture == "aarch64"', platform)

        generation = source.split("fn generation_requirements_for_loaded", 1)[1].split(
            "#[cfg(unix)]\nfn with_qualified_loaded_runtime", 1
        )[0]
        self.assertIn("#[cfg(test)]", generation)
        self.assertIn('std::env::var_os("CODEX_B10_RELEASE_CORE")', generation)
        self.assertIn('_manifest.expected_platform == "android"', generation)
        self.assertIn('_manifest.expected_architecture == "aarch64"', generation)

    def test_android_dependent_tests_are_focused_not_dropped(self) -> None:
        workflow = (ROOT / ".github/workflows/rald7-full-acceptance.yml").read_text()
        for name in [
            "test_r6_builder_publish_output_enters_existing_signed_release_admission",
            "test_rald1_local_derived_fallback_and_explicit_build_preserve_public_authority",
        ]:
            self.assertGreaterEqual(workflow.count(name), 3)
        self.assertIn('CODEX_B10_RELEASE_CORE="$RALD7_ANDROID_CORE"', workflow)
        self.assertNotIn("CODEX_B10_RELEASE_CORE=%s", workflow)

    def test_publication_workflows_remain_fail_closed(self) -> None:
        auto = (ROOT / ".github/workflows/auto-release-termux.yml").read_text()
        pages = (ROOT / ".github/workflows/publish-termux-update-pages.yml").read_text()
        self.assertIn("force:false", auto)
        self.assertIn("expected_main_sha", auto)
        self.assertIn("expected_main_sha", pages)
        self.assertIn("needs.verify_public.result == 'success'", auto)
        self.assertIn("pages: write", pages)
        self.assertNotIn("force: true", auto)
        self.assertNotIn("force: true", pages)


if __name__ == "__main__":
    unittest.main()
