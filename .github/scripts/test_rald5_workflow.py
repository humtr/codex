#!/usr/bin/env python3
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[1]
AUTO = ROOT / "workflows" / "auto-release-termux.yml"
PAGES = ROOT / "workflows" / "publish-termux-update-pages.yml"
RALD5_SOURCE_SHA = "fa1b887b7e726202e309a2eb731aad303a6c7e03"
CONFIGURE_PAGES_SHA = "983d7736d9b0ae728b81ab479565c72886d7745b"
UPLOAD_PAGES_SHA = "7b1f4a764d45c48632c6b24a0339c27f5614fb0b"
DEPLOY_PAGES_SHA = "d6db90164ac5ed86f2b6aed7e0febac5b3c0c03e"


class Rald5WorkflowContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.auto = AUTO.read_text()
        cls.pages = PAGES.read_text()
        cls.header = cls.auto.split("\njobs:\n", 1)[0]
        cls.stage = cls.auto.split("\n  stage_release:\n", 1)[1].split("\n  pages:\n", 1)[0]
        cls.pages_job = cls.auto.split("\n  pages:\n", 1)[1].split("\n  verify_public:\n", 1)[0]
        cls.verify = cls.auto.split("\n  verify_public:\n", 1)[1].split("\n  promote:\n", 1)[0]
        cls.promote = cls.auto.split("\n  promote:\n", 1)[1]
        cls.decision = cls.auto.split("- name: Resolve official upstream stable", 1)[1].split(
            "- name: Cross-build Core and Manager", 1
        )[0]

    def test_publication_source_and_actions_are_immutable(self) -> None:
        self.assertIn(f"RALD5_SOURCE_SHA: '{RALD5_SOURCE_SHA}'", self.auto)
        self.assertGreaterEqual(self.auto.count('fetch --no-tags --depth=1 origin "$RALD5_SOURCE_SHA"'), 3)
        self.assertGreaterEqual(
            self.auto.count('test "$(git -C publication-source rev-parse HEAD)" = "$RALD5_SOURCE_SHA"'),
            3,
        )
        for required in [
            "test_rald5_publication.py",
            "test_rald5_publication_contract.py",
            "test_rald5_promotion.py",
            "rald5_promotion.py",
        ]:
            self.assertIn(required, self.auto)
        for workflow in [self.auto, self.pages]:
            for action in re.findall(r"(?m)^\s*uses:\s*([^\s]+)\s*$", workflow):
                if action.startswith("./"):
                    continue
                self.assertRegex(action, r"@[0-9a-f]{40}\Z")
        self.assertIn(f"actions/configure-pages@{CONFIGURE_PAGES_SHA}", self.pages)
        self.assertIn(f"actions/upload-pages-artifact@{UPLOAD_PAGES_SHA}", self.pages)
        self.assertIn(f"actions/deploy-pages@{DEPLOY_PAGES_SHA}", self.pages)

    def test_public_mutation_requires_explicit_dispatch_authorization(self) -> None:
        self.assertIn("rald5_publication_authorized:", self.header)
        auth_block = self.header.split("rald5_publication_authorized:", 1)[1].split(
            "rald5_same_version_acceptance:", 1
        )[0]
        self.assertIn("type: boolean", auth_block)
        self.assertIn("default: false", auth_block)
        self.assertIn(
            "RALD5_PUBLICATION_AUTHORIZED: ${{ github.event_name == 'workflow_dispatch' && inputs.rald5_publication_authorized }}",
            self.decision,
        )
        self.assertIn("publication_authorized=%s", self.decision)
        self.assertIn('test "$GITHUB_EVENT_NAME" = workflow_dispatch', self.decision)
        self.assertIn('test "$RALD5_PUBLICATION_AUTHORIZED" = true', self.decision)
        for job in [self.stage, self.pages_job, self.verify, self.promote]:
            self.assertIn("needs.producer.outputs.publication_authorized == 'true'", job)
        self.assertIn("workflow_call:", self.pages)
        self.assertNotIn("workflow_dispatch:", self.pages)

    def test_same_version_acceptance_is_exact_manual_and_publication_gated(self) -> None:
        self.assertIn("rald5_same_version_acceptance:", self.header)
        bridge_input = self.header.split("rald5_same_version_acceptance:", 1)[1].split(
            "rald5_negative_gate:", 1
        )[0]
        self.assertIn("type: boolean", bridge_input)
        self.assertIn("default: false", bridge_input)
        self.assertIn("RALD5_SAME_VERSION_ACCEPTANCE_VERSION: '0.154.0'", self.header)
        self.assertIn(
            "RALD5_SAME_VERSION_ACCEPTANCE: ${{ github.event_name == 'workflow_dispatch' && inputs.rald5_same_version_acceptance }}",
            self.decision,
        )
        bridge = self.decision.split('if test "$RALD5_SAME_VERSION_ACCEPTANCE" = true; then', 1)[1].split(
            'if test "$RALD5_NEGATIVE_GATE" = true; then', 1
        )[0]
        self.assertIn('test "$GITHUB_EVENT_NAME" = workflow_dispatch', bridge)
        self.assertIn('test "$GITHUB_REF" = refs/heads/rewrite/rust-core', bridge)
        self.assertIn('test "$RALD4_POSITIVE_GATE" != true', bridge)
        self.assertIn('test "$RALD5_PUBLICATION_AUTHORIZED" = true', bridge)
        self.assertIn("steps.stable.outputs.current_version", bridge)
        self.assertGreaterEqual(bridge.count("RALD5_SAME_VERSION_ACCEPTANCE_VERSION"), 2)
        self.assertIn('test "$candidate" = false', bridge)
        self.assertIn("candidate=true", bridge)
        self.assertIn("-rald5-same-version-acceptance", bridge)
        self.assertIn("same_version_acceptance=%s", self.decision)
        self.assertLess(
            self.decision.index("rald3_preflight.py compare"),
            self.decision.index('if test "$RALD5_SAME_VERSION_ACCEPTANCE" = true; then'),
        )
        rald4 = self.decision.split('if test "$RALD4_POSITIVE_GATE" = true; then', 2)[2].split(
            'if test "$RALD5_PUBLICATION_AUTHORIZED" = true; then', 1
        )[0]
        self.assertIn('test "$RALD5_SAME_VERSION_ACCEPTANCE" != true', rald4)
        for job in [self.stage, self.pages_job, self.verify, self.promote]:
            self.assertNotIn("same_version_acceptance == 'true'", job)

    def test_write_authority_is_job_local_and_after_signing(self) -> None:
        self.assertIn("permissions:\n  contents: read\n", self.header)
        self.assertNotRegex(self.header, r"(?m)^\s+(contents|pages|id-token):\s*write\s*$")
        write_lines = re.findall(r"(?m)^\s+(contents|pages|id-token):\s*write\s*$", self.auto)
        self.assertEqual(sorted(write_lines), ["contents", "contents", "id-token", "pages"])
        self.assertIn("needs: [producer, sign]", self.stage)
        self.assertIn("needs.sign.result == 'success'", self.stage)
        self.assertIn("contents: write", self.stage)
        self.assertIn("needs: [producer, verify_public]", self.promote)
        self.assertIn("needs.verify_public.result == 'success'", self.promote)
        self.assertIn("contents: write", self.promote)
        self.assertNotIn("secrets.", self.stage + self.verify + self.promote)
        self.assertNotIn("CODEX_RELEASE_SIGNING_KEY", self.stage + self.verify + self.promote)
        self.assertNotIn("secrets.", self.pages)
        self.assertNotIn("CODEX_RELEASE_SIGNING_KEY", self.pages)
        self.assertNotIn("git push", (self.stage + self.verify + self.promote).lower())

    def test_release_staging_is_exact_prerelease_and_rechecks_lkg_parent(self) -> None:
        stage = self.stage
        self.assertIn("rald5_publication.py prepare-assets", stage)
        self.assertLess(stage.index("expected_main='${{ needs.producer.outputs.expected_main_sha }}'"), stage.index("tag_sha="))
        self.assertIn("git/ref/heads/main", stage)
        self.assertIn('test "$tag_sha" = "$expected_main"', stage)
        self.assertIn('-f target_commitish="$expected_main"', stage)
        self.assertNotIn('-f ref="refs/tags/$generation" -f sha="$CODEX_SOURCE_SHA"', stage)
        self.assertIn("-F draft=true -F prerelease=true", stage)
        self.assertIn("-F draft=false -F prerelease=true", stage)
        self.assertIn("test \"$prerelease\" = true", stage)
        self.assertIn("assets?per_page=100", stage)
        self.assertIn("--jq 'length'", stage)
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
        self.assertIn("sha256sum \"site/$CURRENT_ID/$rel\"", pages)
        self.assertIn("sha256sum \"site/$GENERATION_ID/$rel\"", pages)
        self.assertIn("1073741824", pages)
        self.assertNotIn('cp current-index "site/update-index-v1"', pages)
        self.assertNotIn('cp current-index.sig "site/update-index-v1.sig"', pages)
        self.assertIn("refs/heads/main", pages)

    def test_negative_gate_cannot_bypass_publication_authorization(self) -> None:
        decision = self.decision
        self.assertIn("RALD5_NEGATIVE_GATE", decision)
        negative = decision.split('if test "$RALD5_NEGATIVE_GATE" = true; then', 1)[1]
        self.assertIn('test "$RALD5_PUBLICATION_AUTHORIZED" = true', negative)
        self.assertIn('test "$candidate" = true', negative)
        self.assertNotIn("comparison_version=", negative)
        self.assertNotIn("RALD4_ACCEPTANCE_BASELINE_VERSION", negative)
        self.assertIn("rald3_preflight.py compare", decision)
        self.assertIn("rald5-negative-gate", self.verify)
        self.assertIn("negative gate rejected staged candidate before promotion", self.verify.lower())
        self.assertIn("needs.producer.outputs.negative_gate != 'true'", self.promote)

    def test_https_readback_compares_pages_to_immutable_release_bytes(self) -> None:
        verify = self.verify
        for mapping in [
            "release.manifest release.manifest",
            "release.sig release.sig",
            "codex-code-mode-host codex-code-mode-host",
            "core core",
            "generation.meta generation.meta",
            "helper-0 helpers/0",
            "helper-1 helpers/1",
            "manager manager",
            "runtime runtime",
            "candidate-update-index-v1 update-index-v1",
            "candidate-update-index-v1.sig update-index-v1.sig",
        ]:
            self.assertIn(f"compare_release_asset {mapping}", verify)
        self.assertIn('cmp "$release_readback/$asset" "$root/$candidate/$relative"', verify)
        self.assertIn('cmp "$release_readback/update-public-key.pem" "$authority"', verify)
        self.assertIn("rald5_publication.py verify-public", verify)

    def test_disposable_runtime_proof_is_pre_promotion_and_noop_bounded(self) -> None:
        verify = self.verify
        self.assertIn("runs-on: ubuntu-24.04-arm", verify)
        self.assertIn('test "$(uname -m)" = aarch64', verify)
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

    def test_promotion_is_atomic_nonforce_cas_and_race_fails_closed(self) -> None:
        promote = self.promote
        self.assertIn("test \"$before\" = \"$expected\"", promote)
        self.assertIn("parents:[$parent]", promote)
        self.assertIn('path:"update-index-v1"', promote)
        self.assertIn('path:"update-index-v1.sig"', promote)
        self.assertIn("force:false", promote)
        self.assertIn("git/refs/heads/main", promote)
        self.assertIn("for attempt in 1 2 3; do", promote)
        self.assertIn('test -n "$actual"', promote)
        self.assertIn("rald5_promotion.py", promote)
        self.assertIn('--actual "$actual"', promote)
        self.assertIn('--update-rc "$update_rc"', promote)
        self.assertNotIn("equivalent-concurrent-commit", promote)
        self.assertNotIn("contents/update-index-v1?ref=$actual", promote)
        self.assertIn("$PUBLIC_INDEX_URL", promote)
        self.assertIn("openssl pkeyutl -verify -pubin", promote)
        self.assertNotIn("force:true", promote)
        self.assertNotIn("git push", promote.lower())

    def test_broken_candidate_or_failed_gate_cannot_reach_promotion(self) -> None:
        self.assertIn("needs.stage_release.result == 'success'", self.pages_job)
        self.assertIn("needs.stage_release.result == 'success'", self.verify)
        self.assertIn("needs.pages.result == 'success'", self.verify)
        self.assertIn("needs.verify_public.result == 'success'", self.promote)
        self.assertIn("rald5-negative-gate", self.verify)
        self.assertIn("exit 1", self.verify)

    def test_rald4_acceptance_output_cannot_enter_rald5_publication(self) -> None:
        self.assertIn("needs.producer.outputs.acceptance_gate != 'true'", self.stage)
        self.assertIn("needs.producer.outputs.acceptance_gate != 'true'", self.pages_job)
        self.assertIn("needs.producer.outputs.acceptance_gate != 'true'", self.verify)
        self.assertIn("needs.producer.outputs.acceptance_gate != 'true'", self.promote)
        self.assertNotIn("rald4-acceptance.invalid", self.stage + self.pages_job + self.verify + self.promote)


if __name__ == "__main__":
    unittest.main()
