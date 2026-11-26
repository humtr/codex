#!/usr/bin/env python3
"""Unit tests for host/chatgpt-desktop/manager.py."""

from __future__ import annotations

import importlib.util
import json
import os
import tempfile
import unittest
from pathlib import Path

# Load the sibling module explicitly, including when unittest is invoked from
# the repository root (which does not add this directory to sys.path).
_spec = importlib.util.spec_from_file_location("manager", Path(__file__).resolve().with_name("manager.py"))
assert _spec is not None and _spec.loader is not None
manager = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(manager)


SAMPLE_INRELEASE = """-----BEGIN PGP SIGNED MESSAGE-----
Hash: SHA512

Codename: stable
Date: Fri, 20 Nov 2026 20:20:56 +0000
Suite: stable
SHA256:
 b4d6c7d4de8625b17581cf1a8494f57bb1633dd5f062e841e76e14db33e99cec             1455 main/binary-arm64/Packages
 cb21ea1a98d814215c5a018e38e4c54a1263718f9c13f43c003c1f7f7fa59598              782 main/binary-arm64/Packages.gz
-----BEGIN PGP SIGNATURE-----
fake
-----END PGP SIGNATURE-----
"""

SAMPLE_PACKAGES = """Package: chatgpt
Version: 26.1007.21434
Architecture: arm64
Filename: pool/main/c/chatgpt/chatgpt_26.1007.21434_arm64.deb
Size: 475083462
SHA256: 55fae1e3fb1d6069a90620e9e70737dcf8bf2255604d157e282cc6748a1baf49

Package: chatgpt
Version: 26.1117.41037
Architecture: arm64
Filename: pool/main/c/chatgpt/chatgpt_26.1117.41037_arm64.deb
Size: 519472018
SHA256: 714146e9c667b26a1bd4e57633c544334a9ed47c399989bfd98cf764140bdc53
"""


def make_fake_generation(root: Path, version: str, complete: bool = True) -> Path:
    gen = root / "generations" / version
    (gen / "rootfs/usr/bin").mkdir(parents=True, exist_ok=True)
    (gen / "rootfs/usr/lib/chatgpt/resources").mkdir(parents=True, exist_ok=True)
    (gen / "generation.json").write_text(
        json.dumps({"version": version, "deb_sha256": "0" * 64, "installed_at": "2026-11-22T00:00:00Z"}),
        encoding="utf-8",
    )
    if complete:
        (gen / "rootfs/usr/bin/chatgpt").write_bytes(b"#!/bin/sh\n")
        (gen / "rootfs/usr/lib/chatgpt/resources/app.asar").write_bytes(b"asar")
    return gen


class ChatGPTDesktopManagerTests(unittest.TestCase):
    def test_pinned_keyring_matches_expected_sha256(self) -> None:
        digest = manager.verify_keyring(manager.DEFAULT_KEYRING_PATH)
        self.assertEqual(digest, manager.PINNED_KEYRING_SHA256)

    def test_parse_inrelease_and_packages_selects_highest_arm64(self) -> None:
        ir = manager.parse_inrelease(SAMPLE_INRELEASE)
        self.assertEqual(ir["date"], "Fri, 20 Nov 2026 20:20:56 +0000")
        self.assertEqual(
            ir["sha256"]["main/binary-arm64/Packages"],
            ("b4d6c7d4de8625b17581cf1a8494f57bb1633dd5f062e841e76e14db33e99cec", 1455),
        )
        pkg = manager.parse_arm64_chatgpt_package(SAMPLE_PACKAGES)
        self.assertEqual(pkg["version"], "26.1117.41037")
        self.assertEqual(pkg["size"], 519472018)

    def test_version_validation_rejects_traversal(self) -> None:
        for bad in ("", "../evil", "1.0/2.0", ".hidden"):
            with self.assertRaises(ValueError):
                manager.validate_version(bad)

    def test_activate_preserves_previous_on_re_activate_and_rolls_back(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            root = Path(td) / "install"
            profile = Path(td) / "profile"
            profile.mkdir(mode=0o700)
            make_fake_generation(root, "26.1007.21434")
            make_fake_generation(root, "26.1117.41037")
            make_fake_generation(root, "26.9999.0", complete=False)

            # Reject incomplete generation
            with self.assertRaises(RuntimeError):
                manager.activate_generation(root, "26.9999.0")

            # Activate v1 -> v2 -> re-activate v2 preserves v1 as previous
            manager.activate_generation(root, "26.1007.21434")
            st1 = manager.status_report(root, profile)
            self.assertEqual(st1["current"], "26.1007.21434")
            self.assertIsNone(st1["previous"])
            self.assertTrue(st1["profile_exists"])

            manager.activate_generation(root, "26.1117.41037")
            manager.activate_generation(root, "26.1117.41037")
            st2 = manager.status_report(root, profile)
            self.assertEqual(st2["current"], "26.1117.41037")
            self.assertEqual(st2["previous"], "26.1007.21434")

            # Rollback swaps current <-> previous without touching profile
            manager.rollback_generation(root)
            st3 = manager.status_report(root, profile)
            self.assertEqual(st3["current"], "26.1007.21434")
            self.assertEqual(st3["previous"], "26.1117.41037")
            self.assertTrue(profile.is_dir())


if __name__ == "__main__":
    unittest.main()
