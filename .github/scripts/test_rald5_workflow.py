#!/usr/bin/env python3
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[1]
AUTO = ROOT / "workflows" / "auto-release-termux.yml"
PAGES = ROOT / "workflows" / "publish-termux-update-pages.yml"
RALD5_SOURCE_SHA = "0122308e75019b9003a379d79bb4710a938fb3ca"
CONFIGURE_PAGES_SHA = "983d7736d9b0ae728b81ab479565c72886d7745b"
UPLOAD_PAGES_SHA = "7b1f4a764d45c48632c6b24a0339c27f5614fb0b"
DEPLOY_PAGES_SHA = "d6db90164ac5ed86f2b6aed7e0febac5b3c0c03e"


class Rald5WorkflowContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.auto = AUTO.read_text()
        cls.pages = PAGES.read_text()
        cls.stage = cls.auto.split("\n  stage_release:\n", 1)[1].split("\n  pages:\n", 1)[0]
        cls.verify = cls.auto.split("\n  verify_public:\n", 1)[1].split("\n  promote:\n", 1)[0]
        cls.promote = cls.auto.split("\n  promote:\n", 1)[1]
        cls.decision = cls.auto.split("- name: Resolve official upstream stable", 1)[1].split(
            "- name: Cross-build Core and Manager", 1
        )[0]

    def test_publication_verifier_and_actions_are_immutable(self) -> None:
        self.assertIn(f"RALD5_SOURCE_SHA: '{RALD5_SOURCE_SHA}'", self.auto)
        self.assertGreaterEqual(self.auto.count('fetch --no-tags --depth=1 origin "$RALD5_SOURCE_SHA"'), 2)
        self.assertGreaterEqual(self.auto.count('test "$(git -C publication-source rev-parse HEAD)" = "$RALD5_SOURCE_SHA"'), 2)
        for workflow in [self.auto, self.pages]:
            for action in re.findall(r"(?m)^\s*uses:\s*([^\s]+)\s*$", workflow):
                if action.startswith("./"):
                    continue
                self.assertRegex(action, r"@[0-9a-f]{40}\Z")
        self.assertIn(f"actions/configure-pages@{CONFIGURE_PAGES_SHA}", self.pages)
        self.assertIn(f"actions/upload-pages-artifact@{UPLOAD_PAGES_SHA}", self.pages)
        self.assertIn(f"actions/deploy-pages@{DEPLOY_PAGES_SHA}", self.pages)

    def test_write_authority_is_job_local_and_after_signing(self) -> None:
        global_header = self.auto.split("\njobs:\n", 1)[0]
        self.assertIn("permissions:\n  contents: read\n", global_header)
        self.assertNotRegex(global_header, r"(?m)^\s+(contents|pages|id-token):\s*write\s*$")
        write_lines = re.findall(r"(?m)^\s+(contents|pages|id-token):\s*write\s*$", self.auto)
        self.assertEqual(sorted(write_lines), ["contents", "contents", "id-token", "pages"])
        self.assertIn("needs: [producer, sign]", self.stage)
        self.assertIn("needs.sign.result == 'success'", self.stage)
        self.assertIn("contents: write", self.stage)
        self.assertIn("needs: [producer, verify_public]", self.promote)
        self.assertIn("needs.verify_public.result == 'success'", self.promote)
        self.assertIn("contents: write", self.promote)
        self.assertNotIn("secrets.", self.stage + self.verify + self.promote)
        self.assertNotIn("git push", (self.stage + self.verify + self.promote).lower())

    def test_release_staging_is_exact_prerelease_and_preserves_current_stable(self) -> None:
        stage = self.stage
        self.assertIn("rald5_publication.py prepare-assets", stage)
        self.assertIn("test \"$tag_sha\" = \"$CODEX_SOURCE_SHA\"", stage)
        self.assertIn("-F draft=true -F prerelease=true", stage)
        self.assertIn("-F draft=false -F prerelease=true", stage)
        self.assertIn("test \"$prerelease\" = true", stage)
        self.assertIn("diff -u \"$work/expected-assets\" \"$work/actual-assets\"", stage)
        self.assertIn("cmp \"$assets/$name\" \"$destination/$name\"", stage)
        self.assertIn("browser_download_url", stage)
        self.assertIn("cmp \"$assets/$name\" \"$readback\"", stage)
        for forbidden in ["gh release delete", "git push --force", "refs/tags/$current", "make_latest"]:
            self.assertNotIn(forbidden, stage.lower())

    def test_pages_keeps_authenticated_lkg_while_staging_candidate(self) -> None:
        pages = self.pages
        self.assertIn("workflow_call:", pages)
        for name in ["generation_id", "release_sequence", "current_generation", "expected_main_sha"]:
            self.assertIn(f"      {name}:\n", pages)
        self.assertIn("$EXPECTED_MAIN_SHA", pages)
        self.assertIn('test "$CURRENT_ID" = "$CURRENT_GENERATION"', pages)
        self.assertIn('test "$CURRENT_ID" != "$GENERATION_ID"', pages)
        self.assertIn("openssl pkeyutl -verify -pubin", pages)
        self.assertIn("site/$CURRENT_ID", pages)
        self.assertIn("site/$GENERATION_ID", pages)
        self.assertIn("1073741824", pages)
        self.assertNotIn('cp current-index "site/update-index-v1"', pages)
        self.assertNotIn('cp current-index.sig "site/update-index-v1.sig"', pages)
        self.assertIn("refs/heads/main", pages)

    def test_negative_gate_never_changes_production_candidate_comparison(self) -> None:
        decision = self.decision
        self.assertIn("RALD5_NEGATIVE_GATE", decision)
        self.assertIn("test \"$candidate\" = true", decision)
        self.assertIn("test \"$RALD4_POSITIVE_GATE\" != true", decision)
        negative = decision.split('if test "$RALD5_NEGATIVE_GATE" = true; then', 1)[1]
        self.assertNotIn("comparison_version=", negative)
        self.assertNotIn("RALD4_ACCEPTANCE_BASELINE_VERSION", negative)
        self.assertIn("rald3_preflight.py compare", decision)
        self.assertIn("printf 'negative_gate=%s\\n'", decision)
        self.assertIn("rald5-negative-gate", self.verify)
        self.assertIn("negative gate rejected staged candidate before promotion", self.verify.lower())
        self.assertIn("needs.producer.outputs.negative_gate != 'true'", self.promote)

    def test_public_readback_and_disposable_runtime_proof_are_pre_promotion(self) -> None:
        verify = self.verify
        self.assertIn("runs-on: ubuntu-24.04-arm", verify)
        self.assertIn('test "$(uname -m)" = aarch64', verify)
        self.assertIn("rald5_publication.py verify-public", verify)
        self.assertIn("https://humtr.github.io/codex/$candidate/update-index-v1", verify)
        self.assertIn("sudo test ! -e /system", verify)
        self.assertIn("ANDROID_RUNTIME_APEX_SHA256", verify)
        self.assertNotIn("apt-get", verify)
        self.assertNotIn("sdkmanager", verify)
        self.assertIn("publication-source/bootstrap/codex-bootstrap", verify)
        self.assertIn("CODEX_TERMUX_UPDATE_INDEX_URL", verify)
        self.assertIn("activated channel generation %s", verify)
        self.assertIn("codex-cli %s", verify)
        self.assertIn('"$PREFIX/bin/codex" doctor', verify)
        self.assertIn("codex is already up to date (generation %s)", verify)
        self.assertIn("rald5-before-noop", verify)
        self.assertIn("rald5-after-noop", verify)
        self.assertIn("diff -u", verify)
        self.assertIn("sudo rm -rf /system", verify)

    def test_promotion_is_non_force_exact_parent_cas_with_ambiguous_result_readback(self) -> None:
        promote = self.promote
        self.assertIn("test \"$before\" = \"$expected\"", promote)
        self.assertIn("parents:[$parent]", promote)
        self.assertIn("force:false", promote)
        self.assertIn("git/refs/heads/main", promote)
        self.assertIn("actual=\"$(gh api", promote)
        self.assertIn("promotion_result=committed", promote)
        self.assertIn("promotion_result=equivalent-concurrent-commit", promote)
        self.assertIn("contents/update-index-v1?ref=$actual", promote)
        self.assertIn("cmp \"$work/update-index-v1\" \"$work/actual-index\"", promote)
        self.assertIn("cmp \"$work/update-index-v1.sig\" \"$work/actual-index.sig\"", promote)
        self.assertIn("$PUBLIC_INDEX_URL", promote)
        self.assertIn("openssl pkeyutl -verify -pubin", promote)
        self.assertNotIn("force:true", promote)
        self.assertNotIn("git push", promote.lower())

    def test_rald4_acceptance_output_cannot_enter_rald5_publication(self) -> None:
        self.assertIn("needs.producer.outputs.acceptance_gate != 'true'", self.stage)
        self.assertIn("needs.producer.outputs.acceptance_gate != 'true'", self.verify)
        self.assertIn("needs.producer.outputs.acceptance_gate != 'true'", self.promote)
        self.assertNotIn("rald4-acceptance.invalid", self.stage + self.verify + self.promote)


if __name__ == "__main__":
    unittest.main()
