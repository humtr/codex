#!/usr/bin/env python3
"""Offline tests for universal Ed25519 SHA256SUMS.sig signing and verification."""
import hashlib
import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest

spec = importlib.util.spec_from_file_location(
    "release_sign", Path(__file__).with_name("release_sign.py")
)
rs = importlib.util.module_from_spec(spec)
spec.loader.exec_module(rs)

COMMIT = "1" * 40


def gen_ed25519_keypair(root: Path, prefix: str) -> tuple[Path, Path, str]:
    priv = root / f"{prefix}.priv.pem"
    pub = root / f"{prefix}.pub.pem"
    subprocess.run(
        ["openssl", "genpkey", "-algorithm", "ED25519", "-out", str(priv)],
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    subprocess.run(
        ["openssl", "pkey", "-in", str(priv), "-pubout", "-out", str(pub)],
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    return priv, pub, hashlib.sha256(pub.read_bytes()).hexdigest()


class ReleaseSignTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix="codex-release-sign-")
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.dist = self.root / "dist"
        self.dist.mkdir()
        (self.dist / "codex-wrapper-aarch64-linux-android.tar.gz").write_bytes(b"tarball-bytes")
        (self.dist / "install.sh").write_bytes(b"#!/bin/sh\nexit 0\n")
        self.priv, self.pub, self.pub_sha = gen_ed25519_keypair(self.root, "k1")

    def sign(self, *, component="wrapper", sequence=1, priv=None):
        return rs.sign_release(
            component=component,
            sequence=sequence,
            commit=COMMIT,
            dist=self.dist,
            files=["codex-wrapper-aarch64-linux-android.tar.gz", "install.sh"],
            private_key=priv or self.priv,
        )

    def test_sign_and_verify_roundtrip(self):
        res = self.sign(sequence=5)
        self.assertEqual(res["component"], "wrapper")
        self.assertEqual(res["sequence"], 5)
        self.assertEqual(res["commit"], COMMIT)
        self.assertFalse(res["rotated"])
        self.assertEqual(res["active_key_sha256"], self.pub_sha)

        verified = rs.verify_release(
            component="wrapper",
            dist=self.dist,
            public_key=self.pub,
            min_sequence=5,
            expected_key_sha256=self.pub_sha,
        )
        self.assertEqual(verified["sequence"], 5)
        self.assertEqual(len(verified["files"]), 2)

    def test_rejects_tampered_sums_signature_and_asset(self):
        self.sign(sequence=2)
        orig_sums = (self.dist / "SHA256SUMS").read_bytes()
        orig_sig = (self.dist / "SHA256SUMS.sig").read_bytes()

        # Tampered SHA256SUMS body
        (self.dist / "SHA256SUMS").write_bytes(orig_sums.replace(b"sequence=2", b"sequence=9"))
        with self.assertRaises(rs.SigningError):
            rs.verify_release(component="wrapper", dist=self.dist, public_key=self.pub)
        (self.dist / "SHA256SUMS").write_bytes(orig_sums)

        # Tampered signature byte
        bad_sig = bytes([orig_sig[0] ^ 0xFF]) + orig_sig[1:]
        (self.dist / "SHA256SUMS.sig").write_bytes(bad_sig)
        with self.assertRaises(rs.SigningError):
            rs.verify_release(component="wrapper", dist=self.dist, public_key=self.pub)
        (self.dist / "SHA256SUMS.sig").write_bytes(orig_sig)

        # Tampered asset bytes
        (self.dist / "install.sh").write_bytes(b"#!/bin/sh\necho pwned\n")
        with self.assertRaises(rs.SigningError):
            rs.verify_release(component="wrapper", dist=self.dist, public_key=self.pub)

    def test_rejects_sequence_rollback_component_mismatch_and_wrong_key_pin(self):
        self.sign(sequence=3)
        with self.assertRaises(rs.SigningError):
            rs.verify_release(
                component="wrapper", dist=self.dist, public_key=self.pub, min_sequence=4
            )
        with self.assertRaises(rs.SigningError):
            rs.verify_release(component="codex-ext", dist=self.dist, public_key=self.pub)
        with self.assertRaises(rs.SigningError):
            rs.verify_release(
                component="wrapper",
                dist=self.dist,
                public_key=self.pub,
                expected_key_sha256="0" * 64,
            )

    def test_authorized_key_rotation_succeeds_and_forged_rotation_fails(self):
        priv2, pub2, pub2_sha = gen_ed25519_keypair(self.root, "k2")
        priv3, _, _ = gen_ed25519_keypair(self.root, "k3")

        # Sign release under new key k2 + authorize k2 with old key k1
        self.sign(sequence=6, priv=priv2)
        (self.dist / "release-public-key.pem").write_bytes(pub2.read_bytes())
        rs.sign_key_rotation(
            old_private_key=self.priv,
            new_public_key=self.dist / "release-public-key.pem",
            output_sig=self.dist / "release-public-key.pem.sig",
        )
        res = rs.verify_release(
            component="wrapper",
            dist=self.dist,
            public_key=self.pub,
            min_sequence=6,
            expected_key_sha256=self.pub_sha,
        )
        self.assertTrue(res["rotated"])
        self.assertEqual(res["active_key_sha256"], pub2_sha)

        # Forged rotation signed by unauthorized k3 must fail
        rs.sign_key_rotation(
            old_private_key=priv3,
            new_public_key=self.dist / "release-public-key.pem",
            output_sig=self.dist / "release-public-key.pem.sig",
        )
        with self.assertRaises(rs.SigningError):
            rs.verify_release(component="wrapper", dist=self.dist, public_key=self.pub)

    def test_rejects_symlinked_assets(self):
        real = self.root / "real.sh"
        real.write_bytes(b"#!/bin/sh\nexit 0\n")
        (self.dist / "install.sh").unlink()
        (self.dist / "install.sh").symlink_to(real)
        with self.assertRaises(rs.SigningError):
            self.sign(sequence=1)


if __name__ == "__main__":
    unittest.main()
