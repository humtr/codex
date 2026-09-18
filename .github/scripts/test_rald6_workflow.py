#!/usr/bin/env python3
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = ROOT / ".github" / "workflows" / "rald6-delivery-e2e.yml"
TEXT = WORKFLOW.read_text()

SOURCE = "9972a3288c0531ba744e9bd1356273d9a080aa79"
PARENT = "6b974145dc5490b22a31fa7dd7d4edff828480b6"
MAIN = "f221de1225471fb5eda5bbdfcbd0d9db0c2f43b1"
PUBLIC = "local-hosted-0-154-0-37fbbd8033b8-rald45-transition"
FIXTURE = "local-rald6-seq12-9972a3288c05"


class Rald6WorkflowContract(unittest.TestCase):
    def test_one_shot_source_and_public_state_are_pinned(self) -> None:
        for value in [SOURCE, PARENT, MAIN, PUBLIC, FIXTURE]:
            self.assertIn(value, TEXT)
        self.assertIn("test \"$GITHUB_EVENT_NAME\" = push", TEXT)
        self.assertIn("test \"$GITHUB_REF\" = refs/heads/rewrite/rust-core", TEXT)
        self.assertIn('git -C trigger rev-parse HEAD^', TEXT)
        self.assertIn('git -C source rev-parse HEAD', TEXT)

    def test_arm_fixture_builder_uses_preinstalled_native_rust_only(self) -> None:
        self.assertIn("command -v cargo >/dev/null", TEXT)
        self.assertIn("command -v rustc >/dev/null", TEXT)
        self.assertIn("cargo build --release --locked", TEXT)
        self.assertNotIn("rustup", TEXT)
        self.assertNotIn("apt-get", TEXT)
        self.assertNotIn("apt install", TEXT)

    def test_secret_is_bounded_to_one_fixture_signing_step(self) -> None:
        secret_expr = "$" + "{{ secrets.CODEX_RELEASE_SIGNING_KEY }}"
        self.assertEqual(TEXT.count(secret_expr), 1)
        self.assertIn("Sign sequence-12 fixture with bounded production authority", TEXT)
        self.assertIn("source/.github/scripts/rald4_signing.py", TEXT)
        self.assertIn("release-sequence \"$FIXTURE_SEQUENCE\"", TEXT)
        self.assertNotIn("actions/upload-artifact", TEXT)
        self.assertNotIn("actions/upload-pages-artifact", TEXT)
        self.assertNotIn("actions/deploy-pages", TEXT)
        self.assertNotIn("git push", TEXT)
        self.assertNotIn("gh api", TEXT)

    def test_fresh_install_uses_public_immutable_installer_and_current_stable(self) -> None:
        self.assertIn(
            'installer_url="https://raw.githubusercontent.com/$GITHUB_REPOSITORY/$RALD6_SOURCE_SHA/install-online.sh"',
            TEXT,
        )
        self.assertIn('cmp source/install-online.sh "$installer"', TEXT)
        self.assertIn("codex-bootstrap: initial installation complete", TEXT)
        self.assertIn('sed -n \'s/^current=//p\'', TEXT)
        self.assertIn('release_sequence', TEXT)
        self.assertIn("codex-cli %s", TEXT)
        self.assertIn("rald6-public-before", TEXT)
        self.assertIn("rald6-public-after", TEXT)

    def test_same_client_fixture_promotion_uses_normal_update_path(self) -> None:
        self.assertIn("stable-v11", TEXT)
        self.assertIn("stable-v12", TEXT)
        self.assertIn('ln -s stable-v11 "$web/active"', TEXT)
        self.assertIn('mv -Tf "$web/active.next" "$web/active"', TEXT)
        self.assertIn('export CODEX_TERMUX_UPDATE_INDEX_URL="$index_url"', TEXT)
        self.assertIn("activated channel generation %s", TEXT)
        self.assertIn('sed -n \'s/^previous=//p\'', TEXT)
        self.assertIn("rald6-fixture-before-noop", TEXT)
        self.assertIn("rald6-fixture-after-noop", TEXT)

    def test_workflow_has_no_public_mutation_permissions(self) -> None:
        self.assertIn("permissions:\n  contents: read", TEXT)
        self.assertNotIn("contents: write", TEXT)
        self.assertNotIn("pages: write", TEXT)
        self.assertNotIn("id-token: write", TEXT)
        self.assertNotIn("RALD-7", TEXT)


if __name__ == "__main__":
    unittest.main()
