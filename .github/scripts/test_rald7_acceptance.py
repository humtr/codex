#!/usr/bin/env python3
from __future__ import annotations

from pathlib import Path
import re
import subprocess
import tempfile
import textwrap
import unittest

ROOT = Path(__file__).resolve().parents[2]
BASE = "f8b456c9155f0f4f9c970f398947d75cfcd4dc66"
ACCEPTED_SOURCE = "9300a68852c879f4730e9191e46837cfab6745d9"
EXPECTED_MAIN = "56ba28e1baee87721767ba34b505cc2bc1303c44"


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
            "LEGACY_LAG_JUMP_REMEDIATION",
            "UX1_SAME_VERSION_DEPLOY",
            "UPDATE_PROGRESS_SAME_VERSION_DEPLOY",
            "EXACT_CURRENT_FASTPATH_SAME_VERSION_DEPLOY",
            "NO_EMOJI_SAME_VERSION_DEPLOY",
        ]:
            self.assertIn(f'test "$' + gate + '" != true', text)
        self.assertIn(
            "github.event_name == 'workflow_dispatch' && inputs.rald5_publication_authorized",
            text,
        )
        self.assertIn("uses: ./.github/workflows/publish-termux-update-pages.yml", text)
        self.assertEqual(
            text.count("transition_args=(--legacy-activation-doctor-unsupported)"),
            1,
        )
        self.assertNotIn("transition_args=()", text)
        self.assertIn('test "$doctor_capability" = unsupported', text)
        self.assertNotIn('test "$doctor_capability" = supported', text)


    def test_update_human_output_proofs_follow_ux1_contract(self) -> None:
        text = (ROOT / ".github/workflows/auto-release-termux.yml").read_text()
        self.assertEqual(text.count("activated channel generation %s"), 1)
        self.assertIn(
            'if test "$current" = "$UX1_DEPLOY_CURRENT_GENERATION"; then',
            text,
        )
        self.assertNotIn("codex is already up to date (generation", text)
        self.assertIn("Updating Codex %s -> %s...", text)
        self.assertIn("Updating the Termux release for Codex %s...", text)
        self.assertIn("Verified and activated the signed Termux release.", text)
        self.assertIn("Codex %s is now active.", text)
        self.assertIn("Codex %s is already up to date.", text)

    def test_legacy_lag_jump_remediation_is_exact_manual_one_shot(self) -> None:
        text = (ROOT / ".github/workflows/auto-release-termux.yml").read_text()
        header = text.split("\njobs:\n", 1)[0]
        self.assertIn("legacy_lag_jump_remediation:", header)
        block = header.split("legacy_lag_jump_remediation:", 1)[1].split("\n  schedule:", 1)[0]
        self.assertIn("type: boolean", block)
        self.assertIn("default: false", block)
        for exact in [
            "LEGACY_LAG_REMEDIATION_VERSION: '0.155.0'",
            "LEGACY_LAG_REMEDIATION_CURRENT_GENERATION: 'local-hosted-0-155-0-566034e1aff4'",
            "LEGACY_LAG_REMEDIATION_CURRENT_SEQUENCE: '12'",
            "LEGACY_LAG_REMEDIATION_TARGET_SEQUENCE: '13'",
            "LEGACY_LAG_SOURCE_SHA: '0621105fd1be8461b370466fbfa981938241074d'",
            "LEGACY_LAG_VERSION: '0.153.4'",
            "LEGACY_LAG_ARCHIVE_SHA256: 'fc395cb043a1093ab0db34f44aba3199bfaa9ce640cd9be7fd588f44b0da64a4'",
        ]:
            self.assertIn(exact, text)
        decision = text.split("- name: Resolve official upstream stable", 1)[1].split(
            "- name: Cross-build Core and Manager", 1
        )[0]
        self.assertIn(
            "LEGACY_LAG_JUMP_REMEDIATION: ${{ github.event_name == 'workflow_dispatch' && inputs.legacy_lag_jump_remediation }}",
            decision,
        )
        remediation = decision.split('if test "$LEGACY_LAG_JUMP_REMEDIATION" = true; then', 1)[1].split(
            'suffix="${upstream_version//./-}', 1
        )[0]
        for required in [
            'test "$GITHUB_EVENT_NAME" = workflow_dispatch',
            'test "$GITHUB_REF" = refs/heads/main',
            'test "$RALD5_PUBLICATION_AUTHORIZED" = true',
            'test "$RALD5_SAME_VERSION_ACCEPTANCE" != true',
            'test "$RALD45_TRANSITION_STAGE" != true',
            'test "$RALD45_TRANSITION_PROMOTE" != true',
            'test "$RALD5_NEGATIVE_GATE" != true',
            'test "$candidate" = false',
            "LEGACY_LAG_REMEDIATION_CURRENT_GENERATION",
            "LEGACY_LAG_REMEDIATION_CURRENT_SEQUENCE",
            "LEGACY_LAG_REMEDIATION_TARGET_SEQUENCE",
            "candidate=true",
            "-legacy-lag-remediation",
        ]:
            self.assertIn(required, remediation)
        self.assertLess(
            decision.index("rald3_preflight.py compare"),
            decision.index('if test "$LEGACY_LAG_JUMP_REMEDIATION" = true; then'),
        )
        self.assertIn("Require exact sequence-12 component bytes for legacy-lag remediation", text)
        self.assertIn("sequence-13 descriptor changes more than generation identity and legacy doctor signal", text)
        self.assertIn("Rebuild exact historical R10 Core for legacy-lag remediation", text)
        self.assertIn("LEGACY_LAG_BUILD_ANDROID_API: '30'", text)
        self.assertIn("legacy-lag-sequence7-signed", text)
        self.assertIn("release-sequence 7", text)
        self.assertIn("Bootstrap protected source state and update through staged Pages", text)
        self.assertIn('test "$(sed -n \'s/^previous=//p\' "$state")" = "$LEGACY_LAG_GENERATION"', text)
        self.assertIn("doctor --json", text)
        self.assertIn("rald5-second-update.out", text)
        self.assertIn("force:false", text)
        self.assertNotIn("force: true", text)

    def test_ux1_same_version_deployment_is_exact_manual_one_shot(self) -> None:
        text = (ROOT / ".github/workflows/auto-release-termux.yml").read_text()
        header = text.split("\njobs:\n", 1)[0]
        self.assertIn("ux1_same_version_deploy:", header)
        block = header.split("ux1_same_version_deploy:", 1)[1].split("\n  schedule:", 1)[0]
        self.assertIn("type: boolean", block)
        self.assertIn("default: false", block)
        for exact in [
            "UX1_ACCEPTED_SOURCE_SHA: '07f77b89a177682954d80ae3f797377c4731de64'",
            "UX1_DEPLOY_VERSION: '0.155.1'",
            "UX1_DEPLOY_CURRENT_GENERATION: 'local-hosted-0-155-1-566034e1aff4'",
            "UX1_DEPLOY_CURRENT_SEQUENCE: '14'",
            "UX1_DEPLOY_TARGET_SEQUENCE: '15'",
        ]:
            self.assertIn(exact, text)
        decision = text.split("- name: Resolve official upstream stable", 1)[1].split(
            "- name: Cross-build Core and Manager", 1
        )[0]
        self.assertIn(
            "UX1_SAME_VERSION_DEPLOY: ${{ github.event_name == 'workflow_dispatch' && inputs.ux1_same_version_deploy }}",
            decision,
        )
        deployment = decision.split('if test "$UX1_SAME_VERSION_DEPLOY" = true; then', 1)[1].split(
            'suffix="${upstream_version//./-}', 1
        )[0]
        for required in [
            'test "$GITHUB_EVENT_NAME" = workflow_dispatch',
            'test "$GITHUB_REF" = refs/heads/main',
            'test "$RALD5_PUBLICATION_AUTHORIZED" = true',
            'test "$RALD5_SAME_VERSION_ACCEPTANCE" != true',
            'test "$RALD45_TRANSITION_STAGE" != true',
            'test "$RALD45_TRANSITION_PROMOTE" != true',
            'test "$RALD5_NEGATIVE_GATE" != true',
            'test "$LEGACY_LAG_JUMP_REMEDIATION" != true',
            'test "$CODEX_SOURCE_SHA" = "$UX1_ACCEPTED_SOURCE_SHA"',
            'test "$candidate" = false',
            "UX1_DEPLOY_CURRENT_GENERATION",
            "UX1_DEPLOY_CURRENT_SEQUENCE",
            "UX1_DEPLOY_TARGET_SEQUENCE",
            "candidate=true",
            "-ux1-human-output",
        ]:
            self.assertIn(required, deployment)
        self.assertLess(
            decision.index("rald3_preflight.py compare"),
            decision.index('if test "$UX1_SAME_VERSION_DEPLOY" = true; then'),
        )
        self.assertIn("Require exact sequence-14 non-Core bytes for UX-1 deployment", text)
        self.assertIn("test \"$candidate_core_digest\" != \"$current_core_digest\"", text)
        self.assertIn("UX-1 descriptor schema/order changed", text)
        self.assertIn("UX-1 current descriptor Core digest does not match sequence-14 Core", text)
        self.assertIn("UX-1 candidate descriptor Core digest does not match UX-1 Core", text)
        self.assertIn('if key in {"generation_id", "core_artifact_digest"}:', text)
        self.assertIn("zip(current_records, candidate_records, strict=True)", text)
        self.assertIn("UX-1 descriptor changed forbidden field", text)

        comparator_marker = (
            'python3 - "$stable/generation.meta" "$candidate/generation.meta" \\\n'
            '            "$UX1_DEPLOY_CURRENT_GENERATION"'
        )
        comparator = text.split(comparator_marker, 1)[1].split("<<'PY'\n", 1)[1].split(
            "\n          PY", 1
        )[0]
        comparator = textwrap.dedent(comparator)
        old_digest = "1" * 64
        new_digest = "2" * 64
        current_id = "local-hosted-0-155-1-566034e1aff4"
        candidate_id = "local-hosted-0-155-1-07f77b89a177-ux1-human-output"
        descriptor = (
            "codex-local-generation-v2\n"
            "generation_id\t{generation_id}\n"
            "upstream_package_identity\topenai/codex:codex-package-aarch64-unknown-linux-musl.tar.gz\n"
            "upstream_package_version\t0.155.1\n"
            "source_artifact_digest\t{source_digest}\n"
            "expected_platform\tandroid\n"
            "expected_architecture\taarch64\n"
            "patch_policy_id\ttermux-fd-remap-v1\n"
            "patch_report\tstable-report\n"
            "runtime_digest\t{runtime_digest}\n"
            "core_artifact_digest\t{core_digest}\n"
            "manager_artifact_digest\t{manager_digest}\n"
            "core_api_identity\tcore-api-v1\n"
            "persistent_schema_identity\tschema-v1\n"
            "qualification\tqualified\n"
            "creation_metadata\tr10-browser-helper-bridge-v1\n"
            "upstream_doctor\tunsupported\n"
            "helper_count\t2\n"
            "helper\ttermux-browser-open-v1\t{open_digest}\n"
            "helper\ttermux-browser-manual-v1\t{manual_digest}\n"
        )
        common = dict(
            source_digest="3" * 64,
            runtime_digest="4" * 64,
            manager_digest="5" * 64,
            open_digest="6" * 64,
            manual_digest="7" * 64,
        )
        with tempfile.TemporaryDirectory() as tmp:
            current = Path(tmp) / "current.meta"
            candidate = Path(tmp) / "candidate.meta"
            current.write_text(descriptor.format(
                generation_id=current_id, core_digest=old_digest, **common
            ))
            candidate.write_text(descriptor.format(
                generation_id=candidate_id, core_digest=new_digest, **common
            ))
            accepted = subprocess.run(
                ["python3", "-c", comparator, str(current), str(candidate),
                 current_id, candidate_id, old_digest, new_digest],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(accepted.returncode, 0, accepted.stderr)
            candidate.write_text(
                descriptor.format(
                    generation_id=candidate_id,
                    core_digest=new_digest,
                    **{**common, "runtime_digest": "8" * 64},
                )
            )
            rejected = subprocess.run(
                ["python3", "-c", comparator, str(current), str(candidate),
                 current_id, candidate_id, old_digest, new_digest],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertNotEqual(rejected.returncode, 0)
            self.assertIn("descriptor changed forbidden field: runtime_digest", rejected.stderr)
        self.assertIn(
            'if test "$current" = "$UX1_DEPLOY_CURRENT_GENERATION"; then',
            text,
        )
        self.assertIn("activated channel generation %s", text)
        self.assertIn("Codex %s is already up to date.", text)
        self.assertIn("force:false", text)
        self.assertNotIn("force: true", text)

    def test_no_emoji_same_version_deployment_is_exact_manual_one_shot(self) -> None:
        text = (ROOT / ".github/workflows/auto-release-termux.yml").read_text()
        header = text.split("\njobs:\n", 1)[0]
        self.assertIn("no_emoji_same_version_deploy:", header)
        block = header.split("no_emoji_same_version_deploy:", 1)[1].split("\n  schedule:", 1)[0]
        self.assertIn("type: boolean", block)
        self.assertIn("default: false", block)
        for exact in [
            "CODEX_SOURCE_SHA: '9300a68852c879f4730e9191e46837cfab6745d9'",
            "NO_EMOJI_ACCEPTED_SOURCE_SHA: '9300a68852c879f4730e9191e46837cfab6745d9'",
            "NO_EMOJI_DEPLOY_VERSION: '0.155.1'",
            "NO_EMOJI_DEPLOY_CURRENT_GENERATION: 'local-hosted-0-155-1-7817b939c81c-exact-current-fastpath'",
            "NO_EMOJI_DEPLOY_CURRENT_SEQUENCE: '17'",
            "NO_EMOJI_DEPLOY_TARGET_SEQUENCE: '18'",
        ]:
            self.assertIn(exact, text)
        decision = text.split("- name: Resolve official upstream stable", 1)[1].split(
            "- name: Cross-build Core and Manager", 1
        )[0]
        self.assertIn(
            "NO_EMOJI_SAME_VERSION_DEPLOY: ${{ github.event_name == 'workflow_dispatch' && inputs.no_emoji_same_version_deploy }}",
            decision,
        )
        deployment = decision.split('if test "$NO_EMOJI_SAME_VERSION_DEPLOY" = true; then', 1)[1].split(
            'suffix="${upstream_version//./-}', 1
        )[0]
        for required in [
            'test "$GITHUB_EVENT_NAME" = workflow_dispatch',
            'test "$GITHUB_REF" = refs/heads/main',
            'test "$RALD5_PUBLICATION_AUTHORIZED" = true',
            'test "$RALD4_POSITIVE_GATE" != true',
            'test "$RALD5_SAME_VERSION_ACCEPTANCE" != true',
            'test "$RALD45_TRANSITION_STAGE" != true',
            'test "$RALD45_TRANSITION_PROMOTE" != true',
            'test "$RALD5_NEGATIVE_GATE" != true',
            'test "$LEGACY_LAG_JUMP_REMEDIATION" != true',
            'test "$UX1_SAME_VERSION_DEPLOY" != true',
            'test "$UPDATE_PROGRESS_SAME_VERSION_DEPLOY" != true',
            'test "$EXACT_CURRENT_FASTPATH_SAME_VERSION_DEPLOY" != true',
            'test "$CODEX_SOURCE_SHA" = "$NO_EMOJI_ACCEPTED_SOURCE_SHA"',
            'test "$candidate" = false',
            "NO_EMOJI_DEPLOY_CURRENT_GENERATION",
            "NO_EMOJI_DEPLOY_CURRENT_SEQUENCE",
            "NO_EMOJI_DEPLOY_TARGET_SEQUENCE",
            "candidate=true",
            "acceptance_suffix=\'-no-emoji-output-r2\'",
        ]:
            self.assertIn(required, deployment)
        self.assertLess(
            decision.index("rald3_preflight.py compare"),
            decision.index('if test "$NO_EMOJI_SAME_VERSION_DEPLOY" = true; then'),
        )
        self.assertIn("Require exact sequence-17 non-Core bytes for no-emoji deployment", text)
        self.assertIn("needs.producer.outputs.no_emoji_same_version_deploy", text)
        self.assertIn(r"Codex %s is now active. \342\234\205\n", text)
        self.assertIn("Codex %s is already up to date.", text)
        self.assertIn('2> "$RUNNER_TEMP/rald5-second-update.err"', text)
        self.assertIn('test ! -s "$RUNNER_TEMP/rald5-second-update.err"', text)
        self.assertIn("test \"$candidate_core_digest\" != \"$current_core_digest\"", text)
        self.assertIn("no-emoji descriptor schema/order changed", text)
        self.assertIn("no-emoji current descriptor Core digest does not match sequence-17 Core", text)
        self.assertIn("no-emoji candidate descriptor Core digest does not match accepted Core", text)
        self.assertIn('if key in {"generation_id", "core_artifact_digest"}:', text)
        self.assertIn("force:false", text)
        self.assertNotIn("force: true", text)
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
