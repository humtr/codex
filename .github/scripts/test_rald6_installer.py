#!/usr/bin/env python3
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
INSTALLER = ROOT / "install-online.sh"
LOCAL_INSTALL = ROOT / "install.sh"
BOOTSTRAP = ROOT / "bootstrap" / "codex-bootstrap"

PINNED_LOCAL_SOURCE = "b58f2432c6a9aa9b018f3afc9502d16680e1b043"
PINNED_KEY_SHA256 = "62ab1640b6b4e63afbd5952d11a0bd0a9f1cb78ddde2472e003a42c4db2b832c"
STABLE_GENERATION = "local-hosted-0-154-0-37fbbd8033b8-rald45-transition"


class Rald6InstallerContract(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.text = INSTALLER.read_text()
        cls.local = LOCAL_INSTALL.read_text()
        cls.bootstrap = BOOTSTRAP.read_text()

    def test_network_frontend_is_no_argument_and_fixed_source(self) -> None:
        self.assertIn('if [ "$#" -ne 0 ]; then', self.text)
        self.assertIn("usage: install-online.sh", self.text)
        self.assertIn(f"LOCAL_INSTALL_SOURCE_SHA='{PINNED_LOCAL_SOURCE}'", self.text)
        self.assertIn(f"BOOTSTRAP_KEY_SHA256='{PINNED_KEY_SHA256}'", self.text)
        self.assertIn(
            f"/releases/download/{STABLE_GENERATION}/update-public-key.pem",
            self.text,
        )
        self.assertIn(
            "STABLE_INDEX_URL='https://raw.githubusercontent.com/humtr/codex/main/update-index-v1'",
            self.text,
        )
        self.assertIn(
            'LOCAL_INSTALL_RAW_BASE="https://raw.githubusercontent.com/humtr/codex/$LOCAL_INSTALL_SOURCE_SHA"',
            self.text,
        )

    def test_signed_control_is_verified_before_network_selected_values_are_used(self) -> None:
        index_verify = self.text.index('verify_ed25519 "$index" "$index_sig" "$key"')
        generation_parse = self.text.index("generation=$(sed -n")
        self.assertLess(index_verify, generation_parse)
        manifest_verify = self.text.index('verify_ed25519 "$manifest" "$release_sig" "$key"')
        inventory_loop = self.text.index('while IFS="$tab" read -r kind rel digest mode extra; do')
        self.assertLess(manifest_verify, inventory_loop)
        self.assertIn('cmp "$index" "$expected_index"', self.text)
        self.assertIn("codex-release-v4", self.text)
        self.assertIn("1073741824", self.text)
        self.assertIn("536870912", self.text)

    def test_frontend_delegates_persistent_installation_to_existing_boundary(self) -> None:
        self.assertIn(
            '"$bundle/install.sh" "$release_dir/core" "$release_dir" "$key"',
            self.text,
        )
        for forbidden in [
            "$HOME/.local/share/codex",
            "$HOME/.local/lib/codex",
            "$PREFIX/bin/codex",
            "CODEX_TERMUX_UPDATE_INDEX_URL",
            "CODEX_RELEASE_SIGNING_KEY",
            "--private-key",
            "gh ",
            "git ",
            "cargo ",
            "pkg ",
            "apt ",
        ]:
            self.assertNotIn(forbidden, self.text)
        self.assertIn('exec "$bootstrap" "$@"', self.local)
        self.assertIn("CODEX_TERMUX_INTERNAL_BOOTSTRAP=activate", self.bootstrap)

    def test_transport_is_https_only_and_does_not_install_dependencies(self) -> None:
        self.assertIn('"$curl" -q --fail --silent --show-error --location', self.text)
        self.assertIn("--proto '=https'", self.text)
        self.assertIn("--proto-redir '=https'", self.text)
        self.assertIn("unset http_proxy https_proxy HTTP_PROXY HTTPS_PROXY", self.text)
        self.assertIn("[ -x \"$curl\" ] || fail 'Termux curl is unavailable'", self.text)
        self.assertIn("[ -x \"$openssl\" ] || fail 'Termux OpenSSL is unavailable'", self.text)


if __name__ == "__main__":
    unittest.main()
