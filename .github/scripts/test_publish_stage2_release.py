#!/usr/bin/env python3
"""Unit tests for fail-closed Stage 2 release publication guards."""
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest


def _load(name: str):
    path = Path(__file__).resolve().with_name(f"{name}.py")
    spec = importlib.util.spec_from_file_location(name, path)
    mod = importlib.util.module_from_spec(spec)
    assert spec and spec.loader
    spec.loader.exec_module(mod)
    return mod


pub_mod = _load("publish_stage2_release")
rs_mod = _load("release_sign")


class FakeProc:
    def __init__(self, rc: int = 0, stdout: str = "", stderr: str = ""):
        self.returncode = rc
        self.stdout = stdout
        self.stderr = stderr


class Stage2ReleasePublishTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix="stage2-pub-test-")
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.dist = self.root / "dist"
        self.dist.mkdir()
        self.priv = self.root / "priv.pem"
        self.pub = self.root / "pub.pem"
        subprocess.run(["openssl", "genpkey", "-algorithm", "ED25519", "-out", str(self.priv)], check=True, capture_output=True)
        subprocess.run(["openssl", "pkey", "-in", str(self.priv), "-pubout", "-out", str(self.pub)], check=True, capture_output=True)
        (self.dist / "comp.tar.gz").write_bytes(b"tarball-bytes")
        (self.dist / "comp.tar.gz.sha256").write_text(f"{rs_mod.sha256_file(self.dist / 'comp.tar.gz')}  comp.tar.gz\n")
        self.sha_old = "a" * 40
        self.sha_new = "b" * 40

    def test_cross_component_tag_clobber_and_sequence_regression_fail_closed(self):
        # Create an existing signed SHA256SUMS belonging to another component
        prev_dir = self.root / "prev-other"
        prev_dir.mkdir()
        (prev_dir / "other.tar.gz").write_bytes(b"other")
        rs_mod.sign_release(
            component="termux-codex-mcp",
            sequence=5,
            commit=self.sha_old,
            dist=prev_dir,
            files=["other.tar.gz"],
            private_key=self.priv,
        )

        def fake_gh(argv):
            if argv[:3] == ["gh", "release", "download"]:
                d = Path(argv[argv.index("--dir") + 1])
                pat = argv[argv.index("--pattern") + 1]
                (d / pat).write_bytes((prev_dir / pat).read_bytes())
                return FakeProc(0)
            return FakeProc(0)

        p = pub_mod.Stage2Publisher(
            repo="owner/repo",
            tag="termux-codex-mcp",
            component="termux-chatgpt-mcp",
            source_sha=self.sha_new,
            dist=self.dist,
            tarball="comp.tar.gz",
            title="Test",
            notes="Notes",
            sequence=6,
            signing_key=self.priv,
            public_key=self.pub,
            runner=fake_gh,
        )
        with self.assertRaisesRegex(pub_mod.Stage2PublicationError, "cross-component tag clobber"):
            p.enforce_existing_release_guards({"assets": [{"name": "SHA256SUMS"}, {"name": "SHA256SUMS.sig"}]})

        # Same component with non-increasing sequence must also fail closed
        p2 = pub_mod.Stage2Publisher(
            repo="owner/repo",
            tag="termux-codex-mcp",
            component="termux-codex-mcp",
            source_sha=self.sha_new,
            dist=self.dist,
            tarball="comp.tar.gz",
            title="Test",
            notes="Notes",
            sequence=5,
            signing_key=self.priv,
            public_key=self.pub,
            runner=fake_gh,
        )
        with self.assertRaisesRegex(pub_mod.Stage2PublicationError, "missing SHA256SUMS"):
            p2.enforce_existing_release_guards({"assets": [{"name": "SHA256SUMS"}]})
        with self.assertRaisesRegex(pub_mod.Stage2PublicationError, "must exceed published sequence 5"):
            p2.enforce_existing_release_guards({"assets": [{"name": "SHA256SUMS"}, {"name": "SHA256SUMS.sig"}]})

        (prev_dir / "SHA256SUMS").write_bytes((prev_dir / "SHA256SUMS").read_bytes().replace(b"sequence=5", b"sequence=1"))
        with self.assertRaisesRegex(pub_mod.Stage2PublicationError, "invalid"):
            p2.enforce_existing_release_guards({"assets": [{"name": "SHA256SUMS"}, {"name": "SHA256SUMS.sig"}]})

    def test_existing_release_moves_lightweight_tag_and_verifies_readback(self):
        prev_dir = self.root / "prev-same"
        prev_dir.mkdir()
        (prev_dir / "comp.tar.gz").write_bytes(b"old")
        rs_mod.sign_release(
            component="termux-chatgpt-mcp",
            sequence=2,
            commit=self.sha_old,
            dist=prev_dir,
            files=["comp.tar.gz"],
            private_key=self.priv,
        )
        tag_sha = {"sha": self.sha_old}
        uploaded = {}
        fault = None
        alternate = {}

        def fake_gh(argv):
            if argv[:2] == ["gh", "api"] and "releases/tags/" in argv[2]:
                return FakeProc(0, json.dumps({"tag_name": "termux-chatgpt-mcp", "assets": [{"name": name} for name in (uploaded or ("SHA256SUMS", "SHA256SUMS.sig"))]}))
            if argv[:2] == ["gh", "api"] and "git/ref/tags/" in argv[2]:
                return FakeProc(0, json.dumps({"object": {"type": "commit", "sha": tag_sha["sha"]}}))
            if argv[:4] == ["gh", "api", "-X", "PATCH"]:
                tag_sha["sha"] = self.sha_new
                return FakeProc(0, "{}")
            if argv[:3] == ["gh", "release", "upload"]:
                for tok in argv[4:]:
                    if tok.startswith("--"):
                        break
                    p = Path(tok)
                    uploaded[p.name] = p.read_bytes()
                return FakeProc(0)
            if argv[:3] == ["gh", "release", "download"]:
                pat = argv[argv.index("--pattern") + 1]
                d = Path(argv[argv.index("--dir") + 1])
                if uploaded:
                    # Inject faults only into post-upload readback, not the
                    # existing-release manifest used by the sequence guard.
                    data = uploaded[pat]
                    if d.name.startswith("stage2-readback-"):
                        data = alternate.get(pat, data) + (b"bad" if fault == pat else b"")
                    (d / pat).write_bytes(data)
                else:
                    (d / pat).write_bytes((prev_dir / pat).read_bytes())
                return FakeProc(0)
            return FakeProc(0)

        p = pub_mod.Stage2Publisher(
            repo="owner/repo",
            tag="termux-chatgpt-mcp",
            component="termux-chatgpt-mcp",
            source_sha=self.sha_new,
            dist=self.dist,
            tarball="comp.tar.gz",
            title="Test",
            notes="Notes",
            sequence=3,
            signing_key=self.priv,
            public_key=self.pub,
            runner=fake_gh,
        )
        checksum = self.dist / "comp.tar.gz.sha256"
        valid = checksum.read_bytes()
        checksum.write_bytes(b"bad")
        with self.assertRaisesRegex(pub_mod.Stage2PublicationError, "checksum mismatch"):
            p.publish()
        checksum.write_bytes(valid)
        p.publish()
        self.assertEqual(tag_sha["sha"], self.sha_new)
        self.assertIn("SHA256SUMS.sig", uploaded)
        p.sequence = 4
        fault = "comp.tar.gz.sha256"
        with self.assertRaisesRegex(pub_mod.Stage2PublicationError, "readback asset bytes differ"):
            p.publish()
        fault = None
        p.sequence = 5
        alt = self.root / "alternate"
        alt.mkdir()
        (alt / "comp.tar.gz").write_bytes(b"different valid archive")
        rs_mod.sign_release(component=p.component, sequence=6, commit=self.sha_old,
                            dist=alt, files=[p.tarball], private_key=self.priv)
        alternate.update({name: (alt / name).read_bytes()
                          for name in (p.tarball, "SHA256SUMS", "SHA256SUMS.sig")})
        with self.assertRaisesRegex(pub_mod.Stage2PublicationError, "readback identity mismatch"):
            p.publish()


if __name__ == "__main__":
    unittest.main()
