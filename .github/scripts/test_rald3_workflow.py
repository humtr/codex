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
        self.assertEqual(text.splitlines().count("    runs-on: ubuntu-24.04"), 2)
        self.assertEqual(text.splitlines().count("    runs-on: ubuntu-24.04-arm"), 1)
        self.assertNotIn("runs-on: macos-15", text)
        self.assertIn("aarch64-linux-android", text)
        self.assertIn("ANDROID_RUNTIME_APEX_COMMIT: '8aeb37cca394ce39c1311744c60960cbd466aa77'", text)
        self.assertIn("ANDROID_RUNTIME_APEX_SHA256: '83bf0dce249728dae48149b80d28b48115c54adad95a352120d58a6ac669d1fc'", text)
        self.assertIn('test "$(uname -m)" = aarch64', text)
        self.assertIn("https://android.googlesource.com/platform/prebuilts/runtime/+/${ANDROID_RUNTIME_APEX_COMMIT}/mainline/runtime/apex/com.android.runtime-arm64.apex?format=TEXT", text)
        self.assertIn('--max-filesize 20971520', text)
        self.assertIn('base64 --decode "$root/runtime.apex.b64"', text)
        self.assertIn('test "$(wc -c < "$root/runtime.apex")" -eq 13881344', text)
        self.assertIn('"$ANDROID_RUNTIME_APEX_SHA256"', text)
        self.assertIn('unzip -p "$root/runtime.apex" apex_payload.img', text)
        self.assertIn('test "$(wc -c < "$root/apex_payload.img")" -eq 13783040', text)
        self.assertIn('debugfs -R "dump -p /bin/linker64 $root/bin/linker64"', text)
        for library in ["libc.so", "libdl.so", "libm.so"]:
            self.assertIn(f'/lib64/bionic/{library}', text)
        self.assertIn("file \"$root/bin/linker64\" | grep -F 'ARM aarch64'", text)
        self.assertIn("readelf -h \"$root/bin/linker64\" | grep -Eq 'Machine:[[:space:]]+AArch64'", text)
        self.assertIn('env -i CODEX_MANAGER_ARTIFACT_PROBE=1 LD_LIBRARY_PATH="$libdir"', text)
        self.assertIn('"$linker" "$candidate/manager" --artifact-probe', text)
        self.assertIn('"$linker" "$candidate/core" update --help', text)
        self.assertIn('env -i "$candidate/runtime" --version', text)
        self.assertIn("manager_rc=$?", text)
        self.assertIn("core_rc=$?", text)
        self.assertIn("runtime_rc=$?", text)
        self.assertIn("stat -c '%s' \"$RUNNER_TEMP/rald4-qualified.tar\"", text)
        self.assertNotIn("stat -f '%z' \"$RUNNER_TEMP/rald4-qualified.tar\"", text)
        for forbidden in [
            "system-images;android-35;google_apis;x86_64",
            "sdkmanager",
            "avdmanager",
            "/dev/kvm",
            "ANDROID_AVD_HOME",
            "ro.dalvik.vm.native.bridge",
            "ro.ndk_translation.version",
            "Stop Android emulator",
        ]:
            self.assertNotIn(forbidden, text)
        self.assertIn(".manager-probe-deferred", text)
        self.assertIn("unsigned candidate passed native Android/AArch64 pre-sign smoke", text)

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
