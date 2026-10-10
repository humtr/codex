"""Exercise the producer's real artifact qualifier against owned Git/artifacts."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

import manager_tui_artifact as qualifier

TOOL = Path(__file__).with_name("manager_tui_artifact.py")


class NativeArtifactTests(unittest.TestCase):
    def fixture(self, root):
        source = root / "source"
        source.mkdir()
        def git(*args):
            return subprocess.check_output(["git", "-C", str(source), *args],
                                           stderr=subprocess.DEVNULL).decode().strip()
        git("init")
        for relative in qualifier.INPUTS:
            path = source / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("owned native source input\n")
        git("add", ".")
        git("-c", "user.name=Owned fixture", "-c", "user.email=owned@example.invalid",
            "commit", "-m", "owned qualification")
        sha = git("rev-parse", "HEAD")
        artifact = root / "artifact"
        artifact.mkdir()
        raw = b"owned bytes; the existing builder owns ELF/FD checks"
        digest = hashlib.sha256(raw).hexdigest()
        (artifact / "codex").write_bytes(raw)
        (artifact / "codex").chmod(0o755)
        (artifact / "version.txt").write_text("codex-cli 0.161.0\n")
        (artifact / "help.txt").write_text("owned help\n")
        (artifact / "source.txt").write_text(
            f"upstream_sha={qualifier.UPSTREAM}\nsource_sha={sha}\n"
            "target=aarch64-unknown-linux-musl\n"
            "profile=release opt-level=z codegen-units=1 strip=symbols\n")
        (artifact / "sha256.txt").write_text(digest + "  /owned/artifact/codex\n")
        (artifact / "lock-audit.json").write_text(json.dumps(qualifier.LOCK))
        patch = b"owned formatted native patch\n"
        (artifact / "native.patch").write_bytes(patch)
        proof = root / "proof" / "_temp"
        proof.mkdir(parents=True)
        (proof / "profile-tui-focused.log").write_text("Summary [3s] 27 tests run: 27 passed, 5632 skipped\n")
        for name in ("profile-tui-formatted.patch", "profile-tui-linted.patch"):
            (proof / name).write_bytes(patch)
        run = {"id": 123, "path": qualifier.INPUTS[0], "event": "push",
               "head_branch": "rewrite/rust-core", "head_sha": sha,
               "status": "completed", "conclusion": "success",
               "repository": {"full_name": "humtr/codex"}}
        run_file = root / "run.json"
        run_file.write_text(json.dumps(run))
        return source, artifact, proof, sha, digest, run, run_file

    def invoke(self, fixture, version="0.161.0", digest=None):
        source, artifact, proof, sha, expected, _, run_file = fixture
        return subprocess.run(
            [sys.executable, str(TOOL), "--source", str(source), "--source-sha", sha,
             "--run-json", str(run_file), "--run-id", "123", "--version", version,
             "--sha256", digest or expected, "--artifact", str(artifact),
             "--proof", str(proof.parent)], capture_output=True, text=True,
            env={**os.environ, "PYTHONDONTWRITEBYTECODE": "1"}, timeout=15)

    def test_actual_cli_accepts_only_successful_same_input_version_digest_and_native_proof(self):
        cases = [None, "backend", "pin", "source-input", "raw", "checksum",
                 "source", "version", "lock", "zero", "failed-tests", "lint", "patch"]
        cases += ["run-" + key for key in ("id", "path", "event", "head_branch",
                                           "head_sha", "status", "conclusion", "repository")]
        for fault in cases:
            with self.subTest(fault=fault), tempfile.TemporaryDirectory() as temp:
                fixture = self.fixture(Path(temp))
                source, artifact, proof, _, digest, run, run_file = fixture
                version = "0.161.0"
                if fault and fault.startswith("run-"):
                    key = fault[4:]
                    run[key] = {"full_name": "foreign/repository"} if key == "repository" else "wrong"
                    run_file.write_text(json.dumps(run))
                elif fault == "backend": version = "0.162.0"
                elif fault == "pin": digest = "a" * 64
                elif fault == "source-input":
                    path = source / qualifier.INPUTS[1]
                    path.write_text("different native source\n")
                    subprocess.run(["git", "-C", str(source), "add", "."], check=True)
                    subprocess.run(["git", "-C", str(source), "-c", "user.name=Owned",
                                    "-c", "user.email=owned@example.invalid", "commit", "-m", "changed"],
                                   check=True, capture_output=True)
                elif fault in ("raw", "checksum", "source", "version", "lock", "patch"):
                    name = {"raw": "codex", "checksum": "sha256.txt", "source": "source.txt",
                            "version": "version.txt", "lock": "lock-audit.json",
                            "patch": "native.patch"}[fault]
                    (artifact / name).write_bytes(b"changed\n")
                elif fault in ("zero", "failed-tests"):
                    (proof / "profile-tui-focused.log").write_text(
                        "Summary [3s] 0 tests run: 0 passed\n" if fault == "zero"
                        else "Summary [3s] 27 tests run: 26 passed, 1 failed\n")
                elif fault == "lint": (proof / "profile-tui-linted.patch").write_bytes(b"changed\n")
                result = self.invoke(fixture, version, digest)
                self.assertEqual(result.returncode == 0, fault is None, result.stderr)
                if fault is None:
                    self.assertEqual(result.stdout, "manager_tui_sha256\t" + digest + "\n")

    def test_actual_cli_refuses_missing_extra_symlink_and_directory_artifacts_or_proof(self):
        cases = ["missing-" + name for name in sorted(qualifier.FILES)]
        cases += ["extra", "symlink", "directory", "artifact-root", "proof-root", "proof-file"]
        for fault in cases:
            with self.subTest(fault=fault), tempfile.TemporaryDirectory() as temp:
                fixture = self.fixture(Path(temp))
                _, artifact, proof, _, _, _, _ = fixture
                if fault.startswith("missing-"): (artifact / fault[8:]).unlink()
                elif fault == "extra": (artifact / "unqualified").write_text("extra")
                elif fault in ("symlink", "directory"):
                    path = artifact / "codex"
                    path.unlink()
                    if fault == "directory": path.mkdir()
                    else: path.symlink_to(artifact / "version.txt")
                elif fault == "artifact-root":
                    other = artifact.with_name("moved-artifact")
                    artifact.rename(other)
                    artifact.symlink_to(other, target_is_directory=True)
                elif fault == "proof-root":
                    other = proof.with_name("moved-proof")
                    proof.rename(other)
                    proof.symlink_to(other, target_is_directory=True)
                elif fault == "proof-file":
                    path = proof / "profile-tui-linted.patch"
                    path.unlink()
                    path.symlink_to(proof / "profile-tui-formatted.patch")
                result = self.invoke(fixture)
                self.assertNotEqual(result.returncode, 0, result.stdout)


if __name__ == "__main__":
    unittest.main()
