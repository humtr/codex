#!/usr/bin/env python3
from pathlib import Path
import re
import unittest

WORKFLOW = Path(__file__).resolve().parents[1] / "workflows" / "auto-release-termux.yml"
SOURCE_SHA = "1cdcb44d035ec5b1ce6339f2aa7b0e95831a6f0a"
UPLOAD_SHA = "ea165f8d65b6e75b540449e92b4886f43607fa02"
DOWNLOAD_SHA = "d3f86a106a0bac45b974a628896c90dbdf5c8093"


class WorkflowContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.text = WORKFLOW.read_text()

    def test_schedule_dispatch_permissions_and_concurrency_are_exact(self) -> None:
        text = self.text
        self.assertIn("workflow_dispatch:\n", text)
        self.assertIn("cron: '0 */6 * * *'", text)
        self.assertIn("permissions:\n  contents: read\n", text)
        self.assertIn("group: termux-hosted-release-producer", text)
        self.assertIn("cancel-in-progress: false", text)
        self.assertNotRegex(text, r"(?m)^\s+(contents|pages|actions|id-token):\s*write\s*$")

    def test_source_and_actions_are_immutable(self) -> None:
        text = self.text
        self.assertIn(f"CODEX_SOURCE_SHA: '{SOURCE_SHA}'", text)
        self.assertGreaterEqual(text.count('git -C source checkout --detach FETCH_HEAD'), 2)
        uses = re.findall(r"(?m)^\s*uses:\s*([^\s]+)\s*$", text)
        self.assertEqual(
            uses,
            [
                f"actions/upload-artifact@{UPLOAD_SHA}",
                f"actions/download-artifact@{DOWNLOAD_SHA}",
                f"actions/upload-artifact@{UPLOAD_SHA}",
                f"actions/download-artifact@{DOWNLOAD_SHA}",
                f"actions/upload-artifact@{UPLOAD_SHA}",
            ],
        )
        for action in uses:
            self.assertRegex(action, r"@[0-9a-f]{40}\Z")

    def test_pre_sign_boundary_has_no_release_authority(self) -> None:
        pre_sign, sign = self.text.split("\n  sign:\n", 1)
        lower = pre_sign.lower()
        for forbidden in [
            "secrets.",
            "--private-key",
            "codex-release-builder publish",
            "gh release",
            "git push",
            "deploy-pages",
            "upload-pages",
            "self-hosted",
        ]:
            self.assertNotIn(forbidden, lower)
        for forbidden in ["gh release", "git push", "deploy-pages", "upload-pages", "self-hosted"]:
            self.assertNotIn(forbidden, sign.lower())
        self.assertIn("--defer-manager-probe", pre_sign)
        self.assertIn("retention-days: 1", pre_sign)
        self.assertIn(".manager-probe-deferred", pre_sign)
        self.assertIn("qualified-candidate", pre_sign)

    def test_hosted_build_and_android_smoke_are_explicit(self) -> None:
        text = self.text
        self.assertIn("runs-on: ubuntu-24.04", text)
        self.assertIn("runs-on: macos-15", text)
        self.assertIn("aarch64-linux-android", text)
        self.assertIn("system-images;android-35;google_apis;arm64-v8a", text)
        for tool in [
            "$ANDROID_SDK_ROOT/cmdline-tools/latest/bin/sdkmanager",
            "$ANDROID_SDK_ROOT/cmdline-tools/latest/bin/avdmanager",
            "$ANDROID_SDK_ROOT/platform-tools/adb",
            "$ANDROID_SDK_ROOT/emulator/emulator",
        ]:
            self.assertIn(tool, text)
        self.assertIn('for tool in "$sdkmanager" "$avdmanager" "$adb" "$emulator"; do', text)
        self.assertIn('test -x "$tool"', text)
        self.assertIn("-accel off -gpu swiftshader", text)
        self.assertNotIn("yes | sdkmanager --licenses", text)
        self.assertNotIn('echo no | avdmanager create avd', text)
        self.assertNotIn('"$adb" wait-for-device', text)
        self.assertIn('emulator_pid=$!', text)
        self.assertIn('trap cleanup_emulator EXIT', text)
        self.assertIn('if ! kill -0 "$emulator_pid" 2>/dev/null; then', text)
        self.assertIn('"$adb" get-state 2>/dev/null || true', text)
        self.assertIn('for _ in $(seq 1 180); do', text)
        self.assertIn("CODEX_MANAGER_ARTIFACT_PROBE=1", text)
        self.assertIn("--artifact-probe", text)
        self.assertIn("$remote/core update --help", text)
        self.assertIn("$remote/runtime --version", text)

    def test_public_stable_is_authenticated_before_comparison(self) -> None:
        text = self.text
        self.assertIn("$WRAPPER_PUBLIC_KEY_SHA256", text)
        self.assertGreaterEqual(text.count("openssl pkeyutl -verify -pubin"), 2)
        self.assertIn("manifest-entry", text)
        self.assertIn("rald3_preflight.py compare", text)
        self.assertIn("UPSTREAM_LATEST_URL", text)
        self.assertIn("comparison_version='${{ steps.stable.outputs.current_version }}'", text)
        self.assertIn("default: false", text)
        self.assertIn("test \"$GITHUB_EVENT_NAME\" = workflow_dispatch", text)
        self.assertIn("test \"$GITHUB_REF\" = refs/heads/rewrite/rust-core", text)


if __name__ == "__main__":
    unittest.main()
