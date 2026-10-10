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
        self.build_signed()

    def build_signed(self, *, frontend: bool = False) -> None:
        files = sorted(FILES + ([("helpers/2", 0o755)] if frontend else []))
        self.signed = self.root / "signed"
        if self.signed.exists():
            shutil.rmtree(self.signed)
        release = self.signed / "releases" / GENERATION
        (release / "helpers").mkdir(parents=True)
        for rel, mode in files:
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
            f"file_count\t{len(files)}",
        ]
        for rel, mode in files:
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
        sizes = [(rel, (release / rel).stat().st_size) for rel, _ in files]
        total = sum(size for _, size in sizes)
        sidecar = self.signed / "download-size-v1"
        sidecar.write_text(
            "\n".join(
                [
                    "codex-download-size-v1",
                    f"manifest_sha256\t{sha256(release / 'release.manifest')}",
                    f"file_count\t{len(files)}",
                    f"total_bytes\t{total}",
                    *[f"file\t{rel}\t{size}" for rel, size in sizes],
                    "",
                ]
            )
        )
        openssl(
            "pkeyutl",
            "-sign",
            "-rawin",
            "-inkey",
            str(self.private_key),
            "-in",
            str(sidecar),
            "-out",
            str(self.signed / "download-size-v1.sig"),
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

    def test_actual_reconstruction_preserves_both_inventories_and_rejects_faults(self) -> None:
        for frontend in (False, True):
            for fault in (None, "missing", "payload", "extra", "symlink", "manifest",
                          "signature", "index", "sidecar", "key", "occupied"):
                with self.subTest(frontend=frontend, fault=fault):
                    self.build_signed(frontend=frontend)
                    assets = self.root / f"restore-assets-{frontend}-{fault}"
                    prepared = self.prepare(assets)
                    self.assertEqual(prepared.returncode, 0, prepared.stderr.decode())
                    helper = assets / ("helper-2" if frontend else "helper-1")
                    if fault == "missing":
                        helper.unlink()
                    elif fault == "payload":
                        helper.write_bytes(b"changed")
                    elif fault == "extra":
                        (assets / "helper-3").write_bytes(b"extra")
                    elif fault == "symlink":
                        helper.unlink(); helper.symlink_to(assets / "core")
                    elif fault == "manifest":
                        path = assets / "release.manifest"
                        path.write_text(path.read_text().replace("helpers/1", "helpers/9"))
                    elif fault == "signature":
                        (assets / "release.sig").write_bytes(b"x" * 64)
                    elif fault == "index":
                        (assets / "candidate-update-index-v1").write_bytes(b"changed")
                    elif fault == "sidecar":
                        (assets / "download-size-v1").write_bytes(b"changed")
                    elif fault == "key":
                        (assets / "update-public-key.pem").write_bytes(b"foreign")
                    site = self.root / f"restore-site-{frontend}-{fault}"
                    destination = site / GENERATION
                    if fault == "occupied":
                        destination.mkdir(parents=True)
                        (destination / "keep").write_bytes(b"owned")
                    result = run("restore-assets", "--assets", str(assets), "--root", str(site),
                                 "--public-key", str(self.public_key), "--generation", GENERATION,
                                 "--release-sequence", SEQUENCE, "--release-base", BASE)
                    if fault is None:
                        self.assertEqual(result.returncode, 0, result.stderr.decode())
                        source = self.signed / "releases" / GENERATION
                        for path in source.rglob("*"):
                            if path.is_file():
                                installed = destination / path.relative_to(source)
                                self.assertEqual(installed.read_bytes(), path.read_bytes())
                                expected_mode = 0o644 if path.name in ("release.manifest", "release.sig") else path.stat().st_mode & 0o7777
                                self.assertEqual(installed.stat().st_mode & 0o7777, expected_mode)
                        self.assertEqual((destination / "helpers/2").exists(), frontend)
                        self.assertEqual({p.name for p in site.iterdir()}, {GENERATION})
                    else:
                        self.assertNotEqual(result.returncode, 0)
                        if fault == "occupied":
                            self.assertEqual((destination / "keep").read_bytes(), b"owned")
                        else:
                            self.assertFalse(destination.exists())

    def test_extended_signed_assets_and_pages_reject_helper_faults(self) -> None:
        for fault in (None, "missing", "tampered", "mode", "extra", "undeclared"):
            with self.subTest(fault=fault):
                self.build_signed(frontend=fault != "undeclared")
                release = self.signed / "releases" / GENERATION
                helper = release / "helpers/2"
                if fault == "missing":
                    helper.unlink()
                elif fault == "tampered":
                    helper.write_bytes(b"changed")
                elif fault == "mode":
                    helper.chmod(0o644)
                elif fault == "extra":
                    (release / "helpers/3").write_bytes(b"extra")
                elif fault == "undeclared":
                    helper.write_bytes(b"undeclared")
                output = self.root / f"assets-{fault}"
                result = self.prepare(output)
                if fault is not None:
                    self.assertNotEqual(result.returncode, 0)
                    self.assertFalse(output.exists())
                else:
                    self.assertEqual(result.returncode, 0, result.stderr.decode())
                    self.assertEqual((output / "helper-2").read_bytes(), helper.read_bytes())
                    self.assertEqual(len(list(output.iterdir())), 15)
                site = self.root / f"site-{fault}"
                destination = site / GENERATION
                shutil.copytree(release, destination, copy_function=shutil.copy2)
                (destination / "compat").mkdir()
                for name in ("download-size-v1", "download-size-v1.sig"):
                    shutil.copy2(self.signed / name, destination / "compat" / name)
                for name in ("update-index-v1", "update-index-v1.sig"):
                    shutil.copy2(self.signed / name, destination / name)
                result = run("verify-public", "--root", str(site), "--public-key", str(self.public_key),
                             "--generation", GENERATION, "--release-sequence", SEQUENCE,
                             "--release-base", BASE, "--require-download-size")
                self.assertEqual(result.returncode == 0, fault is None, result.stderr.decode())

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
                "download-size-v1",
                "download-size-v1.sig",
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
        self.assertIn(b"asset_count=14\n", result.stdout)

    def test_verify_public_accepts_exact_pages_tree(self) -> None:
        site = self.root / "site"
        release = site / GENERATION
        shutil.copytree(self.signed / "releases" / GENERATION, release, copy_function=shutil.copy2)
        compat = release / "compat"
        compat.mkdir()
        shutil.copy2(self.signed / "download-size-v1", compat / "download-size-v1")
        shutil.copy2(self.signed / "download-size-v1.sig", compat / "download-size-v1.sig")
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

    def test_verify_public_allows_legacy_tree_but_requirement_is_fail_closed(self) -> None:
        site = self.root / "legacy-site"
        release = site / GENERATION
        shutil.copytree(self.signed / "releases" / GENERATION, release, copy_function=shutil.copy2)
        shutil.copy2(self.signed / "update-index-v1", release / "update-index-v1")
        shutil.copy2(self.signed / "update-index-v1.sig", release / "update-index-v1.sig")
        common = [
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
        ]
        legacy = run(*common)
        self.assertEqual(legacy.returncode, 0, legacy.stderr.decode())
        required = run(*common, "--require-download-size")
        self.assertNotEqual(required.returncode, 0)
        self.assertIn(b"download-size sidecar is required", required.stderr)

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
