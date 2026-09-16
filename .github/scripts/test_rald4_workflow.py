#!/usr/bin/env python3
from pathlib import Path
import re
import unittest

WORKFLOW = Path(__file__).resolve().parents[1] / "workflows" / "auto-release-termux.yml"
SOURCE_SHA = "dfcdbead5fdcde454bedebf1a269816977f4f544"
SECRET = "secrets.CODEX_RELEASE_SIGNING_KEY"
UPLOAD_SHA = "ea165f8d65b6e75b540449e92b4886f43607fa02"
DOWNLOAD_SHA = "d3f86a106a0bac45b974a628896c90dbdf5c8093"


class Rald4WorkflowContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.text = WORKFLOW.read_text()
        cls.pre_sign, cls.sign = cls.text.split("\n  sign:\n", 1)

    def test_secret_is_exactly_one_step_local_reference(self) -> None:
        self.assertNotIn("secrets.", self.pre_sign)
        self.assertEqual(self.text.count(SECRET), 1)
        self.assertRegex(
            self.sign,
            r"(?m)^          CODEX_RELEASE_SIGNING_KEY: \$\{\{ secrets\.CODEX_RELEASE_SIGNING_KEY \}\}$",
        )
        global_env = self.text.split("\njobs:\n", 1)[0]
        self.assertNotIn("CODEX_RELEASE_SIGNING_KEY", global_env)

    def test_signing_requires_candidate_and_successful_smoke(self) -> None:
        self.assertIn("needs: [producer, smoke]", self.sign)
        self.assertIn(
            "if: needs.producer.outputs.candidate == 'true' && needs.smoke.result == 'success'",
            self.sign,
        )
        self.assertLess(self.pre_sign.index("qualified-candidate"), len(self.pre_sign))
        self.assertLess(self.sign.index("qualified-candidate"), self.sign.index(SECRET))
        self.assertIn("release_sequence: ${{ steps.stable.outputs.release_sequence }}", self.text)
        self.assertIn("rald3_preflight.py next-sequence", self.text)

    def test_signing_source_and_tools_are_pinned_and_fail_closed(self) -> None:
        self.assertIn(f"CODEX_SOURCE_SHA: '{SOURCE_SHA}'", self.text)
        self.assertIn("rald4_signing.py", self.sign)
        self.assertIn("test_rald4_signing.py", self.sign)
        self.assertIn("$WRAPPER_PUBLIC_KEY_SHA256", self.sign)
        self.assertIn("openssl pkeyutl -verify -pubin", self.sign)
        self.assertIn("find \"$RUNNER_TEMP\" -maxdepth 1 -name '.rald4-signing-*'", self.sign)
        self.assertNotIn("actions/cache", self.text.lower())

    def test_acceptance_gate_is_manual_ref_bound_and_non_publishable(self) -> None:
        text = self.text
        self.assertIn("rald4_positive_gate:", text)
        self.assertIn("type: boolean", text)
        self.assertIn("default: false", text)
        self.assertIn("RALD4_ACCEPTANCE_BASELINE_VERSION: '0.153.4'", text)
        self.assertIn("RALD4_ACCEPTANCE_TARGET_VERSION: '0.154.0'", text)
        self.assertIn(
            "RALD4_POSITIVE_GATE: ${{ github.event_name == 'workflow_dispatch' && inputs.rald4_positive_gate }}",
            self.pre_sign,
        )
        self.assertIn("test \"$GITHUB_EVENT_NAME\" = workflow_dispatch", self.pre_sign)
        self.assertIn("test \"$GITHUB_REF\" = refs/heads/rewrite/rust-core", self.pre_sign)
        self.assertIn(
            "test '${{ steps.stable.outputs.current_version }}' = \"$RALD4_ACCEPTANCE_TARGET_VERSION\"",
            self.pre_sign,
        )
        self.assertIn("test \"$upstream_version\" = \"$RALD4_ACCEPTANCE_TARGET_VERSION\"", self.pre_sign)
        self.assertIn("comparison_version='${{ steps.stable.outputs.current_version }}'", self.pre_sign)
        self.assertIn('comparison_version="$RALD4_ACCEPTANCE_BASELINE_VERSION"', self.pre_sign)
        self.assertIn("release_base='https://rald4-acceptance.invalid/", self.sign)
        self.assertIn("Remove acceptance-only signed output", self.sign)
        self.assertIn("if: needs.producer.outputs.acceptance_gate == 'true'", self.sign)
        self.assertIn("if: needs.producer.outputs.acceptance_gate != 'true'", self.sign)

    def test_only_signed_output_crosses_signing_job_boundary(self) -> None:
        self.assertIn(f"actions/download-artifact@{DOWNLOAD_SHA}", self.sign)
        self.assertIn(f"actions/upload-artifact@{UPLOAD_SHA}", self.sign)
        self.assertIn("path: ${{ runner.temp }}/rald4-sign/signed-candidate.tar", self.sign)
        self.assertNotRegex(self.sign, r"(?m)^\s*path:\s*\$\{\{ runner\.temp \}\}/?\s*$")
        self.assertNotIn("release-key.pem", self.sign)
        self.assertNotIn(".rald4-signing-", self.sign.split("Upload signed candidate only", 1)[1])

    def test_rald4_has_no_publication_authority(self) -> None:
        lower = self.text.lower()
        for forbidden in [
            "gh release",
            "git push",
            "deploy-pages",
            "upload-pages",
            "contents: write",
            "pages: write",
            "id-token: write",
        ]:
            self.assertNotIn(forbidden, lower)
        self.assertEqual(self.text.count("permissions:\n  contents: read\n"), 1)


if __name__ == "__main__":
    unittest.main()
