#!/usr/bin/env python3
from __future__ import annotations

import os
from pathlib import Path
import shutil
import stat
import subprocess
import sys
import tempfile
import textwrap
import unittest

HERE = Path(__file__).resolve().parent
SIGNER = HERE / "rald4_signing.py"


class SigningBoundaryTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp_ctx = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp_ctx.name).resolve()
        openssl = shutil.which("openssl")
        if openssl is None:
            self.skipTest("OpenSSL is required")
        self.openssl = Path(openssl).resolve()
        self.runner_temp = self.root / "runner-temp"
        self.runner_temp.mkdir(mode=0o700)
        self.candidate = self.root / "candidate"
        self.candidate.mkdir()
        self.generation = "fixture-generation-1"
        (self.candidate / "generation.meta").write_text(
            "codex-local-generation-v2\n"
            f"generation_id\t{self.generation}\n"
        )
        self.good_key = self.root / "fixture-good-key.pem"
        self.good_public = self.root / "fixture-good-public.pem"
        self.wrong_key = self.root / "fixture-wrong-key.pem"
        self.wrong_public = self.root / "fixture-wrong-public.pem"
        self.rsa_key = self.root / "fixture-rsa-key.pem"
        self._run_openssl(["genpkey", "-algorithm", "ED25519", "-out", str(self.good_key)])
        self._run_openssl(
            ["pkey", "-in", str(self.good_key), "-pubout", "-out", str(self.good_public)]
        )
        self._run_openssl(["genpkey", "-algorithm", "ED25519", "-out", str(self.wrong_key)])
        self._run_openssl(
            ["pkey", "-in", str(self.wrong_key), "-pubout", "-out", str(self.wrong_public)]
        )
        self._run_openssl(
            ["genpkey", "-algorithm", "RSA", "-pkeyopt", "rsa_keygen_bits:2048", "-out", str(self.rsa_key)]
        )
        self.builder = self.root / "fixture-builder.py"
        self.called = self.root / "fixture-builder.called"
        self.builder.write_text(self._builder_source())
        self.builder.chmod(0o755)

    def tearDown(self) -> None:
        self.tmp_ctx.cleanup()

    def _run_openssl(self, args: list[str]) -> None:
        result = subprocess.run(
            [str(self.openssl), *args],
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr.decode(errors="replace"))

    def _builder_source(self) -> str:
        return textwrap.dedent(
            f'''\
            #!{sys.executable}
            from pathlib import Path
            import hashlib
            import os
            import shutil
            import subprocess
            import sys

            args = sys.argv[1:]
            if not args or args[0] != "publish":
                raise SystemExit(64)
            values = {{}}
            it = iter(args[1:])
            for flag in it:
                values[flag] = next(it)
            Path({str(self.called)!r}).write_text("called\\n")
            candidate = Path(values["--generation"])
            key = Path(values["--private-key"])
            openssl = values["--openssl"]
            output = Path(values["--output"])
            sequence = values["--release-sequence"]
            release_base = values["--release-base"]
            generation = next(
                line.split("\\t", 1)[1]
                for line in (candidate / "generation.meta").read_text().splitlines()
                if line.startswith("generation_id\\t")
            )
            der = subprocess.check_output(
                [openssl, "pkey", "-in", str(key), "-pubout", "-outform", "DER"],
                stderr=subprocess.DEVNULL,
                env={{}},
            )
            prefix = bytes.fromhex("302a300506032b6570032100")
            if len(der) != 44 or not der.startswith(prefix):
                raise SystemExit(1)
            raw = der[len(prefix):].hex()
            release = output / "releases" / generation
            release.mkdir(parents=True)
            shutil.copyfile(candidate / "generation.meta", release / "generation.meta")
            digest = hashlib.sha256((release / "generation.meta").read_bytes()).hexdigest()
            manifest = (
                "codex-release-v4\\n"
                f"generation_id\\t{{generation}}\\n"
                f"release_sequence\\t{{sequence}}\\n"
                "channel\\tstable\\n"
                "expected_platform\\tandroid\\n"
                "expected_architecture\\taarch64\\n"
                "core_api_identity\\tcore-api-v1\\n"
                "persistent_schema_identity\\tschema-v1\\n"
                f"release_public_key\\t{{raw}}\\n"
                "file_count\\t1\\n"
                f"file\\tgeneration.meta\\t{{digest}}\\t0644\\n"
            )
            (release / "release.manifest").write_text(manifest)
            subprocess.check_call(
                [openssl, "pkeyutl", "-sign", "-rawin", "-inkey", str(key),
                 "-in", str(release / "release.manifest"), "-out", str(release / "release.sig")],
                stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, env={{}},
            )
            index = (
                "codex-update-index-v1\\n"
                "channel\\tstable\\n"
                f"generation_id\\t{{generation}}\\n"
                f"release_base\\t{{release_base}}\\n"
            )
            (output / "update-index-v1").write_text(index)
            subprocess.check_call(
                [openssl, "pkeyutl", "-sign", "-rawin", "-inkey", str(key),
                 "-in", str(output / "update-index-v1"), "-out", str(output / "update-index-v1.sig")],
                stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, env={{}},
            )
            '''
        )

    def _invoke(self, secret: str | None, accepted_public: Path | None = None) -> subprocess.CompletedProcess[bytes]:
        if self.called.exists():
            self.called.unlink()
        output = self.root / "signed-output"
        if output.exists():
            shutil.rmtree(output)
        env = os.environ.copy()
        if secret is None:
            env.pop("CODEX_RELEASE_SIGNING_KEY", None)
        else:
            env["CODEX_RELEASE_SIGNING_KEY"] = secret
        accepted_public = accepted_public or self.good_public
        return subprocess.run(
            [
                sys.executable,
                str(SIGNER),
                "--candidate",
                str(self.candidate),
                "--accepted-public-key",
                str(accepted_public),
                "--release-sequence",
                "11",
                "--release-base",
                f"https://example.invalid/releases/{self.generation}/",
                "--builder",
                str(self.builder),
                "--openssl",
                str(self.openssl),
                "--runner-temp",
                str(self.runner_temp),
                "--output",
                str(output),
            ],
            env=env,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )

    def _assert_failed_before_builder(self, result: subprocess.CompletedProcess[bytes]) -> None:
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(self.called.exists())
        self.assertFalse((self.root / "signed-output").exists())
        self.assertEqual(list(self.runner_temp.glob(".rald4-signing-*")), [])

    def test_absent_secret_fails_closed(self) -> None:
        self._assert_failed_before_builder(self._invoke(None))

    def test_malformed_pem_fails_closed(self) -> None:
        result = self._invoke("not a PEM private key\n")
        self._assert_failed_before_builder(result)
        self.assertNotIn(b"not a PEM private key", result.stdout + result.stderr)

    def test_non_ed25519_private_key_fails_closed(self) -> None:
        secret = self.rsa_key.read_text()
        result = self._invoke(secret)
        self._assert_failed_before_builder(result)
        self.assertNotIn(secret.encode(), result.stdout + result.stderr)

    def test_wrong_derived_public_key_fails_closed(self) -> None:
        secret = self.wrong_key.read_text()
        result = self._invoke(secret)
        self._assert_failed_before_builder(result)
        self.assertNotIn(secret.encode(), result.stdout + result.stderr)

    def test_correct_key_signs_and_verifies_without_leakage(self) -> None:
        secret = self.good_key.read_text()
        result = self._invoke(secret)
        self.assertEqual(result.returncode, 0, result.stderr.decode(errors="replace"))
        self.assertTrue(self.called.exists())
        self.assertEqual(list(self.runner_temp.glob(".rald4-signing-*")), [])
        self.assertNotIn(secret.encode(), result.stdout + result.stderr)
        output = self.root / "signed-output"
        for path in output.rglob("*"):
            if path.is_file():
                self.assertNotIn(secret.encode(), path.read_bytes())
        release = output / "releases" / self.generation
        for payload, signature in [
            (release / "release.manifest", release / "release.sig"),
            (output / "update-index-v1", output / "update-index-v1.sig"),
        ]:
            verify = subprocess.run(
                [
                    str(self.openssl),
                    "pkeyutl",
                    "-verify",
                    "-pubin",
                    "-inkey",
                    str(self.good_public),
                    "-rawin",
                    "-in",
                    str(payload),
                    "-sigfile",
                    str(signature),
                ],
                stdin=subprocess.DEVNULL,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                env={},
                check=False,
            )
            self.assertEqual(verify.returncode, 0, verify.stderr.decode(errors="replace"))

    def test_deferred_candidate_fails_before_secret_materialization(self) -> None:
        marker = self.candidate / ".manager-probe-deferred"
        marker.write_text("codex-manager-probe-deferred-v1\n")
        result = self._invoke(self.good_key.read_text())
        self._assert_failed_before_builder(result)


if __name__ == "__main__":
    unittest.main()
