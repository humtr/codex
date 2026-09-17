#!/usr/bin/env python3
import hashlib
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).with_name("rald5_publication.py")
GENERATION = "local-rald5-fixture-1"
SEQUENCE = "11"
BASE = f"https://humtr.github.io/codex/{GENERATION}/"
FILES = [
    ("codex-code-mode-host", 0o755),
    ("core", 0o755),
    ("generation.meta", 0o644),
    ("helpers/0", 0o755),
    ("helpers/1", 0o755),
    ("manager", 0o755),
    ("runtime", 0o755),
]


def run(*args: str) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(
        [sys.executable, str(SCRIPT), *args],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )


def openssl(*args: str) -> None:
    subprocess.run(["openssl", *args], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


class Rald5PublicationTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory(prefix="rald5-publication-test-")
        self.root = Path(self.temp.name)
        self.private_key = self.root / "private.pem"
        self.public_key = self.root / "public.pem"
        openssl("genpkey", "-algorithm", "ED25519", "-out", str(self.private_key))
        openssl("pkey", "-in", str(self.private_key), "-pubout", "-out", str(self.public_key))
        self.signed = self.root / "signed"
        release = self.signed / "releases" / GENERATION
        (release / "helpers").mkdir(parents=True)
        for rel, mode in FILES:
            path = release / rel
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes((f"fixture:{rel}\n").encode())
            path.chmod(mode)
        der = self.root / "public.der"
        openssl("pkey", "-pubin", "-in", str(self.public_key), "-outform", "DER", "-out", str(der))
        raw_key = der.read_bytes()[-32:].hex()
        manifest = [
            "codex-release-v4",
            f"generation_id\t{GENERATION}",
            f"release_sequence\t{SEQUENCE}",
            "channel\tstable",
            "expected_platform\tandroid",
            "expected_architecture\taarch64",
            "core_api_identity\tcore-api-v1",
            "persistent_schema_identity\tschema-v1",
            f"release_public_key\t{raw_key}",
            "file_count\t7",
        ]
        for rel, mode in FILES:
            manifest.append(f"file\t{rel}\t{sha256(release / rel)}\t{mode:04o}")
        (release / "release.manifest").write_text("\n".join(manifest) + "\n")
        openssl(
            "pkeyutl",
            "-sign",
            "-rawin",
            "-inkey",
            str(self.private_key),
            "-in",
            str(release / "release.manifest"),
            "-out",
            str(release / "release.sig"),
        )
        index = self.signed / "update-index-v1"
        index.parent.mkdir(parents=True, exist_ok=True)
        index.write_text(
            "codex-update-index-v1\n"
            "channel\tstable\n"
            f"generation_id\t{GENERATION}\n"
            f"release_base\t{BASE}\n"
        )
        openssl(
            "pkeyutl",
            "-sign",
            "-rawin",
            "-inkey",
            str(self.private_key),
            "-in",
            str(index),
            "-out",
            str(self.signed / "update-index-v1.sig"),
        )

    def tearDown(self) -> None:
        self.temp.cleanup()

    def prepare(self, output: Path) -> subprocess.CompletedProcess[bytes]:
        return run(
            "prepare-assets",
            "--signed-root",
            str(self.signed),
            "--public-key",
            str(self.public_key),
            "--output",
            str(output),
            "--generation",
            GENERATION,
            "--release-sequence",
            SEQUENCE,
            "--release-base",
            BASE,
        )

    def test_prepare_assets_flattens_only_exact_signed_publication_set(self) -> None:
        output = self.root / "assets"
        result = self.prepare(output)
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        self.assertEqual(
            {path.name for path in output.iterdir()},
            {
                "candidate-update-index-v1",
                "candidate-update-index-v1.sig",
                "codex-code-mode-host",
                "core",
                "generation.meta",
                "helper-0",
                "helper-1",
                "manager",
                "release.manifest",
                "release.sig",
                "runtime",
                "update-public-key.pem",
            },
        )
        self.assertEqual((output / "helper-0").read_bytes(), (self.signed / "releases" / GENERATION / "helpers/0").read_bytes())
        self.assertIn(b"asset_count=12\n", result.stdout)

    def test_verify_public_accepts_exact_pages_tree(self) -> None:
        site = self.root / "site"
        release = site / GENERATION
        shutil.copytree(self.signed / "releases" / GENERATION, release, copy_function=shutil.copy2)
        shutil.copy2(self.signed / "update-index-v1", release / "update-index-v1")
        shutil.copy2(self.signed / "update-index-v1.sig", release / "update-index-v1.sig")
        result = run(
            "verify-public",
            "--root",
            str(site),
            "--public-key",
            str(self.public_key),
            "--generation",
            GENERATION,
            "--release-sequence",
            SEQUENCE,
            "--release-base",
            BASE,
        )
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        self.assertIn(f"verified_generation={GENERATION}".encode(), result.stdout)

    def test_tampered_signed_file_fails_before_asset_output(self) -> None:
        (self.signed / "releases" / GENERATION / "runtime").write_bytes(b"tampered\n")
        output = self.root / "assets"
        result = self.prepare(output)
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(output.exists())

    def test_extra_release_entry_fails_closed(self) -> None:
        (self.signed / "releases" / GENERATION / "extra").write_bytes(b"no\n")
        result = self.prepare(self.root / "assets")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"unexpected entries", result.stderr)

    def test_sequence_mismatch_fails_closed(self) -> None:
        result = run(
            "prepare-assets",
            "--signed-root",
            str(self.signed),
            "--public-key",
            str(self.public_key),
            "--output",
            str(self.root / "assets"),
            "--generation",
            GENERATION,
            "--release-sequence",
            "12",
            "--release-base",
            BASE,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"manifest does not match", result.stderr)

    def test_noncanonical_release_base_fails_closed(self) -> None:
        result = run(
            "prepare-assets",
            "--signed-root",
            str(self.signed),
            "--public-key",
            str(self.public_key),
            "--output",
            str(self.root / "assets"),
            "--generation",
            GENERATION,
            "--release-sequence",
            SEQUENCE,
            "--release-base",
            "https://example.invalid/",
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"release base is not canonical", result.stderr)

    def test_wrong_public_key_fails_signature_verification(self) -> None:
        wrong_private = self.root / "wrong-private.pem"
        wrong_public = self.root / "wrong-public.pem"
        openssl("genpkey", "-algorithm", "ED25519", "-out", str(wrong_private))
        openssl("pkey", "-in", str(wrong_private), "-pubout", "-out", str(wrong_public))
        result = run(
            "prepare-assets",
            "--signed-root",
            str(self.signed),
            "--public-key",
            str(wrong_public),
            "--output",
            str(self.root / "assets"),
            "--generation",
            GENERATION,
            "--release-sequence",
            SEQUENCE,
            "--release-base",
            BASE,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"OpenSSL verification failed", result.stderr)


if __name__ == "__main__":
    unittest.main()
